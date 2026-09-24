//! Immutable large-result snapshots. Event logs retain their separate lifecycle.
use crate::*;
use std::io::Read;
use std::time::{Duration, SystemTime};
const QUOTA: u64 = 1024 * 1024 * 1024;
const TTL: Duration = Duration::from_secs(7 * 24 * 60 * 60);
static WRITER: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub fn page_output(result: Value) -> Result<Value> {
    let text = result.to_string();
    if text.len() <= 64 * 1024 {
        return Ok(result);
    }
    let output = id();
    let dir = root().join("outputs");
    let _writer = WRITER.lock().map_err(|e| e.to_string())?;
    private_dir(&dir)?;
    make_room(&dir, text.len() as u64)?;
    save(&dir.join(format!("{output}.json")), &result)?;
    Ok(
        json!({"outputId":output,"bytes":text.len(),"characters":text.encode_utf16().count(),"sha256":hash(&text),"nextAction":"control_output","format":"JSON; offsets count UTF-16 code units"}),
    )
}
fn make_room(dir: &Path, incoming: u64) -> Result<()> {
    if incoming > QUOTA {
        return Err("OUTPUT_STORAGE_LIMIT".into());
    }
    let mut snapshots = Vec::new();
    let mut total = 0u64;
    for entry in std::fs::read_dir(dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        // Never evict append-only native event logs or unrelated files.
        if path.extension().is_none_or(|ext| ext != "json")
            || path
                .file_stem()
                .and_then(|s| s.to_str())
                .is_none_or(|s| uuid::Uuid::parse_str(s).is_err())
        {
            continue;
        }
        let meta = entry.metadata().map_err(|e| e.to_string())?;
        if !meta.is_file() {
            continue;
        }
        let modified = meta.modified().map_err(|e| e.to_string())?;
        total = total.saturating_add(meta.len());
        snapshots.push((modified, path, meta.len()));
    }
    snapshots.sort_by_key(|(modified, _, _)| *modified);
    for (modified, path, size) in snapshots {
        let expired = SystemTime::now()
            .duration_since(modified)
            .unwrap_or_default()
            >= TTL;
        if !expired && total.saturating_add(incoming) <= QUOTA {
            break;
        }
        // On Windows an open reader may prevent deletion. Try other candidates;
        // readers never hold the writer lock and never observe a replaced snapshot.
        if std::fs::remove_file(path).is_ok() {
            total = total.saturating_sub(size);
        }
    }
    if total.saturating_add(incoming) > QUOTA {
        return Err("OUTPUT_STORAGE_LIMIT".into());
    }
    Ok(())
}
pub fn read_output(args: &Value) -> Result<Value> {
    let output = string(args, "outputId");
    uuid::Uuid::parse_str(output).map_err(|_| "INVALID_OUTPUT_ID")?;
    let dir = root().join("outputs");
    let path = dir.join(format!("{output}.json"));
    let snapshot = path.exists();
    let file = std::fs::File::open(if snapshot {
        path
    } else {
        dir.join(format!("{output}.jsonl"))
    })
    .map_err(|e| e.to_string())?;
    if snapshot
        && file
            .metadata()
            .and_then(|m| m.modified())
            .map_err(|e| e.to_string())?
            .elapsed()
            .unwrap_or_default()
            >= TTL
    {
        return Err("OUTPUT_EXPIRED".into());
    }
    let offset = num(args, "offset", 0);
    let length = num(args, "length", 10000).min(12000);
    let (text, characters) = read_slice(file, offset, length)?;
    let end = offset.saturating_add(length).min(characters);
    Ok(
        json!({"outputId":output,"offset":offset,"text":text,"characters":characters,"nextOffset":if end<characters{Some(end)}else{None}}),
    )
}
// Preserve the published UTF-16 offsets without allocating the entire file or a
// second full UTF-16 copy. UTF-8 tails cross chunk boundaries intact.
fn read_slice(mut file: std::fs::File, offset: usize, length: usize) -> Result<(String, usize)> {
    let mut buffer = vec![0u8; 64 * 1024 + 4];
    let mut carry = 0;
    let mut characters = 0usize;
    let mut selected = Vec::with_capacity(length);
    // Bound event-log reads to the length at open; later appends belong to the next call.
    let mut remaining = file.metadata().map_err(|e| e.to_string())?.len();
    while remaining > 0 || carry > 0 {
        let limit = remaining.min(64 * 1024) as usize;
        let read = file
            .read(&mut buffer[carry..carry + limit])
            .map_err(|e| e.to_string())?;
        remaining = remaining.saturating_sub(read as u64);
        let available = carry + read;
        let valid = match std::str::from_utf8(&buffer[..available]) {
            Ok(_) => available,
            Err(error) if error.error_len().is_none() && read > 0 && remaining > 0 => {
                error.valid_up_to()
            }
            Err(error) => return Err(error.to_string()),
        };
        let text = std::str::from_utf8(&buffer[..valid]).map_err(|e| e.to_string())?;
        for unit in text.encode_utf16() {
            if characters >= offset && selected.len() < length {
                selected.push(unit);
            }
            characters += 1;
        }
        carry = available - valid;
        buffer.copy_within(valid..available, 0);
        if read == 0 {
            break;
        }
    }
    Ok((String::from_utf16_lossy(&selected), characters))
}
