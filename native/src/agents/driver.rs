use super::{process::Process, *};

#[derive(Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub id: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
}
#[derive(Clone)]
pub enum AgentDriver {
    CodexNative,
    Pi,
    Acp(Manifest),
}
impl AgentDriver {
    pub fn protocol(&self) -> &str {
        match self {
            Self::CodexNative => "codex-native",
            Self::Pi => "pi-rpc",
            Self::Acp(_) => "acp-v1",
        }
    }
    pub fn binary(&self) -> Option<PathBuf> {
        match self {
            Self::CodexNative => crate::desktop::installation().map(|i| i.binary),
            Self::Pi => discover("pi"),
            Self::Acp(m) => discover(&m.command),
        }
    }
    pub async fn start(&self, task: Value, resume: bool) -> Result<Arc<Process>> {
        let binary = self.binary().ok_or("AGENT_NOT_INSTALLED")?;
        let cwd = PathBuf::from(string(&task, "cwd"));
        let args = match self {
            Self::Pi => {
                let mut args = vec![
                    "--mode".into(),
                    "rpc".into(),
                    "--session-dir".into(),
                    root()
                        .join("agents/pi-sessions")
                        .to_string_lossy()
                        .into_owned(),
                ];
                if resume {
                    let path = string(&task["metadata"], "sessionFile");
                    if path.is_empty() {
                        return Err("SESSION_NOT_PERSISTED".into());
                    }
                    args.extend(["--session".into(), path.into()]);
                }
                args
            }
            Self::Acp(m) => m.args.clone(),
            Self::CodexNative => return Err("USE_NATIVE_CONTROL".into()),
        };
        let p = Process::start(&binary, &args, &cwd, matches!(self, Self::Pi), task).await?;
        let init=async {
            if p.pi {
                let state=p.call("get_state",json!({}),15000).await?;
                *p.capabilities.lock().await=json!({"protocol":"pi-rpc","resume":true,"cancel":true,"streaming":true,"permissions":"extension-ui-only","completionEvent":"agent_settled"});
                let mut t=p.state.lock().await;t["sessionId"]=state["sessionId"].clone();t["metadata"]=state;
            } else {
                let init=p.call("initialize",json!({"protocolVersion":1,"clientCapabilities":{},"clientInfo":{"name":"local-connector","version":env!("CARGO_PKG_VERSION")}}),15000).await?;
                if init["protocolVersion"]!=1 {return Err("ACP_VERSION_UNSUPPORTED".into());}
                *p.capabilities.lock().await=init.clone();
                let t=p.state.lock().await.clone();
                if resume && init["agentCapabilities"]["loadSession"]!=true {return Err("RESUME_UNSUPPORTED".into());}
                let mut params=json!({"cwd":cwd,"mcpServers":[]});
                if resume {params["sessionId"]=t["sessionId"].clone();}
                let result=p.call(if resume {"session/load"}else{"session/new"},params,60000).await?;
                let mut t=p.state.lock().await;
                if !resume {if string(&result,"sessionId").is_empty(){return Err("MISSING_SESSION_ID".into());}t["sessionId"]=result["sessionId"].clone();}
                t["metadata"]=result;
            }
            let mut t=p.state.lock().await;t["status"]=json!("idle");t["processState"]=json!("running");save_task(&t)?;
            Ok(())
        }.await;
        if let Err(e) = init {
            p.close().await;
            return Err(e);
        }
        Ok(p)
    }
    pub async fn prompt(&self, p: &Arc<Process>, text: &str) -> Result<Value> {
        {
            let mut t = p.state.lock().await;
            if !matches!(
                string(&t, "status"),
                "idle" | "completed" | "cancelled" | "failed"
            ) {
                return Err("TASK_BUSY_OR_UNCONFIRMED".into());
            }
            t["status"] = json!("running");
            t["turnId"] = json!(id());
            t["output"] = json!("");
            t["lastMessage"] = Value::Null;
            t["error"] = Value::Null;
            t["stopReason"] = Value::Null;
            t["outputTruncated"] = json!(false);
            save_task(&t)?;
        }
        let sid = p.state.lock().await["sessionId"].clone();
        let result = if p.pi {
            p.call("prompt", json!({"message":text}), 30000).await
        } else {
            p.call(
                "session/prompt",
                json!({"sessionId":sid,"prompt":[{"type":"text","text":text}]}),
                86400000,
            )
            .await
        };
        let mut t = p.state.lock().await;
        match &result {
            Ok(v) if !p.pi => {
                t["status"] = json!(if v["stopReason"] == "cancelled" {
                    "cancelled"
                } else {
                    "completed"
                });
                t["stopReason"] = v["stopReason"].clone();
            }
            Err(e) => {
                t["status"] = json!(if e.starts_with("RPC_REJECTED") {
                    "failed"
                } else {
                    "unknown"
                });
                t["error"] = json!(e);
            }
            _ => {}
        }
        t["updatedAt"] = json!(now());
        save_task(&t)?;
        result.map(|_| t.clone())
    }
    pub async fn interrupt(&self, p: &Arc<Process>) -> Result<Value> {
        let requests = p
            .events
            .lock()
            .await
            .pending
            .values()
            .cloned()
            .collect::<Vec<_>>();
        for request in requests {
            self.respond(p, &json!({"interactionId":request["id"],"cancelled":true}))
                .await?;
        }
        if p.pi {
            p.call("abort", json!({}), 30000).await?;
        } else {
            let sid = p.state.lock().await["sessionId"].clone();
            p.write(json!({"jsonrpc":"2.0","method":"session/cancel","params":{"sessionId":sid}}))
                .await?;
        }
        let mut t = p.state.lock().await;
        if !matches!(
            string(&t, "status"),
            "idle" | "completed" | "cancelled" | "failed"
        ) {
            t["status"] = json!(if p.pi { "cancelled" } else { "cancelling" });
        }
        save_task(&t)?;
        Ok(t.clone())
    }
    pub async fn respond(&self, p: &Arc<Process>, a: &Value) -> Result<Value> {
        let key = a["interactionId"].to_string();
        let mut pending = p.events.lock().await;
        let req = pending.pending.get(&key).ok_or("INTERACTION_NOT_PENDING")?;
        let response = if p.pi {
            let mut v = json!({"type":"extension_ui_response","id":req["id"]});
            if a["cancelled"] == true {
                v["cancelled"] = json!(true);
            } else {
                match string(req, "method") {
                    "confirm" => {
                        if !a["confirmed"].is_boolean() {
                            return Err("CONFIRMATION_REQUIRED".into());
                        }
                        v["confirmed"] = a["confirmed"].clone();
                    }
                    _ => {
                        if !a["value"].is_string() {
                            return Err("VALUE_REQUIRED".into());
                        }
                        if req["method"] == "select"
                            && !req["options"]
                                .as_array()
                                .is_some_and(|o| o.contains(&a["value"]))
                        {
                            return Err("INVALID_OPTION".into());
                        }
                        v["value"] = a["value"].clone();
                    }
                }
            }
            v
        } else {
            let outcome = if a["cancelled"] == true {
                json!({"outcome":"cancelled"})
            } else {
                if !req["params"]["options"]
                    .as_array()
                    .is_some_and(|v| v.iter().any(|o| o["optionId"] == a["optionId"]))
                {
                    return Err("INVALID_OPTION".into());
                }
                json!({"outcome":"selected","optionId":a["optionId"]})
            };
            json!({"jsonrpc":"2.0","id":req["id"],"result":{"outcome":outcome}})
        };
        p.write(response).await?;
        pending.pending.remove(&key);
        let empty = pending.pending.is_empty();
        drop(pending);
        let mut t = p.state.lock().await;
        if empty && t["status"] == "waiting-permission" {
            t["status"] = json!("running");
        }
        save_task(&t)?;
        Ok(json!({"responded":true}))
    }
}
/// GUI launches often have a minimal PATH. Known user-owned CLI directories are also searched.
pub(super) fn search_paths() -> Vec<PathBuf> {
    let mut paths =
        std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()).collect::<Vec<_>>();
    if let Some(home) = dirs::home_dir() {
        paths.extend([
            home.join(".local/bin"),
            home.join(".opencode/bin"),
            home.join(".bun/bin"),
        ]);
    }
    #[cfg(unix)]
    paths.extend([
        PathBuf::from("/opt/homebrew/bin"),
        PathBuf::from("/usr/local/bin"),
    ]);
    #[cfg(windows)]
    if let Some(app) = dirs::config_dir() {
        paths.push(app.join("npm"));
    }
    paths
}
pub fn discover(name: &str) -> Option<PathBuf> {
    let p = Path::new(name);
    if p.is_absolute() {
        return p.is_file().then(|| p.into());
    }
    for path in search_paths() {
        for suffix in if cfg!(windows) {
            vec![".exe", ".cmd", ""]
        } else {
            vec![""]
        } {
            let p = path.join(format!("{name}{suffix}"));
            if p.is_file() {
                return Some(p);
            }
        }
    }
    None
}
