use serde_json::{json, Value};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::sync::Mutex;
pub mod control;
pub mod desktop;
pub mod projects;
pub mod rpc;
pub mod service;
pub mod transport;
pub mod https;
pub type Result<T> = std::result::Result<T, String>;
pub fn now() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}
pub fn id() -> String {
    uuid::Uuid::new_v4().to_string()
}
pub fn hash(value: impl AsRef<[u8]>) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(value.as_ref()))
}
pub fn string<'a>(v: &'a Value, key: &str) -> &'a str {
    v[key].as_str().unwrap_or("")
}
pub fn num(v: &Value, key: &str, default: usize) -> usize {
    v[key].as_u64().unwrap_or(default as u64) as usize
}
pub fn root() -> PathBuf {
    std::env::var_os("CLC_STATE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            if cfg!(windows) {
                dirs::data_local_dir()
                    .unwrap()
                    .join("chatgpt-local-connector")
            } else {
                dirs::home_dir()
                    .unwrap()
                    .join(".local/state/chatgpt-local-connector")
            }
        })
}
pub fn private_dir(path: &Path) -> Result<()> {
    std::fs::create_dir_all(path).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))
            .map_err(|e| e.to_string())?;
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        static PROTECTED: std::sync::OnceLock<
            std::sync::Mutex<std::collections::HashSet<PathBuf>>,
        > = std::sync::OnceLock::new();
        let mut protected = PROTECTED
            .get_or_init(Default::default)
            .lock()
            .map_err(|e| e.to_string())?;
        if !protected.contains(path) {
            let who = std::process::Command::new("whoami.exe")
                .args(["/user", "/fo", "csv", "/nh"])
                .creation_flags(0x08000000)
                .output()
                .map_err(|e| e.to_string())?;
            let text = String::from_utf8_lossy(&who.stdout);
            let sid = text
                .trim()
                .rsplit(',')
                .next()
                .unwrap_or("")
                .trim_matches('"');
            if !who.status.success() || !sid.starts_with("S-1-") {
                return Err("无法确认本机账户".into());
            }
            let status = std::process::Command::new("icacls.exe")
                .arg(path)
                .args(["/inheritance:r", "/grant:r", &format!("*{sid}:(OI)(CI)F")])
                .creation_flags(0x08000000)
                .output()
                .map_err(|e| e.to_string())?;
            if !status.status.success() {
                return Err("无法保护私有目录".into());
            }
            protected.insert(path.into());
        }
    }
    Ok(())
}
pub fn save(path: &Path, value: &Value) -> Result<()> {
    private_dir(path.parent().ok_or("invalid path")?)?;
    let temp = path.with_extension(format!("{}.tmp", id()));
    std::fs::write(&temp, serde_json::to_vec(value).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&temp, std::fs::Permissions::from_mode(0o600))
            .map_err(|e| e.to_string())?;
    }
    std::fs::rename(&temp, path).map_err(|e| e.to_string())
}
pub fn load(path: &Path) -> Result<Value> {
    serde_json::from_slice(&std::fs::read(path).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
}
pub fn command(binary: impl AsRef<std::ffi::OsStr>) -> tokio::process::Command {
    let mut c = tokio::process::Command::new(binary);
    c.kill_on_drop(true);
    #[cfg(windows)]
    c.creation_flags(0x08000000);
    c
}
pub async fn output(binary: impl AsRef<std::ffi::OsStr>, args: &[&str]) -> Result<String> {
    let out = tokio::time::timeout(
        std::time::Duration::from_secs(60),
        command(binary).args(args).output(),
    )
    .await
    .map_err(|_| "COMMAND_TIMEOUT")?
    .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().into());
    }
    Ok(String::from_utf8_lossy(&out.stdout).into())
}
#[derive(Default)]
pub struct Events {
    pub rows: Vec<Value>,
    pub bytes: usize,
    pub sequence: u64,
    pub pending: std::collections::HashMap<String, Value>,
    pub storage_error: Option<String>,
}
pub type SharedEvents = Arc<Mutex<Events>>;
pub async fn event(events: &SharedEvents, session: &str, method: &str, params: Value) {
    let mut e = events.lock().await;
    e.sequence += 1;
    let row = json!({"cursor":e.sequence,"backendSession":session,"method":method,"params":params,"time":now()});
    e.bytes += row.to_string().len();
    e.rows.push(row.clone());
    while e.rows.len() > 2000 || e.bytes > 8 * 1024 * 1024 {
        let removed = e.rows.remove(0);
        e.bytes -= removed.to_string().len();
    }
    let dir = root().join("outputs");
    let result = (|| -> Result<()> {
        use std::io::Write;
        private_dir(&dir)?;
        let mut opts = std::fs::OpenOptions::new();
        opts.create(true).append(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            opts.mode(0o600);
        }
        let mut f = opts
            .open(dir.join(format!("{session}.jsonl")))
            .map_err(|x| x.to_string())?;
        writeln!(f, "{row}").map_err(|x| x.to_string())
    })();
    if let Err(err) = result {
        e.storage_error = Some(err);
    }
}
pub fn catalog() -> Value {
    serde_json::from_str(include_str!("catalog.json")).expect("tool catalog")
}

// Bound allocation before accepting a complete line from a child or socket.
pub async fn line<R: tokio::io::AsyncBufRead + Unpin>(
    reader: &mut R,
    cap: usize,
) -> Result<Vec<u8>> {
    use tokio::io::AsyncBufReadExt;
    let mut out = Vec::new();
    loop {
        let buf = reader.fill_buf().await.map_err(|e| e.to_string())?;
        if buf.is_empty() {
            return Ok(out);
        }
        let n = buf
            .iter()
            .position(|b| *b == b'\n')
            .map(|i| i + 1)
            .unwrap_or(buf.len());
        if out.len() + n > cap {
            return Err("FRAME_TOO_LARGE".into());
        }
        let done = buf[n - 1] == b'\n';
        out.extend_from_slice(&buf[..n]);
        reader.consume(n);
        if done {
            return Ok(out);
        }
    }
}

pub fn init_crypto() {
    static INIT: std::sync::Once = std::sync::Once::new();
    INIT.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}
