//! One execution core, with independently owned ingress runtimes.
pub(crate) use crate::ingress::write_secret;
use crate::{control::Control, ingress::Ingress, *};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU16, Ordering};

pub struct Service {
    pub control: Arc<Control>,
    pub agents: Arc<crate::agents::AgentHost>,
    pub token: String,
    pub port: AtomicU16,
    ingresses: Mutex<BTreeMap<String, Arc<Ingress>>>,
    drafts: Mutex<BTreeMap<String, (Arc<Ingress>, std::time::Instant, tokio::task::AbortHandle)>>,
    configuration: Mutex<()>,
    closing: std::sync::atomic::AtomicBool,
}
const SECRETS: &[&str] = &["apiKey", "cloudflareToken", "ngrokAuthtoken"];
pub(crate) fn defaults() -> Value {
    json!({"tunnelId":"","apiKey":"","tunnelBinary":"tunnel-client","codexBinary":"codex","autoStart":false,"cloudflareMode":"quick","cloudflareToken":"","httpsProvider":"cloudflare","ngrokAuthtoken":"","proxyMode":"system","proxyUrl":"","connectionMode":"tunnel","httpsUrl":"","httpsHost":"127.0.0.1","httpsPort":8787})
}
// Secrets use the existing private-directory/atomic-file storage, separately from configuration.
pub(crate) fn store_entry(entry: &Value) -> Result<()> {
    let dir = root().join("ingresses").join(string(entry, "id"));
    private_dir(&dir)?;
    let mut public = entry.clone();
    public["config"]
        .as_object_mut()
        .unwrap()
        .remove("connectionMode");
    let mut secret = json!({"bearerToken":public.as_object_mut().unwrap().remove("bearerToken").unwrap_or(Value::Null)});
    for key in SECRETS {
        secret[*key] = public["config"]
            .as_object_mut()
            .unwrap()
            .remove(*key)
            .unwrap_or(Value::Null);
    }
    save(&dir.join("secrets.json"), &secret)?;
    save(&dir.join("config.json"), &public)
}
fn read_entry(id: &str) -> Result<Value> {
    valid_id(id)?;
    let dir = root().join("ingresses").join(id);
    let mut entry = load(&dir.join("config.json"))?;
    if entry["id"] != id {
        return Err("ingress id mismatch".into());
    }
    let secret = load(&dir.join("secrets.json"))?;
    entry["bearerToken"] = secret["bearerToken"].clone();
    for key in SECRETS {
        entry["config"][*key] = secret[*key].clone();
    }
    Ok(entry)
}
fn valid_id(id: &str) -> Result<()> {
    if id.is_empty()
        || id.len() > 64
        || !id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
    {
        return Err("id must contain 1-64 ASCII letters, digits, - or _".into());
    }
    Ok(())
}
fn normalize(mut entry: Value) -> Result<Value> {
    if entry
        .as_object()
        .ok_or("expected ingress object")?
        .keys()
        .any(|k| {
            ![
                "id",
                "name",
                "controlSource",
                "transport",
                "auth",
                "bearerToken",
                "enabled",
                "toolPolicy",
                "config",
            ]
            .contains(&k.as_str())
        })
    {
        return Err("unknown ingress field; see cli help".into());
    }
    valid_id(string(&entry, "id"))?;
    for field in ["name", "controlSource"] {
        let text = entry[field]
            .as_str()
            .ok_or("name and controlSource are required")?;
        if text.is_empty() || text.len() > 120 || text.chars().any(char::is_control) {
            return Err("invalid ingress label".into());
        }
    }
    if !entry["enabled"].is_boolean() {
        return Err("enabled must be boolean".into());
    }
    if !["openai-tunnel", "https"].contains(&string(&entry, "transport")) {
        return Err("transport must be openai-tunnel or https".into());
    }
    if entry["transport"] == "openai-tunnel" {
        if entry["auth"] != "openai" {
            return Err("OpenAI Tunnel requires auth=openai".into());
        }
    } else if !["none", "bearer"].contains(&string(&entry, "auth")) {
        return Err("HTTPS auth must be none or bearer".into());
    }
    if entry["toolPolicy"] != "all" {
        let allowed = entry["toolPolicy"]["allowlist"]
            .as_array()
            .ok_or("toolPolicy must be all or {allowlist:[tool names]}")?;
        for name in allowed {
            if !catalog()["tools"]
                .as_array()
                .unwrap()
                .iter()
                .any(|t| t["name"] == *name)
            {
                return Err("unknown tool in allowlist".into());
            }
        }
    }
    let mut config = defaults();
    if let Ok(prefs) = load(&root().join("web/preferences.json")) {
        for key in ["proxyMode", "proxyUrl"] {
            if let Some(v) = prefs.get(key) {
                config[key] = v.clone();
            }
        }
    }
    for (k, v) in entry["config"]
        .as_object()
        .ok_or("config must be an object")?
    {
        config[k] = v.clone();
    }
    config["connectionMode"] = json!(if entry["transport"] == "https" {
        "https"
    } else {
        "tunnel"
    });
    crate::ingress::validate_config(&config)?;
    entry["config"] = config;
    if entry["auth"] == "bearer" {
        let token = string(&entry, "bearerToken");
        if token.len() < 32 || token.len() > 4096 || !token.bytes().all(|b| b.is_ascii_graphic()) {
            return Err(
                "bearerToken requires 32-4096 printable ASCII characters; supply via stdin".into(),
            );
        }
    }
    Ok(entry)
}
impl Service {
    pub fn new() -> Result<Arc<Self>> {
        init_crypto();
        private_dir(&root())?;
        let index = root().join("ingresses/index.json");
        if !index.exists() {
            let old = root().join("web/settings.json");
            let legacy = if old.exists() { load(&old)? } else { json!({}) };
            let mut config = defaults();
            for (k, v) in legacy.as_object().ok_or("invalid legacy config")? {
                if config.get(k).is_some() {
                    config[k] = v.clone();
                }
            }
            if legacy.get("httpsProvider").is_none() && !string(&legacy, "httpsUrl").is_empty() {
                config["httpsProvider"] = json!("custom");
            }
            let https = config["connectionMode"] == "https";
            let entry = normalize(
                json!({"id":"default","name":"ChatGPT","controlSource":"chatgpt","transport":if https {"https"}else{"openai-tunnel"},"enabled":true,"auth":if https {"none"}else{"openai"},"toolPolicy":"all","config":config}),
            )?;
            store_entry(&entry)?;
            save(
                &root().join("web/preferences.json"),
                &json!({"proxyMode":entry["config"]["proxyMode"],"proxyUrl":entry["config"]["proxyUrl"]}),
            )?;
            if let Ok(v) = load(&root().join("web/chatgpt.json")) {
                save(&root().join("ingresses/default/verification.json"), &v)?;
            }
            save(&index, &json!(["default"]))?;
            for path in [
                old,
                root().join("web/chatgpt.json"),
                root().join("web/connection-logs.json"),
            ] {
                if path.exists() {
                    std::fs::remove_file(path).map_err(|e| e.to_string())?;
                }
            }
        }
        let control = Control::new(
            crate::desktop::installation()
                .map(|i| i.binary)
                .unwrap_or_else(|| PathBuf::from("codex")),
        );
        let agents = crate::agents::AgentHost::new()?;
        let token = id() + &id();
        let mut ingresses = BTreeMap::new();
        for id in load(&index)?.as_array().ok_or("invalid ingress index")? {
            let entry = normalize(read_entry(id.as_str().ok_or("invalid ingress id")?)?)?;
            let ingress = Ingress::new(entry, control.clone(), agents.clone(), token.clone())?;
            ingresses.insert(ingress.id.clone(), ingress);
        }
        Ok(Arc::new(Self {
            control,
            agents,
            token,
            port: AtomicU16::new(0),
            ingresses: Mutex::new(ingresses),
            drafts: Mutex::new(BTreeMap::new()),
            configuration: Mutex::new(()),
            closing: std::sync::atomic::AtomicBool::new(false),
        }))
    }
    #[cfg(feature = "test-fixture")]
    pub fn fixture(binary: PathBuf) -> Result<Arc<Self>> {
        let mut service = Self::new()?;
        let service_mut = Arc::get_mut(&mut service).unwrap();
        service_mut.control = Control::new(binary);
        for ingress in service_mut.ingresses.get_mut().values_mut() {
            let i = Arc::get_mut(ingress).unwrap();
            i.control = service_mut.control.clone();
            i.connected.store(true, Ordering::SeqCst);
        }
        Ok(service)
    }
    pub async fn ingress(&self, id: &str) -> Result<Arc<Ingress>> {
        self.ingresses
            .lock()
            .await
            .get(id)
            .cloned()
            .ok_or("ingress not found".into())
    }
    pub async fn primary(&self) -> Result<Arc<Ingress>> {
        let items = self.ingresses.lock().await;
        items
            .get("default")
            .or_else(|| items.values().next())
            .cloned()
            .ok_or("no ingresses; run cli ingress add --stdin".into())
    }
    async fn entries(&self) -> Vec<Arc<Ingress>> {
        self.ingresses.lock().await.values().cloned().collect()
    }
    pub async fn network_proxy(&self) -> Result<crate::proxy::NetworkProxy> {
        crate::proxy::NetworkProxy::resolve(
            &load(&root().join("web/preferences.json")).unwrap_or_else(|_| defaults()),
        )
        .await
    }
    pub async fn log(&self, level: &str, msg: &str) {
        if let Ok(i) = self.primary().await {
            i.log(level, msg).await;
        }
    }
    pub async fn status(&self) -> Result<Value> {
        let mut status = if let Ok(primary) = self.primary().await {
            primary.status().await?
        } else {
            let mut config = defaults();
            config["configured"] = json!(false);
            for key in SECRETS {
                config.as_object_mut().unwrap().remove(*key);
            }
            json!({"config":config,"core":{"desktop":{"state":"unknown"},"appServer":{"state":"unknown"},"chatgpt":{},"logs":[]},"connection":{"running":false,"mcpUrl":""},"connector":{"state":"stopped"},"tunnel":{"state":"stopped","error":""},"logs":[],"version":env!("CARGO_PKG_VERSION"),"platform":if cfg!(target_os="macos"){"darwin"}else{"win32"},"taskApprovalEnabled":self.control.approval_mode()?,"autoOpenCodex":self.control.auto_open_codex()?})
        };
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
        let health = self.control.health().await;
        for (key,value) in json!({"pid":std::process::id(),"activeTurns":0,"liveProcesses":0,"uncertain":false,"draining":false,"account":{"state":"unknown","observedAt":null},"lastInbound":null,"schemaDiscovered":"unknown","operationVerified":"business-delivery-not-assessed"}).as_object().unwrap() {status["core"][key]=value.clone();}
        status["core"]["desktop"] = desktop;
        status["core"]["appServer"] = json!({"state":health["appServer"],"observedAt":now(),"evidence":"native-connector","stale":false});
        status["core"]["version"] = json!(env!("CARGO_PKG_VERSION"));
        status["core"]["backendSession"] = json!(self.control.session);
        status["core"]["activeWrites"] = health["activeWrites"].clone();
        status["core"]["pendingInteractions"] =
            json!(self.control.events.lock().await.pending.len());
        status["core"]["toolCount"] = json!(catalog()["tools"].as_array().unwrap().len());
        status["core"]["package"] = json!({"version":env!("CARGO_PKG_VERSION"),"sha":"native"});
        let mut items = Vec::new();
        for i in self.entries().await {
            items.push(i.summary().await?);
        }
        let running = items.iter().filter(|i| i["running"] == true).count();
        let ready = items.iter().filter(|i| i["state"] == "ready").count();
        status["ingresses"] = json!(items);
        status["ingressSummary"] = json!({"running":running,"ready":ready,"total":items.len()});
        status["core"]["transport"] =
            json!({"state":if ready>0{"ready"}else if running>0{"starting"}else{"stopped"}});
        status["connector"] = status["core"]["transport"].clone();
        let mut logs: Vec<Value> = items
            .iter()
            .flat_map(|i| {
                i["logs"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|line| serde_json::from_str::<Value>(line.as_str()?).ok())
                    .map(|mut log| {
                        log["ingressId"] = i["id"].clone();
                        log
                    })
            })
            .collect();
        logs.sort_by(|a, b| string(a, "time").cmp(string(b, "time")));
        let logs: Vec<_> = logs
            .into_iter()
            .rev()
            .take(200)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .map(|v| v.to_string())
            .collect();
        status["logs"] = json!(logs);
        status["core"]["logs"] = status["logs"].clone();
        status["core"]["registered"] = json!(running > 0);
        Ok(status)
    }
    pub async fn stop(&self) -> Result<()> {
        let _configuration = self.configuration.lock().await;
        self.closing.store(true, Ordering::SeqCst);
        let entries = self.entries().await;
        for ingress in &entries {
            ingress.retired.store(true, Ordering::SeqCst);
        }
        let mut errors = Vec::new();
        for ingress in entries {
            let _runtime = ingress.operation.lock().await;
            if let Err(error) = ingress.stop().await {
                errors.push(error);
            }
        }
        let drafts = std::mem::take(&mut *self.drafts.lock().await);
        for (id, (draft, _, task)) in drafts {
            task.abort();
            let _runtime = draft.operation.lock().await;
            draft.retired.store(true, Ordering::SeqCst);
            let _ = draft.stop().await;
            let _ = std::fs::remove_dir_all(root().join("ingresses").join(id));
        }
        self.agents.close().await;
        self.control.close().await;
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join("; "))
        }
    }
    async fn all(&self, action: &str) -> Result<Value> {
        let mut jobs = tokio::task::JoinSet::new();
        for ingress in self.entries().await {
            if action == "start" && ingress.meta.lock().await["enabled"] != true {
                continue;
            }
            ingress
                .port
                .store(self.port.load(Ordering::SeqCst), Ordering::SeqCst);
            let action = action.to_owned();
            jobs.spawn(async move {
                let result = ingress.request(&action, "POST", json!({})).await;
                json!({"id":ingress.id,"ok":result.is_ok(),"error":result.err()})
            });
        }
        let mut results = Vec::new();
        while let Some(result) = jobs.join_next().await {
            results.push(result.map_err(|e| e.to_string())?);
        }
        results.sort_by(|a, b| string(a, "id").cmp(string(b, "id")));
        Ok(json!({"results":results}))
    }
    async fn discard_draft(&self, id: &str) -> Result<()> {
        if let Some((draft, _, task)) = self.drafts.lock().await.remove(id) {
            task.abort();
            let _runtime = draft.operation.lock().await;
            draft.retired.store(true, Ordering::SeqCst);
            draft.stop().await?;
            let _ = std::fs::remove_dir_all(root().join("ingresses").join(id));
        }
        Ok(())
    }
    pub async fn request(
        self: &Arc<Self>,
        route: &str,
        method: &str,
        body: Value,
    ) -> Result<Value> {
        if self.closing.load(Ordering::SeqCst) {
            return Err("core shutting down".into());
        }
        if route == "ingress-drafts" && method == "POST" {
            let _guard = self.configuration.lock().await;
            if self.closing.load(Ordering::SeqCst) {
                return Err("core shutting down".into());
            }
            if self.drafts.lock().await.len() >= 8 {
                return Err("too many pending connections".into());
            }
            let mut config = body["config"].clone();
            if !config.is_object()
                || !["cloudflare", "ngrok"].contains(&string(&config, "httpsProvider"))
            {
                return Err("managed HTTPS provider required".into());
            }
            let listener = std::net::TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
            config["httpsHost"] = json!("127.0.0.1");
            config["httpsPort"] = json!(listener.local_addr().map_err(|e| e.to_string())?.port());
            config["httpsUrl"] = json!("");
            drop(listener);
            let entry = normalize(
                json!({"id":format!("draft-{}",id()),"name":"Pending connection","controlSource":"custom","transport":"https","enabled":true,"auth":"bearer","bearerToken":id()+&id(),"toolPolicy":{"allowlist":[]},"config":config}),
            )?;
            let draft = Ingress::new(
                entry,
                self.control.clone(),
                self.agents.clone(),
                self.token.clone(),
            )?;
            let draft_id = draft.id.clone();
            draft
                .port
                .store(self.port.load(Ordering::SeqCst), Ordering::SeqCst);
            let starter = draft.clone();
            let task = tokio::spawn(async move {
                let _ = starter.request("start", "POST", json!({})).await;
            });
            self.drafts.lock().await.insert(
                draft_id.clone(),
                (
                    draft.clone(),
                    std::time::Instant::now(),
                    task.abort_handle(),
                ),
            );
            let owner = Arc::downgrade(self);
            let expires_id = draft_id.clone();
            tokio::spawn(async move {
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                    let Some(owner) = owner.upgrade() else { break };
                    let _guard = owner.configuration.lock().await;
                    let expired = owner
                        .drafts
                        .lock()
                        .await
                        .get(&expires_id)
                        .map(|(_, last, _)| last.elapsed().as_secs() >= 120);
                    match expired {
                        None => break,
                        Some(true) => {
                            let _ = owner.discard_draft(&expires_id).await;
                            break;
                        }
                        _ => {}
                    }
                }
            });
            return draft.summary().await;
        }
        if let Some(path) = route.strip_prefix("ingress-drafts/") {
            let (id, action) = path.split_once('/').unwrap_or((path, ""));
            valid_id(id)?;
            let _guard = self.configuration.lock().await;
            if self.closing.load(Ordering::SeqCst) {
                return Err("core shutting down".into());
            }
            if method == "DELETE" && action.is_empty() {
                self.discard_draft(id).await?;
                return Ok(json!({"removed":id}));
            }
            let draft = {
                let mut drafts = self.drafts.lock().await;
                let (draft, last, _) = drafts
                    .get_mut(id)
                    .ok_or("pending connection expired; obtain the MCP URL again")?;
                *last = std::time::Instant::now();
                draft.clone()
            };
            if method == "GET" && action.is_empty() {
                return draft.summary().await;
            }
            if method == "POST" && action == "commit" {
                let _runtime = draft.operation.lock().await;
                let mut entry = draft.meta.lock().await.clone();
                for key in ["name", "controlSource", "auth", "bearerToken"] {
                    if let Some(value) = body.get(key) {
                        entry[key] = value.clone();
                    }
                }
                if !["none", "bearer"].contains(&string(&entry, "auth")) {
                    return Err("invalid authentication".into());
                }
                entry["toolPolicy"] = json!("all");
                entry["config"]["httpsUrl"] = body["url"].clone();
                let entry = normalize(entry)?;
                // Validate the live URL before publishing or enabling any tools.
                draft
                    .validate_draft_url(string(&entry["config"], "httpsUrl"))
                    .await?;
                store_entry(&entry)?;
                let mut entries = self.ingresses.lock().await;
                let mut ids: Vec<_> = entries.keys().cloned().collect();
                ids.push(id.to_owned());
                save(&root().join("ingresses/index.json"), &json!(ids))?;
                draft.activate_draft(entry).await?;
                entries.insert(id.to_owned(), draft.clone());
                self.drafts.lock().await.remove(id);
                drop(entries);
                drop(_runtime);
                return draft.summary().await;
            }
            return Err("UNKNOWN_ROUTE".into());
        }
        if route == "status" || route == "core" {
            let v = self.status().await?;
            return Ok(if route == "core" {
                v["core"].clone()
            } else {
                v
            });
        }
        if route == "ingress" && method == "GET" {
            let mut items = Vec::new();
            for i in self.entries().await {
                items.push(i.summary().await?);
            }
            return Ok(json!(items));
        }
        if route == "ingress/start-all" && method == "POST" {
            return self.all("start").await;
        }
        if route == "ingress/stop-all" && method == "POST" {
            return self.all("stop").await;
        }
        if route == "ingress" && method == "POST" {
            let _guard = self.configuration.lock().await;
            if self.closing.load(Ordering::SeqCst) {
                return Err("core shutting down".into());
            }
            if !body.is_object() {
                return Err("expected ingress object".into());
            }
            let mut entry = body;
            if entry.get("id").is_none() {
                entry["id"] = json!(id());
            }
            for (k, v) in
                json!({"enabled":true,"toolPolicy":"all","config":{},"name":entry["controlSource"]})
                    .as_object()
                    .unwrap()
            {
                entry
                    .as_object_mut()
                    .ok_or("invalid ingress")?
                    .entry(k.clone())
                    .or_insert(v.clone());
            }
            if entry["transport"] == "https" && entry["config"].get("httpsPort").is_none() {
                let listener =
                    std::net::TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
                entry["config"]["httpsPort"] =
                    json!(listener.local_addr().map_err(|e| e.to_string())?.port());
            }
            let entry = normalize(entry)?;
            if self
                .ingresses
                .lock()
                .await
                .contains_key(string(&entry, "id"))
            {
                return Err("ingress already exists".into());
            }
            store_entry(&entry)?;
            let i = Ingress::new(
                entry,
                self.control.clone(),
                self.agents.clone(),
                self.token.clone(),
            )?;
            let mut entries = self.ingresses.lock().await;
            let mut ids: Vec<_> = entries.keys().cloned().collect();
            ids.push(i.id.clone());
            save(&root().join("ingresses/index.json"), &json!(ids))?;
            entries.insert(i.id.clone(), i.clone());
            drop(entries);
            return i.summary().await;
        }
        if let Some(path) = route.strip_prefix("ingress/") {
            let (id, action) = path.split_once('/').unwrap_or((path, ""));
            valid_id(id)?;
            let _guard = if matches!(method, "PUT" | "DELETE") || action == "token" {
                Some(self.configuration.lock().await)
            } else {
                None
            };
            if self.closing.load(Ordering::SeqCst) {
                return Err("core shutting down".into());
            }
            let i = self.ingress(id).await?;
            if method == "POST" && action == "credentials" {
                let credentials = i.request("config/credentials", "GET", json!({})).await?;
                return Ok(json!({"apiKey": credentials["apiKey"]}));
            }
            if method == "GET" && action.is_empty() {
                return i.summary().await;
            }
            if method == "DELETE" && action.is_empty() {
                let _runtime = i.operation.lock().await;
                i.stop().await?;
                i.retired.store(true, Ordering::SeqCst);
                let mut entries = self.ingresses.lock().await;
                let ids: Vec<_> = entries.keys().filter(|k| *k != id).cloned().collect();
                save(&root().join("ingresses/index.json"), &json!(ids))?;
                entries.remove(id);
                drop(entries);
                std::fs::remove_dir_all(root().join("ingresses").join(id))
                    .map_err(|e| e.to_string())?;
                return Ok(json!({"removed":id}));
            }
            if method == "PUT" && action.is_empty() || action == "token" && method == "POST" {
                let _runtime = i.operation.lock().await;
                if i.summary().await?["running"] == true {
                    return Err("stop this ingress before updating or rotating its token".into());
                }
                let mut entry = read_entry(id)?;
                let previous = normalize(entry.clone())?;
                if action == "token" && entry["auth"] != "bearer" {
                    return Err("token rotation requires auth=bearer".into());
                }
                if action == "token" {
                    entry["bearerToken"] = json!(crate::id() + &crate::id());
                } else {
                    for (k, v) in body.as_object().ok_or("expected JSON object")? {
                        if k == "config" {
                            for (k, v) in v.as_object().ok_or("config must be an object")? {
                                entry["config"][k] = v.clone();
                            }
                        } else {
                            entry[k] = v.clone();
                        }
                    }
                }
                if entry["id"] != id {
                    return Err("cannot change ingress id".into());
                }
                let entry = normalize(entry)?;
                store_entry(&entry)?;
                let next = Ingress::new(
                    entry.clone(),
                    self.control.clone(),
                    self.agents.clone(),
                    self.token.clone(),
                )?;
                let identity_changed = crate::ingress::binding(&previous["config"])
                    != crate::ingress::binding(&entry["config"])
                    || ["transport", "controlSource", "auth", "bearerToken"]
                        .iter()
                        .any(|key| previous[*key] != entry[*key]);
                if identity_changed {
                    next.request("verification/reset", "POST", json!({}))
                        .await?;
                }
                i.retired.store(true, Ordering::SeqCst);
                self.ingresses
                    .lock()
                    .await
                    .insert(id.to_owned(), next.clone());
                return if action == "token" {
                    Ok(
                        json!({"id":id,"token":entry["bearerToken"],"delivery":"secret; store locally, do not paste into chat"}),
                    )
                } else {
                    next.summary().await
                };
            }
            if method != "POST" || !["start", "stop", "verification/reset"].contains(&action) {
                return Err("UNKNOWN_ROUTE".into());
            }
            if action == "start" && i.meta.lock().await["enabled"] != true {
                return Err("ingress disabled".into());
            }
            i.port
                .store(self.port.load(Ordering::SeqCst), Ordering::SeqCst);
            return i.request(action, method, body).await;
        }
        if [
            "task-settings",
            "tasks/decision",
            "tasks",
            "tasks/open",
            "agents",
            "agents/open",
            "codex/login",
        ]
        .contains(&route)
            || route.starts_with("tasks/runtime/")
        {
            return match (method, route) {
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
                ("POST", "tasks/open") => {
                    crate::desktop::open_thread(string(&body, "threadId")).await
                }
                ("GET", route) if route.starts_with("tasks/runtime/") => {
                    self.control.task_runtime(&route[14..]).await
                }
                ("POST", "agents/open") => {
                    self.agents.open_interactive(string(&body, "agent")).await
                }
                ("GET", "agents") => Ok(self.agents.inventory().await),
                ("PUT", "agents") => self.agents.set_enabled(&body).await,
                ("GET", "codex/login") => {
                    let binary = crate::desktop::installation()
                        .ok_or("请先安装 Codex Desktop")?
                        .binary;
                    let out = command(binary)
                        .args(["login", "status"])
                        .output()
                        .await
                        .map_err(|e| e.to_string())?;
                    Ok(
                        json!({"state":if out.status.success(){"authenticated"}else{"unauthenticated"}}),
                    )
                }
                ("POST", "codex/login") => {
                    crate::desktop::open_app().await?;
                    Ok(json!({"state":"unauthenticated","message":"请在 Codex Desktop 中完成登录"}))
                }
                _ => Err("UNKNOWN_ROUTE".into()),
            };
        }
        match (method, route) {
            ("PUT", "network") => {
                let mode = body["proxyMode"].as_str().ok_or("invalid proxy mode")?;
                let url = body["proxyUrl"].as_str().ok_or("invalid proxy URL")?;
                crate::proxy::validate(mode, url)?;
                save(&root().join("web/preferences.json"), &body)?;
                for ingress in self.entries().await {
                    ingress.request("network", "PUT", body.clone()).await?;
                }
                return Ok(json!({"ok":true}));
            }
            _ => {}
        }
        if method == "PUT" && route == "config" && self.ingresses.lock().await.is_empty() {
            let _guard = self.configuration.lock().await;
            if self.closing.load(Ordering::SeqCst) {
                return Err("core shutting down".into());
            }
            if !self.ingresses.lock().await.is_empty() {
                return Err("ingress was created; refresh configuration".into());
            }
            let https = body["connectionMode"] == "https";
            let entry = normalize(
                json!({"id":"default","name":"ChatGPT","controlSource":"chatgpt","transport":if https{"https"}else{"openai-tunnel"},"auth":if https{"none"}else{"openai"},"enabled":true,"toolPolicy":"all","config":body}),
            )?;
            store_entry(&entry)?;
            let ingress = Ingress::new(
                entry,
                self.control.clone(),
                self.agents.clone(),
                self.token.clone(),
            )?;
            save(&root().join("ingresses/index.json"), &json!(["default"]))?;
            self.ingresses
                .lock()
                .await
                .insert("default".into(), ingress);
            return Ok(json!({"ok":true}));
        }
        if route == "stop" && self.ingresses.lock().await.is_empty() {
            return Ok(json!({"ok":true}));
        }
        let i = self.primary().await?;
        i.port
            .store(self.port.load(Ordering::SeqCst), Ordering::SeqCst);
        i.request(route, method, body).await
    }
}
