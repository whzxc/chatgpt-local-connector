use super::*;
pub async fn credential(control: &Control) -> std::result::Result<Credential, Failure> {
    let rpc = control
        .utility()
        .await
        .map_err(|_| Failure::from("network-error"))?;
    let account = rpc
        .call("account/read", json!({"refreshToken":false}), 15000)
        .await
        .map_err(|_| Failure::from("credentials-expired"))?;
    let a = &account["account"];
    if a.is_null() || a["type"] != "chatgpt" {
        return Err("credentials-missing".into());
    }
    let auth_root = std::env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|p| p.join(".codex")));
    let bytes = auth_root.and_then(|p| std::fs::read(p.join("auth.json")).ok());
    let identity = bytes
        .as_ref()
        .and_then(|bytes| serde_json::from_slice::<Value>(bytes).ok())
        .and_then(|v| {
            v["tokens"]["account_id"]
                .as_str()
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
        });
    let source = identity
        .as_ref()
        .map(hash)
        .or_else(|| bytes.map(hash))
        .unwrap_or_default();
    let mut credential = Credential::new(hash(format!("{}:{source}", a)), "codex-account")?;
    credential.identity = identity;
    Ok(credential)
}
pub async fn read(
    control: &Control,
    credential: &Credential,
) -> std::result::Result<Reading, Failure> {
    let rpc = control
        .utility()
        .await
        .map_err(|_| Failure::from("network-error"))?;
    let v = rpc
        .call("account/rateLimits/read", json!({}), 15000)
        .await
        .map_err(|_| Failure::from("network-error"))?;
    if credential
        .identity
        .as_deref()
        .zip(v["accountId"].as_str())
        .is_some_and(|(expected, actual)| expected != actual)
    {
        return Err("credentials-expired".into());
    }
    let groups: Vec<(String, &Value)> = if let Some(map) = v["rateLimitsByLimitId"].as_object() {
        map.iter().map(|(k, v)| (k.clone(), v)).collect()
    } else if v["rateLimits"].is_object() {
        vec![(
            v["rateLimits"]["limitId"]
                .as_str()
                .unwrap_or("codex")
                .into(),
            &v["rateLimits"],
        )]
    } else {
        vec![]
    };
    let mut windows = vec![];
    let mut blocked = vec![];
    for (pool, g) in groups {
        if g["rateLimitReachedType"].as_str().is_some() || g["spendControlReached"] == true {
            blocked.push(pool.clone());
        }
        for slot in ["primary", "secondary"] {
            let q = &g[slot];
            let reset = q["resetsAt"]
                .as_i64()
                .and_then(|n| chrono::DateTime::from_timestamp(n, 0))
                .map(|d| d.to_rfc3339());
            let label = q["windowDurationMins"]
                .as_u64()
                .map(|n| format!("{n} min"))
                .unwrap_or(slot.into());
            if let Some(mut w) = window(
                &format!("{pool}/{slot}"),
                &pool,
                &label,
                number(&q["usedPercent"]),
                reset,
            ) {
                w.scope = g["limitName"].as_str().map(str::to_owned);
                windows.push(w)
            }
        }
    }
    let mut r = Reading::new(windows, v.clone())?;
    r.blocked_pool_ids = blocked;
    r.account_blocked = v["ordinaryUsageAllowed"].as_bool().map(|allowed| !allowed);
    Ok(r)
}
