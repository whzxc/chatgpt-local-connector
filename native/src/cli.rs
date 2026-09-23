use crate::{json, string, transport::forward_request, Value};
use std::io::Read;

const GUIDE: &str = include_str!("../../docs/codex-setup.md");

fn onboarding(ingress: &Value) -> Value {
    json!({"ingressId":ingress["id"],"controlSource":ingress["controlSource"],
        "stage":if ingress["config"]["configured"]!=true {"configure"}else if ingress["state"]!="ready" {"start"}else if ingress["verification"]["challengeVerifiedAt"].is_string(){"inbound_verified"}else{"client_setup_or_verify"},
        "transport":ingress["transport"],"authentication":ingress["auth"],"url":ingress["url"],"tunnelId":ingress["config"]["tunnelId"],
        "verification":ingress["verification"],"diagnostics":crate::diagnostics::ingress(ingress),
        "preset":crate::control_sources::preset(string(ingress,"controlSource")),
        "verificationPrompt":format!("调用 connector_verify，code 为 {}。只验证连接，不创建任务。",string(&ingress["verification"],"code")),
        "executionPrompt":"创建一个无害 Agent 任务：不调用工具，不读取或修改文件，只回复 CLC_ONBOARDING_OK。使用唯一 UUID requestId，读取回执、taskId、turnId，再用默认 30 秒的 agent_wait/codex_wait；timeout 后沿用原 ID 和上一轮 snapshotHash 作为 expectedHash 继续 wait 至终态或交互，读取最终输出。timeout 不终止任务，不重新创建。未知状态回读原 requestId，不重复创建。",
        "identityNote":"controlSource is a configured label, not an authenticated user identity. Bearer proves possession only.",
        "clientSetup":"Read preset.status, supportedAuth and caveat before connecting. Conditional paths require client verification; auth-limited paths must not silently fall back to none. A preset is not an installed integration.",
        "secretDelivery":"Supply credentials using stdin or a protected local file. Never paste tokens into chat or use command-line arguments."})
}
async fn ingresses() -> crate::Result<Value> {
    forward_request("ingress", "GET", json!({})).await
}
async fn doctor() -> crate::Result<Value> {
    let status = forward_request("status", "GET", json!({})).await?;
    let mut items = Vec::new();
    for ingress in status["ingresses"].as_array().into_iter().flatten() {
        items.push(json!({"id":ingress["id"],"configured":ingress["config"]["configured"],"state":ingress["state"],"error":ingress["error"],"verification":ingress["verification"],"diagnostics":crate::diagnostics::ingress(ingress),
        "preset":crate::control_sources::preset(string(ingress,"controlSource")),"onboarding":onboarding(ingress)}));
    }
    Ok(
        json!({"ingresses":items,"summary":status["ingressSummary"],"desktop":status["core"]["desktop"],"appServer":status["core"]["appServer"],"executionVerified":false,"diagnostics":status["diagnostics"]}),
    )
}
fn stdin_json() -> crate::Result<Value> {
    let mut input = String::new();
    std::io::stdin()
        .take(16385)
        .read_to_string(&mut input)
        .map_err(|_| "无法读取标准输入".to_string())?;
    if input.len() > 16384 {
        return Err("输入超过 16 KiB".into());
    }
    // Never echo malformed input: it may contain a credential.
    serde_json::from_str(&input).map_err(|_| "标准输入必须是 JSON 对象".into())
}

async fn execute(args: &[String]) -> crate::Result<Value> {
    let args: Vec<&str> = args
        .iter()
        .map(String::as_str)
        .filter(|a| *a != "--json")
        .collect();
    match args.as_slice() {
        [] | ["help"] | ["--help"] => Ok(json!({"version":env!("CARGO_PKG_VERSION"),
            "usage":"<应用可执行文件> cli <command> [--json]",
            "commands":["subscriptions get","subscriptions set --stdin","subscriptions refresh [providerId]","subscriptions open [providerId]","panel get","panel set --stdin","help","guide","context --stdin","status","onboarding","doctor","logs","configure --stdin","network --stdin","connect","disconnect","verify","verify --fresh","ingress list","ingress presets","ingress add --stdin","ingress update <id> --stdin","ingress remove <id>","ingress start <id>","ingress stop <id>","ingress start-all","ingress stop-all","ingress token rotate <id>","ingress oauth list <id>","ingress oauth register <id> --stdin","ingress oauth revoke <id> --stdin","ingress doctor <id>","ingress verify <id> [--fresh]"],
            "subscriptionsInput":{"enabled":"boolean; required","providers":"complete array of IDs from subscriptions get; required","pinnedWindows":"complete provider-to-window map; required"},
            "subscriptionsNote":"get reads memory only. set replaces the complete settings: read get.settings first and preserve all intended selections/pins. refresh requests an online read for a selected eligible provider, or all such providers when omitted; accepted is not fresh-data success. Read observedAt/state/error afterwards. open activates the target instance's Agent detail panel, or the Agents panel when omitted.",
            "panelInput":{"autoCollapse":"boolean","size":"small | standard | large","spacing":"compact | standard | roomy","ends":"softened | round","horizontalPercentages":"boolean","alertColor":"boolean","notchFusion":"boolean","warningAt":"60 | 70 | 75 | 80 | 85 | 90 percent used","dock":"left | right | top | bottom | floating","resetPosition":"true; resets placement, takes precedence over dock"},
            "panelNote":"set partially updates existing preferences. get never creates/shows a panel. Results separate preferences from runtime; saved/deferred/controller-applied does not mean rendered or animation complete. expanded is the controller target; acceptedGeometry matches only received geometry. Native panel is macOS-only; other platforms can save preferences and report unsupported runtime. Observe subsequent state with get; no input simulation.",
            "controlSourcePresets":crate::control_sources::presets(),
            "ingressInput":{"id":"optional on add; immutable","name":"optional display name; defaults to preset name with an available numeric suffix","controlSource":"see ingress presets; any other client label remains valid","transport":"openai-tunnel | https","auth":"openai for OpenAI Tunnel; none | bearer | oauth for HTTPS","bearerToken":"32+ printable ASCII characters, stdin only; never returned by list/status","enabled":true,"toolPolicy":"all or {allowlist:[connector_verify,agents,agent_create,agent_read,agent_wait,...]}","config":{"httpsProvider":"cloudflare | ngrok | pinggy | localxpose | custom","cloudflareMode":"quick | named","httpsUrl":"https://hostname/mcp; required for named/custom, optional fixed ngrok address","httpsHost":"127.0.0.1 default; custom only","httpsPort":"8787 default; choose distinct ports for named/custom and match the external route","cloudflareToken":"named Tunnel token; stdin only","ngrokAuthtoken":"ngrok credential; stdin only","pinggyMode":"quick | named","pinggyToken":"Pinggy named domain token; stdin only","localxposeMode":"named; quick is unavailable because the first request is redirected","localxposeAccessToken":"LocalXpose credential; stdin only","localxposeRegion":"us | eu | ap","tunnelId":"official Tunnel ID","apiKey":"official Tunnel runtime key; stdin only","tunnelBinary":"optional executable path"}},
            "waitContract":"create/send → wait(30s default; 20–30s recommended) → timeout → same taskId/threadId and turnId, previous snapshotHash as expectedHash → wait again until terminal/interaction. Event-driven slices, not read/sleep polling. Timeout/cancelling wait never stops or recreates the task; unconfirmed is not failure. Ordinary Chat/general MCP clients should not block for minutes by default. Explicit maximum 300000ms requires upstream support; stdio forwarding budget remains 330s.",
            "update":"Partial merge; stop the target before updating or rotating. Other ingresses remain online. Changes reset only this ingress verification.",
            "authNote":"none exposes allowed tools to anyone with network access. Prefer a fixed URL with OAuth (local owner consent, PKCE, DCR or preregistration) or bearer for clients that support it.",
            "tokenDelivery":"token rotate returns the new secret once in stdout. Redirect to a protected local file (umask 077); do not capture it into chat/logs. Supply an initial bearerToken via stdin when adding.",
            "configureInput":{"tunnelId":"可选；省略保留原值","apiKey":"可选；空字符串保留原值"},
            "networkInput":{"proxyMode":"system | direct | custom","proxyUrl":"自定义 HTTP/HTTPS 地址，其余为空"},
            "note":"所有命令输出 JSON。help、guide、ingress presets 可离线读取；其余命令需要已打开的同版本应用。凭据仅从 stdin 输入，禁止放入命令参数或聊天。"})),
        ["subscriptions", "get"] => forward_request("subscriptions", "GET", json!({})).await,
        ["subscriptions", "set", "--stdin"] => {
            forward_request("subscriptions/settings", "PUT", stdin_json()?).await
        }
        ["subscriptions", action @ ("refresh" | "open")] => {
            forward_request(&format!("subscriptions/{action}"), "POST", json!({})).await
        }
        ["subscriptions", action @ ("refresh" | "open"), id] if !id.starts_with('-') => {
            forward_request(
                &format!("subscriptions/{action}"),
                "POST",
                json!({"providerId":id}),
            )
            .await
        }
        ["panel", "get"] => forward_request("usage-panel", "GET", json!({})).await,
        ["panel", "set", "--stdin"] => forward_request("usage-panel", "PUT", stdin_json()?).await,
        ["ingress", "oauth", "list", id] => {
            forward_request(&format!("ingress/{id}/oauth"), "GET", json!({})).await
        }
        ["ingress", "oauth", "register", id, "--stdin"] => {
            forward_request(
                &format!("ingress/{id}/oauth/register"),
                "POST",
                stdin_json()?,
            )
            .await
        }
        ["ingress", "oauth", "revoke", id, "--stdin"] => {
            forward_request(&format!("ingress/{id}/oauth/revoke"), "POST", stdin_json()?).await
        }
        ["guide"] => Ok(json!({"markdown":GUIDE})),
        ["doctor"] => doctor().await,
        ["context", "--stdin"] => forward_request("context", "POST", stdin_json()?).await,
        ["onboarding"] => Ok(
            json!({"ingresses":ingresses().await?.as_array().into_iter().flatten().map(onboarding).collect::<Vec<_>>()}),
        ),
        ["ingress", "presets"] => Ok(
            json!({"curated":crate::control_sources::presets(),"custom":crate::control_sources::preset("custom")}),
        ),
        ["ingress", "list"] => {
            let mut items = ingresses().await?;
            for item in items.as_array_mut().into_iter().flatten() {
                item["preset"] = crate::control_sources::preset(string(item, "controlSource"));
            }
            Ok(items)
        }
        ["ingress", "add", "--stdin"] => forward_request("ingress", "POST", stdin_json()?).await,
        ["ingress", "update", id, "--stdin"] => {
            forward_request(&format!("ingress/{id}"), "PUT", stdin_json()?).await
        }
        ["ingress", "remove", id] => {
            forward_request(&format!("ingress/{id}"), "DELETE", json!({})).await
        }
        ["ingress", action @ ("start" | "stop"), id] => {
            forward_request(&format!("ingress/{id}/{action}"), "POST", json!({})).await
        }
        ["ingress", action @ ("start-all" | "stop-all")] => {
            forward_request(&format!("ingress/{action}"), "POST", json!({})).await
        }
        ["ingress", "token", "rotate", id] => {
            forward_request(&format!("ingress/{id}/token"), "POST", json!({})).await
        }
        ["ingress", "doctor", id] => {
            let status = forward_request("status", "GET", json!({})).await?;
            let i = status["ingresses"]
                .as_array()
                .into_iter()
                .flatten()
                .find(|i| i["id"] == *id)
                .ok_or("ingress not found")?;
            let findings: Vec<_> = status["diagnostics"]["findings"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|f| f["scope"]["id"] == *id || f["scope"]["ingressId"] == *id)
                .cloned()
                .collect();
            Ok(json!({"ingress":i,"onboarding":onboarding(i),"findings":findings}))
        }
        ["ingress", "verify", id] => {
            let i = forward_request(&format!("ingress/{id}"), "GET", json!({})).await?;
            Ok(json!({"ingress":i,"onboarding":onboarding(&i)}))
        }
        ["ingress", "verify", id, "--fresh"] => {
            forward_request(
                &format!("ingress/{id}/verification/reset"),
                "POST",
                json!({}),
            )
            .await?;
            Ok(onboarding(
                &forward_request(&format!("ingress/{id}"), "GET", json!({})).await?,
            ))
        }
        ["status"] => forward_request("status", "GET", json!({})).await,
        ["logs"] => Ok(forward_request("status", "GET", json!({})).await?["logs"].clone()),
        ["configure", "--stdin"] => forward_request("config/tunnel", "PATCH", stdin_json()?).await,
        ["network", "--stdin"] => forward_request("network", "PUT", stdin_json()?).await,
        ["connect"] => forward_request("start", "POST", json!({})).await,
        ["disconnect"] => forward_request("stop", "POST", json!({})).await,
        ["verify"] => Ok(
            json!({"ingresses":ingresses().await?.as_array().into_iter().flatten().map(onboarding).collect::<Vec<_>>()}),
        ),
        ["verify", "--fresh"] => {
            for i in ingresses().await?.as_array().into_iter().flatten() {
                forward_request(
                    &format!("ingress/{}/verification/reset", string(i, "id")),
                    "POST",
                    json!({}),
                )
                .await?;
            }
            Ok(ingresses().await?)
        }
        _ => Err("参数无效，请运行 cli help；不接受命令行密钥".into()),
    }
}

pub async fn run(args: Vec<String>) -> i32 {
    crate::init_crypto();
    let (output, code) = match execute(&args).await {
        Ok(result) => (json!({"schemaVersion":1,"ok":true,"result":result}), 0),
        Err(message) => {
            let code = if message.contains("请先打开 Local Connector") {
                "APP_UNAVAILABLE"
            } else if message == "UNKNOWN_ROUTE" {
                "APP_VERSION_MISMATCH"
            } else {
                "COMMAND_FAILED"
            };
            (
                json!({"schemaVersion":1,"ok":false,"error":{"code":code,"message":message}}),
                1,
            )
        }
    };
    println!("{output}");
    code
}
