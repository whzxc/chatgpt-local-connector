//! Common task host. Codex's native Control remains the owner of all native semantics.
use crate::*;
use std::collections::{HashMap, HashSet};
mod driver;
mod process;
mod wait;

struct Operation {
    task: String,
    previous_turn: Value,
    job: tokio::task::JoinHandle<()>,
}
use driver::{AgentDriver, Manifest};
use process::Process;

pub struct AgentHost {
    drivers: HashMap<String, AgentDriver>,
    disabled: Mutex<HashSet<String>>,
    processes: Mutex<HashMap<String, Arc<Process>>>,
    operations: Mutex<HashMap<String, Operation>>,
    session: String,
    lifecycle: Mutex<()>,
    wake: Arc<tokio::sync::Notify>,
}
fn task_path(task: &str) -> Result<PathBuf> {
    uuid::Uuid::parse_str(task).map_err(|_| "INVALID_TASK_ID")?;
    Ok(root().join("agents/tasks").join(format!("{task}.json")))
}
fn save_task(task: &Value) -> Result<()> {
    save(&task_path(string(task, "taskId"))?, task)
}
fn receipt_path(request: &str) -> Result<PathBuf> {
    uuid::Uuid::parse_str(request).map_err(|_| "INVALID_REQUEST_ID")?;
    Ok(root()
        .join("agents/requests")
        .join(format!("{request}.json")))
}
impl AgentHost {
    pub fn new() -> Result<Arc<Self>> {
        let mut drivers = HashMap::from([
            ("codex".into(), AgentDriver::CodexNative),
            ("pi".into(), AgentDriver::Pi),
            (
                "opencode".into(),
                AgentDriver::Acp(Manifest {
                    id: "opencode".into(),
                    command: "opencode".into(),
                    args: vec!["acp".into()],
                }),
            ),
        ]);
        let path = root().join("agents/manifests.json");
        if path.exists() {
            let manifests: Vec<Manifest> =
                serde_json::from_value(load(&path)?).map_err(|e| e.to_string())?;
            for m in manifests {
                if m.id.is_empty()
                    || !m
                        .id
                        .bytes()
                        .all(|c| c.is_ascii_alphanumeric() || b"_-".contains(&c))
                    || drivers.contains_key(&m.id)
                {
                    return Err("INVALID_OR_DUPLICATE_AGENT_ID".into());
                }
                drivers.insert(m.id.clone(), AgentDriver::Acp(m));
            }
        }
        let preferences = root().join("agents/disabled.json");
        let disabled: HashSet<String> = if preferences.exists() {
            serde_json::from_value(load(&preferences)?).map_err(|e| e.to_string())?
        } else {
            HashSet::new()
        };
        Ok(Arc::new(Self {
            disabled: Mutex::new(disabled),
            drivers,
            processes: Default::default(),
            operations: Default::default(),
            session: id(),
            lifecycle: Mutex::new(()),
            wake: Default::default(),
        }))
    }
    async fn ensure_enabled(&self, agent: &str) -> Result<()> {
        if agent != "codex" && self.disabled.lock().await.contains(agent) {
            return Err("AGENT_DISABLED".into());
        }
        Ok(())
    }
    pub async fn set_enabled(&self, body: &Value) -> Result<Value> {
        let agent = string(body, "agent");
        if !self.drivers.contains_key(agent) {
            return Err("UNKNOWN_AGENT".into());
        }
        let enabled = body["enabled"]
            .as_bool()
            .ok_or("ENABLED_BOOLEAN_REQUIRED")?;
        if agent == "codex" && !enabled {
            return Err("CODEX_ALWAYS_ENABLED".into());
        }
        let mut disabled = self.disabled.lock().await;
        let mut next = disabled.clone();
        if enabled {
            next.remove(agent);
        } else {
            next.insert(agent.to_owned());
        }
        save(&root().join("agents/disabled.json"), &json!(next))?;
        *disabled = next;
        Ok(json!({"agent":agent,"enabled":enabled}))
    }
    pub async fn inventory(&self) -> Value {
        let mut rows = Vec::new();
        let mut ids = self.drivers.keys().collect::<Vec<_>>();
        ids.sort();
        for agent in ids {
            let driver = &self.drivers[agent];
            let binary = driver.binary();
            let version = if let Some(p) = &binary {
                tokio::time::timeout(std::time::Duration::from_secs(4), process::version(p))
                    .await
                    .ok()
                    .and_then(Result::ok)
            } else {
                None
            };
            let ready = self.processes.lock().await.values().any(|p| {
                p.alive.load(std::sync::atomic::Ordering::SeqCst)
                    && p.state.try_lock().is_ok_and(|s| s["agent"] == *agent)
            });
            rows.push(json!({"agent":agent,"protocol":driver.protocol(),"installed":binary.is_some(),"enabled":self.ensure_enabled(agent).await.is_ok(),"status":if ready {"ready"}else if binary.is_some() && version.is_some(){"installed"}else{"unavailable"},"version":version.as_deref().map(str::trim),"path":binary,"configuration":"inherited","readiness":"ready means a live initialized session, not provider authentication"}));
        }
        json!({"agents":rows,"defaultAgent":"codex","observedAt":now()})
    }
    async fn task(&self, agent: &str, task: &str) -> Result<Value> {
        let p = self.processes.lock().await.get(task).cloned();
        let mut value = if let Some(p) = p {
            p.state.lock().await.clone()
        } else {
            let mut t = load(&task_path(task)?)?;
            t["processState"] = json!("stopped");
            if !matches!(
                string(&t, "status"),
                "completed" | "cancelled" | "failed" | "idle"
            ) {
                t["status"] = json!("unknown");
            }
            t
        };
        if value["agent"] != agent {
            return Err("AGENT_TASK_MISMATCH".into());
        }
        value["observedAt"] = json!(now());
        Ok(value)
    }
    async fn process(&self, agent: &str, task: &str, resume: bool) -> Result<Arc<Process>> {
        let _lock = self.lifecycle.lock().await;
        if let Some(p) = self.processes.lock().await.get(task).cloned() {
            if p.alive.load(std::sync::atomic::Ordering::SeqCst) {
                return Ok(p);
            }
        }
        if !resume {
            return Err("PROCESS_NOT_RUNNING".into());
        }
        let state = self.task(agent, task).await?;
        if state["status"] == "unknown" {
            return Err("TASK_UNCONFIRMED_RECONCILE_BEFORE_RESUME".into());
        }
        let p = self.drivers[agent]
            .start(state, true, self.wake.clone())
            .await?;
        self.processes.lock().await.insert(task.into(), p.clone());
        Ok(p)
    }
    pub async fn close(&self) {
        let _lock = self.lifecycle.lock().await;
        for (request, job) in self.operations.lock().await.drain() {
            job.job.abort();
            let _ = job.job.await;
            if let Ok(path) = receipt_path(&request) {
                if let Ok(mut r) = load(&path) {
                    if r["state"] == "pending" {
                        r["state"] = json!("unconfirmed");
                        let _ = save(&path, &r);
                    }
                }
            }
        }
        let processes = self
            .processes
            .lock()
            .await
            .drain()
            .map(|(_, p)| p)
            .collect::<Vec<_>>();
        self.wake.notify_waiters();
        for p in processes {
            p.close().await;
        }
    }
    pub async fn tool(
        self: &Arc<Self>,
        native: &Arc<control::Control>,
        name: &str,
        args: Value,
    ) -> Result<Value> {
        let catalog = crate::catalog();
        let spec = catalog["tools"]
            .as_array()
            .unwrap()
            .iter()
            .find(|t| t["name"] == name)
            .ok_or("UNKNOWN_TOOL")?;
        jsonschema::validator_for(&spec["inputSchema"])
            .map_err(|e| e.to_string())?
            .validate(&args)
            .map_err(|e| e.to_string())?;
        if name == "agents" {
            return Ok(self.inventory().await);
        }
        let agent = args["agent"].as_str().unwrap_or("codex");
        let driver = self.drivers.get(agent).ok_or("UNKNOWN_AGENT")?;
        self.ensure_enabled(agent).await?;
        if matches!(driver, AgentDriver::CodexNative) {
            return native_tool(native, name, args).await;
        }
        if name == "agent_wait" {
            return wait::agent_wait(self, native, &args).await;
        }
        if name == "agent_request" {
            let request = string(&args, "requestId");
            let path = receipt_path(request)?;
            let mut ops = self.operations.lock().await;
            let mut r = load(&path)?;
            if r["agent"] != agent {
                return Err("AGENT_REQUEST_MISMATCH".into());
            }
            if r["backendSession"] != self.session && r["state"] == "pending" {
                r["state"] = json!("unconfirmed");
            }
            let action = args["action"].as_str().unwrap_or("read");
            if !matches!(action, "read" | "approve" | "bypass" | "reject") {
                return Err("INVALID_APPROVAL_ACTION".into());
            }
            if action != "read" && r["state"] == "awaiting-approval" {
                r["state"] = json!(if action == "reject" {
                    "not-executed"
                } else {
                    "pending"
                });
                r["approval"] = json!(action);
                r["backendSession"] = json!(self.session);
                if action != "reject" && r["operation"] != "agent_create" {
                    r["previousTurnId"] =
                        self.task(agent, string(&r, "taskId")).await?["turnId"].clone();
                }
                save(&path, &r)?;
                if action != "reject" {
                    let job = self.launch(driver.clone(), r.clone(), path);
                    ops.insert(request.into(), job);
                }
            }
            return Ok(r);
        }
        if name == "agent_tasks" {
            let dir = root().join("agents/tasks");
            private_dir(&dir)?;
            let mut tasks = Vec::new();
            for entry in std::fs::read_dir(dir).map_err(|e| e.to_string())? {
                let path = entry.map_err(|e| e.to_string())?.path();
                if path.extension().is_some_and(|e| e == "json") {
                    let t = load(&path)?;
                    if t["agent"] == agent {
                        tasks.push(self.task(agent, string(&t, "taskId")).await?);
                    }
                }
            }
            tasks.sort_by(|a, b| string(b, "updatedAt").cmp(string(a, "updatedAt")));
            let offset = num(&args, "offset", 0);
            let limit = num(&args, "limit", 50).clamp(1, 200);
            let total = tasks.len();
            return Ok(
                json!({"tasks":tasks.into_iter().skip(offset).take(limit).collect::<Vec<_>>(),"total":total,"nextOffset":if offset+limit<total{Some(offset+limit)}else{None}}),
            );
        }
        if name == "agent_capabilities" {
            if let Some(task) = args["taskId"].as_str() {
                let _ = self.task(agent, task).await?;
                let p = self.process(agent, task, false).await?;
                return Ok(p.capabilities.lock().await.clone());
            }
            return Ok(
                json!({"agent":agent,"protocol":driver.protocol(),"installed":driver.binary().is_some(),"create":true,"prompt":"text","cancel":true,"resume":if matches!(driver,AgentDriver::Pi){json!(true)}else{json!("negotiated per session")},"permission":if matches!(driver,AgentDriver::Pi){"Pi extension dialogs; not a sandbox or universal tool approval"}else{"ACP session/request_permission"},"negotiated":false,"nextAction":"agent_create then agent_capabilities(taskId)"}),
            );
        }
        if name == "agent_read" {
            return self.task(agent, string(&args, "taskId")).await;
        }
        if matches!(name, "agent_events" | "agent_pending") {
            let task = string(&args, "taskId");
            let _ = self.task(agent, task).await?;
            let p = self.process(agent, task, false).await?;
            let backend = p.state.lock().await["processSession"].clone();
            let e = p.events.lock().await;
            if name == "agent_pending" {
                return Ok(json!({"requests":e.pending.values().collect::<Vec<_>>() }));
            }
            let after = num(&args, "after", 0) as u64;
            return Ok(
                json!({"events":e.rows.iter().filter(|r|r["cursor"].as_u64().unwrap_or(0)>after).take(num(&args,"limit",100).min(2000)).collect::<Vec<_>>(),"backendSession":backend,"reset":args.get("backendSession").is_some()&&args["backendSession"]!=backend,"latestCursor":e.sequence,"gap":after+1<e.rows.first().and_then(|r|r["cursor"].as_u64()).unwrap_or(1),"persistedOutput":{"outputId":task,"format":"JSONL","error":e.storage_error}}),
            );
        }
        if !matches!(
            name,
            "agent_create" | "agent_send" | "agent_interrupt" | "agent_respond"
        ) {
            return Err("UNKNOWN_TOOL".into());
        }
        let request = string(&args, "requestId");
        let path = receipt_path(request)?;
        let digest = hash(json!({"name":name,"args":args}).to_string());
        let mut ops = self.operations.lock().await;
        if path.exists() {
            let mut r = load(&path)?;
            if r["digest"] != digest {
                return Err("REQUEST_ID_CONFLICT".into());
            }
            if r["backendSession"] != self.session && r["state"] == "pending" {
                r["state"] = json!("unconfirmed");
            }
            r["replayed"] = json!(true);
            return Ok(r);
        }
        let task = if name == "agent_create" {
            id()
        } else {
            string(&args, "taskId").into()
        };
        if name != "agent_create" {
            self.task(agent, &task).await?;
        }
        if name == "agent_create" {
            let cwd = Path::new(string(&args, "cwd"));
            if !cwd.is_absolute() || !cwd.is_dir() {
                return Err("ABSOLUTE_CWD_REQUIRED".into());
            }
        }
        if name == "agent_send" && string(&args, "prompt").is_empty() {
            return Err("PROMPT_REQUIRED".into());
        }
        let needs_approval = matches!(name, "agent_create" | "agent_send" | "agent_interrupt")
            && native.approval_mode()?
            && !matches!(string(&args, "approval"), "approved" | "bypass");
        let previous_turn = if name == "agent_create" {
            Value::Null
        } else {
            self.task(agent, &task).await?["turnId"].clone()
        };
        let receipt = json!({"requestId":request,"agent":agent,"taskId":task,"previousTurnId":previous_turn,"operation":name,"arguments":args,"digest":digest,"state":if needs_approval{"awaiting-approval"}else{"pending"},"backendSession":self.session,"createdAt":now(),"nextAction":"agent_request"});
        private_dir(path.parent().ok_or("INVALID_RECEIPT_PATH")?)?;
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options
            .open(&path)
            .map_err(|e| format!("RECEIPT_RESERVATION_FAILED:{e}"))?;
        use std::io::Write;
        file.write_all(receipt.to_string().as_bytes())
            .map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        if !needs_approval {
            let job = self.launch(driver.clone(), receipt.clone(), path);
            ops.insert(request.into(), job);
        }
        Ok(receipt)
    }
    fn launch(self: &Arc<Self>, driver: AgentDriver, mut r: Value, path: PathBuf) -> Operation {
        let host = self.clone();
        let task = string(&r, "taskId").to_owned();
        let previous_turn = r["previousTurnId"].clone();
        let job = tokio::spawn(async move {
            let result = host
                .perform(
                    &driver,
                    string(&r, "operation"),
                    &r["arguments"],
                    string(&r, "taskId"),
                )
                .await;
            match result {
                Ok(v) => {
                    r["state"] = json!("completed");
                    r["result"] = v;
                }
                Err(e) => {
                    r["state"] = json!(if e.starts_with("RPC_REJECTED")
                        || e == "TASK_BUSY_OR_UNCONFIRMED"
                        || e == "AGENT_DISABLED"
                    {
                        "rejected"
                    } else {
                        "unconfirmed"
                    });
                    r["error"] = json!(e);
                }
            }
            r["updatedAt"] = json!(now());
            // Failure leaves the original pending receipt; replay is always prohibited.
            let _ = save(&path, &r);
            host.operations.lock().await.remove(string(&r, "requestId"));
            host.wake.notify_waiters();
        });
        Operation {
            task,
            previous_turn,
            job,
        }
    }
    async fn perform(
        &self,
        driver: &AgentDriver,
        name: &str,
        args: &Value,
        task: &str,
    ) -> Result<Value> {
        let agent = string(args, "agent");
        self.ensure_enabled(agent).await?;
        let p = if name == "agent_create" {
            let _guard = self.lifecycle.lock().await;
            let t = json!({"taskId":task,"agent":agent,"runtimeSource":driver.protocol(),"sessionId":null,"threadId":null,"cwd":args["cwd"],"title":args["title"],"status":"starting","createdAt":now(),"updatedAt":now(),"metadata":{},"output":""});
            save_task(&t)?;
            let p = match driver.start(t, false, self.wake.clone()).await {
                Ok(p) => p,
                Err(e) => {
                    let mut t = load(&task_path(task)?)?;
                    t["status"] = json!("unknown");
                    t["error"] = json!(e);
                    save_task(&t)?;
                    return Err(e);
                }
            };
            self.processes.lock().await.insert(task.into(), p.clone());
            p
        } else {
            self.process(agent, task, name == "agent_send").await?
        };
        match name {
            "agent_create" if string(args, "prompt").is_empty() => Ok(p.state.lock().await.clone()),
            "agent_create" | "agent_send" => driver.prompt(&p, string(args, "prompt")).await,
            "agent_interrupt" => driver.interrupt(&p).await,
            "agent_respond" => driver.respond(&p, args).await,
            _ => Err("UNKNOWN_TOOL".into()),
        }
    }
}
async fn native_tool(native: &Arc<control::Control>, name: &str, mut a: Value) -> Result<Value> {
    let suffix = name.strip_prefix("agent_").ok_or("UNKNOWN_TOOL")?;
    a.as_object_mut()
        .ok_or("INVALID_ARGUMENTS")?
        .remove("agent");
    if let Some(task) = a.as_object_mut().unwrap().remove("taskId") {
        a["threadId"] = task;
    }
    if let Some(cwd) = a.as_object_mut().unwrap().remove("cwd") {
        a["project"] = cwd;
    }
    if suffix == "capabilities" {
        a.as_object_mut().unwrap().remove("threadId");
    }
    if suffix == "tasks" {
        a.as_object_mut().unwrap().remove("offset");
    }
    if suffix == "events" || suffix == "pending" {
        a.as_object_mut().unwrap().remove("threadId");
    }
    if suffix == "respond" {
        a.as_object_mut().unwrap().remove("threadId");
        if let Some(v) = a.as_object_mut().unwrap().remove("interactionId") {
            a["id"] = json!(v
                .as_str()
                .map(str::to_owned)
                .unwrap_or_else(|| v.to_string()));
        }
    }
    if suffix == "interrupt" && string(&a, "turnId").is_empty() {
        return Err("CODEX_TURN_ID_REQUIRED".into());
    }
    let result = native.tool(&format!("codex_{suffix}"), a).await?;
    let mut value = result.clone();
    value["agent"] = json!("codex");
    if value.get("runtimeSource").is_none_or(Value::is_null) {
        value["runtimeSource"] = json!("codex-native");
    }
    let thread = if result["threadId"].is_string() {
        result["threadId"].clone()
    } else {
        result["result"]["threadId"].clone()
    };
    value["taskId"] = thread.clone();
    value["sessionId"] = thread;
    if suffix == "wait" {
        if let Some(interactions) = value["interaction"].as_array_mut() {
            for interaction in interactions {
                interaction["interactionId"] = interaction["responseId"].clone();
            }
        }
        if value["interaction"]
            .as_array()
            .is_some_and(|v| !v.is_empty())
        {
            value["interactionAction"] = json!("Use agent_respond with agent=codex, taskId, interactionId, backendSession, requestId and native result/error");
        }
    }
    if suffix == "read" {
        value["status"] = json!(match string(&result, "runtimeStatus") {
            "active" => "running",
            "idle" => "idle",
            _ => "unknown",
        });
    }
    if suffix == "tasks" {
        if let Some(tasks) = value["tasks"].as_array_mut() {
            for task in tasks {
                task["agent"] = json!("codex");
                task["taskId"] = task["threadId"].clone();
                task["sessionId"] = task["threadId"].clone();
            }
        }
    }
    value["native"] = result;
    Ok(value)
}
