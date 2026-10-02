//! Standalone local plugin entrypoint over the same leased Core used by Desktop.
use crate::*;
use base64::{engine::general_purpose::STANDARD, Engine};
use tokio::io::{AsyncWriteExt, BufReader};
const HTML: &str = include_str!("../../dist/plugin/app.html");
const ICON: &[u8] = include_bytes!("../../plugins/clc/assets/icon.svg");
fn icons() -> Value {
    json!([{"src":format!("data:image/svg+xml;base64,{}", STANDARD.encode(ICON)),"mimeType":"image/svg+xml","sizes":["any"]}])
}
pub fn tools() -> Value {
    let mut tools: Vec<Value> = catalog()["tools"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|tool| tool["name"] != "connector_verify")
        .cloned()
        .collect();
    tools.extend([
        tool("connector_overview", "Connector 总览", Some("global")),
        tool("connector_task_usage", "任务用量", Some("thread")),
        tool("connector_usage_refresh", "刷新用量", None),
    ]);
    json!({"tools":tools})
}
fn uri() -> String {
    let dev = cfg!(debug_assertions)
        && std::env::var("CLC_PLUGIN_DEV_HTML").is_ok_and(|path| !path.is_empty());
    format!(
        "ui://clc/usage-{}-{}{}.html",
        env!("CARGO_PKG_VERSION"),
        &hash(&if dev {
            format!("{HTML}{}", include_str!("../../ui/plugin-dev.js"))
        } else {
            HTML.to_owned()
        })[..12],
        if dev { "-dev-reload" } else { "" }
    )
}
fn tool(name: &str, title: &str, entry: Option<&str>) -> Value {
    let mut meta = json!({"ui":{"resourceUri":uri(),"visibility":["app"]}});
    if let Some(entry) = entry {
        meta["openai/ui"] = json!({"entrypoints":[{"type":entry}]});
    }
    json!({"name":name,"title":title,"icons":icons(),"description":"Read device-local usage from the running Local Connector. No model turn or account changes.","inputSchema":{"type":"object","properties":{"scope":{"enum":["global","thread"]},"threadId":{"type":"string","maxLength":128},"days":{"enum":[1,7,30]},"turnId":{"type":"string","maxLength":128},"turnOffset":{"type":"integer","minimum":0},"responseOffset":{"type":"integer","minimum":0},"toolOffset":{"type":"integer","minimum":0}},"additionalProperties":false},"annotations":{"readOnlyHint":true,"destructiveHint":false,"openWorldHint":false},"_meta":meta})
}
pub async fn dispatch(core: &crate::runtime::Client, request: Value) -> Value {
    let id = request["id"].clone();
    let params = &request["params"];
    let result: Result<Value> = match string(&request, "method") {
        "initialize" => Ok(
            json!({"protocolVersion":"2025-11-25","capabilities":{"tools":{},"resources":{}},"serverInfo":{"name":"local-connector","version":env!("CARGO_PKG_VERSION"),"icons":icons()},"instructions":catalog()["instructions"]}),
        ),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(tools()),
        "resources/list" => Ok(
            json!({"resources":[{"uri":uri(),"name":"Connector 用量工作台","mimeType":"text/html;profile=mcp-app"}]}),
        ),
        "resources/read" if params["uri"] == uri() => resource().await,
        "tools/call" => {
            let name = string(params, "name");
            if ![
                "connector_overview",
                "connector_task_usage",
                "connector_usage_refresh",
            ]
            .contains(&name)
            {
                core.request("plugin/call", "POST", json!({"name":name,"arguments":params.get("arguments").cloned().unwrap_or(json!({}))})).await
            } else {
                let args = params.get("arguments").cloned().unwrap_or(json!({}));
                let scope = if name == "connector_overview" {
                    "global"
                } else if name == "connector_task_usage" {
                    "thread"
                } else if args["scope"] == "global" {
                    "global"
                } else {
                    "thread"
                };
                let meta = json!({"threadId":params["_meta"]["threadId"],"thread_id":params["_meta"]["thread_id"]});
                let data=match core.request("usage/query","POST",json!({"schemaVersion":1,"pluginVersion":env!("CARGO_PKG_VERSION"),"scope":scope,"arguments":args,"metadata":meta})).await {
                    Ok(v)=>v,
                    Err(e)=>json!({"schemaVersion":1,"scope":scope,"state":if e=="UNKNOWN_ROUTE" || e=="PLUGIN_VERSION_MISMATCH" {"incompatible"}else{"unavailable"},"message":if e=="UNKNOWN_ROUTE" || e=="PLUGIN_VERSION_MISMATCH" {"请升级 Local Connector 和插件到相同版本，然后重新打开面板。"}else{"本机 Core 暂不可用。请重新加载插件后刷新。"},"connectorVersion":null}),
                };
                Ok(
                    json!({"content":[{"type":"text","text":"Connector 本机用量面板；统计仅在面板中展示。"}],"structuredContent":{"scope":scope,"state":data["state"]},"_meta":{"ui":{"resourceUri":uri()},"usage":data}}),
                )
            }
        }
        _ => Err("method not found".into()),
    };
    if id.is_null() {
        return Value::Null;
    }
    match result {
        Ok(result) => json!({"jsonrpc":"2.0","id":id,"result":result}),
        Err(message) => json!({"jsonrpc":"2.0","id":id,"error":{"code":-32601,"message":message}}),
    }
}
async fn resource() -> Result<Value> {
    let mut html = HTML.to_owned();
    let mut meta =
        json!({"ui":{"prefersBorder":false,"csp":{"connectDomains":[],"resourceDomains":[]}}});
    if cfg!(debug_assertions) {
        if let Some(path) = std::env::var("CLC_PLUGIN_DEV_HTML")
            .ok()
            .filter(|path| !path.is_empty())
        {
            // The explicit dev manifest owns this path. Release builds never read it.
            if let Ok(current) = std::fs::read_to_string(&path) {
                html = current;
            }
            let revision = hash(&html);
            meta["clc/devRevision"] = json!(revision);
            meta["ui"]["csp"]["frameDomains"] = json!(["blob:"]);
            let config = json!({"uri":uri(),"revision":revision})
                .to_string()
                .replace('<', "\\u003c");
            let loader = include_str!("../../ui/plugin-dev.js");
            html = html.replace(
                "</body>",
                &format!("<script>window.__CLC_DEV__={config};{loader}</script></body>"),
            );
        }
    }
    Ok(
        json!({"contents":[{"uri":uri(),"mimeType":"text/html;profile=mcp-app","text":html,"_meta":meta}]}),
    )
}
pub async fn stdio() -> Result<()> {
    init_crypto();
    let core = crate::runtime::Client::connect(crate::runtime::binary()?, "plugin").await?;
    let mut reader = BufReader::new(tokio::io::stdin());
    let stdout = Arc::new(Mutex::new(tokio::io::stdout()));
    let mut jobs = tokio::task::JoinSet::new();
    let mut waits = std::collections::HashMap::<String, tokio::task::AbortHandle>::new();
    loop {
        let bytes = crate::line(&mut reader, 1024 * 1024).await?;
        if bytes.is_empty() {
            break;
        }
        while let Some(Ok(key)) = jobs.try_join_next() {
            waits.remove(&key);
        }
        let request: Value = serde_json::from_slice(&bytes).map_err(|_| "invalid JSON-RPC")?;
        if request["method"] == "notifications/cancelled" {
            if let Some(wait) = waits.remove(&request["params"]["requestId"].to_string()) {
                wait.abort();
            }
            continue;
        }
        if request["id"].is_null() {
            continue;
        }
        let key = request["id"].to_string();
        // Keep reading cancellation/EOF when the bounded request pool is full.
        if jobs.len() >= 8 || waits.contains_key(&key) {
            let mut out = stdout.lock().await;
            out.write_all(format!("{}\n", json!({"jsonrpc":"2.0","id":request["id"],"error":{"code":-32000,"message":"Too many requests or duplicate wait id"}})).as_bytes()).await.map_err(|e| e.to_string())?;
            out.flush().await.map_err(|e| e.to_string())?;
            continue;
        }
        let waiting = request["method"] == "tools/call"
            && matches!(
                string(&request["params"], "name"),
                "agent_wait" | "codex_wait"
            );
        let (out, core, completed) = (stdout.clone(), core.clone(), key.clone());
        let task = jobs.spawn(async move {
            let result = dispatch(&core, request).await;
            if !result.is_null() {
                let mut o = out.lock().await;
                let _ = o.write_all(format!("{result}\n").as_bytes()).await;
                let _ = o.flush().await;
            }
            completed
        });
        if waiting {
            waits.insert(key, task);
        }
    }
    jobs.abort_all();
    while jobs.join_next().await.is_some() {}
    core.close().await;
    Ok(())
}

/// Export a platform plugin with its own executable, independent of Desktop's installation.
pub fn export(target: &Path) -> Result<Value> {
    let manifest_path = target.join(".codex-plugin/plugin.json");
    if target.exists() && load(&manifest_path).ok().is_none_or(|v| v["name"] != "clc") {
        return Err("目标目录已存在且不是 CLC 插件；请选择新目录。".into());
    }
    let mut manifest: Value =
        serde_json::from_str(include_str!("../../plugins/clc/.codex-plugin/plugin.json"))
            .map_err(|e| e.to_string())?;
    manifest["version"] = json!(env!("CARGO_PKG_VERSION"));
    manifest["mcpServers"] = json!("./.mcp.json");
    manifest.as_object_mut().unwrap().remove("apps");
    if target.join(".app.json").exists() {
        std::fs::remove_file(target.join(".app.json")).map_err(|e| e.to_string())?;
    }
    let binary = crate::runtime::binary()?;
    let name = if cfg!(windows) {
        "local-connector.exe"
    } else {
        "local-connector"
    };
    private_dir(&target.join("bin"))?;
    let destination = target.join("bin").join(name);
    if binary != destination {
        let temporary = target.join("bin").join(format!("{name}.{}", id()));
        std::fs::copy(&binary, &temporary).map_err(|e| e.to_string())?;
        std::fs::rename(&temporary, &destination).map_err(|e| {
            let _ = std::fs::remove_file(&temporary);
            format!("关闭使用该目录的旧插件进程后重新导出：{e}")
        })?;
    }
    save(&manifest_path, &manifest)?;
    save(
        &target.join(".mcp.json"),
        &json!({"mcpServers":{"clc":{"command":format!("./bin/{name}"),"args":["mcp"],"cwd":".","env_vars":["PATH","CODEX_HOME","CLC_STATE_DIR","HTTP_PROXY","HTTPS_PROXY","ALL_PROXY","NO_PROXY"],"tool_timeout_sec":330}}}),
    )?;
    let skill = target.join("skills/local-connector/SKILL.md");
    private_dir(skill.parent().unwrap())?;
    std::fs::write(
        skill,
        include_str!("../../plugins/clc/skills/local-connector/SKILL.md"),
    )
    .map_err(|e| e.to_string())?;
    std::fs::write(target.join("LICENSE"), include_str!("../../LICENSE"))
        .map_err(|e| e.to_string())?;
    private_dir(&target.join("assets"))?;
    std::fs::write(target.join("assets/icon.svg"), ICON).map_err(|e| e.to_string())?;
    Ok(
        json!({"path":target,"version":env!("CARGO_PKG_VERSION"),"state":"exported","install":"Install Local Connector through the host's local marketplace. The plugin starts its own shared Core; Connector Desktop is optional."}),
    )
}
