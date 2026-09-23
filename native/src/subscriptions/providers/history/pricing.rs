//! Public price feeds only: no account credentials or usage records leave the device.
use super::*;
use crate::{load, root, save};
use std::sync::{Arc, RwLock};
const SOURCES: [(&str, &str, &str); 3] = [
    ("models_dev", "https://models.dev/api.json", include_str!("../../../../../shared/pricing/models_dev_snapshot.json")),
    ("litellm", "https://raw.githubusercontent.com/BerriAI/litellm/main/model_prices_and_context_window.json", include_str!("../../../../../shared/pricing/litellm_snapshot.json")),
    ("supplement", "https://robinebers.github.io/openusage/pricing_supplement.json", include_str!("../../../../../shared/pricing/supplement.json")),
];
pub(super) struct Pricing {
    pub data: Value,
    pub aliases: Vec<(regex::Regex, String)>,
}
struct Store {
    current: RwLock<Arc<Pricing>>,
    refreshing: std::sync::atomic::AtomicBool,
    next: Mutex<i64>,
}
fn directory() -> PathBuf {
    root().join("subscriptions/pricing")
}
fn build() -> Pricing {
    let mut models = serde_json::Map::new();
    let mut aliases = json!([]);
    let mut fast = json!({});
    let mut updated = String::new();
    for (id, _, bundled) in SOURCES {
        let mut source: Value = serde_json::from_str(bundled).expect("bundled pricing");
        if let Ok(cached) = load(&directory().join(format!("{id}.json"))) {
            if id == "supplement" {
                if cached["updated_at"].as_str().unwrap_or("")
                    >= source["updated_at"].as_str().unwrap_or("")
                    && compact(id, &cached).is_some()
                {
                    source = cached;
                }
            } else if let Some(rows) = cached["models"].as_object().filter(|rows| !rows.is_empty())
            {
                if let Some(base) = source["models"].as_object_mut() {
                    base.extend(rows.clone());
                }
            }
        }
        let data = if id == "supplement" {
            compact(id, &source).expect("bundled supplement")
        } else {
            source.clone()
        };
        if let Some(rows) = data["models"].as_object() {
            models.extend(rows.clone());
        }
        if id == "supplement" {
            aliases = source["alias_rules"].clone();
            fast = source["fast_multipliers"].clone();
        }
        for key in ["updated_at", "retrieved_at"] {
            if let Some(at) = source[key].as_str() {
                if at > updated.as_str() {
                    updated = at.into();
                }
            }
        }
    }
    let rules = aliases
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|a| {
            Some((
                regex::RegexBuilder::new(a["pattern"].as_str()?)
                    .case_insensitive(true)
                    .build()
                    .ok()?,
                a["canonical"].as_str()?.into(),
            ))
        })
        .collect();
    Pricing {
        data: json!({"models":models,"fast":fast,"updatedAt":updated}),
        aliases: rules,
    }
}
pub(super) fn current() -> Arc<Pricing> {
    static STORE: OnceLock<Arc<Store>> = OnceLock::new();
    let store = STORE.get_or_init(|| {
        Arc::new(Store {
            current: RwLock::new(Arc::new(build())),
            refreshing: false.into(),
            next: Mutex::new(0),
        })
    });
    if Utc::now().timestamp() >= *store.next.lock().unwrap()
        && !store
            .refreshing
            .swap(true, std::sync::atomic::Ordering::AcqRel)
    {
        let store = store.clone();
        tokio::spawn(async move {
            let settings =
                load(&root().join("web/preferences.json")).unwrap_or(json!({"proxyMode":"system"}));
            let client = match crate::proxy::NetworkProxy::resolve(&settings).await {
                Ok(proxy) => proxy
                    .client(
                        reqwest::Client::builder()
                            .timeout(std::time::Duration::from_secs(20))
                            .redirect(reqwest::redirect::Policy::none()),
                    )
                    .build()
                    .ok(),
                Err(_) => None,
            };
            if let Some(client) = client {
                tokio::join!(
                    refresh(&client, 0),
                    refresh(&client, 1),
                    refresh(&client, 2)
                );
                *store.current.write().unwrap() = Arc::new(build());
            }
            *store.next.lock().unwrap() = Utc::now().timestamp() + 60;
            store
                .refreshing
                .store(false, std::sync::atomic::Ordering::Release);
        });
    }
    store.current.read().unwrap().clone()
}
async fn refresh(client: &reqwest::Client, index: usize) {
    let (id, url, _) = SOURCES[index];
    let path = directory().join(format!("{id}-state.json"));
    let mut state = load(&path).unwrap_or(json!({}));
    let now = Utc::now().timestamp();
    if state["nextAt"].as_i64().is_some_and(|at| at > now) {
        return;
    }
    let mut request = client.get(url);
    if let Some(etag) = state["etag"].as_str() {
        request = request.header(reqwest::header::IF_NONE_MATCH, etag);
    }
    let result: Option<()> = async {
        let response = request.send().await.ok()?;
        if response.status() == reqwest::StatusCode::NOT_MODIFIED
            && directory().join(format!("{id}.json")).exists()
        {
            return Some(());
        }
        if !response.status().is_success() {
            return None;
        }
        let etag = response
            .headers()
            .get(reqwest::header::ETAG)
            .and_then(|s| s.to_str().ok())
            .map(str::to_owned);
        let bytes = super::super::limited_bytes(response, 32 * 1024 * 1024)
            .await
            .ok()?;
        let raw: Value = serde_json::from_slice(&bytes).ok()?;
        let mut data = compact(id, &raw)?;
        if id == "supplement" {
            data = raw;
        } else {
            data["retrieved_at"] = json!(Utc::now().to_rfc3339());
        }
        save(&directory().join(format!("{id}.json")), &data).ok()?;
        state["etag"] = json!(etag);
        Some(())
    }
    .await;
    state["nextAt"] = json!(now + if result.is_some() { 3600 } else { 1800 });
    let _ = save(&path, &state);
}
fn number(v: &Value) -> Option<f64> {
    v.as_f64().filter(|n| n.is_finite() && *n >= 0.)
}
fn rates(v: &Value, i: &str, o: &str, cw: &str, cr: &str, scale: f64) -> Option<Value> {
    let input = number(&v[i])?;
    let output = number(&v[o])?;
    Some(
        json!({"i":input*scale,"o":output*scale,"cw":number(&v[cw]).unwrap_or(input)*scale,"cr":number(&v[cr]).unwrap_or(input*0.1)*scale,"cre":number(&v[cr]).is_some()}),
    )
}
fn compact(id: &str, raw: &Value) -> Option<Value> {
    let mut models = serde_json::Map::new();
    let root = raw.as_object()?;
    match id {
        "litellm" => {
            for (name, value) in root {
                if let Some(mut r) = rates(
                    value,
                    "input_cost_per_token",
                    "output_cost_per_token",
                    "cache_creation_input_token_cost",
                    "cache_read_input_token_cost",
                    1_000_000.,
                ) {
                    for (key, field) in [
                        ("ia", "input_cost_per_token_above_200k_tokens"),
                        ("oa", "output_cost_per_token_above_200k_tokens"),
                        ("cwa", "cache_creation_input_token_cost_above_200k_tokens"),
                        ("cra", "cache_read_input_token_cost_above_200k_tokens"),
                    ] {
                        if let Some(n) = number(&value[field]) {
                            r[key] = json!(n * 1_000_000.);
                        }
                    }
                    if let Some(n) = number(&value["provider_specific_entry"]["fast"]) {
                        r["fast"] = json!(n);
                    }
                    models.insert(name.clone(), r);
                }
            }
        }
        "models_dev" => {
            for provider in root.values() {
                for (name, value) in provider["models"].as_object().into_iter().flatten() {
                    if let Some(r) = rates(
                        &value["cost"],
                        "input",
                        "output",
                        "cache_write",
                        "cache_read",
                        1.,
                    ) {
                        models.entry(name.clone()).or_insert(r);
                    }
                }
            }
        }
        "supplement" => {
            for (name, value) in raw["pricing"].as_object()? {
                if let Some(r) = rates(
                    value,
                    "input_per_million",
                    "output_per_million",
                    "cache_write_per_million",
                    "cache_read_per_million",
                    1.,
                ) {
                    models.insert(name.clone(), r);
                }
            }
        }
        _ => return None,
    }
    (!models.is_empty()).then(|| json!({"models":models}))
}
