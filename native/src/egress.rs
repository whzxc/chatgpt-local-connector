//! Pure MCP output projection. Storage and execution live outside this module.
use crate::*;
pub fn tool(value: Value, error: bool) -> Value {
    json!({"content":[{"type":"text","text":value.to_string()}],"structuredContent":{"result":value},"isError":error})
}
pub fn failure(e: &str) -> Value {
    let rpc = e
        .strip_prefix("RPC_REJECTED:")
        .and_then(|s| serde_json::from_str::<Value>(s).ok());
    json!({"code":if rpc.is_some(){"NATIVE_RPC_ERROR"}else{if e == "REQUEST_ID_CONFLICT" { "REQUEST_ID_CONFLICT" } else { "CONTROL_ERROR" }},"message":e,"rpcError":rpc})
}
