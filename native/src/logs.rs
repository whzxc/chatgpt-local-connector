//! Append-only diagnostics, independent of connection configuration lifetimes.
use crate::*;
use std::io::{Read, Seek, SeekFrom, Write};
use std::sync::{Mutex as StdMutex, OnceLock};

fn lock() -> &'static StdMutex<()> {
    static LOCK: OnceLock<StdMutex<()>> = OnceLock::new();
    LOCK.get_or_init(Default::default)
}
fn append_locked(row: &Value) -> Result<()> {
    let dir = root().join("logs");
    private_dir(&dir)?;
    let path = dir.join(format!("{}.jsonl", chrono::Utc::now().format("%Y-%m-%d")));
    let mut options = std::fs::OpenOptions::new();
    options.create(true).append(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path).map_err(|e| e.to_string())?;
    let mut data = serde_json::to_vec(row).map_err(|e| e.to_string())?;
    data.push(b'\n');
    file.write_all(&data).map_err(|e| e.to_string())
}
pub fn append(row: Value) -> Result<()> {
    let _guard = lock().lock().map_err(|e| e.to_string())?;
    append_locked(&row)
}
pub fn record(level: &str, msg: &str, ingress: Option<&str>) {
    if let Err(error) = append(json!({"time":now(),"level":level,"msg":msg,"ingressId":ingress})) {
        eprintln!("Cannot persist diagnostics: {error}");
    }
}
/// Move existing connection logs before that connection can be deleted.
pub fn migrate(dir: &Path, ingress: &str) -> Result<()> {
    let path = dir.join("logs.json");
    if !path.exists() {
        return Ok(());
    }
    let _guard = lock().lock().map_err(|e| e.to_string())?;
    let rows = load(&path)?;
    for line in rows.as_array().ok_or("invalid connection logs")? {
        let mut row: Value = serde_json::from_str(line.as_str().ok_or("invalid log line")?)
            .map_err(|e| e.to_string())?;
        row["ingressId"] = json!(ingress);
        append_locked(&row)?;
    }
    std::fs::remove_file(path).map_err(|e| e.to_string())
}
/// Bound the display read, not the persisted history.
pub fn recent(ingress: Option<&str>) -> Vec<String> {
    let Ok(_guard) = lock().lock() else {
        return vec![];
    };
    let Ok(files) = std::fs::read_dir(root().join("logs")) else {
        return vec![];
    };
    let mut paths: Vec<_> = files
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "jsonl"))
        .collect();
    paths.sort();
    let mut rows = Vec::new();
    for path in paths.into_iter().rev() {
        let Ok(mut file) = std::fs::File::open(path) else {
            continue;
        };
        let Ok(meta) = file.metadata() else {
            continue;
        };
        let offset = meta.len().saturating_sub(256 * 1024);
        if file.seek(SeekFrom::Start(offset)).is_err() {
            continue;
        }
        let mut data = Vec::new();
        if file.read_to_end(&mut data).is_err() {
            continue;
        }
        for line in data.split(|b| *b == b'\n').rev() {
            let Ok(row) = serde_json::from_slice::<Value>(line) else {
                continue;
            };
            if ingress.is_none_or(|id| row["ingressId"] == id) {
                rows.push(row.to_string());
                if rows.len() == 200 {
                    rows.reverse();
                    return rows;
                }
            }
        }
    }
    rows.reverse();
    rows
}
