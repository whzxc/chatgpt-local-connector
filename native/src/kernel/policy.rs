use crate::*;
pub fn allows(policy: &Value, name: &str) -> bool {
    policy == "all"
        || policy["allowlist"]
            .as_array()
            .is_some_and(|tools| tools.iter().any(|v| v == name))
}
