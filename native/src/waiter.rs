//! Bounded, read-only waiting. Notifications are hints; only fresh owner reads decide state.
use crate::{control::Control, desktop::Ipc, rpc::Rpc, *};
use std::{future::Future, time::Duration};
use tokio::{
    sync::{watch, Notify},
    time::Instant,
};

/// Agent adapters provide a fresh observation and a cheap connection health check.
/// No registry/global mutex is held while this engine waits or reads the owner.
pub(crate) trait WaitSource {
    fn observe(&mut self) -> impl Future<Output = Result<Value>> + Send;
    fn alive(&self) -> bool;
    fn classify(&mut self, observation: &Value, selected: &mut Option<String>) -> Value;
    fn deadline_is_timeout(&self) -> bool {
        false
    }
}

pub(crate) async fn wait(
    source: &mut impl WaitSource,
    wake: Arc<Notify>,
    mut shutdown: watch::Receiver<u64>,
    args: &Value,
    started: Instant,
) -> Value {
    let deadline = started + Duration::from_millis(args["timeoutMs"].as_u64().unwrap_or(60000));
    let mut last = json!({"threadId":args["threadId"],"turnId":args["turnId"],"recordedStatus":null,"runtimeStatus":"unknown","interaction":[],"finalResponse":null});
    let mut baseline = args["expectedHash"].as_str().map(str::to_owned);
    let mut selected = args["turnId"].as_str().map(str::to_owned);
    loop {
        if !source.alive() {
            return finish(
                last,
                "unconfirmed",
                "connection-closed",
                started,
                baseline.as_deref(),
            );
        }
        if shutdown.has_changed().unwrap_or(true) {
            return finish(
                last,
                "unconfirmed",
                "connector-shutdown",
                started,
                baseline.as_deref(),
            );
        }
        if Instant::now() >= deadline {
            return finish(last, "timeout", "deadline", started, baseline.as_deref());
        }
        let observation = tokio::select! {
            biased;
            _ = shutdown.changed() => return finish(last, "unconfirmed", "connector-shutdown", started, baseline.as_deref()),
            result = tokio::time::timeout_at(deadline.min(Instant::now()+Duration::from_secs(10)), source.observe()) => result,
        };
        let observed = match observation {
            Ok(Ok(v)) => v,
            Ok(Err(e)) => return finish(last, "unconfirmed", &e, started, baseline.as_deref()),
            Err(_) if source.deadline_is_timeout() && Instant::now() >= deadline => {
                return finish(last, "timeout", "deadline", started, baseline.as_deref());
            }
            Err(_) => {
                return finish(
                    last,
                    "unconfirmed",
                    "native-read-timeout",
                    started,
                    baseline.as_deref(),
                )
            }
        };
        if !source.alive() {
            return finish(
                last,
                "unconfirmed",
                "connection-closed",
                started,
                baseline.as_deref(),
            );
        }
        last = source.classify(&observed, &mut selected);
        let current_hash = snapshot_hash(&last);
        baseline.get_or_insert(current_hash);
        let state = string(&last, "state").to_owned();
        // Terminal and interaction states always break a wait: an interaction must never
        // be hidden by until=terminal, nor should a finished turn wait for interaction.
        if state != "running" {
            let reason = string(&last, "reason").to_owned();
            return finish(last, &state, &reason, started, baseline.as_deref());
        }
        // Register after the read: Desktop reads themselves publish snapshots. Do not
        // let these self-generated hints cause a read/notification feedback loop.
        // A missed/coalesced hint (including buffer gap/reset) is repaired in 2 seconds.
        let notified = wake.notified();
        tokio::pin!(notified);
        notified.as_mut().enable();
        tokio::select! {
            biased;
            _ = shutdown.changed() => return finish(last, "unconfirmed", "connector-shutdown", started, baseline.as_deref()),
            _ = tokio::time::sleep_until(deadline) => return finish(last, "timeout", "deadline", started, baseline.as_deref()),
            _ = notified => {
                // Coalesce streaming deltas; cap reads at four/second even under load.
                tokio::select! {
                    biased;
                    _ = shutdown.changed() => return finish(last, "unconfirmed", "connector-shutdown", started, baseline.as_deref()),
                    _ = tokio::time::sleep_until(deadline.min(Instant::now()+Duration::from_millis(250))) => {},
                }
            },
            _ = tokio::time::sleep(Duration::from_secs(2)) => {},
        }
    }
}
fn snapshot_hash(v: &Value) -> String {
    hash(json!({"turnId":v["turnId"],"recordedStatus":v["recordedStatus"],"runtimeStatus":v["runtimeStatus"],"interaction":v["interaction"],"finalResponse":v["finalResponse"],"error":v["error"]}).to_string())
}
fn finish(
    mut v: Value,
    state: &str,
    reason: &str,
    started: Instant,
    baseline: Option<&str>,
) -> Value {
    let digest = snapshot_hash(&v);
    v["state"] = json!(state);
    v["reason"] = json!(reason);
    v["changed"] = json!(baseline.is_some_and(|h| h != digest));
    v["snapshotHash"] = json!(digest);
    v["observedAt"] = v.get("observedAt").cloned().unwrap_or(Value::Null);
    v["returnedAt"] = json!(now());
    v["elapsedMs"] = json!(started.elapsed().as_millis() as u64);
    v
}
fn classify(observation: &Value, selected: &mut Option<String>) -> Value {
    let t = &observation["thread"];
    let turns = t["turns"].as_array().cloned().unwrap_or_default();
    if selected.is_none() {
        *selected = turns
            .iter()
            .rev()
            .find(|t| t["status"] == "inProgress")
            .or_else(|| turns.last())
            .and_then(|t| t["id"].as_str())
            .map(str::to_owned);
    }
    let turn = turns
        .iter()
        .find(|t| t["id"].as_str() == selected.as_deref())
        .cloned()
        .unwrap_or(Value::Null);
    let runtime = t["status"]["type"].as_str().unwrap_or("unknown");
    let mut interactions: Vec<_> = observation["pending"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|r| {
            r["params"]["threadId"] == t["id"]
                && (r["params"]["turnId"].is_null()
                    || r["params"]["turnId"].as_str() == selected.as_deref())
        })
        .cloned()
        .map(|mut r| {
            r["responseId"] = json!(r["id"].to_string());
            r
        })
        .collect();
    interactions.sort_by(|a, b| string(a, "responseId").cmp(string(b, "responseId")));
    let flags = &t["status"]["activeFlags"];
    let blocked = flags.as_array().is_some_and(|a| {
        a.iter()
            .any(|f| matches!(f.as_str(), Some("waitingOnApproval" | "waitingOnUserInput")))
    });
    let recorded = string(&turn, "status");
    let (state, reason) = if matches!(runtime, "unknown" | "notLoaded" | "") {
        ("unconfirmed", "runtime-ownership-unknown")
    } else if turn.is_null() {
        ("unconfirmed", "turn-not-found")
    } else if recorded == "completed" {
        ("completed", "native-turn-terminal")
    } else if recorded == "failed" {
        ("failed", "native-turn-terminal")
    } else if matches!(recorded, "interrupted" | "cancelled") {
        ("cancelled", "native-turn-terminal")
    } else if !interactions.is_empty() || blocked {
        ("interaction-required", "native-interaction")
    } else if recorded == "inProgress" && runtime == "active" {
        ("running", "native-turn-active")
    } else {
        ("unconfirmed", "inconsistent-native-state")
    };
    let assistant = turn["items"]
        .as_array()
        .into_iter()
        .flatten()
        .rev()
        .find(|i| i["type"] == "agentMessage")
        .map(|i| crate::control::item(i, 0, 6000));
    json!({"state":state,"reason":reason,"threadId":t["id"],"turnId":selected,"recordedStatus":turn["status"],"runtimeStatus":runtime,
        "runtimeSource":t["runtimeSource"],"finalResponse":assistant,"error":turn["error"],"interaction":interactions,
        "activeFlags":flags,"interactionAction":if blocked && interactions.is_empty(){Some("Inspect Codex Desktop for the pending interaction; this owner exposes no callback ID to Connector")}else if !interactions.is_empty(){Some("Use codex_respond with id=responseId and backendSession; do not resend the task")}else{None},
        "observedAt":now()})
}

enum CodexOwner {
    Background(Arc<Rpc>),
    Desktop(Ipc),
}
struct CodexSource {
    owner: CodexOwner,
    thread: String,
    desktop_owner: Option<String>,
    events: SharedEvents,
}
impl WaitSource for CodexSource {
    fn classify(&mut self, observation: &Value, selected: &mut Option<String>) -> Value {
        classify(observation, selected)
    }
    fn alive(&self) -> bool {
        match &self.owner {
            CodexOwner::Background(r) => r.alive(),
            CodexOwner::Desktop(i) => i.alive(),
        }
    }
    async fn observe(&mut self) -> Result<Value> {
        let mut value = match &self.owner {
            CodexOwner::Background(r) => {
                let mut v = r
                    .call(
                        "thread/read",
                        json!({"threadId":self.thread,"includeTurns":true}),
                        10000,
                    )
                    .await?;
                v["thread"]["runtimeSource"] = json!("connector-app-server");
                v
            }
            CodexOwner::Desktop(i) => {
                let owner = i.owner(&self.thread).await?;
                if self.desktop_owner.as_ref().is_some_and(|o| *o != owner) {
                    return Err("runtime-owner-changed".into());
                }
                self.desktop_owner = Some(owner.clone());
                let state = i.read_owned(&self.thread, true, &owner).await?;
                json!({"thread":crate::control::thread(&state, true)?})
            }
        };
        if value["thread"]["id"] != self.thread {
            return Err("native-thread-id-mismatch".into());
        }
        let events = self.events.lock().await;
        value["pending"] = json!(events.pending.values().cloned().collect::<Vec<_>>());
        Ok(value)
    }
}
pub async fn codex_wait(control: &Arc<Control>, args: &Value) -> Result<Value> {
    let started = Instant::now();
    let mut shutdown = control.shutdown.subscribe();
    let wake = control.events.lock().await.wake.clone();
    let setup = async {
        let background = control.background(string(args, "threadId"))?;
        let owner = if background {
            CodexOwner::Background(control.utility().await?)
        } else {
            CodexOwner::Desktop(Ipc::open(control.events.clone(), &control.session).await?)
        };
        Ok::<_, String>(CodexSource {
            owner,
            thread: string(args, "threadId").into(),
            desktop_owner: None,
            events: control.events.clone(),
        })
    };
    let source = tokio::select! {
        biased;
        _ = shutdown.changed() => Err("connector-shutdown".into()),
        r = tokio::time::timeout(Duration::from_millis(args["timeoutMs"].as_u64().unwrap_or(60000).min(10000)), setup) => r.unwrap_or_else(|_| Err("native-connect-timeout".into())),
    };
    let mut result = match source {
        Ok(mut source) => {
            let wake = match &source.owner {
                CodexOwner::Desktop(ipc) => ipc.changes.clone(),
                CodexOwner::Background(_) => wake,
            };
            wait(&mut source, wake, shutdown, args, started).await
        }
        Err(e) => finish(
            json!({"threadId":args["threadId"],"turnId":args["turnId"],"recordedStatus":null,"runtimeStatus":"unknown","finalResponse":null,"interaction":[]}),
            "unconfirmed",
            &e,
            started,
            None,
        ),
    };
    condition(&mut result, args);
    Ok(result)
}
pub(crate) fn condition(result: &mut Value, args: &Value) {
    let terminal = matches!(
        string(result, "state"),
        "completed" | "failed" | "cancelled"
    );
    let interaction = result["state"] == "interaction-required";
    result["conditionMet"] = json!(match string(args, "until") {
        "terminal" => terminal,
        "interaction-required" => interaction,
        _ => terminal || interaction,
    });
}
