use super::*;
use std::io::{BufRead, BufReader, Read};
fn counts(v: &Value) -> [u64; 3] {
    [
        v["input_tokens"].as_u64().unwrap_or(0),
        v["cached_input_tokens"].as_u64().unwrap_or(0),
        v["output_tokens"].as_u64().unwrap_or(0),
    ]
}
pub(super) fn scan(path: &Path) -> Scan {
    let mut out = Scan::default();
    let Ok(file) = std::fs::File::open(path) else {
        out.incomplete = true;
        return out;
    };
    let mut reader = BufReader::new(file);
    let mut bytes = Vec::new();
    let mut model = String::new();
    let mut previous = [0; 3];
    let mut saw_meta = false;
    let mut gate = None;
    let mut fast = false;
    loop {
        bytes.clear();
        let read = reader
            .by_ref()
            .take(2 * 1024 * 1024)
            .read_until(b'\n', &mut bytes);
        match read {
            Ok(0) => break,
            Err(_) => {
                out.incomplete = true;
                break;
            }
            _ => (),
        }
        if bytes.len() == 2 * 1024 * 1024 && bytes.last() != Some(&b'\n') {
            out.incomplete = true;
            loop {
                bytes.clear();
                if reader
                    .by_ref()
                    .take(64 * 1024)
                    .read_until(b'\n', &mut bytes)
                    .unwrap_or(0)
                    == 0
                    || bytes.last() == Some(&b'\n')
                {
                    break;
                }
            }
            continue;
        }
        if !bytes.windows(11).any(|w| w == b"token_count")
            && !bytes.windows(12).any(|w| w == b"turn_context")
            && !bytes.windows(12).any(|w| w == b"session_meta")
            && !bytes.windows(12).any(|w| w == b"task_started")
            && !bytes.windows(23).any(|w| w == b"thread_settings_applied")
        {
            continue;
        }
        let Ok(v) = serde_json::from_slice::<Value>(&bytes) else {
            out.incomplete = true;
            continue;
        };
        let p = &v["payload"];
        let at = v["timestamp"].as_str().and_then(time);
        if v["type"] == "session_meta" && !saw_meta {
            saw_meta = true;
            if !p["forked_from_id"].is_null()
                || !p["parent_thread_id"].is_null()
                || p["thread_source"] == "subagent"
                || !p["source"]["subagent"].is_null()
            {
                gate = Some(at.map(|n| n / 1000).unwrap_or(i64::MAX));
            }
        }
        if v["type"] == "turn_context" {
            if let Some(name) = p["model"]
                .as_str()
                .or(p["model_name"].as_str())
                .or(p["metadata"]["model"].as_str())
            {
                model = name.into()
            }
        }
        if p["type"] == "thread_settings_applied" {
            if let Some(tier) = p["service_tier"]
                .as_str()
                .or(p["thread_settings"]["service_tier"].as_str())
            {
                fast = ["fast", "priority"].contains(&tier);
            }
        }
        if p["type"] == "task_started" {
            if let (Some(threshold), Some(start)) = (gate, p["started_at"].as_f64()) {
                if start >= threshold as f64 {
                    gate = None;
                }
            }
        }
        if v["type"] != "event_msg" || p["type"] != "token_count" {
            continue;
        }
        let info = &p["info"];
        let total = info["total_token_usage"]
            .is_object()
            .then(|| counts(&info["total_token_usage"]));
        if gate.is_some() {
            if let Some(t) = total {
                previous = t
            }
            continue;
        }
        if total == Some(previous) {
            continue;
        }
        let current = if info["last_token_usage"].is_object() {
            counts(&info["last_token_usage"])
        } else if let Some(t) = total {
            std::array::from_fn(|i| t[i].saturating_sub(previous[i]))
        } else {
            continue;
        };
        if let Some(t) = total {
            previous = t
        }
        let Some(at) = at else {
            out.incomplete = true;
            continue;
        };
        let name = p["model"]
            .as_str()
            .or(info["model"].as_str())
            .unwrap_or(&model)
            .to_owned();
        out.events.push(Event {
            at,
            model: name,
            input: current[0].saturating_sub(current[1]),
            cached: current[1].min(current[0]),
            output: current[2],
            write: 0,
            fast,
        });
    }
    out
}
