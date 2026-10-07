use super::{types::*, Slot};
use crate::*;
use serde::{Deserialize, Serialize};

// Last successful readings are displayed as stale until refreshed.
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct CachedReading {
    observed_at: String,
    windows: Vec<QuotaWindow>,
    raw_usage: Value,
    account_blocked: Option<bool>,
    blocked_pool_ids: Vec<String>,
}
fn path(id: &str) -> PathBuf {
    root()
        .join("subscriptions/readings")
        .join(format!("{id}.json"))
}
pub(super) fn remove(id: &str) {
    if let Err(error) = std::fs::remove_file(path(id)) {
        if error.kind() != std::io::ErrorKind::NotFound {
            eprintln!("could not remove subscription cache ({id}): {error}");
        }
    }
}
pub(super) fn restore(slot: &mut Slot) {
    let id = &slot.view.provider_id;
    let Some(reading) = load(&path(id))
        .ok()
        .and_then(|v| serde_json::from_value::<CachedReading>(v).ok())
    else {
        return;
    };
    slot.view.windows = reading.windows;
    slot.view.raw_usage = reading.raw_usage;
    slot.view.observed_at = Some(reading.observed_at);
    slot.view.account_blocked = reading.account_blocked;
    slot.view.blocked_pool_ids = reading.blocked_pool_ids;
    compact_history(&mut slot.view);
    slot.view.state = "stale".into();
}
pub(super) fn persist(slot: &Slot) {
    let Some(observed_at) = &slot.view.observed_at else {
        return;
    };
    let reading = CachedReading {
        observed_at: observed_at.clone(),
        windows: slot.view.windows.clone(),
        raw_usage: slot.view.raw_usage.clone(),
        account_blocked: slot.view.account_blocked,
        blocked_pool_ids: slot.view.blocked_pool_ids.clone(),
    };
    if let Err(error) = save(
        &path(&slot.view.provider_id),
        &serde_json::to_value(reading).unwrap(),
    ) {
        eprintln!(
            "could not save subscription cache ({}): {error}",
            slot.view.provider_id
        );
    }
}

/// UI consumers need a sample per quota window, not the entire event timeline.
/// Keep exact event-time boundaries; never bucket events across a reset boundary.
pub(super) fn compact_history(p: &mut ProviderSnapshot) {
    fn millis(value: &Value) -> Option<i64> {
        value
            .as_i64()
            .map(|n| if n < 1_000_000_000_000 { n * 1000 } else { n })
            .or_else(|| {
                chrono::DateTime::parse_from_rfc3339(value.as_str()?)
                    .ok()
                    .map(|d| d.timestamp_millis())
            })
    }
    fn period(w: &QuotaWindow, p: &ProviderSnapshot) -> Option<i64> {
        if let Some((count, unit)) = w.label.split_once(' ') {
            if let Ok(count) = count.parse::<i64>() {
                match unit {
                    "min" => return count.checked_mul(60000),
                    "s" => return count.checked_mul(1000),
                    _ => {}
                }
            }
        }
        match w.label.as_str() {
            "rolling" | "five_hour" | "session" => return Some(5 * 3600000),
            "weekly" | "seven_day" | "weekly_all" | "weekly_scoped" => return Some(7 * 86400000),
            _ => {}
        }
        if p.agent_id == "cursor" {
            let summary = p
                .raw_usage
                .get("usage")
                .filter(|v| !v.is_null())
                .unwrap_or(&p.raw_usage["summary"]);
            let start = summary
                .get("billingCycleStart")
                .filter(|v| !v.is_null())
                .unwrap_or(&summary["billingCycleStartDate"]);
            let end = summary
                .get("billingCycleEnd")
                .filter(|v| !v.is_null())
                .unwrap_or(&summary["billingCycleEndDate"]);
            return Some(
                millis(end)
                    .zip(millis(start))
                    .map(|(b, a)| b - a)
                    .filter(|n| *n > 0)
                    .unwrap_or(30 * 86400000),
            );
        }
        (w.label == "monthly").then_some(30 * 86400000)
    }
    let history = &p.raw_usage["history"];
    let Some(timeline) = history["timeline"].as_array() else {
        return;
    };
    let end = p
        .observed_at
        .as_deref()
        .and_then(|v| millis(&json!(v)))
        .zip(millis(&history["observedAt"]))
        .map(|(a, b)| a.min(b).min(chrono::Utc::now().timestamp_millis()));
    let mut samples = serde_json::Map::new();
    for w in &p.windows {
        let bounds = w
            .resets_at
            .as_deref()
            .and_then(|v| millis(&json!(v)))
            .zip(period(w, p))
            .zip(end);
        let Some(((reset, length), end)) = bounds else {
            continue;
        };
        let start = reset - length;
        let (mut tokens, mut usd, mut priced) = (0u64, 0f64, 0u64);
        for row in timeline {
            if row[0].as_i64().is_some_and(|at| at >= start && at <= end) {
                tokens = tokens.saturating_add(row[1].as_u64().unwrap_or(0));
                usd += row[2].as_f64().unwrap_or(0.);
                priced = priced.saturating_add(row[3].as_u64().unwrap_or(0));
            }
        }
        samples.insert(w.id.clone(), json!({"start":start,"end":end,"tokens":tokens,"estimatedUsd":usd,"pricedTokens":priced}));
    }
    if let Some(history) = p.raw_usage["history"].as_object_mut() {
        history.remove("timeline");
        history.insert("samples".into(), Value::Object(samples));
    }
}
