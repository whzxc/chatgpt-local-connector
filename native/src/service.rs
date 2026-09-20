use crate::control::Control;
use crate::*;
use std::{
    process::Stdio,
    time::{Duration, Instant},
};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Child,
};
pub struct Service {
    pub control: Arc<Control>,
    pub agents: Arc<crate::agents::AgentHost>,
    pub(crate) wait_calls: crate::transport::WaitCalls,
    pub token: String,
    pub port: std::sync::atomic::AtomicU16,
    settings: Mutex<Value>,
    verification: Mutex<Value>,
    logs: Mutex<Vec<String>>,
    tunnel: Mutex<Option<Child>>,
    https: Mutex<Option<tokio::task::JoinHandle<()>>>,
    managed_tunnel: Mutex<Option<crate::https_tunnel::Tunnel>>,
    pub(crate) mcp_url: Mutex<String>,
    starting: std::sync::atomic::AtomicBool,
    run_dir: Mutex<Option<PathBuf>>,
    started: Mutex<Option<Instant>>,
    failure: Mutex<String>,
    operation: Mutex<()>,
    pub connected: std::sync::atomic::AtomicBool,
    stopping: std::sync::atomic::AtomicBool,
    ready_logged: std::sync::atomic::AtomicBool,
}
impl Service {
    pub fn new() -> Result<Arc<Self>> {
        init_crypto();
        private_dir(&root())?;
        let file = root().join("web/settings.json");
        let mut settings = if file.exists() {
            load(&file)?
        } else {
            json!({"tunnelId":"","apiKey":"","tunnelBinary":"tunnel-client","codexBinary":"codex","autoStart":false})
        };
        let provider = if string(&settings, "httpsUrl").is_empty() {
            "cloudflare"
        } else {
            "custom"
        };
        for (key, value) in json!({"cloudflareMode":"quick","cloudflareToken":"","httpsProvider":provider,"ngrokAuthtoken":"","proxyMode":"system","proxyUrl":"","connectionMode":"tunnel","httpsUrl":"","httpsHost":"127.0.0.1","httpsPort":8787}).as_object().unwrap() {
            settings.as_object_mut().ok_or("配置格式错误")?.entry(key.clone()).or_insert(value.clone());
        }
        // Discard the removed, unreleased MCP authentication settings and stored secret.
        let fields = settings.as_object_mut().ok_or("配置格式错误")?;
        let removed_key = fields.remove("httpsApiKey").is_some();
        let removed_auth = fields.remove("httpsRequireAuth").is_some();
        validate_config(&settings)?;
        if removed_key || removed_auth {
            save(&file, &settings)?;
        }
        let binding = binding(&settings);
        let mut verification = load(&root().join("web/chatgpt.json")).unwrap_or(Value::Null);
        if verification["binding"] != binding {
            verification = json!({"binding":binding,"code":id(),"verifiedAt":null});
            save(&root().join("web/chatgpt.json"), &verification)?;
        }
        let binary = crate::desktop::installation()
            .map(|i| i.binary)
            .unwrap_or_else(|| PathBuf::from(string(&settings, "codexBinary")));
        let logs = load(&root().join("web/connection-logs.json"))
            .ok()
            .and_then(|v| v.as_array().cloned())
            .unwrap_or_default()
            .into_iter()
            .filter_map(|v| v.as_str().map(str::to_owned))
            .collect();
        Ok(Arc::new(Self {
            control: Control::new(binary),
            agents: crate::agents::AgentHost::new()?,
            wait_calls: Default::default(),
            token: id() + &id(),
            port: std::sync::atomic::AtomicU16::new(0),
            settings: Mutex::new(settings),
            verification: Mutex::new(verification),
            logs: Mutex::new(logs),
            stopping: std::sync::atomic::AtomicBool::new(false),
            ready_logged: std::sync::atomic::AtomicBool::new(false),
            tunnel: Mutex::new(None),
            https: Mutex::new(None),
            managed_tunnel: Mutex::new(None),
            mcp_url: Mutex::new(String::new()),
            starting: std::sync::atomic::AtomicBool::new(false),
            run_dir: Mutex::new(None),
            started: Mutex::new(None),
            failure: Mutex::new(String::new()),
            operation: Mutex::new(()),
            connected: std::sync::atomic::AtomicBool::new(false),
        }))
    }
    #[cfg(feature = "test-fixture")]
    pub fn fixture(binary: PathBuf) -> Result<Arc<Self>> {
        let mut service = Self::new()?;
        Arc::get_mut(&mut service).unwrap().control = Control::new(binary);
        service
            .connected
            .store(true, std::sync::atomic::Ordering::SeqCst);
        Ok(service)
    }
    pub async fn network_proxy(&self) -> Result<crate::proxy::NetworkProxy> {
        let settings = self.settings.lock().await.clone();
        crate::proxy::NetworkProxy::resolve(&settings).await
    }
    pub async fn log(&self, level: &str, msg: &str) {
        let settings = self.settings.lock().await;
        let mut msg = msg.replace(&self.token, "[REDACTED]");
        for key in ["apiKey", "tunnelId", "ngrokAuthtoken", "cloudflareToken"] {
            let value = string(&settings, key);
            if !value.is_empty() {
                msg = msg.replace(value, "[REDACTED]");
            }
        }
        drop(settings);
        let mut logs = self.logs.lock().await;
        logs.push(json!({"time":now(),"level":level,"msg":msg}).to_string());
        if logs.len() > 200 {
            logs.remove(0);
        }
        let _ = save(&root().join("web/connection-logs.json"), &json!(*logs));
    }
    pub async fn status(&self) -> Result<Value> {
        // Cleanup must not race a start/stop operation; status remains readable during downloads.
        let lifecycle = self.operation.try_lock().ok();
        let mut child = self.tunnel.lock().await;
        let exited = if let Some(c) = child.as_mut() {
            c.try_wait().map_err(|e| e.to_string())?.is_some()
        } else {
            false
        };
        if exited && lifecycle.is_some() {
            child.take();
            self.connected
                .store(false, std::sync::atomic::Ordering::SeqCst);
            self.control.stop_waits();
            *self.failure.lock().await = "连接进程已退出，请重新连接".into();
        }
        let tunnel_running = child.is_some();
        drop(child);
        let mut https = self.https.lock().await;
        if lifecycle.is_some() && https.as_ref().is_some_and(|task| task.is_finished()) {
            https.take();
            self.connected
                .store(false, std::sync::atomic::Ordering::SeqCst);
            self.control.stop_waits();
            *self.failure.lock().await = "MCP 监听已退出，请重新连接".into();
        }
        let https_running = https.is_some();
        drop(https);
        let starting = self.starting.load(std::sync::atomic::Ordering::SeqCst);
        let mut managed = self.managed_tunnel.lock().await;
        if lifecycle.is_some() && !https_running && !starting {
            if let Some(tunnel) = managed.take() {
                tunnel.stop().await;
            }
            self.mcp_url.lock().await.clear();
        }
        let managed_ready = if let Some(tunnel) = managed.as_mut() {
            match tunnel.ready().await {
                Ok(ready) => ready,
                Err(error) if lifecycle.is_some() => {
                    self.control.stop_waits();
                    *self.failure.lock().await = error;
                    self.connected
                        .store(false, std::sync::atomic::Ordering::SeqCst);
                    if let Some(tunnel) = managed.take() {
                        tunnel.stop().await;
                    }
                    if let Some(task) = self.https.lock().await.take() {
                        task.abort();
                    }
                    self.mcp_url.lock().await.clear();
                    false
                }
                Err(_) => false,
            }
        } else {
            true
        };
        drop(managed);
        let running = starting
            || tunnel_running
            || (https_running && self.connected.load(std::sync::atomic::Ordering::SeqCst));
        let mut ready = !starting
            && https_running
            && self.connected.load(std::sync::atomic::Ordering::SeqCst)
            && managed_ready;
        if tunnel_running {
            if let Some(dir) = self.run_dir.lock().await.as_ref() {
                if let Ok(url) = std::fs::read_to_string(dir.join("health.url")) {
                    if let Ok(url) = reqwest::Url::parse(url.trim()) {
                        if url.scheme() == "http" && url.host_str() == Some("127.0.0.1") {
                            if let Ok(r) = reqwest::Client::builder()
                                .no_proxy()
                                .build()
                                .map_err(|e| e.to_string())?
                                .get(url.join("/readyz").map_err(|e| e.to_string())?)
                                .timeout(Duration::from_secs(2))
                                .send()
                                .await
                            {
                                ready = r.status().is_success();
                            }
                        }
                    }
                }
            }
        }
        let error = self.failure.lock().await.clone();
        let state = if starting {
            "starting"
        } else if !running {
            if error.is_empty() {
                "stopped"
            } else {
                "error"
            }
        } else if ready {
            "ready"
        } else if self
            .started
            .lock()
            .await
            .is_some_and(|t| t.elapsed() > Duration::from_secs(30))
        {
            "degraded"
        } else {
            "starting"
        };
        drop(lifecycle);
        let desktop = match crate::desktop::Ipc::open(Default::default(), &self.control.session)
            .await
        {
            Ok(_) => {
                json!({"supported":true,"state":"ready","executionOwner":"desktop","message":"Codex Desktop 已就绪"})
            }
            Err(e) => {
                json!({"supported":crate::desktop::installation().is_some(),"state":if crate::desktop::installation().is_some(){"disconnected"}else{"unavailable"},"executionOwner":"desktop","message":e})
            }
        };
        let mut config = self.settings.lock().await.clone();
        config["configured"] = json!(configured(&config));
        config["hasCloudflareToken"] = json!(!string(&config, "cloudflareToken").is_empty());
        config.as_object_mut().unwrap().remove("cloudflareToken");
        config["hasNgrokAuthtoken"] = json!(!string(&config, "ngrokAuthtoken").is_empty());
        config.as_object_mut().unwrap().remove("ngrokAuthtoken");
        config["hasApiKey"] = json!(!string(&config, "apiKey").is_empty());
        config.as_object_mut().unwrap().remove("apiKey");
        let mcp_url = self.mcp_url.lock().await.clone();
        let v = self.verification.lock().await;
        let chat = json!({"code":v["code"],"verifiedAt":v["verifiedAt"],"challengeVerifiedAt":v["challengeVerifiedAt"]});
        let logs = self.logs.lock().await.clone();
        if ready
            && !self
                .ready_logged
                .swap(true, std::sync::atomic::Ordering::SeqCst)
        {
            self.log("INFO", "本机连接已启动，远程连通请在 ChatGPT 中验证")
                .await;
        }
        let health = self.control.health().await;
        let core = json!({"version":env!("CARGO_PKG_VERSION"),"pid":std::process::id(),"backendSession":self.control.session,"package":{"version":env!("CARGO_PKG_VERSION"),"sha":"native"},"desktop":desktop,"chatgpt":chat,"appServer":{"state":health["appServer"],"observedAt":now(),"evidence":"native-connector","stale":false},"account":{"state":"unknown","observedAt":null},"transport":{"state":state,"error":error},"logs":logs,"activeTurns":0,"activeWrites":health["activeWrites"],"pendingInteractions":self.control.events.lock().await.pending.len(),"liveProcesses":0,"uncertain":false,"draining":false,"lastInbound":null,"toolCount":catalog()["tools"].as_array().unwrap().len(),"registered":running,"schemaDiscovered":"unknown","operationVerified":"business-delivery-not-assessed"});
        Ok(
            json!({"core":core,"connection":{"running":running,"updateAvailable":false,"mcpUrl":mcp_url},"config":config,"tunnel":{"state":state,"error":error},"connector":{"state":state},"logs":logs,"version":env!("CARGO_PKG_VERSION"),"platform":if cfg!(target_os="macos"){"darwin"}else{"win32"},"deviceName":std::env::var("HOSTNAME").unwrap_or_else(|_|"本机".into()),"taskApprovalEnabled":self.control.approval_mode()?,"autoOpenCodex":self.control.auto_open_codex()?,"chatgptUrl":"https://chatgpt.com/plugins"}),
        )
    }
    pub async fn stop(&self) -> Result<()> {
        self.control.stop_waits();
        self.stopping
            .store(true, std::sync::atomic::Ordering::SeqCst);
        self.ready_logged
            .store(false, std::sync::atomic::Ordering::SeqCst);
        let was_connected = self
            .connected
            .swap(false, std::sync::atomic::Ordering::SeqCst);
        if let Some(task) = self.https.lock().await.take() {
            task.abort();
            let _ = task.await;
        }
        if let Some(tunnel) = self.managed_tunnel.lock().await.take() {
            tunnel.stop().await;
        }
        self.mcp_url.lock().await.clear();
        let mut guard = self.tunnel.lock().await;
        if let Some(child) = guard.as_mut() {
            #[cfg(unix)]
            if let Some(pid) = child.id() {
                unsafe {
                    libc::kill(-(pid as i32), libc::SIGTERM);
                }
            }
            #[cfg(windows)]
            if let Some(pid) = child.id() {
                let _ = output("taskkill.exe", &["/PID", &pid.to_string(), "/T", "/F"]).await;
            }
            if tokio::time::timeout(Duration::from_secs(10), child.wait())
                .await
                .is_err()
            {
                #[cfg(unix)]
                if let Some(pid) = child.id() {
                    unsafe {
                        libc::kill(-(pid as i32), libc::SIGKILL);
                    }
                }
                let _ = child.kill().await;
            }
        }
        guard.take();
        drop(guard);
        self.agents.close().await;
        self.control.close().await;
        if let Some(dir) = self.run_dir.lock().await.take() {
            let _ = std::fs::remove_dir_all(dir);
        }
        *self.failure.lock().await = String::new();
        if was_connected {
            self.log("INFO", "连接已关闭").await;
        }
        Ok(())
    }
    async fn start(self: &Arc<Self>) -> Result<()> {
        if self.connected.load(std::sync::atomic::Ordering::SeqCst) {
            return Ok(());
        }
        self.stop().await?;
        if self.control.auto_open_codex()? && crate::desktop::installation().is_some() {
            if crate::desktop::Ipc::open(Default::default(), &self.control.session)
                .await
                .is_err()
            {
                crate::desktop::open_app().await?;
                let deadline = Instant::now() + Duration::from_secs(15);
                loop {
                    if crate::desktop::Ipc::open(Default::default(), &self.control.session)
                        .await
                        .is_ok()
                    {
                        break;
                    }
                    if Instant::now() >= deadline {
                        return Err("请确认 Codex Desktop 已打开".into());
                    }
                    tokio::time::sleep(Duration::from_millis(250)).await;
                }
            }
        }
        if !self.control.auto_open_codex()? {
            self.control.utility().await?;
        }
        let settings = self.settings.lock().await.clone();
        if !configured(&settings) {
            return Err("请先填写连接信息".into());
        }
        if settings["connectionMode"] == "https" {
            let (task, address) = crate::https::listen(self.clone(), &settings).await?;
            *self.https.lock().await = Some(task);
            let url = if settings["httpsProvider"] == "custom" {
                string(&settings, "httpsUrl").to_owned()
            } else {
                self.log("INFO", "正在准备 HTTPS 隧道，首次使用需要下载组件")
                    .await;
                let proxy = crate::proxy::NetworkProxy::resolve(&settings).await?;
                let tunnel =
                    crate::https_tunnel::start(&settings, &format!("http://{address}"), &proxy)
                        .await?;
                let url = tunnel.url.clone();
                *self.managed_tunnel.lock().await = Some(tunnel);
                url
            };
            if self
                .https
                .lock()
                .await
                .as_ref()
                .is_none_or(|task| task.is_finished())
            {
                return Err("MCP 监听未能保持运行，请重新连接".into());
            }
            *self.mcp_url.lock().await = url.clone();
            if settings["httpsProvider"] != "custom" {
                let mut v = self.verification.lock().await;
                if v["endpoint"] != url {
                    *v = json!({"binding":binding(&settings),"endpoint":url,"code":id(),"verifiedAt":null});
                    save(&root().join("web/chatgpt.json"), &v)?;
                }
            }
            *self.started.lock().await = Some(Instant::now());
            self.stopping
                .store(false, std::sync::atomic::Ordering::SeqCst);
            self.connected
                .store(true, std::sync::atomic::Ordering::SeqCst);
            self.log(
                "INFO",
                "MCP 本机监听已开启，公网 HTTPS 连通性需在 ChatGPT 验证",
            )
            .await;
            return Ok(());
        }
        let proxy = crate::proxy::NetworkProxy::resolve(&settings).await?;
        self.log("INFO", proxy.message).await;
        let binary = match executable(string(&settings, "tunnelBinary")) {
            Some(binary) => binary,
            None => {
                self.log("INFO", "正在准备连接组件，首次使用需要下载").await;
                let binary = install_tunnel(&proxy)
                    .await
                    .map_err(|e| format!("准备连接组件失败，请检查网络后重试：{e}"))?;
                let mut config = self.settings.lock().await;
                config["tunnelBinary"] = json!(binary);
                save(&root().join("web/settings.json"), &config)?;
                binary
            }
        };
        let dir = root().join("runs").join(id());
        private_dir(&dir)?;
        let key = dir.join("api-key");
        write_secret(&key, string(&settings, "apiKey"))?;
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let mcp = format!(
            "{} stdio",
            serde_json::to_string(exe.to_str().ok_or("invalid executable")?).unwrap()
        );
        let key_ref = format!("file:{}", key.display());
        let port = self.port.load(std::sync::atomic::Ordering::SeqCst);
        if port == 0 {
            return Err("本机通信尚未启动".into());
        }
        let result = async {
            let mut init = tunnel_command(&binary);
            proxy.apply(&mut init);
            init.args([
                "init",
                "--sample",
                "sample_mcp_stdio_local",
                "--profile",
                "clc",
                "--profile-dir",
                dir.to_str().unwrap(),
                "--control-plane-api-key-ref",
                &key_ref,
                "--tunnel-id",
                string(&settings, "tunnelId"),
                "--health-listen-addr",
                "127.0.0.1:0",
                "--mcp-command",
                &mcp,
            ]);
            let o = tokio::time::timeout(Duration::from_secs(30), init.output())
                .await
                .map_err(|_| "Tunnel 配置超时")?
                .map_err(|e| e.to_string())?;
            if !o.status.success() {
                return Err("生成 Tunnel 配置失败".into());
            }
            let mut cmd = tunnel_command(&binary);
            proxy.apply(&mut cmd);
            cmd.args([
                "run",
                "--profile",
                "clc",
                "--profile-dir",
                dir.to_str().unwrap(),
                "--health.url-file",
                dir.join("health.url").to_str().unwrap(),
            ])
            // The official Tunnel admin bridge otherwise resolves the npm Codex launcher.
            .env(
                "TUNNEL_CLIENT_CODEX_APP_SERVER_CMD",
                crate::desktop::installation()
                    .map(|i| i.binary)
                    .unwrap_or_else(|| PathBuf::from(string(&settings, "codexBinary"))),
            )
            .env("TUNNEL_CLIENT_CODEX_APP_SERVER_ARGS", "app-server")
            .env("CLC_NATIVE_PORT", port.to_string())
            .env("CLC_NATIVE_TOKEN", &self.token)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
            #[cfg(unix)]
            cmd.process_group(0);
            self.stopping
                .store(false, std::sync::atomic::Ordering::SeqCst);
            let mut child = cmd.spawn().map_err(|e| e.to_string())?;
            let stdout = child.stdout.take().unwrap();
            let stderr = child.stderr.take().unwrap();
            for stream in [
                Box::new(stdout) as Box<dyn tokio::io::AsyncRead + Send + Unpin>,
                Box::new(stderr),
            ] {
                let service = Arc::downgrade(self);
                tokio::spawn(async move {
                    let mut lines = BufReader::new(stream).lines();
                    while let Ok(Some(line)) = lines.next_line().await {
                        if line.len() > 65536 {
                            continue;
                        }
                        let Ok(entry) = serde_json::from_str::<Value>(&line) else {
                            continue;
                        };
                        let level = string(&entry, "level").to_uppercase();
                        let msg = string(&entry, "msg");
                        let Some(s) = service.upgrade() else { break };
                        if s.stopping.load(std::sync::atomic::Ordering::SeqCst) {
                            continue;
                        }
                        if matches!(
                            level.as_str(),
                            "WARN" | "WARNING" | "ERROR" | "FATAL" | "PANIC"
                        ) {
                            let detail = entry
                                .get("error")
                                .or_else(|| entry.get("err"))
                                .filter(|v| !v.is_null());
                            s.log(
                                &level,
                                &detail
                                    .map(|v| format!("{msg}: {v}"))
                                    .unwrap_or_else(|| msg.to_owned()),
                            )
                            .await;
                        }
                    }
                });
            }
            *self.tunnel.lock().await = Some(child);
            Ok(())
        }
        .await;
        if let Err(e) = result {
            let _ = std::fs::remove_dir_all(dir);
            return Err(e);
        }
        *self.run_dir.lock().await = Some(dir);
        *self.started.lock().await = Some(Instant::now());
        self.connected
            .store(true, std::sync::atomic::Ordering::SeqCst);
        self.log("INFO", "正在开启连接").await;
        Ok(())
    }
    pub async fn request(
        self: &Arc<Self>,
        route: &str,
        method: &str,
        body: Value,
    ) -> Result<Value> {
        let _guard = if method != "GET" {
            Some(self.operation.lock().await)
        } else {
            None
        };
        match (method, route) {
            ("GET", "task-settings") => Ok(
                json!({"enabled":self.control.approval_mode()?,"autoOpenCodex":self.control.auto_open_codex()?}),
            ),
            ("PUT", "task-settings") => {
                let mut settings = json!({"enabled":self.control.approval_mode()?,"autoOpenCodex":self.control.auto_open_codex()?});
                for key in ["enabled", "autoOpenCodex"] {
                    if let Some(value) = body.get(key) {
                        settings[key] = json!(value.as_bool().ok_or("invalid task setting")?);
                    }
                }
                save(&root().join("task-settings.json"), &settings)?;
                Ok(settings)
            }
            ("POST", "tasks/decision") => {
                self.control
                    .decide(string(&body, "requestId"), string(&body, "action"), "local")
                    .await
            }
            ("GET", "tasks") => Ok(json!({"records":self.control.task_records().await?})),
            ("POST", "tasks/open") => crate::desktop::open_thread(string(&body, "threadId")).await,
            ("GET", route) if route.starts_with("tasks/runtime/") => {
                self.control.task_runtime(&route[14..]).await
            }
            ("GET", "status") => self.status().await,
            ("POST", "verification/reset") => {
                let mut v = self.verification.lock().await;
                let next = json!({"binding":v["binding"],"endpoint":v["endpoint"],"code":id(),"verifiedAt":null});
                save(&root().join("web/chatgpt.json"), &next)?;
                *v = next;
                Ok(json!({"code":v["code"],"verifiedAt":null}))
            }
            ("GET", "core") => Ok(self.status().await?["core"].clone()),
            ("GET", "config/credentials") => {
                let settings = self.settings.lock().await;
                Ok(
                    json!({"apiKey":settings["apiKey"],"ngrokAuthtoken":settings["ngrokAuthtoken"],"cloudflareToken":settings["cloudflareToken"]}),
                )
            }
            ("GET", "service") => Ok(
                json!({"supported":cfg!(target_os="macos")||cfg!(windows),"enabled":autostart_enabled().await}),
            ),
            ("POST", "service") => {
                let enabled = body["enabled"].as_bool().ok_or("invalid enabled")?;
                autostart(enabled).await?;
                let mut s = self.settings.lock().await;
                s["autoStart"] = json!(enabled);
                save(&root().join("web/settings.json"), &s)?;
                Ok(json!({"supported":true,"enabled":enabled}))
            }
            ("PUT", "network") => {
                let mode = body["proxyMode"].as_str().ok_or("代理模式无效")?;
                let address = body["proxyUrl"].as_str().ok_or("代理地址无效")?.trim();
                crate::proxy::validate(mode, address)?;
                let mut settings = self.settings.lock().await;
                let mut next = settings.clone();
                next["proxyMode"] = json!(mode);
                next["proxyUrl"] = json!(address);
                save(&root().join("web/settings.json"), &next)?;
                *settings = next;
                Ok(json!({"ok":true}))
            }
            ("PUT", "config") | ("PATCH", "config/tunnel") => {
                if self.connected.load(std::sync::atomic::Ordering::SeqCst) {
                    return Err("请先关闭连接".into());
                }
                let mut body = if method == "PATCH" {
                    let fields = body.as_object().ok_or("配置格式错误")?;
                    if fields
                        .keys()
                        .any(|k| !["tunnelId", "apiKey"].contains(&k.as_str()))
                    {
                        return Err("仅接受 tunnelId 和 apiKey".into());
                    }
                    let mut next = self.settings.lock().await.clone();
                    next["connectionMode"] = json!("tunnel");
                    for (key, value) in fields {
                        next[key] = value.clone();
                    }
                    next
                } else {
                    body
                };
                validate_config(&body)?;
                if string(&body, "apiKey").is_empty() {
                    body["apiKey"] = self.settings.lock().await["apiKey"].clone();
                }
                if string(&body, "ngrokAuthtoken").is_empty() {
                    body["ngrokAuthtoken"] = self.settings.lock().await["ngrokAuthtoken"].clone();
                }
                if string(&body, "cloudflareToken").is_empty() {
                    body["cloudflareToken"] = self.settings.lock().await["cloudflareToken"].clone();
                }
                if !configured(&body) {
                    return Err("请补齐所选连接方式的信息".into());
                }
                save(&root().join("web/settings.json"), &body)?;
                let binding = binding(&body);
                *self.settings.lock().await = body;
                let mut v = self.verification.lock().await;
                if v["binding"] != binding {
                    *v = json!({"binding":binding,"code":id(),"verifiedAt":null});
                    save(&root().join("web/chatgpt.json"), &v)?;
                }
                Ok(json!({"ok":true}))
            }
            ("POST", "start") => {
                self.starting
                    .store(true, std::sync::atomic::Ordering::SeqCst);
                let result = self.start().await;
                if let Err(e) = &result {
                    let _ = self.stop().await;
                    *self.failure.lock().await = e.clone();
                    self.log("ERROR", e).await;
                }
                self.starting
                    .store(false, std::sync::atomic::Ordering::SeqCst);
                result.map(|_| json!({"ok":true}))
            }
            ("POST", "stop") => {
                self.stop().await?;
                Ok(json!({"ok":true}))
            }
            ("GET", "agents") => Ok(self.agents.inventory().await),
            ("GET", "dependencies") => {
                let s = self.settings.lock().await.clone();
                let describe = |binary: Option<PathBuf>| async move {
                    match binary {
                        None => json!({"source":"missing","path":null,"version":"unknown"}),
                        Some(p) => {
                            let v = output(&p, &["--version"]).await.unwrap_or_default();
                            json!({"source":if p.starts_with(root()){ "managed" }else{"external"},"path":p,"version":v.trim()})
                        }
                    }
                };
                Ok(
                    json!({"codex":describe(crate::desktop::installation().map(|i|i.binary)).await,"tunnel":describe(executable(string(&s,"tunnelBinary"))).await,"observedAt":now()}),
                )
            }
            ("GET", "codex/login") => {
                let binary = crate::desktop::installation()
                    .ok_or("请先安装 Codex Desktop")?
                    .binary;
                let out = command(binary)
                    .args(["login", "status"])
                    .output()
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(json!({"state":if out.status.success(){"authenticated"}else{"unauthenticated"}}))
            }
            ("POST", "codex/login") => {
                crate::desktop::open_app().await?;
                Ok(json!({"state":"unauthenticated","message":"请在 Codex Desktop 中完成登录"}))
            }
            ("POST", "install") => {
                if self.connected.load(std::sync::atomic::Ordering::SeqCst) {
                    return Err("请先关闭连接".into());
                }
                if body["component"] != "tunnel" {
                    return Err("请安装官方 Codex Desktop".into());
                }
                let proxy = self.network_proxy().await?;
                let binary = install_tunnel(&proxy).await?;
                let mut s = self.settings.lock().await;
                s["tunnelBinary"] = json!(binary);
                save(&root().join("web/settings.json"), &s)?;
                Ok(json!({"ok":true}))
            }
            _ => Err("UNKNOWN_ROUTE".into()),
        }
    }
    pub async fn call(self: &Arc<Self>, name: &str, args: Value) -> Result<Value> {
        if !self.connected.load(std::sync::atomic::Ordering::SeqCst) {
            return Err("连接已关闭".into());
        }
        let result = if name == "connector_verify" {
            let mut v = self.verification.lock().await;
            if args["code"] != v["code"] {
                return Err("验证码不匹配".into());
            }
            let mut next = v.clone();
            next["challengeVerifiedAt"] = json!(now());
            next["verifiedAt"] = next["challengeVerifiedAt"].clone();
            save(&root().join("web/chatgpt.json"), &next)?;
            *v = next;
            Ok(json!({"code":args["code"],"received":true}))
        } else if name == "agents" || name.starts_with("agent_") {
            self.agents.tool(&self.control, name, args).await
        } else {
            self.control.tool(name, args).await
        };
        if result.is_ok() && name != "connector_verify" {
            let mut v = self.verification.lock().await;
            if v["verifiedAt"].is_null() {
                v["verifiedAt"] = json!(now());
                save(&root().join("web/chatgpt.json"), &v)?;
            }
        }
        self.log(
            if result.is_ok() { "INFO" } else { "ERROR" },
            &format!(
                "{}：{name}",
                if result.is_ok() {
                    "请求已处理"
                } else {
                    "请求处理失败"
                }
            ),
        )
        .await;
        let (value, error) = match result {
            Ok(v) => (crate::control::page_output(v)?, false),
            Err(e) => (json!({"error":crate::control::failure(&e)}), true),
        };
        Ok(
            json!({"content":[{"type":"text","text":value.to_string()}],"structuredContent":{"result":value},"isError":error}),
        )
    }
}
fn configured(s: &Value) -> bool {
    if s["connectionMode"] == "https" {
        match string(s, "httpsProvider") {
            "cloudflare" => {
                s["cloudflareMode"] == "quick"
                    || (!string(s, "cloudflareToken").is_empty()
                        && !string(s, "httpsUrl").is_empty())
            }
            "ngrok" => !string(s, "ngrokAuthtoken").is_empty(),
            _ => !string(s, "httpsUrl").is_empty(),
        }
    } else {
        !string(s, "tunnelId").is_empty() && !string(s, "apiKey").is_empty()
    }
}
fn binding(s: &Value) -> String {
    if s["connectionMode"] == "https" {
        hash(
            json!([
                "https",
                s["httpsProvider"],
                s["cloudflareMode"],
                s["cloudflareToken"],
                s["ngrokAuthtoken"],
                s["httpsUrl"],
                s["httpsHost"],
                s["httpsPort"]
            ])
            .to_string(),
        )
    } else {
        hash(json!([s["tunnelId"], s["apiKey"]]).to_string())
    }
}
fn validate_config(s: &Value) -> Result<()> {
    let o = s.as_object().ok_or("配置格式错误")?;
    crate::proxy::validate(
        s["proxyMode"].as_str().ok_or("代理模式无效")?,
        s["proxyUrl"].as_str().ok_or("代理地址无效")?,
    )?;
    if o.len() != 15 || !s["autoStart"].is_boolean() {
        return Err("配置字段错误".into());
    }
    for (k, max) in [
        ("connectionMode", 16),
        ("httpsProvider", 16),
        ("cloudflareMode", 16),
        ("cloudflareToken", 4096),
        ("ngrokAuthtoken", 4096),
        ("httpsUrl", 2048),
        ("httpsHost", 128),
        ("tunnelId", 160),
        ("apiKey", 4096),
        ("tunnelBinary", 2048),
        ("codexBinary", 2048),
    ] {
        let v = s[k].as_str().ok_or("配置类型错误")?;
        if v.len() > max || v.contains('\0') {
            return Err("配置内容无效".into());
        }
    }
    if !["tunnel", "https"].contains(&string(s, "connectionMode")) {
        return Err("连接方式无效".into());
    }
    if !["cloudflare", "ngrok", "custom"].contains(&string(s, "httpsProvider")) {
        return Err("HTTPS 服务商无效".into());
    }
    if !["quick", "named"].contains(&string(s, "cloudflareMode")) {
        return Err("Cloudflare 模式无效".into());
    }
    if string(s, "cloudflareToken")
        .chars()
        .any(char::is_whitespace)
    {
        return Err("请粘贴 Tunnel Token，不要粘贴完整安装命令".into());
    }
    if string(s, "ngrokAuthtoken").chars().any(char::is_whitespace) {
        return Err("ngrok Authtoken 不能包含空白字符".into());
    }
    string(s, "httpsHost")
        .parse::<std::net::IpAddr>()
        .map_err(|_| "监听地址必须是 IP 地址")?;
    if !s["httpsPort"]
        .as_u64()
        .is_some_and(|p| (1..=65535).contains(&p))
    {
        return Err("监听端口无效".into());
    }
    let url = string(s, "httpsUrl");
    if !url.is_empty() {
        let u = reqwest::Url::parse(url).map_err(|_| "HTTPS URL 无效")?;
        if u.scheme() != "https"
            || u.host_str().is_none()
            || u.path() != "/mcp"
            || u.query().is_some()
            || u.fragment().is_some()
            || !u.username().is_empty()
            || u.password().is_some()
        {
            return Err("请填写以 https:// 开头、以 /mcp 结尾且不含凭据或查询参数的地址".into());
        }
    }
    let id = string(s, "tunnelId");
    if !id.is_empty()
        && (!id.starts_with("tunnel_")
            || !id
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-'))
    {
        return Err("Tunnel ID 格式错误".into());
    }
    Ok(())
}
pub(crate) fn write_secret(path: &Path, text: &str) -> Result<()> {
    use std::io::Write;
    let mut o = std::fs::OpenOptions::new();
    o.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        o.mode(0o600);
    }
    o.open(path)
        .map_err(|e| e.to_string())?
        .write_all(text.as_bytes())
        .map_err(|e| e.to_string())
}
fn tunnel_command(binary: &Path) -> tokio::process::Command {
    let mut c = command(binary);
    for (k, _) in std::env::vars() {
        if [
            "TUNNEL_CLIENT_",
            "CONTROL_PLANE_",
            "HEALTH_",
            "CLOUDFLARED_",
            "MCP_",
            "HARPOON_",
            "ADMIN_UI_",
            "LOG_",
        ]
        .iter()
        .any(|p| k.starts_with(p))
            || k == "ALLOW_REMOTE_UI"
            || k == "OPEN_WEB_UI"
        {
            c.env_remove(k);
        }
    }
    c.env("CLC_STATE_DIR", root());
    c
}
fn executable(name: &str) -> Option<PathBuf> {
    let p = Path::new(name);
    if p.is_absolute() {
        return p.is_file().then(|| p.into());
    }
    std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
        .map(|p| p.join(name))
        .find(|p| p.is_file())
}
async fn install_tunnel(proxy: &crate::proxy::NetworkProxy) -> Result<PathBuf> {
    let client = proxy
        .client(reqwest::Client::builder())
        .user_agent("chatgpt-local-connector")
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|e| e.to_string())?;
    let release: Value = client
        .get("https://api.github.com/repos/openai/tunnel-client/releases/latest")
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    let tag = string(&release, "tag_name");
    if !tag.starts_with('v')
        || !tag
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c))
    {
        return Err("无效发行版本".into());
    }
    let platform = if cfg!(target_os = "macos") {
        "darwin"
    } else if cfg!(windows) {
        "windows"
    } else {
        return Err("不支持的平台".into());
    };
    let arch = if cfg!(target_arch = "aarch64") {
        "arm64"
    } else {
        "amd64"
    };
    let name = format!("tunnel-client-{tag}-{platform}-{arch}.zip");
    let asset = |name: &str| -> Result<String> {
        let url = release["assets"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|a| a["name"] == name)
            .and_then(|a| a["browser_download_url"].as_str())
            .ok_or("官方资产缺失")?;
        if !url.starts_with("https://github.com/openai/tunnel-client/releases/download/") {
            return Err("非官方资产地址".into());
        }
        Ok(url.into())
    };
    let sums = client
        .get(asset("SHA256SUMS.txt")?)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .text()
        .await
        .map_err(|e| e.to_string())?;
    let expected = sums
        .lines()
        .find_map(|l| {
            let a: Vec<_> = l.split_whitespace().collect();
            (a.len() == 2 && a[1].trim_start_matches('*') == name).then(|| a[0].to_owned())
        })
        .ok_or("官方校验和缺失")?;
    let bytes = client
        .get(asset(&name)?)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .bytes()
        .await
        .map_err(|e| e.to_string())?;
    if hash(&bytes) != expected.to_lowercase() {
        return Err("SHA256 校验失败".into());
    }
    let dir = root().join("dependencies/tunnel").join(tag);
    private_dir(&dir)?;
    let archive = dir.join("download.zip");
    std::fs::write(&archive, &bytes).map_err(|e| e.to_string())?;
    #[cfg(not(windows))]
    output(
        "/usr/bin/unzip",
        &["-o", archive.to_str().unwrap(), "-d", dir.to_str().unwrap()],
    )
    .await?;
    #[cfg(windows)]
    output(
        "powershell.exe",
        &[
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            &format!(
                "Expand-Archive -LiteralPath '{}' -DestinationPath '{}' -Force",
                archive.display().to_string().replace('\'', "''"),
                dir.display().to_string().replace('\'', "''")
            ),
        ],
    )
    .await?;
    let _ = std::fs::remove_file(archive);
    let binary = dir.join(if cfg!(windows) {
        "tunnel-client.exe"
    } else {
        "tunnel-client"
    });
    for name in [
        if cfg!(windows) {
            "tunnel-client.exe"
        } else {
            "tunnel-client"
        },
        if cfg!(windows) {
            "cloudflared.exe"
        } else {
            "cloudflared"
        },
    ] {
        let p = dir.join(name);
        if !p.is_file() {
            return Err("组件不完整".into());
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(p, std::fs::Permissions::from_mode(0o700))
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(binary)
}
async fn autostart_enabled() -> bool {
    let Ok(exe) = std::env::current_exe() else {
        return false;
    };
    #[cfg(target_os = "macos")]
    {
        let Some(home) = dirs::home_dir() else {
            return false;
        };
        let path = home.join("Library/LaunchAgents/com.whzxc.chatgpt-local-connector.plist");
        let path = path.to_string_lossy();
        let program = output(
            "plutil",
            &["-extract", "ProgramArguments.0", "raw", "-o", "-", &path],
        )
        .await;
        let state = output(
            "plutil",
            &[
                "-extract",
                "EnvironmentVariables.CLC_STATE_DIR",
                "raw",
                "-o",
                "-",
                &path,
            ],
        )
        .await;
        return program.is_ok_and(|s| s.trim() == exe.to_string_lossy())
            && state.is_ok_and(|s| s.trim() == root().to_string_lossy());
    }
    #[cfg(windows)]
    {
        return output(
            "schtasks.exe",
            &["/Query", "/TN", "ChatGPT Local Connector", "/XML"],
        )
        .await
        .is_ok_and(|s| {
            s.contains(&xml_escape(&exe.to_string_lossy()))
                && s.contains("--autostart")
                && s.contains(&xml_escape(&root().to_string_lossy()))
        });
    }
    #[allow(unreachable_code)]
    false
}
#[cfg(windows)]
fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
async fn autostart(enabled: bool) -> Result<()> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    #[cfg(target_os = "macos")]
    {
        let path = dirs::home_dir()
            .unwrap()
            .join("Library/LaunchAgents/com.whzxc.chatgpt-local-connector.plist");
        if enabled {
            let xml = |v: &str| {
                v.replace('&', "&amp;")
                    .replace('<', "&lt;")
                    .replace('>', "&gt;")
                    .replace('"', "&quot;")
            };
            private_dir(path.parent().unwrap())?;
            std::fs::write(path,format!("<?xml version=\"1.0\"?><plist version=\"1.0\"><dict><key>Label</key><string>com.whzxc.chatgpt-local-connector</string><key>ProgramArguments</key><array><string>{}</string><string>--autostart</string></array><key>EnvironmentVariables</key><dict><key>CLC_STATE_DIR</key><string>{}</string></dict><key>RunAtLoad</key><true/></dict></plist>",xml(&exe.to_string_lossy()),xml(&root().to_string_lossy()))).map_err(|e|e.to_string())?;
        } else if path.exists() {
            std::fs::remove_file(path).map_err(|e| e.to_string())?;
        }
        return Ok(());
    }
    #[cfg(windows)]
    {
        if enabled {
            output(
                "schtasks.exe",
                &[
                    "/Create",
                    "/TN",
                    "ChatGPT Local Connector",
                    "/SC",
                    "ONLOGON",
                    "/TR",
                    &format!(
                        "\"{}\" --autostart --state-dir \"{}\"",
                        exe.display(),
                        root().display()
                    ),
                    "/F",
                ],
            )
            .await?;
        } else if output(
            "schtasks.exe",
            &["/Query", "/TN", "ChatGPT Local Connector"],
        )
        .await
        .is_ok()
        {
            output(
                "schtasks.exe",
                &["/Delete", "/TN", "ChatGPT Local Connector", "/F"],
            )
            .await?;
        }
        return Ok(());
    }
    #[allow(unreachable_code)]
    Err("不支持开机启动".into())
}
