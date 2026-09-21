//! Client metadata only. Ingress identity and authentication remain generic.
use crate::{json, string, Value};
use std::sync::OnceLock;

pub fn presets() -> &'static Vec<Value> {
    static PRESETS: OnceLock<Vec<Value>> = OnceLock::new();
    PRESETS.get_or_init(|| {
        serde_json::from_str(include_str!("../../shared/control-sources.json"))
            .expect("valid bundled control source registry")
    })
}
pub fn preset(source: &str) -> Value {
    presets()
        .iter()
        .find(|p| p["id"] == source)
        .cloned()
        .unwrap_or_else(|| json!({"id":source,"displayName":source,"curated":false,"status":"custom","recommendedTransport":"https","recommendedAuth":"bearer","caveat":{"en":"Check the client's transport and auth support; unknown sources remain valid."}}))
}
pub fn default_name(source: &str, names: &[String]) -> String {
    let p = preset(source);
    let base = string(&p, "displayName");
    let mut candidate = base.to_owned();
    let mut suffix = 2;
    while names.contains(&candidate) {
        candidate = format!("{base} {suffix}");
        suffix += 1;
    }
    candidate
}
