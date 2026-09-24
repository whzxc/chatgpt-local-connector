use super::{types::*, Slot};
use crate::*;
use serde::{Deserialize, Serialize};

// Only successful, credential-bound readings are persisted, never scheduler/error state.
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct CachedReading {
    fingerprint: String,
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
pub(super) fn restore(slot: &mut Slot, fingerprint: &str) {
    let id = &slot.view.provider_id;
    let Some(reading) = load(&path(id))
        .ok()
        .and_then(|v| serde_json::from_value::<CachedReading>(v).ok())
    else {
        return;
    };
    let age = chrono::DateTime::parse_from_rfc3339(&reading.observed_at)
        .ok()
        .map(|at| chrono::Utc::now().signed_duration_since(at).num_seconds());
    if reading.fingerprint != fingerprint
        || !age.is_some_and(|age| (0..86400).contains(&age))
        || reading
            .windows
            .iter()
            .any(|w| !w.used_percent.is_finite() || w.used_percent < 0.)
    {
        remove(id);
        return;
    }
    slot.view.windows = reading.windows;
    slot.view
        .windows
        .retain(|w| active(w) && (w.resets_at.is_some() || age.is_some_and(|age| age < 900)));
    slot.view.raw_usage = reading.raw_usage;
    slot.view.observed_at = Some(reading.observed_at);
    slot.view.account_blocked = reading.account_blocked;
    slot.view.blocked_pool_ids = reading.blocked_pool_ids;
    slot.view.state = "stale".into();
}
pub(super) fn persist(slot: &Slot) {
    let (Some(fingerprint), Some(observed_at)) = (&slot.fingerprint, &slot.view.observed_at) else {
        return;
    };
    let reading = CachedReading {
        fingerprint: fingerprint.clone(),
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
