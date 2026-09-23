use super::*;
pub fn credential() -> std::result::Result<Credential, Failure> {
    let v = file(&home()?.join(".grok/auth.json"))?;
    let entries = v
        .as_object()
        .ok_or_else(|| Failure::from("invalid-response"))?;
    let mut candidates = vec![];
    let mut seen = false;
    for q in entries.values() {
        if let Some(token) = q["key"].as_str().filter(|s| !s.is_empty()) {
            seen = true;
            let expiry = date(&q["expires_at"]);
            if let Some(t) = expiry {
                let d = chrono::DateTime::parse_from_rfc3339(&t).unwrap();
                if d <= chrono::Utc::now() {
                    continue;
                }
                candidates.push((d.timestamp(), token));
            } else {
                candidates.push((0, token));
            }
        }
    }
    candidates.sort_by_key(|c| c.0);
    let token = candidates
        .last()
        .ok_or_else(|| {
            Failure::from(if seen {
                "credentials-expired"
            } else {
                "credentials-missing"
            })
        })?
        .1;
    Credential::new(token.into(), "grok-login")
}
pub async fn read(
    client: &reqwest::Client,
    c: &Credential,
) -> std::result::Result<Reading, Failure> {
    let v = get(client
        .get("https://cli-chat-proxy.grok.com/v1/billing?format=credits")
        .bearer_auth(&c.secret)
        .header("x-xai-token-auth", "xai-grok-cli"))
    .await?;
    let q = &v["config"];
    let start = date(&q["currentPeriod"]["start"]).or_else(|| date(&q["billingPeriodStart"]));
    let end = date(&q["currentPeriod"]["end"]).or_else(|| date(&q["billingPeriodEnd"]));
    let valid = start.as_ref().zip(end.as_ref()).is_some_and(|(s, e)| {
        let s = chrono::DateTime::parse_from_rfc3339(s).unwrap();
        let e = chrono::DateTime::parse_from_rfc3339(e).unwrap();
        s <= chrono::Utc::now() && e > chrono::Utc::now() && e > s
    });
    if !valid {
        return Err("no-limits-reported".into());
    }
    // Only an omitted field has the protobuf zero default; null/invalid is unknown.
    let percent = if q.get("creditUsagePercent").is_none() {
        Some(0.)
    } else {
        number(&q["creditUsagePercent"])
    };
    Reading::new(
        window("current-period", "grok", "current-period", percent, end)
            .into_iter()
            .collect(),
        v,
    )
}
