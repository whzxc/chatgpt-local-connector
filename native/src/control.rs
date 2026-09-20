use crate::*;
use crate::{
    desktop::{self, Ipc},
    rpc::Rpc,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use std::{collections::HashMap, time::Duration};
pub struct Control {
    pub session: String,
    pub events: SharedEvents,
    pub(crate) shutdown: tokio::sync::watch::Sender<u64>,
    binary: PathBuf,
    rpc: Mutex<Option<Arc<Rpc>>>,
    monitor: Mutex<Option<Ipc>>,
    jobs: Mutex<HashMap<String, tokio::task::JoinHandle<()>>>,
    schemas: Mutex<Option<Value>>,
    archived_tasks: Mutex<Option<(std::time::Instant, HashMap<String, Value>)>>,
}
impl Control {
    pub fn new(binary: PathBuf) -> Arc<Self> {
        Arc::new(Self {
            session: id(),
            events: Default::default(),
            shutdown: tokio::sync::watch::channel(0).0,
            binary,
            rpc: Mutex::new(None),
            monitor: Mutex::new(None),
            jobs: Default::default(),
            schemas: Mutex::new(None),
            archived_tasks: Mutex::new(None),
        })
    }
    pub async fn utility(&self) -> Result<Arc<Rpc>> {
        let mut rpc = self.rpc.lock().await;
        if rpc.as_ref().is_none_or(|r| !r.alive()) {
            *rpc = Some(Rpc::start(&self.binary, self.events.clone(), &self.session).await?)
        }
        Ok(rpc.as_ref().unwrap().clone())
    }
    async fn ipc(&self) -> Result<Ipc> {
        Ipc::open(self.events.clone(), &self.session).await
    }
    async fn follow(&self, thread: &str) -> Result<()> {
        let mut ipc = self.monitor.lock().await;
        if ipc.as_ref().is_none_or(|i| !i.alive()) {
            *ipc = Some(self.ipc().await?)
        }
        ipc.as_ref().unwrap().follow(thread).await
    }
    pub async fn health(&self) -> Value {
        json!({"appServer":if self.rpc.lock().await.as_ref().is_some_and(|r|r.alive()){"ready"}else{"stopped"},"activeWrites":self.jobs.lock().await.len()})
    }
    pub fn stop_waits(&self) {
        self.shutdown.send_modify(|v| *v = v.wrapping_add(1));
    }
    pub async fn close(&self) {
        self.stop_waits();
        for (request, job) in self.jobs.lock().await.drain() {
            job.abort();
            let _ = job.await;
            if let Ok(mut receipt) = load(&root().join(format!("{request}.json"))) {
                if !matches!(
                    string(&receipt, "state"),
                    "completed" | "rejected" | "not-executed" | "unconfirmed"
                ) {
                    receipt["previousState"] = receipt["state"].clone();
                    receipt["state"] = json!("unconfirmed");
                    let _ = self.checkpoint(&mut receipt);
                }
            }
        }
        self.monitor.lock().await.take();
        if let Some(rpc) = self.rpc.lock().await.take() {
            rpc.close().await;
        }
    }
    pub fn auto_open_codex(&self) -> Result<bool> {
        let path = root().join("task-settings.json");
        Ok(!path.exists() || load(&path)?["autoOpenCodex"] != false)
    }
    pub(crate) fn background(&self, thread: &str) -> Result<bool> {
        if thread.is_empty() {
            return Ok(false);
        }
        uuid::Uuid::parse_str(thread).map_err(|_| "INVALID_THREAD_ID")?;
        let path = root().join("execution").join(format!("{thread}.json"));
        Ok(path.exists() && load(&path)?["owner"] == "connector")
    }
    async fn create_background(
        &self,
        args: Value,
        title: &str,
        receipt: Option<&mut Value>,
    ) -> Result<Value> {
        if args["ephemeral"] == true {
            return Err("BACKGROUND_TASK_MUST_PERSIST".into());
        }
        let rpc = self.utility().await?;
        let mut args = args;
        args["ephemeral"] = json!(false);
        let result = rpc.call("thread/start", args, 60000).await?;
        let thread = string(&result["thread"], "id");
        uuid::Uuid::parse_str(thread).map_err(|_| "INVALID_THREAD_ID")?;
        save(
            &root().join("execution").join(format!("{thread}.json")),
            &json!({"owner":"connector"}),
        )?;
        if let Some(r) = receipt {
            r["threadId"] = json!(thread);
            r["task"]["directory"] = result["thread"]["cwd"].clone();
            r["state"] = json!("thread-created");
            self.checkpoint(r)?;
        }
        if !title.is_empty() {
            rpc.call(
                "thread/name/set",
                json!({"threadId":thread,"name":title}),
                60000,
            )
            .await?;
        }
        Ok(result)
    }
    async fn background_request(&self, method: &str, args: Value, timeout: u64) -> Result<Value> {
        let rpc = self.utility().await?;
        // Resume only an explicit new-turn write; reads and interrupts never restart a task.
        if method == "turn/start" {
            let state = rpc
                .call(
                    "thread/read",
                    json!({"threadId":args["threadId"],"includeTurns":false}),
                    60000,
                )
                .await?;
            if state["thread"]["status"]["type"] == "notLoaded" {
                rpc.call("thread/resume", json!({"threadId":args["threadId"]}), 60000)
                    .await?;
            }
        }
        let mut result = rpc.call(method, args, timeout).await?;
        if method == "thread/read" {
            result["thread"]["runtimeSource"] = json!("connector-app-server");
        }
        Ok(result)
    }
    async fn seed(&self, args: Value, title: &str, receipt: Option<&mut Value>) -> Result<Value> {
        self.ipc().await?;
        if args["ephemeral"] == true {
            return Err("DESKTOP_TASK_MUST_PERSIST".into());
        }
        let seed = Rpc::start(&self.binary, Default::default(), &id()).await?;
        let result = async {
            let mut args = args;
            args["ephemeral"] = json!(false);
            let response = seed.call("thread/start", args, 60000).await?;
            let thread = string(&response["thread"], "id");
            if let Some(r) = receipt {
                r["threadId"] = json!(thread);
                if r["task"].is_object() {
                    r["task"]["directory"] = response["thread"]["cwd"].clone();
                }
                r["state"] = json!("thread-created");
                self.checkpoint(r)?;
            }
            if !title.is_empty() {
                seed.call(
                    "thread/name/set",
                    json!({"threadId":thread,"name":title}),
                    60000,
                )
                .await?;
            }
            seed.call(
                "thread/read",
                json!({"threadId":thread,"includeTurns":true}),
                60000,
            )
            .await?;
            let path = string(&response["thread"], "path");
            let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
            while std::fs::metadata(path)
                .map(|m| m.len() == 0)
                .unwrap_or(true)
            {
                if tokio::time::Instant::now() >= deadline {
                    return Err("DESKTOP_SEED_NOT_PERSISTED".into());
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            seed.call("thread/unsubscribe", json!({"threadId":thread}), 60000)
                .await?;
            Ok(response)
        }
        .await;
        seed.close().await;
        result
    }
    pub async fn request(&self, method: &str, args: Value, timeout: u64) -> Result<Value> {
        #[cfg(feature = "test-fixture")]
        {
            return self.utility().await?.call(method, args, timeout).await;
        }
        #[allow(unreachable_code)]
        if self.background(string(&args, "threadId"))? {
            return self.background_request(method, args, timeout).await;
        }
        if matches!(method, "turn/start" | "turn/steer" | "turn/interrupt") {
            let ipc = self.ipc().await?;
            self.follow(string(&args, "threadId")).await?;
            return ipc.mutate(method, &args).await;
        }
        if method == "thread/start" {
            if !self.auto_open_codex()? {
                return self.create_background(args, "", None).await;
            }
            let result = self.seed(args, "", None).await?;
            desktop::reveal(&self.ipc().await?, string(&result["thread"], "id")).await?;
            return Ok(result);
        }
        if method == "thread/resume" {
            if args
                .as_object()
                .is_none_or(|o| o.keys().any(|k| k != "threadId"))
            {
                return Err("Desktop resume only accepts threadId".into());
            }
            let ipc = self.ipc().await?;
            desktop::reveal(&ipc, string(&args, "threadId")).await?;
            let state = ipc.read(string(&args, "threadId"), true).await?;
            return Ok(json!({"thread":thread(&state,true)?}));
        }
        if matches!(
            method,
            "thread/read" | "thread/turns/list" | "thread/items/list"
        ) {
            let id = string(&args, "threadId");
            let complete = method != "thread/read" || args["includeTurns"] == true;
            let result = async { self.ipc().await?.read(id, complete).await }.await;
            match result {
                Ok(state) => {
                    self.follow(id).await?;
                    if method == "thread/read" {
                        return Ok(json!({"thread":thread(&state,complete)?}));
                    }
                    let mut data = desktop::turns(&state)?;
                    if method == "thread/items/list" {
                        data = data
                            .iter()
                            .filter(|t| args["turnId"].is_null() || t["id"] == args["turnId"])
                            .flat_map(|t| {
                                t["items"]
                                    .as_array()
                                    .into_iter()
                                    .flatten()
                                    .map(|i| json!({"turnId":t["id"],"item":i}))
                            })
                            .collect();
                    } else if args["itemsView"] == "notLoaded" {
                        for t in &mut data {
                            t["items"] = json!([]);
                        }
                    }
                    if (method == "thread/turns/list" && args["sortDirection"] != "asc")
                        || (method == "thread/items/list" && args["sortDirection"] == "desc")
                    {
                        data.reverse()
                    }
                    return page(method, &args, data);
                }
                Err(e)
                    if e.contains("no-client-found")
                        || e.contains("No such file")
                        || e.contains("Connection refused") =>
                {
                    if string(&args, "cursor").starts_with("desktop:") {
                        return Err("DESKTOP_CURSOR_STALE".into());
                    }
                }
                Err(e) => return Err(e),
            }
        } else if (method.starts_with("thread/")
            && !matches!(method, "thread/list" | "thread/loaded/list"))
            || method.starts_with("turn/")
        {
            return Err(format!("DESKTOP_METHOD_UNSUPPORTED: {method}"));
        }
        self.utility().await?.call(method, args, timeout).await
    }
    pub async fn schema(&self, args: &Value) -> Result<Value> {
        let mut cache = self.schemas.lock().await;
        if cache.is_none() {
            let dir = std::env::temp_dir().join(format!("clc-schema-{}", id()));
            private_dir(&dir)?;
            let result=async{output(&self.binary,&["app-server","generate-json-schema","--experimental","--out",dir.to_str().ok_or("invalid schema path")?]).await?;let mut responses=json!({});let server=load(&dir.join("ServerRequest.json"))?;for branch in server["oneOf"].as_array().into_iter().flatten(){let method=string(&branch["properties"]["method"]["enum"],"0");let method=branch["properties"]["method"]["enum"][0].as_str().unwrap_or(method);if let Some(param)=branch["properties"]["params"]["$ref"].as_str().and_then(|v|v.rsplit('/').next()){let file=format!("{}.json",param.trim_end_matches("Params").to_owned()+"Response");if let Ok(v)=load(&dir.join(file)){responses[method]=v;}}}Ok::<_,String>(json!({"client":load(&dir.join("ClientRequest.json"))?,"server":server,"responses":responses}))}.await;
            let _ = std::fs::remove_dir_all(dir);
            *cache = Some(result?);
        }
        let data = cache.as_ref().unwrap();
        let direction = args["direction"].as_str().unwrap_or("client");
        let root = &data[direction];
        let all = root["oneOf"].as_array().ok_or("SCHEMA_UNAVAILABLE")?;
        if let Some(method) = args["method"].as_str() {
            let b = all
                .iter()
                .find(|b| b["properties"]["method"]["enum"][0] == method)
                .ok_or("METHOD_NOT_FOUND")?;
            let mut params = b["properties"]["params"].clone();
            if params.is_null() {
                params = json!({"type":"null"})
            }
            params["definitions"] = root["definitions"].clone();
            return Ok(
                json!({"method":method,"direction":direction,"description":b["description"],"paramsSchema":params,"responseSchema":data["responses"][method]}),
            );
        }
        let search = string(args, "search").to_lowercase();
        let methods:Vec<Value>=all.iter().filter(|b|b["properties"]["method"]["enum"][0].as_str().unwrap_or("").to_lowercase().contains(&search)).map(|b|json!({"method":b["properties"]["method"]["enum"][0],"description":b["description"]})).collect();
        let offset = num(args, "offset", 0);
        let end = offset.saturating_add(num(args, "limit", 100).min(1000));
        Ok(
            json!({"source":"current-binary-generated-experimental-schema","total":methods.len(),"methods":methods.iter().skip(offset).take(end-offset).collect::<Vec<_>>(),"nextOffset":if end<methods.len(){Some(end)}else{None}}),
        )
    }
    fn checkpoint(&self, r: &mut Value) -> Result<()> {
        r["updatedAt"] = json!(now());
        save(&root().join(format!("{}.json", string(r, "requestId"))), r)
    }
    pub async fn receipt(&self, request: &str) -> Result<Value> {
        uuid::Uuid::parse_str(request).map_err(|_| "INVALID_REQUEST_ID")?;
        let path = root().join(format!("{request}.json"));
        if !path.exists() {
            return Ok(
                json!({"requestId":request,"state":"not-found","executionState":"no-reservation"}),
            );
        }
        let mut r = load(&path)?;
        if r["backendSession"] != self.session
            && !matches!(
                string(&r, "state"),
                "completed" | "rejected" | "not-executed" | "unconfirmed" | "awaiting-approval"
            )
        {
            r["previousState"] = r["state"].clone();
            r["state"] = json!("unconfirmed");
        }
        Ok(r)
    }
    pub async fn mutate(self: &Arc<Self>, operation: &str, args: Value) -> Result<Value> {
        let request = string(&args, "requestId").to_owned();
        uuid::Uuid::parse_str(&request).map_err(|_| "INVALID_REQUEST_ID")?;
        let digest = hash(json!({"operation":operation,"args":args}).to_string());
        let mut jobs = self.jobs.lock().await;
        private_dir(&root())?;
        let path = root().join(format!("{request}.json"));
        if path.exists() {
            let mut old = self.receipt(&request).await?;
            if old["digest"] != digest {
                return Err("REQUEST_ID_CONFLICT".into());
            }
            old["replayed"] = json!(true);
            return Ok(old);
        }
        let mut receipt = json!({"requestId":request,"backendSession":self.session,"digest":digest,"operation":operation,"state":"reserved","createdAt":now(),"updatedAt":now()});
        if operation == "create" || (operation == "native" && args["method"] == "thread/start") {
            receipt["executionOwner"] = json!(if self.auto_open_codex()? {
                "desktop"
            } else {
                "connector"
            });
        }
        if let Some(task) = task_details(operation, &args)? {
            let decision = match string(&args, "approval") {
                "approved" => "approved",
                "bypass" => "bypass",
                _ if self.approval_mode()? => "pending",
                _ => "automatic",
            };
            receipt["task"] = task;
            receipt["arguments"] = args.clone();
            receipt["approval"] = json!({"decision":decision,"source":"cloud","at":now()});
            if decision == "pending" {
                receipt["state"] = json!("awaiting-approval");
                receipt["nextAction"] = json!("codex_request: approve, bypass or reject; user intent overrides the default approval mode");
            }
        }
        use std::io::Write;
        let mut opts = std::fs::OpenOptions::new();
        opts.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            opts.mode(0o600);
        }
        let mut file = opts.open(&path).map_err(|e| e.to_string())?;
        file.write_all(receipt.to_string().as_bytes())
            .map_err(|e| e.to_string())?;
        if receipt["state"] == "awaiting-approval" {
            return Ok(receipt);
        }
        let rx = self.launch(operation, args, receipt, &mut jobs);
        drop(jobs);
        match tokio::time::timeout(Duration::from_millis(100), rx).await {
            Ok(Ok(r)) => r,
            _ => Ok(json!({"requestId":request,"state":"pending","nextAction":"codex_request"})),
        }
    }
    fn launch(
        self: &Arc<Self>,
        operation: &str,
        args: Value,
        mut receipt: Value,
        jobs: &mut HashMap<String, tokio::task::JoinHandle<()>>,
    ) -> tokio::sync::oneshot::Receiver<Result<Value>> {
        let request = string(&receipt, "requestId").to_owned();
        let c = self.clone();
        let op = operation.to_owned();
        let key = request.clone();
        let (tx, rx) = tokio::sync::oneshot::channel();
        let job = tokio::spawn(async move {
            let outcome = c.perform(&op, &args, &mut receipt).await;
            match outcome {
                Ok(result) => {
                    receipt["result"] = result;
                    receipt["state"] = json!("completed")
                }
                Err(err) => {
                    receipt["failureStage"] = receipt["state"].clone();
                    receipt["state"] = json!(if err.starts_with("RPC_REJECTED")
                        || err.starts_with("DESKTOP_REJECTED")
                    {
                        "rejected"
                    } else {
                        "unconfirmed"
                    });
                    receipt["result"] = json!({"error":failure(&err)});
                }
            }
            let result = c.checkpoint(&mut receipt).map(|_| receipt);
            let _ = tx.send(result);
            c.jobs.lock().await.remove(&key);
        });
        jobs.insert(request.clone(), job);
        rx
    }
    pub fn approval_mode(&self) -> Result<bool> {
        let path = root().join("task-settings.json");
        Ok(path.exists() && load(&path)?["enabled"] == true)
    }
    pub async fn decide(
        self: &Arc<Self>,
        request: &str,
        action: &str,
        source: &str,
    ) -> Result<Value> {
        if !matches!(action, "approve" | "bypass" | "reject") {
            return Err("invalid approval action".into());
        }
        let mut jobs = self.jobs.lock().await;
        let mut r = self.receipt(request).await?;
        if r["state"] != "awaiting-approval" {
            return Ok(r);
        }
        r["approval"] = json!({"decision":if action=="approve"{"approved"}else{action},"source":source,"at":now()});
        r.as_object_mut().unwrap().remove("nextAction");
        r["state"] = json!(if action == "reject" {
            "not-executed"
        } else {
            "reserved"
        });
        r["backendSession"] = json!(self.session);
        self.checkpoint(&mut r)?;
        if action != "reject" {
            let op = string(&r, "operation").to_owned();
            let _rx = self.launch(&op, r["arguments"].clone(), r.clone(), &mut jobs);
        }
        Ok(r)
    }
    pub async fn task_records(&self) -> Result<Vec<Value>> {
        let mut records = Vec::new();
        for entry in std::fs::read_dir(root()).map_err(|e| e.to_string())? {
            let path = entry.map_err(|e| e.to_string())?.path();
            if path.extension().is_none_or(|e| e != "json") {
                continue;
            }
            let Some(request) = path.file_stem().and_then(|v| v.to_str()) else {
                continue;
            };
            if uuid::Uuid::parse_str(request).is_err() {
                continue;
            }
            let mut r = self.receipt(request).await?;
            if !r["task"].is_object() {
                let kind = string(&r, "operation");
                let thread_id = r["threadId"]
                    .as_str()
                    .or_else(|| r["result"]["threadId"].as_str())
                    .or_else(|| r["result"]["resolved"]["thread"]["id"].as_str())
                    .or_else(|| r["result"]["thread"]["id"].as_str());
                if !matches!(kind, "create" | "send" | "interrupt") && thread_id.is_none() {
                    continue;
                }
                let thread_id = thread_id.map(str::to_owned);
                let resolved = &r["result"]["resolved"];
                let thread = resolved.get("thread").unwrap_or(&r["result"]["thread"]);
                r["task"] = json!({"kind":kind,"title":thread["name"],"prompt":"",
                    "project":null,"directory":thread.get("cwd").unwrap_or(&resolved["cwd"]),
                    "threadId":thread_id,"model":r["requested"]["model"],"effort":r["requested"]["effort"],
                    "detailsRecorded":false});
                r["threadId"] = json!(thread_id);
            }
            if !r["createdAt"].is_string() {
                r["createdAt"] = r["updatedAt"].clone();
            }
            if !r["approval"].is_object() {
                r["approval"] =
                    json!({"decision":"unrecorded","source":"cloud","at":r["createdAt"]});
            }
            if let Some(thread_id) = r["threadId"]
                .as_str()
                .or_else(|| r["task"]["threadId"].as_str())
            {
                if uuid::Uuid::parse_str(thread_id).is_ok() {
                    if let Ok(mut runtime) =
                        load(&root().join("tasks").join(format!("{thread_id}.json")))
                    {
                        runtime["stale"] = json!(true);
                        r["runtime"] = runtime;
                    }
                }
            }
            if let Some(thread_id) = r["threadId"].as_str() {
                r["executionOwner"] = json!(if self.background(thread_id)? {
                    "connector"
                } else {
                    "desktop"
                });
            }
            r["error"] = r["result"]["error"].clone();
            r.as_object_mut().unwrap().remove("arguments");
            r.as_object_mut().unwrap().remove("result");
            records.push(r);
        }
        records.sort_by(|a, b| string(b, "createdAt").cmp(string(a, "createdAt")));
        Ok(records)
    }
    async fn archived_task(&self, id: &str) -> Result<Option<Value>> {
        let mut cache = self.archived_tasks.lock().await;
        if cache
            .as_ref()
            .is_none_or(|(at, _)| at.elapsed() >= Duration::from_secs(30))
        {
            let rpc = self.utility().await?;
            let mut tasks = HashMap::new();
            let mut params = json!({"archived":true,"limit":100,"sourceKinds":[]});
            loop {
                let page = rpc.call("thread/list", params.clone(), 5000).await?;
                for thread in page["data"].as_array().into_iter().flatten() {
                    let mut task = summary(thread);
                    task["archived"] = json!(true);
                    tasks.insert(string(thread, "id").to_owned(), task);
                }
                match page["nextCursor"].as_str() {
                    Some(cursor) => params["cursor"] = json!(cursor),
                    None => break,
                }
            }
            *cache = Some((std::time::Instant::now(), tasks));
        }
        Ok(cache.as_ref().and_then(|(_, tasks)| tasks.get(id).cloned()))
    }
    pub async fn task_runtime(&self, id: &str) -> Result<Value> {
        uuid::Uuid::parse_str(id).map_err(|_| "INVALID_THREAD_ID")?;
        let path = root().join("tasks").join(format!("{id}.json"));
        let archived = self.archived_task(id).await;
        if let Ok(Some(task)) = &archived {
            save(&path, task)?;
            return Ok(task.clone());
        }
        let live = tokio::time::timeout(
            Duration::from_secs(5),
            self.request(
                "thread/read",
                json!({"threadId":id,"includeTurns":false}),
                5000,
            ),
        )
        .await;
        let mut snapshot = match live {
            Ok(Ok(result)) => summary(&result["thread"]),
            _ => {
                // Unloaded tasks still have native basic metadata; never load conversation items.
                let result = self
                    .utility()
                    .await?
                    .call(
                        "thread/read",
                        json!({"threadId":id,"includeTurns":false}),
                        5000,
                    )
                    .await?;
                let mut thread = result["thread"].clone();
                thread["status"] = json!({"type":"notLoaded"});
                let mut snapshot = summary(&thread);
                if let Ok(previous) = load(&path) {
                    snapshot["runtimeStatus"] = previous["runtimeStatus"].clone();
                    snapshot["observedAt"] = previous["observedAt"].clone();
                }
                snapshot["stale"] = json!(true);
                snapshot
            }
        };
        if archived.is_ok() {
            snapshot["archived"] = json!(false);
        } else if let Ok(previous) = load(&path) {
            snapshot["archived"] = previous["archived"].clone();
        }
        save(&path, &snapshot)?;
        Ok(snapshot)
    }
    async fn perform(&self, operation: &str, args: &Value, r: &mut Value) -> Result<Value> {
        if operation == "respond" {
            if args["backendSession"] != self.session {
                return Err("STALE_INTERACTION".into());
            }
            let key = string(args, "id");
            let pending = self
                .events
                .lock()
                .await
                .pending
                .get(key)
                .cloned()
                .ok_or("STALE_INTERACTION")?;
            if args.get("result").is_some() == args.get("error").is_some() {
                return Err("provide exactly one of result/error".into());
            }
            if let Some(value) = args.get("result") {
                let schema = self
                    .schema(&json!({"method":pending["method"],"direction":"server"}))
                    .await?;
                if !schema["responseSchema"].is_null() {
                    jsonschema::validator_for(&schema["responseSchema"])
                        .map_err(|e| e.to_string())?
                        .validate(value)
                        .map_err(|e| e.to_string())?;
                }
            } else if !args["error"]["code"].is_number() || !args["error"]["message"].is_string() {
                return Err("INVALID_RPC_ERROR".into());
            }
            let rpc = self.utility().await?;
            r["state"] = json!("responding");
            self.checkpoint(r)?;
            let mut response = json!({"id":pending["id"]});
            let field = if args.get("result").is_some() {
                "result"
            } else {
                "error"
            };
            response[field] = args[field].clone();
            rpc.write(response).await?;
            self.events.lock().await.pending.remove(key);
            return Ok(json!({"responseSent":true,"backendSession":self.session,"id":key}));
        }
        let mut params = args["params"].clone();
        let mut method = string(args, "method").to_owned();
        let timeout = args["timeoutMs"].as_u64().unwrap_or(86400000);
        if operation == "create" {
            let input = task_input(args)?;
            let mut seed = args["thread"].as_object().cloned().unwrap_or_default();
            for k in ["model", "serviceTier"] {
                if let Some(v) = args.get(k) {
                    seed.entry(k).or_insert(v.clone());
                }
            }
            if let Some(project) = args["project"].as_str() {
                let p = crate::projects::resolve(self, project).await?;
                seed.insert("cwd".into(), p["root"].clone());
                let pid = if Path::new(string(&p, "id")).is_absolute() {
                    crate::projects::native_id(self, string(&p, "root")).await?
                } else {
                    Some(string(&p, "id").to_owned())
                };
                if let Some(pid) = pid {
                    seed.entry("projectId").or_insert(json!(pid));
                }
            }
            r["task"]["directory"] = seed.get("cwd").cloned().unwrap_or(Value::Null);
            r["state"] = json!("thread-submitting");
            self.checkpoint(r)?;
            let background = r["executionOwner"] == "connector";
            let result = if background {
                self.create_background(json!(seed), string(args, "title"), Some(r))
                    .await?
            } else {
                self.seed(json!(seed), string(args, "title"), Some(r))
                    .await?
            };
            let thread = string(&result["thread"], "id");
            let mut turn = configured(args, string(&result["thread"], "cwd"))?;
            if !background && turn["model"].is_null() {
                turn["model"] = result["model"].clone();
            }
            turn["threadId"] = json!(thread);
            turn["input"] = input;
            r["state"] = json!("turn-submitting");
            self.checkpoint(r)?;
            let out = if background {
                self.request("turn/start", turn, timeout).await?
            } else {
                // Prepare the input before navigation and keep the same IPC connection
                // through owner discovery and submission to shorten the empty-page interval.
                let ipc = self.ipc().await?;
                desktop::reveal(&ipc, thread).await?;
                let out = ipc.mutate("turn/start", &turn).await?;
                r["turnId"] = out["turn"]["id"].clone();
                self.checkpoint(r)?;
                // Monitoring is best effort after acceptance, never a submission failure.
                let _ = self.follow(thread).await;
                out
            };
            r["turnId"] = out["turn"]["id"].clone();
            return Ok(
                json!({"threadId":thread,"turnId":r["turnId"],"resolved":result,"submitted":true,"desktopUrl":format!("codex://threads/{thread}"),"executionOwner":if background {"connector"} else {"desktop"},"desktop":{"state":if background {"not-opened"} else {"owner-confirmed"},"sharedBackend":false}}),
            );
        }
        if operation == "send" {
            if args["resume"]
                .as_object()
                .is_some_and(|m| m.keys().any(|k| k != "threadId"))
            {
                return Err("resume 仅支持 threadId；配置请通过 turn 提交".into());
            }
            let input = task_input(args)?;
            let thread = string(args, "threadId");
            r["threadId"] = json!(thread);
            let state = if self.background(thread)? {
                self.request(
                    "thread/read",
                    json!({"threadId":thread,"includeTurns":true}),
                    60000,
                )
                .await?["thread"]
                    .clone()
            } else {
                let ipc = self.ipc().await?;
                desktop::reveal(&ipc, thread).await?;
                crate::control::thread(&ipc.read(thread, true).await?, true)?
            };
            let turns = state["turns"].as_array().cloned().unwrap_or_default();
            let config = configured(args, string(&state, "cwd"))?;
            if let Some(active) = turns
                .iter()
                .rev()
                .find(|t| state["status"]["type"] == "active" && t["status"] == "inProgress")
            {
                if config.as_object().is_some_and(|m| !m.is_empty()) {
                    return Err("steer cannot change turn configuration".into());
                }
                method = "turn/steer".into();
                params = json!({"threadId":thread,"expectedTurnId":active["id"],"input":input});
            } else {
                method = "turn/start".into();
                params = config;
                params["threadId"] = json!(thread);
                params["input"] = input;
            }
        }
        if operation == "interrupt" {
            method = "turn/interrupt".into();
            params = json!({"threadId":args["threadId"],"turnId":args["turnId"]});
        }
        r["threadId"] = params["threadId"].clone();
        r["state"] = json!("submitting");
        self.checkpoint(r)?;
        let result = if method == "thread/start" && r["executionOwner"] == "connector" {
            self.create_background(params, "", Some(r)).await?
        } else if method == "thread/start" {
            let out = self.seed(params, "", Some(r)).await?;
            desktop::reveal(&self.ipc().await?, string(&out["thread"], "id")).await?;
            out
        } else {
            self.request(&method, params, timeout).await?
        };
        if result["turn"]["id"].is_string() {
            r["turnId"] = result["turn"]["id"].clone();
        }
        Ok(result)
    }
    pub async fn tool(self: &Arc<Self>, name: &str, mut args: Value) -> Result<Value> {
        let catalog = catalog();
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
        for (k, v) in spec["inputSchema"]["properties"]
            .as_object()
            .into_iter()
            .flatten()
        {
            if args.get(k).is_none() {
                if let Some(default) = v.get("default") {
                    args[k] = default.clone();
                }
            }
        }
        let domain = &catalog["domains"][name];
        if domain.is_object() {
            let action = string(&args, "action");
            let mapping = &domain["actions"][action];
            let method = mapping[0].as_str().ok_or("UNKNOWN_ACTION")?;
            let params = args.get("params").cloned().unwrap_or_else(|| {
                if method == "account/workspaceMessages/read" {
                    Value::Null
                } else {
                    json!({})
                }
            });
            if mapping[1] == true {
                return self
                    .request(method, params, args["timeoutMs"].as_u64().unwrap_or(60000))
                    .await;
            }
            return self.mutate("native",json!({"method":method,"params":params,"requestId":args["requestId"],"timeoutMs":args["timeoutMs"].as_u64().unwrap_or(86400000)})).await;
        }
        match name {
            "projects" => Ok(
                json!({"observedAt":now(),"source":"codex-app-server:project/list","projects":crate::projects::list(self).await?}),
            ),
            "overview" | "tree" | "search" | "read" | "git" => {
                crate::projects::query(self, name, &args).await
            }
            "codex_create" => self.mutate("create", args).await,
            "codex_send" => self.mutate("send", args).await,
            "codex_interrupt" => self.mutate("interrupt", args).await,
            "codex_call" => self.mutate("native", args).await,
            "codex_query" => {
                self.request(string(&args, "method"), args["params"].clone(), 60000)
                    .await
            }
            "file_search" => self.request("fuzzyFileSearch", args, 60000).await,
            "codex_schema" => self.schema(&args).await,
            "codex_request" => {
                let action = args["action"].as_str().unwrap_or("read");
                if action == "read" {
                    self.receipt(string(&args, "requestId")).await
                } else {
                    self.decide(string(&args, "requestId"), action, "cloud")
                        .await
                }
            }
            "codex_pending" => Ok(
                json!({"backendSession":self.session,"requests":self.events.lock().await.pending.values().cloned().collect::<Vec<_>>() }),
            ),
            "codex_respond" => self.mutate("respond", args).await,
            "codex_events" => {
                let e = self.events.lock().await;
                let after = num(&args, "after", 0) as u64;
                Ok(
                    json!({"backendSession":self.session,"reset":args.get("backendSession").is_some()&&args["backendSession"]!=self.session,"gap":after+1<e.rows.first().and_then(|r|r["cursor"].as_u64()).unwrap_or(1),"events":e.rows.iter().filter(|r|r["cursor"].as_u64().unwrap_or(0)>after).take(num(&args,"limit",100).min(2000)).collect::<Vec<_>>(),"latestCursor":e.sequence,"persistedOutput":{"outputId":self.session,"format":"JSONL","error":e.storage_error}}),
                )
            }
            "control_output" => read_output(&args),
            "codex_tasks" => {
                let mut params = args.clone();
                params.as_object_mut().unwrap().remove("project");
                params["sortKey"] = json!("updated_at");
                params["sortDirection"] = json!("desc");
                if let Some(p) = args["project"].as_str() {
                    params["cwd"] = json!([crate::projects::resolve(self, p).await?["root"]]);
                }
                let page = self.request("thread/list", params, 60000).await?;
                Ok(
                    json!({"observedAt":now(),"tasks":page["data"].as_array().into_iter().flatten().map(summary).collect::<Vec<_>>(),"nextCursor":page["nextCursor"]}),
                )
            }
            "codex_wait" => crate::waiter::codex_wait(self, &args).await,
            "codex_read" => {
                let id = string(&args, "threadId");
                let mut t = self
                    .request(
                        "thread/read",
                        json!({"threadId":id,"includeTurns":false}),
                        60000,
                    )
                    .await?["thread"]
                    .clone();
                let deadline = tokio::time::Instant::now()
                    + Duration::from_millis(num(&args, "waitMs", 0).min(10000) as u64);
                while t["status"]["type"] == "active" && tokio::time::Instant::now() < deadline {
                    tokio::time::sleep(Duration::from_millis(250)).await;
                    t = self
                        .request(
                            "thread/read",
                            json!({"threadId":id,"includeTurns":false}),
                            60000,
                        )
                        .await?["thread"]
                        .clone();
                }
                let mut result = summary(&t);
                if args["statusOnly"] != true {
                    let page=self.request("thread/turns/list",json!({"threadId":id,"cursor":args["cursor"],"limit":num(&args,"limit",2),"sortDirection":"desc","itemsView":"full"}),60000).await?;
                    result["nextCursor"] = page["nextCursor"].clone();
                    let configurations = turn_configurations(&t);
                    result["turns"]=json!(page["data"].as_array().into_iter().flatten().map(|t|{let items=t["items"].as_array().cloned().unwrap_or_default();json!({"turnId":t["id"],"recordedStatus":t["status"],"error":t["error"],"effectiveConfiguration":configurations.get(string(t,"id")).cloned().unwrap_or(json!({"source":"unavailable","model":"unknown","effort":"unknown"})),"items":items.iter().skip(items.len().saturating_sub(15)).map(|i|item(i,0,600)).collect::<Vec<_>>(),"itemsTruncated":items.len()>15})}).collect::<Vec<_>>());
                }
                Ok(result)
            }
            "codex_items" => {
                let mut params = args.clone();
                params["sortDirection"] = json!("asc");
                for key in ["offset", "length", "expectedHash"] {
                    params.as_object_mut().unwrap().remove(key);
                }
                let page = self.request("thread/items/list", params, 60000).await?;
                let items: Vec<_> = page["data"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|r| {
                        let mut i = item(
                            &r["item"],
                            num(&args, "offset", 0),
                            num(&args, "length", 6000),
                        );
                        i["turnId"] = r["turnId"].clone();
                        i
                    })
                    .collect();
                if let Some(h) = args["expectedHash"].as_str() {
                    if items.first().is_none_or(|i| i["textHash"] != h) {
                        return Err("CONTENT_CHANGED".into());
                    }
                }
                Ok(
                    json!({"threadId":args["threadId"],"items":items,"nextCursor":page["nextCursor"],"observedAt":now()}),
                )
            }
            "codex_capabilities" => Ok(
                json!({"version":env!("CARGO_PKG_VERSION"),"backendSession":self.session,"executionOwner":if self.auto_open_codex()? {"desktop"} else {"connector"},"models":self.request("model/list",json!({}),60000).await?,"configuration":self.request("config/read",json!({}),60000).await?}),
            ),
            _ => Err("UNKNOWN_TOOL".into()),
        }
    }
}
fn task_details(operation: &str, args: &Value) -> Result<Option<Value>> {
    let kind = match operation {
        "create" | "send" | "interrupt" => operation,
        "native" => match string(args, "method") {
            "thread/start" => "create",
            "turn/start" | "turn/steer" | "thread/resume" => "send",
            "turn/interrupt" => "interrupt",
            _ => return Ok(None),
        },
        _ => return Ok(None),
    };
    let native = operation == "native";
    let p = if native {
        args["params"].clone()
    } else {
        configured(args, "")?
    };
    let input = if native {
        p["input"].clone()
    } else if kind != "interrupt" {
        task_input(args)?
    } else {
        json!([])
    };
    let prompt = input
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|v| v["text"].as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let model = p
        .get("model")
        .or_else(|| args["thread"].get("model"))
        .cloned()
        .unwrap_or(Value::Null);
    Ok(Some(
        json!({"kind":kind,"title":args["title"],"prompt":prompt,"input":input,
        "project":args["project"],"directory":if native{p["cwd"].clone()}else{args["thread"]["cwd"].clone()},
        "threadId":if native{p["threadId"].clone()}else{args["threadId"].clone()},
        "model":model,"effort":p["effort"],
        "settings":if native{json!({"params":p})}else{json!({"thread":args["thread"],"turn":args["turn"],"resume":args["resume"],"mode":args["mode"],"networkAccess":args["networkAccess"],"serviceTier":args["serviceTier"]})}}),
    ))
}
fn task_input(args: &Value) -> Result<Value> {
    desktop::input(
        &args
            .get("input")
            .cloned()
            .unwrap_or_else(|| json!([{"type":"text","text":args["prompt"]}])),
    )
}
fn configured(args: &Value, cwd: &str) -> Result<Value> {
    let mut p = json!({});
    for k in ["model", "effort", "serviceTier"] {
        if let Some(v) = args.get(k) {
            p[k] = v.clone();
        }
    }
    if let Some(o) = args["turn"].as_object() {
        for (k, v) in o {
            p[k] = v.clone();
        }
    }
    if !p["sandboxPolicy"].is_object() && p["permissions"].is_null() {
        if let Some(mode) = args["mode"].as_str() {
            p["sandboxPolicy"] = match mode {
                "danger-full-access" => json!({"type":"dangerFullAccess"}),
                "read-only" => {
                    json!({"type":"readOnly","networkAccess":args["networkAccess"].as_bool().unwrap_or(false)})
                }
                _ => {
                    json!({"type":"workspaceWrite","writableRoots":if cwd.is_empty(){vec![]}else{vec![cwd]},"networkAccess":args["networkAccess"].as_bool().unwrap_or(false),"excludeTmpdirEnvVar":false,"excludeSlashTmp":false})
                }
            };
        }
    }
    if args.get("networkAccess").is_some() && p["sandboxPolicy"].is_null() {
        return Err("networkAccess requires mode or sandboxPolicy".into());
    }
    Ok(p)
}
pub(crate) fn thread(s: &Value, complete: bool) -> Result<Value> {
    let mut t = json!({"id":s["id"],"name":s["title"],"cwd":s["cwd"],"path":s["rolloutPath"],"createdAt":s["createdAt"].as_u64().unwrap_or(0)/1000,"updatedAt":s["updatedAt"].as_u64().unwrap_or(0)/1000,"model":s["latestModel"],"reasoningEffort":s["latestReasoningEffort"],"status":s["threadRuntimeStatus"],"runtimeSource":"codex-desktop-owner"});
    if complete {
        t["turns"] = json!(desktop::turns(s)?)
    }
    Ok(t)
}
fn page(kind: &str, args: &Value, data: Vec<Value>) -> Result<Value> {
    let binding = json!({"kind":kind,"threadId":args["threadId"],"turnId":args["turnId"],"direction":args["sortDirection"]});
    let mut offset = 0;
    if let Some(cursor) = args["cursor"].as_str() {
        let bytes = URL_SAFE_NO_PAD
            .decode(
                cursor
                    .strip_prefix("desktop:")
                    .ok_or("DESKTOP_CURSOR_MISMATCH")?,
            )
            .map_err(|e| e.to_string())?;
        let c: Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        if c["binding"] != binding {
            return Err("DESKTOP_CURSOR_MISMATCH".into());
        }
        offset = num(&c, "offset", 0);
    }
    let limit = num(args, "limit", 50).clamp(1, 100);
    let end = offset.saturating_add(limit);
    Ok(
        json!({"data":data.iter().skip(offset).take(limit).collect::<Vec<_>>(),"nextCursor":if end<data.len(){Some(format!("desktop:{}",URL_SAFE_NO_PAD.encode(json!({"binding":binding,"offset":end}).to_string())))}else{None}}),
    )
}
fn summary(t: &Value) -> Value {
    let state = match t["status"]["type"].as_str() {
        Some("notLoaded") | None => "unknown",
        Some(v) => v,
    };
    json!({"threadId":t["id"],"project":t["cwd"],"title":t["name"],"preview":t["preview"],"cwd":t["cwd"],"runtimeStatus":state,"runtimeSource":if state=="unknown"{json!("persisted-history-only")}else{t["runtimeSource"].clone()},"configuration":{"scope":"thread-defaults","source":"thread/read-or-list","model":t.get("model").unwrap_or(&json!("unknown")),"effort":t.get("reasoningEffort").unwrap_or(&json!("unknown")),"modelProvider":t.get("modelProvider").unwrap_or(&json!("unknown")),"turnEffective":"unknown"},"actions":{"read":true,"send":if state=="idle"{"new-turn"}else if state=="active"{"steer"}else{"requires-native-ownership-check"},"interrupt":"native-ownership-check","configurationChange":if state=="idle"{"new-turn-and-subsequent"}else{"requires-idle"}},"activeFlags":t["status"].get("activeFlags").unwrap_or(&json!([])),"updatedAt":t["updatedAt"],"observedAt":now(),"businessDelivery":"not-assessed","desktopUrl":format!("codex://threads/{}",string(t,"id"))})
}
pub(crate) fn item(v: &Value, offset: usize, len: usize) -> Value {
    let text = match string(v, "type") {
        "agentMessage" => string(v, "text").to_owned(),
        "userMessage" => v["content"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|v| v["type"] == "text")
            .map(|v| string(v, "text"))
            .collect::<Vec<_>>()
            .join("\n"),
        _ => v.to_string(),
    };
    let units: Vec<u16> = text.encode_utf16().collect();
    let end = offset.saturating_add(len);
    json!({"itemId":v["id"],"type":v["type"],"text":String::from_utf16_lossy(&units[offset.min(units.len())..end.min(units.len())]),"textHash":hash(&text),"offset":offset,"textLength":units.len(),"nextOffset":if end<units.len(){Some(end)}else{None},"textTruncated":end<units.len()})
}
pub fn failure(e: &str) -> Value {
    let rpc = e
        .strip_prefix("RPC_REJECTED:")
        .and_then(|s| serde_json::from_str::<Value>(s).ok());
    json!({"code":if rpc.is_some(){"NATIVE_RPC_ERROR"}else{if e == "REQUEST_ID_CONFLICT" { "REQUEST_ID_CONFLICT" } else { "CONTROL_ERROR" }},"message":e,"executionState":if rpc.is_some(){"rejected"}else{"unknown"},"rpcError":rpc,"nextAction":"read-request-before-retry"})
}
pub fn page_output(result: Value) -> Result<Value> {
    let text = result.to_string();
    if text.len() <= 64 * 1024 {
        return Ok(result);
    }
    let output = id();
    save(
        &root().join("outputs").join(format!("{output}.json")),
        &result,
    )?;
    Ok(
        json!({"outputId":output,"bytes":text.len(),"characters":text.encode_utf16().count(),"sha256":hash(&text),"nextAction":"control_output","format":"JSON; offsets count UTF-16 code units"}),
    )
}
fn read_output(args: &Value) -> Result<Value> {
    let output = string(args, "outputId");
    uuid::Uuid::parse_str(output).map_err(|_| "INVALID_OUTPUT_ID")?;
    let dir = root().join("outputs");
    let path = dir.join(format!("{output}.json"));
    let text = std::fs::read_to_string(if path.exists() {
        path
    } else {
        dir.join(format!("{output}.jsonl"))
    })
    .map_err(|e| e.to_string())?;
    let units: Vec<u16> = text.encode_utf16().collect();
    let offset = num(args, "offset", 0);
    let end = offset
        .saturating_add(num(args, "length", 10000).min(12000))
        .min(units.len());
    Ok(
        json!({"outputId":output,"offset":offset,"text":String::from_utf16_lossy(&units[offset.min(end)..end]),"characters":units.len(),"nextOffset":if end<units.len(){Some(end)}else{None}}),
    )
}

fn turn_configurations(thread: &Value) -> HashMap<String, Value> {
    let mut result = HashMap::new();
    let read = || -> Result<String> {
        use std::io::Read;
        let sessions = std::env::var_os("CODEX_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| dirs::home_dir().unwrap().join(".codex"))
            .join("sessions")
            .canonicalize()
            .map_err(|e| e.to_string())?;
        let path = PathBuf::from(string(thread, "path"));
        let canonical = path.canonicalize().map_err(|e| e.to_string())?;
        if canonical != path
            || !path.starts_with(sessions)
            || path.extension().is_none_or(|e| e != "jsonl")
        {
            return Err("invalid history path".into());
        }
        let mut opts = std::fs::OpenOptions::new();
        opts.read(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            opts.custom_flags(libc::O_NOFOLLOW);
        }
        let file = opts.open(path).map_err(|e| e.to_string())?;
        let stat = file.metadata().map_err(|e| e.to_string())?;
        if !stat.is_file() || stat.len() > 32 * 1024 * 1024 {
            return Err("history too large".into());
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            if stat.nlink() != 1 {
                return Err("linked history".into());
            }
        }
        let mut text = String::new();
        file.take(32 * 1024 * 1024)
            .read_to_string(&mut text)
            .map_err(|e| e.to_string())?;
        Ok(text)
    };
    if let Ok(text) = read() {
        let mut matched = false;
        for line in text.lines().filter(|l| l.len() <= 1024 * 1024) {
            let Ok(event) = serde_json::from_str::<Value>(line) else {
                continue;
            };
            let p = &event["payload"];
            if event["type"] == "session_meta" {
                matched = p["id"] == thread["id"];
            }
            if matched
                && event["type"] == "turn_context"
                && p["cwd"] == thread["cwd"]
                && p["turn_id"].is_string()
            {
                let value = |key: &str| p.get(key).cloned().unwrap_or(json!("unknown"));
                result.insert(string(p,"turn_id").into(), json!({"scope":"turn","source":"native-persisted-turn-context","turnId":p["turn_id"],"model":value("model"),"effort":value("effort"),"mode":p["sandbox_policy"].get("type").unwrap_or(&json!("unknown")),"networkAccess":p["sandbox_policy"].get("network_access").unwrap_or(&json!("unknown")),"serviceTier":value("service_tier"),"approvalPolicy":value("approval_policy"),"observedAt":now()}));
            }
        }
    }
    result
}
