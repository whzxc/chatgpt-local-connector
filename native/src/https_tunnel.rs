//! User-owned public tunnels. Binaries, credentials and processes stay on this device.
use crate::*;
use std::{
    process::Stdio,
    time::{Duration, Instant},
};
use tokio::process::Child;

pub struct Tunnel {
    child: Child,
    dir: PathBuf,
    health: String,
    provider: String,
    cloudflare_named: bool,
    upstream: String,
    pub url: String,
    pub urls: Vec<String>,
    pub(crate) selected_url: String,
    pub notice: String,
}
impl Drop for Tunnel {
    fn drop(&mut self) {
        let _ = self.child.start_kill();
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}
impl Tunnel {
    pub async fn stop(mut self) {
        let _ = self.child.kill().await;
    }
    pub async fn ready(&mut self) -> Result<bool> {
        if self.child.try_wait().map_err(|e| e.to_string())?.is_some() {
            return Err(format!(
                "{} 隧道进程已退出，请检查凭据、网络、账号并发会话和端点额度后重试",
                self.provider
            ));
        }
        let client = reqwest::Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(2))
            .build()
            .map_err(|e| e.to_string())?;
        let Ok(response) = client.get(&self.health).send().await else {
            return Ok(false);
        };
        if !response.status().is_success() {
            return Ok(false);
        }
        if self.provider == "cloudflare" {
            if self.cloudflare_named {
                let response = client
                    .get(self.health.replace("/ready", "/config"))
                    .send()
                    .await
                    .map_err(|_| "无法读取 Cloudflare 路由配置，请检查 cloudflared 版本")?;
                if !response.status().is_success() {
                    return Err("无法读取 Cloudflare 路由配置，请更新 cloudflared".into());
                }
                let value = response
                    .json::<Value>()
                    .await
                    .map_err(|_| "Cloudflare 路由配置无效")?;
                if value["version"].as_i64().unwrap_or(-1) < 0 {
                    return Ok(false);
                }
                self.urls = matching_urls(&value, &self.upstream);
                self.url = if self.urls.contains(&self.selected_url) {
                    self.selected_url.clone()
                } else if self.urls.len() == 1 {
                    self.urls[0].clone()
                } else {
                    String::new()
                };
                self.notice = if self.urls.is_empty() {
                    format!("未发现指向 {}/mcp 的公网路由，请在 Cloudflare 配置对应 HTTP 服务及明确的域名", self.upstream)
                } else if self.url.is_empty() {
                    "发现多个公网域名，请选择此入口使用的地址".into()
                } else {
                    String::new()
                };
                return Ok(true);
            }
            let Ok(response) = client
                .get(self.health.replace("/ready", "/quicktunnel"))
                .send()
                .await
            else {
                return Ok(false);
            };
            let Ok(value) = response.json::<Value>().await else {
                return Ok(false);
            };
            let hostname = string(&value, "hostname");
            let address = if hostname.starts_with("https://") {
                hostname.to_owned()
            } else {
                format!("https://{hostname}")
            };
            let Some(url) = public_url(&address, true) else {
                return Ok(false);
            };
            if self.url.is_empty() {
                self.url = url.clone();
            }
            return Ok(self.url == url);
        }
        let Ok(value) = response.json::<Value>().await else {
            return Ok(false);
        };
        for endpoint in value["endpoints"].as_array().into_iter().flatten() {
            if string(&endpoint["upstream"], "url").trim_end_matches('/') != self.upstream {
                continue;
            }
            if let Some(url) = public_url(string(endpoint, "url"), false) {
                // A different endpoint requires a fresh connection and ChatGPT configuration.
                if self.url.is_empty() {
                    self.url = url.clone();
                }
                return Ok(self.url == url);
            }
        }
        Ok(false)
    }
}
// Only unambiguous, exact public hosts routed to this ingress's HTTP origin.
fn matching_urls(value: &Value, upstream: &str) -> Vec<String> {
    let mut urls = Vec::new();
    let mut previous: Vec<String> = Vec::new();
    for rule in value["config"]["ingress"].as_array().into_iter().flatten() {
        let host = string(rule, "hostname");
        let path = string(rule, "path");
        let path_matches = if path.is_empty() {
            Some(true)
        } else {
            regex::Regex::new(path)
                .ok()
                .map(|path| path.is_match("/mcp"))
        };
        // Only rules matching /mcp can shadow this endpoint. Unknown expressions
        // remain potential blockers, but cannot establish a discovered route.
        if path_matches == Some(false) {
            continue;
        }
        let shadowed = previous
            .iter()
            .any(|h| h.is_empty() || h.contains('*') || h == host);
        previous.push(host.to_owned());
        if shadowed || host.is_empty() || host.contains('*') || path_matches != Some(true) {
            continue;
        }
        let Ok(origin) = reqwest::Url::parse(string(rule, "service")) else {
            continue;
        };
        let Ok(target) = reqwest::Url::parse(upstream) else {
            continue;
        };
        let local = |host: Option<&str>| matches!(host, Some("localhost" | "127.0.0.1"));
        if origin.scheme() != target.scheme()
            || origin.port_or_known_default() != target.port_or_known_default()
            || !(origin.host_str() == target.host_str()
                || local(origin.host_str()) && local(target.host_str()))
            || origin.path() != "/"
            || origin.query().is_some()
            || origin.fragment().is_some()
            || !origin.username().is_empty()
            || origin.password().is_some()
        {
            continue;
        }
        if let Some(url) = public_url(&format!("https://{host}"), false) {
            if !urls.contains(&url) {
                urls.push(url);
            }
        }
    }
    urls.sort();
    urls
}
fn public_url(text: &str, cloudflare: bool) -> Option<String> {
    let u = reqwest::Url::parse(text).ok()?;
    if u.scheme() != "https"
        || u.host_str().is_none()
        || !u.username().is_empty()
        || u.password().is_some()
        || u.query().is_some()
        || u.fragment().is_some()
        || u.path() != "/"
    {
        return None;
    }
    if cloudflare && !u.host_str()?.ends_with(".trycloudflare.com") {
        return None;
    }
    Some(format!("{}/mcp", u.origin().ascii_serialization()))
}
pub async fn start(
    settings: &Value,
    upstream: &str,
    proxy: &crate::proxy::NetworkProxy,
) -> Result<Tunnel> {
    let provider = string(settings, "httpsProvider");
    let binary = install(provider, proxy).await?;
    let dir = root().join("runs").join(id());
    private_dir(&dir)?;
    let result = spawn(settings, upstream, proxy, &binary, &dir).await;
    if result.is_err() {
        let _ = std::fs::remove_dir_all(&dir);
    }
    result
}
async fn spawn(
    settings: &Value,
    upstream: &str,
    proxy: &crate::proxy::NetworkProxy,
    binary: &Path,
    dir: &Path,
) -> Result<Tunnel> {
    let provider = string(settings, "httpsProvider");
    // Reserve an isolated loopback admin/metrics port; never use the user's existing agent API.
    let reservation = std::net::TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
    let admin = reservation
        .local_addr()
        .map_err(|e| e.to_string())?
        .to_string();
    let config = dir.join("config.yml");
    let mut cmd = command(binary);
    for (name, _) in std::env::vars() {
        if name.starts_with("TUNNEL_")
            || name.starts_with("CLOUDFLARED_")
            || name.starts_with("NGROK_")
        {
            cmd.env_remove(name);
        }
    }
    proxy.apply(&mut cmd);
    let cloudflare_named = provider == "cloudflare" && settings["cloudflareMode"] == "named";
    let health;
    if provider == "cloudflare" {
        save(&config, &json!({}))?;
        cmd.args(["tunnel", "--config"]).arg(&config).args([
            "--no-autoupdate",
            "--protocol",
            "http2",
            "--metrics",
            &admin,
        ]);
        if cloudflare_named {
            let token_file = dir.join("tunnel-token");
            crate::service::write_secret(&token_file, string(settings, "cloudflareToken"))?;
            cmd.args(["run", "--token-file"]).arg(token_file);
        } else {
            cmd.args(["--url", upstream]);
        }
        health = format!("http://{admin}/ready");
    } else {
        let mut agent = json!({"authtoken":settings["ngrokAuthtoken"],"web_addr":admin,
            "console_ui":false,"log_format":"json","log":"stdout","update_check":false,"remote_management":false});
        if let Some(url) = proxy.tunnel_proxy_url() {
            agent["proxy_url"] = json!(url);
        }
        save(&config, &json!({"version":"3","agent":agent}))?;
        cmd.args(["http", upstream, "--config"])
            .arg(&config)
            .arg("--inspect=false");
        if let Ok(url) = reqwest::Url::parse(string(settings, "httpsUrl")) {
            cmd.arg("--url").arg(url.origin().ascii_serialization());
        }
        health = format!("http://{admin}/api/endpoints");
    }
    // Agent logs can include credentials and private requests. Read only the loopback status API.
    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    drop(reservation);
    let child = cmd
        .spawn()
        .map_err(|e| format!("无法启动 {provider}: {e}"))?;
    let mut tunnel = Tunnel {
        child,
        dir: dir.into(),
        health,
        provider: provider.into(),
        cloudflare_named,
        upstream: upstream.into(),
        url: String::new(),
        urls: Vec::new(),
        selected_url: string(settings, "httpsUrl").to_owned(),
        notice: String::new(),
    };
    let deadline = Instant::now() + Duration::from_secs(90);
    loop {
        if tunnel.ready().await? {
            return Ok(tunnel);
        }
        if Instant::now() >= deadline {
            return Err(format!(
                "{provider} 连接超时，请检查网络{}",
                if provider == "ngrok" {
                    "、Authtoken 和账号的隧道额度"
                } else {
                    "；Cloudflare 需允许出站连接到 7844 端口"
                }
            ));
        }
        tokio::time::sleep(Duration::from_millis(300)).await;
    }
}
async fn install(provider: &str, proxy: &crate::proxy::NetworkProxy) -> Result<PathBuf> {
    static INSTALL: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
    let _guard = INSTALL.lock().await;
    let platform = if cfg!(target_os = "macos") {
        "darwin"
    } else if cfg!(windows) {
        "windows"
    } else {
        return Err("仅支持 macOS 和 Windows".into());
    };
    let arch = if cfg!(target_arch = "aarch64") {
        "arm64"
    } else if cfg!(target_arch = "x86_64") {
        "amd64"
    } else {
        return Err("不支持此 CPU 架构".into());
    };
    let name = if provider == "cloudflare" {
        "cloudflared"
    } else {
        "ngrok"
    };
    let executable = format!("{name}{}", if cfg!(windows) { ".exe" } else { "" });
    let dir = root()
        .join("dependencies")
        .join(provider)
        .join(format!("{platform}-{arch}"));
    private_dir(&dir)?;
    let binary = dir.join(&executable);
    let manifest = dir.join("installed.json");
    if let (Ok(bytes), Ok(meta)) = (std::fs::read(&binary), load(&manifest)) {
        if meta["sha256"] == hash(bytes) {
            return Ok(binary);
        }
    }
    let client = proxy
        .client(reqwest::Client::builder())
        .user_agent("chatgpt-local-connector")
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|e| e.to_string())?;
    let (url, digest, extension) = if provider == "cloudflare" {
        let release: Value = client
            .get("https://api.github.com/repos/cloudflare/cloudflared/releases/latest")
            .send()
            .await
            .map_err(|e| e.to_string())?
            .error_for_status()
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())?;
        let extension = if cfg!(windows) { "exe" } else { "tgz" };
        let asset_name = format!("cloudflared-{platform}-{arch}.{extension}");
        let asset = release["assets"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|a| a["name"] == asset_name)
            .ok_or("Cloudflare 未提供此平台的下载")?;
        let url = string(asset, "browser_download_url");
        if !url.starts_with("https://github.com/cloudflare/cloudflared/releases/download/") {
            return Err("无效的 Cloudflare 下载地址".into());
        }
        let digest = string(asset, "digest")
            .strip_prefix("sha256:")
            .ok_or("Cloudflare 官方校验和缺失")?;
        (url.to_owned(), Some(digest.to_owned()), extension)
    } else {
        (
            format!("https://bin.ngrok.com/c/bNyj1mQVY4c/ngrok-v3-stable-{platform}-{arch}.zip"),
            None,
            "zip",
        )
    };
    let mut response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?;
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
        if bytes.len() + chunk.len() > 128 * 1024 * 1024 {
            return Err("隧道组件下载过大".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    if digest.is_some_and(|digest| hash(&bytes) != digest) {
        return Err("隧道组件 SHA256 校验失败".into());
    }
    let staging = dir.join(id());
    private_dir(&staging)?;
    let result = async {
        let archive = staging.join(format!("download.{extension}"));
        std::fs::write(&archive, &bytes).map_err(|e| e.to_string())?;
        let extracted = staging.join(&executable);
        if extension == "exe" {
            std::fs::rename(&archive, &extracted).map_err(|e| e.to_string())?;
        } else {
            // Extract only the expected executable from the official archive.
            output(
                if cfg!(windows) {
                    "tar.exe"
                } else {
                    "/usr/bin/tar"
                },
                &[
                    "-xf",
                    archive.to_str().ok_or("invalid path")?,
                    "-C",
                    staging.to_str().ok_or("invalid path")?,
                    &executable,
                ],
            )
            .await?;
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&extracted, std::fs::Permissions::from_mode(0o700))
                .map_err(|e| e.to_string())?;
        }
        let digest = hash(std::fs::read(&extracted).map_err(|e| e.to_string())?);
        if binary.exists() {
            std::fs::remove_file(&binary).map_err(|e| e.to_string())?;
        }
        std::fs::rename(&extracted, &binary).map_err(|e| e.to_string())?;
        save(&manifest, &json!({"sha256":digest}))?;
        Ok(binary)
    }
    .await;
    let _ = std::fs::remove_dir_all(staging);
    result
}
