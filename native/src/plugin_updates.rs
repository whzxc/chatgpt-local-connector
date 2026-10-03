//! Plugin delivery through the host CLI. Never write the host's installed cache.
use crate::*;
use base64::{engine::general_purpose::STANDARD, Engine};
use std::time::Duration;

const FEED: &str =
    "https://github.com/whzxc/chatgpt-local-connector/releases/latest/download/latest.json";
const RELEASES: &str = "https://github.com/whzxc/chatgpt-local-connector/releases/download";
const PLUGIN: &str = "clc@local-connector";
const VERSION: &str = env!("CARGO_PKG_VERSION");

struct PendingExport(Option<PathBuf>);
impl Drop for PendingExport {
    fn drop(&mut self) {
        if let Some(path) = &self.0 {
            let _ = std::fs::remove_dir_all(path);
        }
    }
}

fn directory() -> PathBuf {
    root().join("plugin-updates")
}
fn version(value: &str) -> Result<semver::Version> {
    semver::Version::parse(value).map_err(|_| "无效的插件版本".into())
}
fn lock() -> Result<std::fs::File> {
    private_dir(&directory())?;
    let file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(directory().join("update.lock"))
        .map_err(|e| e.to_string())?;
    file.try_lock().map_err(|_| "插件正在更新，请稍后重试")?;
    Ok(file)
}
async fn command(binary: &Path, args: &[&str]) -> Result<String> {
    let mut command = tokio::process::Command::new(binary);
    command.args(args).kill_on_drop(true);
    #[cfg(windows)]
    command.creation_flags(0x08000000);
    let output = tokio::time::timeout(Duration::from_secs(120), command.output())
        .await
        .map_err(|_| "插件安装命令超时；请检查宿主安装状态后重试")?
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(format!(
            "插件安装命令失败：{}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    String::from_utf8(output.stdout).map_err(|e| e.to_string())
}
async fn host(args: &[&str]) -> Result<Value> {
    let binary = crate::desktop::require_installation()?.binary;
    serde_json::from_str(&command(&binary, args).await?).map_err(|e| e.to_string())
}
async fn installed() -> Result<Option<Value>> {
    let list = host(&["plugin", "list", "--json"]).await?;
    Ok(list["installed"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|p| p["pluginId"] == PLUGIN)
        .cloned())
}
fn enabled(plugin: &Value) -> Result<()> {
    if plugin["enabled"] == false {
        return Err("CLC 插件已禁用；请先在宿主中启用，再更新。".into());
    }
    Ok(())
}
async fn bytes(client: &reqwest::Client, url: &str, limit: usize) -> Result<Vec<u8>> {
    let mut response = client
        .get(url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?;
    if response
        .content_length()
        .is_some_and(|size| size > limit as u64)
    {
        return Err("插件更新文件超过大小限制".into());
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
        if bytes.len() + chunk.len() > limit {
            return Err("插件更新文件超过大小限制".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}
async fn client() -> Result<reqwest::Client> {
    let preferences = load(&root().join("web/preferences.json"))
        .unwrap_or_else(|_| json!({"proxyMode":"system","proxyUrl":""}));
    crate::proxy::NetworkProxy::resolve(&preferences)
        .await?
        .client(reqwest::Client::builder())
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|e| e.to_string())
}
async fn feed(force: bool) -> Result<Value> {
    let path = directory().join("latest.json");
    if !force {
        if let Ok(cached) = load(&path) {
            let age = chrono::Utc::now().timestamp() - cached["checkedAt"].as_i64().unwrap_or(0);
            if (0..3600).contains(&age) {
                return Ok(cached["feed"].clone());
            }
        }
    }
    let value: Value = serde_json::from_slice(&bytes(&client().await?, FEED, 256 * 1024).await?)
        .map_err(|e| e.to_string())?;
    version(string(&value, "version"))?;
    save(
        &path,
        &json!({"checkedAt":chrono::Utc::now().timestamp(),"feed":value}),
    )?;
    Ok(value)
}
fn asset(feed: &Value) -> Result<&Value> {
    let platform = if cfg!(target_os = "macos") {
        "darwin-aarch64"
    } else {
        "windows-x86_64"
    };
    let asset = &feed["plugins"][platform];
    let expected = format!(
        "{RELEASES}/v{}/CLC.Core_{}_{platform}.bin",
        string(feed, "version"),
        string(feed, "version")
    );
    if asset["url"] != expected || !asset["signature"].is_string() {
        return Err("该版本尚未提供可验证的插件自动更新包，请从发布页手动安装。".into());
    }
    Ok(asset)
}
pub async fn check(force: bool) -> Result<Value> {
    let plugin = installed().await?;
    let installed_version = plugin
        .as_ref()
        .and_then(|p| p["version"].as_str())
        .unwrap_or(VERSION);
    let latest = feed(force).await?;
    let next = string(&latest, "version");
    let newer = version(next)? > version(installed_version)?;
    Ok(
        json!({"currentVersion":VERSION,"installedVersion":installed_version,"version":next,
        "available":newer,"installable":newer && asset(&latest).is_ok() && plugin.is_some(),
        "reloadRequired":version(installed_version)? > version(VERSION)?,
        "notes":latest["notes"],"installed":plugin.is_some(),"enabled":plugin.as_ref().map(|p|p["enabled"].clone()),
        "downloadsUrl":"https://github.com/whzxc/chatgpt-local-connector/releases/latest"}),
    )
}
fn verify(bytes: &[u8], signature: &str) -> Result<()> {
    let config: Value = serde_json::from_str(include_str!("../../desktop/tauri.conf.json"))
        .map_err(|e| e.to_string())?;
    let decode = |value: &str| -> Result<String> {
        String::from_utf8(STANDARD.decode(value.trim()).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())
    };
    let key = minisign_verify::PublicKey::decode(&decode(string(
        &config["plugins"]["updater"],
        "pubkey",
    ))?)
    .map_err(|e| e.to_string())?;
    let sig = minisign_verify::Signature::decode(&decode(signature)?).map_err(|e| e.to_string())?;
    key.verify(bytes, &sig, true)
        .map_err(|_| "插件签名校验失败；未执行或安装下载的文件".into())
}

// The source is stable; only its relative plugin path changes. The host owns its cache.
async fn install_source(binary: &Path, target_version: &str, plugin: &Value) -> Result<Value> {
    enabled(plugin)?;
    let actual = command(binary, &["--version"]).await?;
    if actual.trim() != target_version {
        return Err("插件内核版本与更新元数据不一致".into());
    }
    if plugin["marketplaceSource"]["sourceType"] != "local" {
        return Err("当前插件由 Git 市场管理；请使用宿主的市场更新功能。".into());
    }
    let source = PathBuf::from(string(&plugin["marketplaceSource"], "source"));
    let manifest = source.join(".agents/plugins/marketplace.json");
    let managed = source.join(".clc-updates");
    private_dir(&managed)?;
    let source_lock = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(managed.join("update.lock"))
        .map_err(|e| e.to_string())?;
    source_lock
        .try_lock()
        .map_err(|_| "该宿主的插件正在更新，请稍后重试")?;
    let previous =
        load(&manifest).map_err(|_| "插件原始市场目录不可读；请恢复该目录或手动安装完整插件包")?;
    if previous["name"] != "local-connector" {
        return Err("插件市场身份不一致".into());
    }
    let current = installed().await?.ok_or("插件已被卸载，请重新安装后重试")?;
    enabled(&current)?;
    if current["marketplaceSource"] != plugin["marketplaceSource"] {
        return Err("插件来源已变化，请重新检查后更新".into());
    }
    if version(string(&current, "version"))? >= version(target_version)? {
        return Ok(
            json!({"state":"current","version":current["version"],"reloadRequired":version(string(&current,"version"))? > version(VERSION)?}),
        );
    }
    let relative = format!("./.clc-updates/{target_version}-{}", id());
    let target = source.join(relative.trim_start_matches("./"));
    if target.exists() {
        return Err("插件暂存目录冲突，请重试".into());
    }
    let mut pending = PendingExport(Some(target.clone()));
    command(
        binary,
        &["export", target.to_str().ok_or("插件目录不是 UTF-8")?],
    )
    .await?;
    let mut next = previous.clone();
    let entry = next["plugins"]
        .as_array_mut()
        .and_then(|entries| entries.iter_mut().find(|entry| entry["name"] == "clc"))
        .ok_or("插件市场缺少 CLC 条目")?;
    entry["source"] = json!({"source":"local","path":relative});
    save(&manifest, &next)?;
    pending.0.take();
    let result: Result<Value> = async {
        host(&["plugin","add",PLUGIN,"--json"]).await?;
        let installed = installed().await?.ok_or("宿主未确认插件安装")?;
        if installed["version"] != target_version { return Err("宿主回读的插件版本不一致".into()); }
        Ok(json!({"version":target_version,"installedVersion":target_version,"state":"installed","reloadRequired":true}))
    }.await;
    if let Err(error) = result {
        save(&manifest, &previous)?;
        let rollback = host(&["plugin", "add", PLUGIN, "--json"]).await;
        let _ = std::fs::remove_dir_all(&target);
        return Err(match rollback {
            Ok(_) => format!("{error}；已恢复原插件来源"),
            Err(e) => format!("{error}；恢复原插件来源失败：{e}"),
        });
    }
    // Keep current and one previous managed source. Preserve the user's original ZIP export.
    let keep = previous["plugins"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|entry| entry["name"] == "clc")
        .and_then(|entry| entry["source"]["path"].as_str());
    if let Ok(entries) = std::fs::read_dir(&managed) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir()
                && path != target
                && keep.is_none_or(|p| source.join(p) != path)
                && load(&path.join(".codex-plugin/plugin.json")).is_ok_and(|v| v["name"] == "clc")
            {
                let _ = std::fs::remove_dir_all(path);
            }
        }
    }
    result
}

/// Desktop startup and explicit retry: use its bundled Core, with no second download.
pub async fn sync() -> Result<Value> {
    if crate::desktop::installation().is_none() {
        return Ok(json!({"state":"not_installed"}));
    }
    let _lock = lock()?;
    let Some(plugin) = installed().await? else {
        return Ok(json!({"state":"not_installed"}));
    };
    if version(string(&plugin, "version"))? >= version(VERSION)? {
        return Ok(json!({"state":"current","version":plugin["version"]}));
    }
    install_source(&crate::runtime::binary()?, VERSION, &plugin).await
}
pub async fn install(expected: &str) -> Result<Value> {
    let _lock = lock()?;
    let plugin = installed().await?.ok_or("请先在宿主安装 CLC 插件")?;
    enabled(&plugin)?;
    let latest = feed(true).await?;
    if latest["version"] != expected {
        return Err("新版本已变化，请重新检查后更新".into());
    }
    if version(expected)? <= version(string(&plugin, "version"))? {
        return Ok(
            json!({"state":"current","version":plugin["version"],"reloadRequired":version(string(&plugin,"version"))? > version(VERSION)?}),
        );
    }
    let asset = asset(&latest)?;
    let downloaded = bytes(&client().await?, string(asset, "url"), 128 * 1024 * 1024).await?;
    verify(&downloaded, string(asset, "signature"))?;
    let staged = directory().join(if cfg!(windows) {
        "download.exe"
    } else {
        "download"
    });
    save_bytes(&staged, &downloaded)?;
    drop(downloaded);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&staged, std::fs::Permissions::from_mode(0o700))
            .map_err(|e| e.to_string())?;
    }
    // export resolves its own executable by name.
    let executable = directory().join(if cfg!(windows) {
        "local-connector.exe"
    } else {
        "local-connector"
    });
    std::fs::rename(&staged, &executable).map_err(|e| e.to_string())?;
    let result = install_source(&executable, expected, &plugin).await;
    let _ = std::fs::remove_file(executable);
    result
}
