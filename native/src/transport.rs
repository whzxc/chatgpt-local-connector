use crate::service::Service;
use crate::*;
use std::time::Duration;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
};
pub enum DesktopRequest {
    PanelGet,
    PanelSet(Value),
    SubscriptionsOpen(Value),
}
pub trait DesktopAccess: Send + Sync {
    fn check_write(&self) -> Result<()>;
    fn request(
        &self,
        request: DesktopRequest,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Value>> + Send>>;
}

pub async fn listen(
    service: Arc<Service>,
    desktop: Option<Arc<dyn DesktopAccess>>,
) -> Result<tokio::task::JoinHandle<()>> {
    service.subscriptions.start();
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| e.to_string())?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    service
        .port
        .store(port, std::sync::atomic::Ordering::SeqCst);
    save(
        &root().join("web/native.json"),
        &json!({"port":port,"pid":std::process::id(),"token":service.token,"instance":service.control.session,"runtime":"rust"}),
    )?;
    Ok(tokio::spawn(async move {
        loop {
            let Ok((stream, peer)) = listener.accept().await else {
                break;
            };
            if !peer.ip().is_loopback() {
                continue;
            }
            let s = service.clone();
            let desktop = desktop.clone();
            tokio::spawn(async move {
                let _ = handle(stream, s, desktop).await;
            });
        }
    }))
}
async fn handle(
    stream: TcpStream,
    s: Arc<Service>,
    desktop: Option<Arc<dyn DesktopAccess>>,
) -> Result<()> {
    let mut reader = BufReader::new(stream);
    let request = tokio::time::timeout(Duration::from_secs(5), async {
        let mut head = String::new();
        loop {
            let bytes = crate::line(&mut reader, 16384).await?;
            if bytes.is_empty() {
                return Err("empty request".into());
            }
            let line = String::from_utf8(bytes).map_err(|_| "invalid header")?;
            head.push_str(&line);
            if head.len() > 16384 {
                return Err("headers too large".into());
            }
            if line == "\r\n" {
                break;
            }
        }
        let mut lines = head.split("\r\n");
        let first = lines.next().unwrap_or("");
        let fields: Vec<_> = first.split_whitespace().collect();
        if fields.len() != 3 {
            return Err("bad request".into());
        }
        let method = fields[0].to_owned();
        let path = fields[1].to_owned();
        let headers: std::collections::HashMap<_, _> = lines
            .filter_map(|l| l.split_once(':'))
            .map(|(k, v)| (k.to_lowercase(), v.trim().to_owned()))
            .collect();
        let port = s.port.load(std::sync::atomic::Ordering::SeqCst);
        if headers.get("host").map(String::as_str) != Some(format!("127.0.0.1:{port}").as_str()) {
            return Err("invalid host".into());
        }
        if headers.get("authorization") != Some(&format!("Bearer {}", s.token)) {
            return Err("unauthorized".into());
        }
        if let Some(origin) = headers.get("origin") {
            if origin != &format!("http://127.0.0.1:{port}") {
                return Err("invalid origin".into());
            }
        }
        if headers.contains_key("transfer-encoding") {
            return Err("unsupported encoding".into());
        }
        let len = headers
            .get("content-length")
            .map(|v| v.parse::<usize>())
            .transpose()
            .map_err(|_| "bad length")?
            .unwrap_or(0);
        if len > 16 * 1024 * 1024 {
            return Err("body too large".into());
        }
        let mut bytes = vec![0; len];
        reader
            .read_exact(&mut bytes)
            .await
            .map_err(|e| e.to_string())?;
        let body = if len == 0 {
            json!({})
        } else {
            serde_json::from_slice(&bytes).map_err(|e| e.to_string())?
        };
        Ok::<_, String>((method, path, body))
    })
    .await
    .map_err(|_| "request timeout")?;
    let mut stream = reader.into_inner();
    let (method, path, body) = match request {
        Ok(v) => v,
        Err(e) => {
            reply(&mut stream, 403, json!({"error":e})).await?;
            return Ok(());
        }
    };
    if method == "GET" && path == "/api/subscriptions/events" {
        let mut changes = s.subscriptions.subscribe();
        stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n").await.map_err(|e|e.to_string())?;
        loop {
            let packet = format!(
                "event: snapshot\ndata: {}\n\n",
                serde_json::to_string(&*changes.borrow_and_update()).map_err(|e| e.to_string())?
            );
            if stream.write_all(packet.as_bytes()).await.is_err() {
                return Ok(());
            }
            tokio::select! {
                result = changes.changed() => if result.is_err() { return Ok(()); },
                _ = tokio::time::sleep(Duration::from_secs(15)) => {},
            }
        }
    }
    if method == "GET" && path == "/api/events" {
        stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n").await.map_err(|e|e.to_string())?;
        loop {
            let state = s.status().await?;
            let packet = format!("event: snapshot\ndata: {}\n\n", state);
            if stream.write_all(packet.as_bytes()).await.is_err() {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    }
    let result = if (path == "/mcp" || path.starts_with("/mcp/")) && method == "POST" {
        let ingress = if let Some(id) = path.strip_prefix("/mcp/") {
            s.ingress(id).await?
        } else {
            s.primary().await?
        };
        if body["method"] == "tools/call" && is_wait(&body) {
            let mut disconnected = [0u8; 1];
            tokio::select! {
                result = mcp_ingress(&ingress, body) => result,
                _ = stream.read(&mut disconnected) => return Ok(()),
            }
        } else {
            mcp_ingress(&ingress, body).await
        }
    } else if method == "GET" && path == "/healthz" {
        Ok(json!({"runtime":"rust","instance":s.control.session,"pid":std::process::id()}))
    } else if let Some(route) = path.strip_prefix("/api/") {
        async {
            if method != "GET" {
                if let Some(owner) = &desktop {
                    owner.check_write()?;
                }
            }
            let operation = match (route, method.as_str()) {
                ("usage-panel", "GET") => Some(DesktopRequest::PanelGet),
                ("usage-panel", "PUT") => Some(DesktopRequest::PanelSet(body.clone())),
                ("subscriptions/open", "POST") => {
                    Some(DesktopRequest::SubscriptionsOpen(body.clone()))
                }
                _ => None,
            };
            if let Some(operation) = operation {
                return desktop
                    .as_ref()
                    .ok_or("desktop access unavailable")?
                    .request(operation)
                    .await;
            }
            s.request(route, &method, body).await
        }
        .await
    } else {
        Err("not found".into())
    };
    match result {
        Ok(v) => reply(&mut stream, 200, v).await,
        Err(e) => reply(&mut stream, 400, json!({"error":e})).await,
    }
}
async fn reply(s: &mut TcpStream, code: u16, v: Value) -> Result<()> {
    let body = v.to_string();
    let head=format!("HTTP/1.1 {code} {}\r\nContent-Type: application/json\r\nCache-Control: no-store\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",if code==200{"OK"}else{"Error"},body.len());
    s.write_all(head.as_bytes())
        .await
        .map_err(|e| e.to_string())?;
    s.write_all(body.as_bytes())
        .await
        .map_err(|e| e.to_string())
}
fn is_wait(request: &Value) -> bool {
    matches!(
        string(&request["params"], "name"),
        "codex_wait" | "agent_wait"
    )
}
async fn call_tool(s: &Arc<crate::ingress::Ingress>, name: &str, args: Value) -> Result<Value> {
    // A closed ingress and invalid verification challenge remain protocol errors.
    if !s.connected.load(std::sync::atomic::Ordering::SeqCst) {
        return Err("Connection closed".into());
    }
    let result = s.call(name, args).await;
    if name == "connector_verify" && result.is_err() {
        return result;
    }
    let (value, error) = match result {
        Ok(value) => (crate::kernel::results::page_output(value)?, false),
        Err(error) => (json!({"error":crate::egress::failure(&error)}), true),
    };
    Ok(crate::egress::tool(value, error))
}
pub async fn mcp(s: &Arc<Service>, request: Value) -> Result<Value> {
    mcp_ingress(&s.primary().await?, request).await
}
pub async fn mcp_ingress(s: &Arc<crate::ingress::Ingress>, request: Value) -> Result<Value> {
    let origin = s.origin().await;
    crate::kernel::origin::ORIGIN
        .scope(origin, dispatch(s, request))
        .await
}
async fn dispatch(s: &Arc<crate::ingress::Ingress>, request: Value) -> Result<Value> {
    if request["jsonrpc"] != "2.0" {
        return Err("invalid jsonrpc".into());
    }
    let id = request.get("id").cloned();
    let method = string(&request, "method");
    if method == "tools/call" && !s.allows(string(&request["params"], "name")).await {
        return Ok(
            json!({"jsonrpc":"2.0","id":id,"error":{"code":-32601,"message":"tool not available"}}),
        );
    }
    let result = match method {
        "initialize" => {
            let protocol = request["params"]["protocolVersion"]
                .as_str()
                .unwrap_or("2025-03-26");
            Ok(
                json!({"protocolVersion":if ["2024-11-05","2025-03-26","2025-06-18","2025-11-25"].contains(&protocol){protocol}else{"2025-03-26"},"capabilities":{"tools":{}},"serverInfo":{"name":"chatgpt-local-connector","version":env!("CARGO_PKG_VERSION")},"instructions":catalog()["instructions"]}),
            )
        }
        "ping" => Ok(json!({})),
        "tools/list" => {
            let mut tools = Vec::new();
            for tool in catalog()["tools"].as_array().unwrap() {
                if s.allows(string(tool, "name")).await {
                    tools.push(tool.clone());
                }
            }
            Ok(json!({"tools":tools}))
        }
        "tools/call" if is_wait(&request) => {
            let key = id.as_ref().ok_or("wait requires a request id")?.to_string();
            let (guard, mut cancel) = s.wait_calls.register(key)?;
            let args = request["params"]
                .get("arguments")
                .cloned()
                .unwrap_or(json!({}));
            let started = std::time::Instant::now();
            let response = tokio::select! {
                result = call_tool(s, string(&request["params"], "name"), args.clone()) => result,
                _ = cancel.changed() => {
                    let mut value = json!({"state":"unconfirmed","reason":"wait-cancelled","threadId":args["threadId"],"turnId":args["turnId"],"runtimeStatus":"unknown","recordedStatus":null,"finalResponse":null,"interaction":[],"changed":false,"conditionMet":false,"observedAt":null,"returnedAt":now(),"elapsedMs":started.elapsed().as_millis() as u64});
                    if request["params"]["name"] == "agent_wait" {
                        value["agent"] = args["agent"].clone();
                        value["taskId"] = args["taskId"].clone();
                    }
                    Ok(crate::egress::tool(value, false))
                }
            };
            drop(guard);
            response
        }
        "tools/call" => {
            call_tool(
                s,
                string(&request["params"], "name"),
                request["params"]
                    .get("arguments")
                    .cloned()
                    .unwrap_or(json!({})),
            )
            .await
        }
        "notifications/cancelled" => {
            s.wait_calls
                .cancel(&request["params"]["requestId"].to_string());
            return Ok(Value::Null);
        }
        "notifications/initialized" => return Ok(Value::Null),
        _ => Err("method not found".into()),
    };
    if id.is_none() {
        return Ok(Value::Null);
    }
    Ok(match result {
        Ok(v) => json!({"jsonrpc":"2.0","id":id,"result":v}),
        Err(e) => {
            json!({"jsonrpc":"2.0","id":id,"error":{"code":if e=="method not found"{-32601}else{-32603},"message":e}})
        }
    })
}
pub async fn stdio() -> Result<()> {
    init_crypto();
    let port = std::env::var("CLC_NATIVE_PORT")
        .map_err(|_| "MCP must be launched by Connector")?
        .parse::<u16>()
        .map_err(|_| "invalid port")?;
    let token = std::env::var("CLC_NATIVE_TOKEN").map_err(|_| "missing connector token")?;
    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|e| e.to_string())?;
    let ingress = std::env::var("CLC_INGRESS_ID").map_err(|_| "missing ingress id")?;
    let stdout = Arc::new(Mutex::new(tokio::io::stdout()));
    let mut reader = BufReader::new(tokio::io::stdin());
    let mut jobs = tokio::task::JoinSet::new();
    loop {
        let bytes = crate::line(&mut reader, 16 * 1024 * 1024).await?;
        if bytes.is_empty() {
            break;
        }
        let request: Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        let (client, token, out) = (client.clone(), token.clone(), stdout.clone());
        let ingress = ingress.clone();
        jobs.spawn(async move {
            let response = client
                .post(format!("http://127.0.0.1:{port}/mcp/{ingress}"))
                .bearer_auth(token)
                .timeout(Duration::from_secs(if is_wait(&request) {
                    330
                } else {
                    120
                }))
                .json(&request)
                .send()
                .await;
            let value = match response {
                Ok(r) => r.json::<Value>().await.map_err(|e| e.to_string()),
                Err(e) => Err(e.to_string()),
            };
            let response = match value {
                Ok(v) => v,
                Err(e) => {
                    json!({"jsonrpc":"2.0","id":request["id"],"error":{"code":-32603,"message":e}})
                }
            };
            if request.get("id").is_some() && !response.is_null() {
                let mut out = out.lock().await;
                let _ = out.write_all(format!("{response}\n").as_bytes()).await;
                let _ = out.flush().await;
            }
        });
        while jobs.try_join_next().is_some() {}
    }
    jobs.abort_all();
    Ok(())
}

pub async fn forward_request(route: &str, method: &str, body: Value) -> Result<Value> {
    crate::init_crypto();
    let unavailable = "无法连接后台，请先打开 Local Connector 桌面应用。";
    let info = load(&root().join("web/native.json")).map_err(|_| unavailable)?;
    let port = info["port"]
        .as_u64()
        .filter(|p| *p > 0 && *p <= 65535)
        .ok_or(unavailable)?;
    let client = reqwest::Client::builder()
        .no_proxy()
        .build()
        .map_err(|e| e.to_string())?;
    let health: Value = client
        .get(format!("http://127.0.0.1:{port}/healthz"))
        .bearer_auth(string(&info, "token"))
        .timeout(Duration::from_secs(2))
        .send()
        .await
        .map_err(|_| unavailable)?
        .error_for_status()
        .map_err(|_| unavailable)?
        .json()
        .await
        .map_err(|_| unavailable)?;
    if health["instance"] != info["instance"] {
        return Err(unavailable.into());
    }
    let method = reqwest::Method::from_bytes(method.as_bytes()).map_err(|e| e.to_string())?;
    let mut request = client
        .request(
            method.clone(),
            format!("http://127.0.0.1:{port}/api/{route}"),
        )
        .bearer_auth(string(&info, "token"))
        .timeout(Duration::from_secs(
            if route == "start" || route.ends_with("/start") || route.ends_with("/start-all") {
                360
            } else {
                120
            },
        ));
    if method != reqwest::Method::GET {
        request = request.json(&body);
    }
    let response = request.send().await.map_err(|e| e.to_string())?;
    let success = response.status().is_success();
    let value: Value = response.json().await.map_err(|e| e.to_string())?;
    if success {
        Ok(value)
    } else {
        Err(string(&value, "error").into())
    }
}

// Stateless ingress has no MCP sessions. Active wait request IDs must be unique
// across callers; collisions are rejected rather than cancelling another wait.
#[derive(Default)]
pub(crate) struct WaitCalls(
    Arc<std::sync::Mutex<std::collections::HashMap<String, tokio::sync::watch::Sender<bool>>>>,
);
struct WaitCallGuard {
    calls:
        Arc<std::sync::Mutex<std::collections::HashMap<String, tokio::sync::watch::Sender<bool>>>>,
    key: String,
}
impl Drop for WaitCallGuard {
    fn drop(&mut self) {
        self.calls.lock().unwrap().remove(&self.key);
    }
}
impl WaitCalls {
    fn register(&self, key: String) -> Result<(WaitCallGuard, tokio::sync::watch::Receiver<bool>)> {
        let mut calls = self.0.lock().unwrap();
        if calls.contains_key(&key) {
            return Err("duplicate active wait request id".into());
        }
        let (tx, rx) = tokio::sync::watch::channel(false);
        calls.insert(key.clone(), tx);
        Ok((
            WaitCallGuard {
                calls: self.0.clone(),
                key,
            },
            rx,
        ))
    }
    fn cancel(&self, key: &str) {
        if let Some(tx) = self.0.lock().unwrap().get(key) {
            let _ = tx.send(true);
        }
    }
}
