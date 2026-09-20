use std::ffi::OsString;

/// A per-launch snapshot: never change the GUI process's environment.
pub(crate) struct TunnelProxy {
    env: Vec<(&'static str, OsString)>,
    pub message: &'static str,
}

impl TunnelProxy {
    pub async fn detect() -> Self {
        let mut proxy = Self {
            env: Vec::new(),
            message: "Tunnel 使用继承的代理环境配置",
        };
        // An explicitly empty value also overrides automatic discovery.
        let explicit = [
            "HTTP_PROXY",
            "http_proxy",
            "HTTPS_PROXY",
            "https_proxy",
            "ALL_PROXY",
            "all_proxy",
        ]
        .iter()
        .any(|key| std::env::var_os(key).is_some());
        if !explicit {
            proxy.message = "Tunnel 未配置代理，使用直连";
            #[cfg(target_os = "macos")]
            proxy.detect_macos().await;
        }
        // Tunnel's MCP child and health traffic must stay on this machine.
        let mut bypass = std::env::var_os("NO_PROXY")
            .filter(|value| !value.is_empty())
            .or_else(|| std::env::var_os("no_proxy"))
            .unwrap_or_default();
        if !bypass.is_empty() {
            bypass.push(",");
        }
        bypass.push("localhost,127.0.0.1,::1");
        proxy.env.push(("NO_PROXY", bypass.clone()));
        proxy.env.push(("no_proxy", bypass));
        proxy
    }

    pub fn apply(&self, command: &mut tokio::process::Command) {
        for (key, value) in &self.env {
            command.env(key, value);
        }
    }

    #[cfg(target_os = "macos")]
    async fn detect_macos(&mut self) {
        use std::time::Duration;
        let mut command = crate::command("/usr/sbin/scutil");
        command.arg("--proxy");
        let output = match tokio::time::timeout(Duration::from_secs(3), command.output()).await {
            Ok(Ok(output)) if output.status.success() => output,
            _ => {
                self.message = "无法读取 macOS 系统代理，Tunnel 使用直连；可通过代理环境变量配置";
                return;
            }
        };
        // Only top-level entries describe the active system proxy. Ignore nested
        // scoped/supplemental dictionaries and the ExceptionsList array.
        let text = String::from_utf8_lossy(&output.stdout);
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
            self.message = "Tunnel 不自动解析系统 PAC/WPAD，请通过代理环境变量配置";
            return;
        }
        for (scheme, upper, lower) in [
            ("HTTP", "HTTP_PROXY", "http_proxy"),
            ("HTTPS", "HTTPS_PROXY", "https_proxy"),
        ] {
            if settings.get(format!("{scheme}Enable").as_str()) != Some(&"1") {
                continue;
            }
            let host = settings.get(format!("{scheme}Proxy").as_str());
            let port = settings
                .get(format!("{scheme}Port").as_str())
                .and_then(|port| port.parse::<u16>().ok())
                .filter(|port| *port != 0);
            if let (Some(host), Some(port)) = (host, port) {
                // HTTPSProxy is an HTTP CONNECT proxy, not an HTTPS proxy URL.
                let mut url = reqwest::Url::parse("http://localhost").unwrap();
                if url.set_host(Some(host)).is_ok() && url.set_port(Some(port)).is_ok() {
                    self.env.push((upper, url.as_str().into()));
                    self.env.push((lower, url.as_str().into()));
                }
            }
        }
        if !self.env.is_empty() {
            self.message = "Tunnel 已采用 macOS 系统 HTTP/HTTPS 代理";
        } else if settings.get("SOCKSEnable") == Some(&"1") {
            self.message = "Tunnel 未检测到系统 HTTP/HTTPS 代理，请通过代理环境变量配置 SOCKS 代理";
        }
    }
}
