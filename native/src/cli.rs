use crate::{json, string, transport::forward_request, Value};
use std::io::Read;

const GUIDE: &str = include_str!("../../docs/codex-setup.md");

fn verification(status: &Value) -> Value {
    let chat = &status["core"]["chatgpt"];
    json!({"code":chat["code"],"verifiedAt":chat["verifiedAt"],"challengeVerifiedAt":chat["challengeVerifiedAt"],
        "connectionRunning":status["connection"]["running"],
        "prompt":format!("请使用 Local Connector 插件调用 connector_verify，code 为 {}。只验证连接，不创建任务。", string(chat,"code")),
        "executionVerified":false,
        "note":"verifiedAt 是历史入站记录，challengeVerifiedAt 仅由匹配验证码的 connector_verify 更新。新验收先执行 verify --fresh，再从 ChatGPT 调用并核对结果。任务执行需另读真实任务终态。"})
}

fn onboarding(status: &Value) -> Value {
    let https = status["config"]["connectionMode"] == "https";
    let ready = status["tunnel"]["state"] == "ready";
    let value = if https {
        &status["connection"]["mcpUrl"]
    } else {
        &status["config"]["tunnelId"]
    };
    json!({
        "stage": if status["config"]["configured"] != true { "configure" }
            else if !ready { "connect" }
            else if status["core"]["chatgpt"]["challengeVerifiedAt"].is_string() { "inbound_verified" }
            else { "chatgpt_setup_or_verify" },
        "chatgptUrl":"https://chatgpt.com/plugins",
        "connection":{"method":if https {"https"} else {"tunnel"},"value":value,
            "authentication":if https {json!("No authentication")} else {Value::Null},
            "name":"Local Connector","description":"连接这台电脑的项目与 Codex 任务"},
        "verification":verification(status),
        "executionPrompt":"请通过 Local Connector 创建一个无害 Codex 任务：不调用工具，不读取或修改文件，只回复 CLC_ONBOARDING_OK。使用唯一 UUID requestId；读取持久化回执、threadId、turnId 和任务终态，确认输出。未知状态用原 requestId 回读，不重复创建任务。",
        "browserSetup":"使用用户提供的已登录网页，按当前可见界面开启 Developer Mode、复用或创建连接、刷新工具并在新对话验证。身份确认、授权、验证码及安全机制阻断交给用户。",
        "installationState":"unknown",
        "note":"没有公开的 ChatGPT 创建连接 API 或预填协议可供本应用使用；打开页面不代表已安装。入站证据不识别调用方身份，需结合 ChatGPT 工具结果确认。任务执行必须单独验收。"
    })
}

async fn doctor() -> crate::Result<Value> {
    let status = forward_request("status", "GET", json!({})).await?;
    let config = &status["config"];
    let login = forward_request("codex/login", "GET", json!({}))
        .await
        .unwrap_or_else(|e| json!({"state":"unavailable","message":e}));
    let mut checks = vec![];
    let mut add = |code: &str, passed: bool, action: &str| {
        checks.push(json!({"code":code,"passed":passed,"action":if passed {""} else {action}}));
    };
    add(
        "PLATFORM_SUPPORTED",
        cfg!(target_os = "macos") || cfg!(windows),
        "Desktop 任务接入支持 macOS 和 Windows。",
    );
    add(
        "CONNECTION_CONFIGURED",
        config["configured"] == true,
        "补齐当前连接方式的配置；保留已有 HTTPS 或 Tunnel 选择。",
    );
    if config["connectionMode"] != "https" {
        add(
            "TUNNEL_ID",
            !string(config, "tunnelId").is_empty(),
            "打开 Platform Tunnel 设置取得 Tunnel ID，确认工作区关联与使用权限。",
        );
        add(
            "RUNTIME_KEY",
            config["hasApiKey"] == true,
            "从安全本机来源配置 runtime API Key；没有来源时请用户直接填入应用。",
        );
    }
    add(
        "CODEX_LOGIN",
        login["state"] == "authenticated",
        "在 Codex Desktop 完成登录。",
    );
    let desktop = status["autoOpenCodex"] != false;
    add(
        "CODEX_EXECUTOR",
        status["core"][if desktop { "desktop" } else { "appServer" }]["state"] == "ready",
        "检查所选 Codex 执行方状态；Desktop 模式打开 Desktop，后台模式检查 appServer。",
    );
    add(
        "TRANSPORT_READY",
        status["tunnel"]["state"] == "ready",
        "配置齐全后执行 connect；失败时读取 logs，按具体网络或凭据错误处理。",
    );
    add(
        "CHATGPT_INBOUND",
        !status["core"]["chatgpt"]["verifiedAt"].is_null(),
        "在 ChatGPT 添加或选用连接并发送 verify 返回的验证消息。",
    );
    let next = checks.iter().find(|v| v["passed"] != true).cloned();
    Ok(
        json!({"version":status["version"],"checks":checks,"next":next,
        "stage":if next.is_some(){"action_required"}else{"inbound_previously_verified"},
        "login":login,"tunnel":status["tunnel"],"desktop":status["core"]["desktop"],
        "verification":verification(&status),"onboarding":onboarding(&status)}),
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
            "commands":["help","guide","status","onboarding","doctor","logs","configure --stdin","network --stdin","connect","disconnect","verify","verify --fresh"],
            "configureInput":{"tunnelId":"可选；省略保留原值","apiKey":"可选；空字符串保留原值"},
            "networkInput":{"proxyMode":"system | direct | custom","proxyUrl":"自定义 HTTP/HTTPS 地址，其余为空"},
            "note":"所有命令输出 JSON。guide 可离线读取；其余命令需要已打开的同版本应用。凭据仅从 stdin 输入，禁止放入命令参数或聊天。"})),
        ["guide"] => Ok(json!({"markdown":GUIDE})),
        ["doctor"] => doctor().await,
        ["onboarding"] => Ok(onboarding(
            &forward_request("status", "GET", json!({})).await?,
        )),
        ["status"] => forward_request("status", "GET", json!({})).await,
        ["logs"] => Ok(forward_request("status", "GET", json!({})).await?["logs"].clone()),
        ["configure", "--stdin"] => forward_request("config/tunnel", "PATCH", stdin_json()?).await,
        ["network", "--stdin"] => forward_request("network", "PUT", stdin_json()?).await,
        ["connect"] => forward_request("start", "POST", json!({})).await,
        ["disconnect"] => forward_request("stop", "POST", json!({})).await,
        ["verify"] => Ok(verification(
            &forward_request("status", "GET", json!({})).await?,
        )),
        ["verify", "--fresh"] => {
            forward_request("verification/reset", "POST", json!({})).await?;
            Ok(verification(
                &forward_request("status", "GET", json!({})).await?,
            ))
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
