//! Read-only findings derived from already observed state.
use crate::*;
fn finding(
    code: &str,
    severity: &str,
    kind: &str,
    id: &Value,
    summary: &str,
    operation: &str,
) -> Value {
    json!({"code":code,"severity":severity,"scope":{"kind":kind,"id":id},"observedAt":now(),"summary":summary,"nextAction":{"kind":"user-action","operation":operation,"target":id}})
}
pub fn ingress(i: &Value) -> Value {
    let mut findings = Vec::new();
    let id = &i["id"];
    let state = string(i, "state");
    if state == "stopped" && i["running"] == false {
        findings.push(finding(
            "INGRESS_STOPPED",
            "info",
            "ingress",
            id,
            "入口已停止；读取诊断不会启动入口。",
            "inspect-ingress",
        ));
    } else if !string(i, "error").is_empty() || state == "error" {
        findings.push(finding(
            "INGRESS_FAILED",
            "error",
            "ingress",
            id,
            "此入口报告故障；只检查此入口的配置和 Provider。",
            "inspect-ingress",
        ));
    } else if i["config"]["configured"] == false {
        findings.push(finding(
            "INGRESS_NOT_CONFIGURED",
            "info",
            "ingress",
            id,
            "入口配置尚未完成。",
            "configure-ingress",
        ));
    }
    let inbound = if i["verification"]["challengeVerifiedAt"].is_string() {
        "verified"
    } else {
        "not_checked"
    };
    if state == "ready" && inbound != "verified" {
        findings.push(finding(
            "INBOUND_NOT_VERIFIED",
            "warning",
            "ingress",
            id,
            "入口已就绪，但当前配置尚无 challenge 入站验证证据。",
            "verify-inbound",
        ));
    }
    if i["auth"] == "none" {
        findings.push(finding(
            "AUTH_DISABLED",
            "warning",
            "ingress",
            id,
            "此入口未启用身份认证。",
            "inspect-authentication",
        ));
    }
    json!({"findings":findings,"checks":{"transport":{"state":state},"authentication":{"state":if i["auth"]=="none"{"disabled"}else{"configured-not-verified"}},"inbound":{"state":inbound,"observedAt":i["verification"]["challengeVerifiedAt"]},"execution":{"state":"not_checked"},"remoteClient":{"state":"unknown"}},"readOnly":true})
}
pub fn status(s: &Value) -> Value {
    let mut findings: Vec<Value> = s["ingresses"]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|i| {
            ingress(i)["findings"]
                .as_array()
                .cloned()
                .unwrap_or_default()
        })
        .collect();
    for (key, code) in [
        ("desktop", "DESKTOP_UNAVAILABLE"),
        ("appServer", "APP_SERVER_UNAVAILABLE"),
    ] {
        let state = string(&s["core"][key], "state");
        if matches!(state, "unavailable" | "disconnected" | "error") {
            findings.push(finding(
                code,
                "warning",
                "execution-endpoint",
                &json!(key),
                "执行端当前不可用；历史证据可用性需单独读取。",
                "inspect-execution-endpoint",
            ));
        }
    }
    json!({"findings":findings,"checks":{"app":{"state":"reachable"},"execution":{"state":"not_checked"},"remoteClient":{"state":"unknown"},"receipts":{"state":"not_checked","nextAction":"read-original-requestId; never replay an unconfirmed write"}},"readOnly":true})
}

/// Reuse the Control receipt reader; never interpret operation completion as delivery.
pub fn receipts(diagnostics: &mut Value, records: Result<Vec<Value>>) {
    match records {
        Ok(records) => {
            let uncertain: Vec<_> = records
                .iter()
                .filter(|r| r["state"] == "unconfirmed")
                .collect();
            diagnostics["checks"]["receipts"] = json!({"state":if uncertain.is_empty(){"no-unconfirmed-observed"}else{"unconfirmed"},"scope":"codex-task-receipts","count":uncertain.len(),"truncated":uncertain.len()>100});
            for r in uncertain.into_iter().take(100) {
                let mut f = finding(
                    "WRITE_UNCONFIRMED",
                    "warning",
                    "request",
                    &r["requestId"],
                    "写操作结果未确认；回读原 requestId 和原生任务，不重放。",
                    "read-original-request",
                );
                f["scope"]["ingressId"] = r["origin"]["ingressId"].clone();
                f["nextAction"]["tool"] = json!("codex_request");
                diagnostics["findings"].as_array_mut().unwrap().push(f);
            }
        }
        Err(_) => {
            diagnostics["checks"]["receipts"] =
                json!({"state":"unknown","scope":"codex-task-receipts"})
        }
    }
}
