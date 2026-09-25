use super::*;
pub fn credential() -> std::result::Result<Credential, Failure> {
    let v = file(&home()?.join(".grok/auth.json"))?;
    let entries = v
        .as_object()
        .ok_or_else(|| Failure::from("invalid-response"))?;
    let token = entries
        .values()
        .find_map(|q| q["key"].as_str().filter(|s| !s.is_empty()))
        .ok_or(Failure::from("credentials-missing"))?;
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
    let end = date(&q["currentPeriod"]["end"]).or_else(|| date(&q["billingPeriodEnd"]));
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
