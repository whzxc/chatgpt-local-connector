use crate::{string, Result};
use reqwest::{ClientBuilder, Url};
use serde_json::Value;
use std::time::Duration;

const PROXY_KEYS: [&str; 6] = [
    "HTTP_PROXY",
    "http_proxy",
    "HTTPS_PROXY",
    "https_proxy",
    "ALL_PROXY",
    "all_proxy",
];

/// One snapshot per operation, shared by downloads and Tunnel children.
#[derive(Clone)]
pub struct NetworkProxy {
    http: Option<Url>,
    https: Option<Url>,
    bypass: String,
    pub message: &'static str,
}

pub fn validate(mode: &str, address: &str) -> Result<()> {
    if !["system", "direct", "custom"].contains(&mode) {
        return Err("代理模式无效".into());
    }
    if mode == "custom" {
        parse_url(address)?;
    } else if !address.is_empty() {
        return Err("仅自定义代理可设置地址".into());
    }
    Ok(())
}

fn parse_url(address: &str) -> Result<Url> {
    let error = "请输入 HTTP/HTTPS 代理地址，例如 http://127.0.0.1:7890；不支持地址内嵌账号密码";
    if address.len() > 2048 {
        return Err(error.into());
    }
    let url = Url::parse(address).map_err(|_| error)?;
    if !["http", "https"].contains(&url.scheme())
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || url.path() != "/"
        || url.port() == Some(0)
    {
        return Err(error.into());
    }
    Ok(url)
}

impl NetworkProxy {
    pub async fn resolve(settings: &Value) -> Result<Self> {
        let mode = string(settings, "proxyMode");
        let address = string(settings, "proxyUrl");
        validate(mode, address)?;
        let mut bypass = ["NO_PROXY", "no_proxy"]
            .iter()
            .filter_map(|key| std::env::var(key).ok())
            .filter(|v| !v.is_empty())
            .collect::<Vec<_>>();
        bypass.push("localhost,127.0.0.1,::1".into());
        let mut proxy = Self {
            http: None,
            https: None,
            bypass: bypass.join(","),
            message: "不使用代理，直接连接",
        };
        match mode {
            "custom" => {
                let url = parse_url(address)?;
                proxy.http = Some(url.clone());
                proxy.https = Some(url);
                proxy.message = "使用自定义代理";
            }
            "system" => {
                proxy.message = "系统未设置 HTTP/HTTPS 代理，直接连接";
                #[cfg(target_os = "macos")]
                proxy.detect_macos().await?;
                #[cfg(windows)]
                proxy.detect_windows().await?;
                if proxy.http.is_some() || proxy.https.is_some() {
                    proxy.message = "使用系统 HTTP/HTTPS 代理";
                }
            }
            _ => {}
        }
        Ok(proxy)
    }

    pub fn apply(&self, command: &mut tokio::process::Command) {
        // The explicit UI choice must not be overridden by the launching shell.
        for key in PROXY_KEYS {
            command.env_remove(key);
        }
        for (keys, url) in [
            (["HTTP_PROXY", "http_proxy"], &self.http),
            (["HTTPS_PROXY", "https_proxy"], &self.https),
        ] {
            if let Some(url) = url {
                for key in keys {
                    command.env(key, url.as_str());
                }
            }
        }
        command
            .env("NO_PROXY", &self.bypass)
            .env("no_proxy", &self.bypass);
    }

    pub fn client(&self, builder: ClientBuilder) -> ClientBuilder {
        let mut builder = builder.no_proxy();
        for (https, url) in [(false, &self.http), (true, &self.https)] {
            if let Some(url) = url {
                let proxy = if https {
                    reqwest::Proxy::https(url.as_str())
                } else {
                    reqwest::Proxy::http(url.as_str())
                }
                .expect("validated proxy URL")
                .no_proxy(reqwest::NoProxy::from_string(&self.bypass));
                builder = builder.proxy(proxy);
            }
        }
        builder
    }

    #[cfg(any(target_os = "macos", windows))]
    async fn output(mut command: tokio::process::Command) -> Result<String> {
        let output = tokio::time::timeout(Duration::from_secs(3), command.output())
            .await
            .map_err(|_| "读取系统代理超时，请选择自定义代理或不使用代理")?
            .map_err(|_| "无法读取系统代理，请选择自定义代理或不使用代理")?;
        if !output.status.success() {
            return Err("无法读取系统代理，请选择自定义代理或不使用代理".into());
        }
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }

    #[cfg(target_os = "macos")]
    async fn detect_macos(&mut self) -> Result<()> {
        let mut command = crate::command("/usr/sbin/scutil");
        command.arg("--proxy");
        let text = Self::output(command).await?;
        let mut depth = 0usize;
        let mut settings = std::collections::HashMap::new();
        for line in text.lines().map(str::trim) {
            if line.ends_with('{') {
                depth += 1;
            } else if line == "}" {
                depth = depth.saturating_sub(1);
            } else if depth == 1 {
                if let Some((key, value)) = line.split_once(" : ") {
                    settings.insert(key, value);
                }
            }
        }
        if ["ProxyAutoConfigEnable", "ProxyAutoDiscoveryEnable"]
            .iter()
            .any(|key| settings.get(key) == Some(&"1"))
        {
            return Err("暂不支持系统 PAC/WPAD，请选择自定义代理或不使用代理".into());
        }
        for scheme in ["HTTP", "HTTPS"] {
            if settings.get(format!("{scheme}Enable").as_str()) != Some(&"1") {
                continue;
            }
            let host = settings
                .get(format!("{scheme}Proxy").as_str())
                .ok_or("系统代理缺少地址")?;
            let port = settings
                .get(format!("{scheme}Port").as_str())
                .and_then(|p| p.parse::<u16>().ok())
                .filter(|p| *p > 0)
                .ok_or("系统代理端口无效")?;
            let mut url = Url::parse("http://localhost").unwrap();
            if let Ok(ip) = host.parse::<std::net::IpAddr>() {
                url.set_ip_host(ip).map_err(|_| "系统代理 IP 无效")?;
            } else {
                url.set_host(Some(host)).map_err(|_| "系统代理地址无效")?;
            }
            url.set_port(Some(port)).map_err(|_| "系统代理端口无效")?;
            if scheme == "HTTP" {
                self.http = Some(url);
            } else {
                self.https = Some(url);
            }
        }
        if self.http.is_none() && self.https.is_none() && settings.get("SOCKSEnable") == Some(&"1")
        {
            return Err("暂不支持仅 SOCKS 的系统代理，请指定 HTTP/HTTPS 代理".into());
        }
        Ok(())
    }

    #[cfg(windows)]
    async fn detect_windows(&mut self) -> Result<()> {
        let mut command = crate::command("powershell.exe");
        command.args(["-NoProfile", "-NonInteractive", "-Command", r#"
$ErrorActionPreference='Stop'
[Console]::OutputEncoding=[System.Text.Encoding]::UTF8
$p=Get-ItemProperty 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Internet Settings'
$c=Get-ItemProperty 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Internet Settings\Connections' -ErrorAction SilentlyContinue
$f=$c.DefaultConnectionSettings
@{ enabled=($p.ProxyEnable -eq 1); server=[string]$p.ProxyServer; auto=([bool]$p.AutoConfigURL -or ($f.Length -gt 8 -and ($f[8] -band 12) -ne 0)) } | ConvertTo-Json -Compress
"#]);
        let text = Self::output(command).await?;
        let settings: Value = serde_json::from_str(text.trim_start_matches('\u{feff}'))
            .map_err(|_| "无法解析 Windows 系统代理")?;
        if settings["auto"] == true {
            return Err("暂不支持系统 PAC/WPAD，请选择自定义代理或不使用代理".into());
        }
        if settings["enabled"] != true {
            return Ok(());
        }
        let server = string(&settings, "server");
        for item in server.split(';').map(str::trim).filter(|v| !v.is_empty()) {
            let (scheme, address) = item.split_once('=').unwrap_or(("all", item));
            if !["all", "http", "https"].contains(&scheme) {
                continue;
            }
            let address = if address.contains("://") {
                address.to_owned()
            } else {
                format!("http://{address}")
            };
            let url = parse_url(&address)?;
            if scheme != "https" {
                self.http = Some(url.clone());
            }
            if scheme != "http" {
                self.https = Some(url);
            }
        }
        if self.http.is_none() && self.https.is_none() {
            return Err("系统未提供可用的 HTTP/HTTPS 代理，请设置自定义代理".into());
        }
        Ok(())
    }
}
