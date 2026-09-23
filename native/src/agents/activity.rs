use crate::*;
use std::{
    collections::BTreeSet,
    fs::File,
    io::{Read, Seek, SeekFrom},
    time::{Duration, Instant, SystemTime},
};

#[derive(Default)]
pub(super) struct Monitor {
    sampled: Option<Instant>,
    selected: Vec<String>,
    active: BTreeSet<String>,
}
impl Monitor {
    pub async fn sample(&mut self, mut selected: Vec<String>) -> BTreeSet<String> {
        selected.retain(|id| matches!(id.as_str(), "codex" | "claude"));
        selected.sort();
        selected.dedup();
        if self.selected != selected
            || self
                .sampled
                .is_none_or(|at| at.elapsed() >= Duration::from_secs(2))
        {
            self.selected = selected.clone();
            self.active = tokio::task::spawn_blocking(move || scan(&selected))
                .await
                .unwrap_or_default();
            self.sampled = Some(Instant::now());
        }
        self.active.clone()
    }
}

fn scan(selected: &[String]) -> BTreeSet<String> {
    let mut active = BTreeSet::new();
    let Some(home) = dirs::home_dir() else {
        return active;
    };
    let now = SystemTime::now();
    for agent in selected {
        let root = if agent == "codex" {
            std::env::var_os("CODEX_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| home.join(".codex"))
                .join("sessions")
        } else {
            std::env::var_os("CLAUDE_CONFIG_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|| home.join(".claude"))
                .join("projects")
        };
        let mut directories = vec![root];
        'tree: while let Some(dir) = directories.pop() {
            let Ok(entries) = std::fs::read_dir(dir) else {
                continue;
            };
            for entry in entries.flatten() {
                if entry.file_name().to_string_lossy().starts_with('.') {
                    continue;
                }
                let Ok(kind) = entry.file_type() else {
                    continue;
                };
                if kind.is_symlink() {
                    continue;
                }
                if kind.is_dir() {
                    directories.push(entry.path());
                    continue;
                }
                if !kind.is_file() || entry.path().extension().is_none_or(|ext| ext != "jsonl") {
                    continue;
                }
                let Ok(modified) = entry.metadata().and_then(|m| m.modified()) else {
                    continue;
                };
                if now.duration_since(modified).unwrap_or_default() > Duration::from_secs(300) {
                    continue;
                }
                if working(&entry.path(), agent, modified, now) {
                    active.insert(agent.clone());
                    break 'tree;
                }
            }
        }
    }
    active
}

fn working(path: &Path, agent: &str, modified: SystemTime, now: SystemTime) -> bool {
    let Ok(mut file) = File::open(path) else {
        return false;
    };
    let Ok(size) = file.seek(SeekFrom::End(0)) else {
        return false;
    };
    let start = size.saturating_sub(128 * 1024);
    if file.seek(SeekFrom::Start(start)).is_err() {
        return false;
    }
    let mut bytes = Vec::new();
    if file.take(128 * 1024).read_to_end(&mut bytes).is_err() {
        return false;
    }
    let mut lines = bytes.split(|b| *b == b'\n');
    if start > 0 {
        lines.next();
    }
    for line in lines.rev() {
        let Ok(record) = serde_json::from_slice::<Value>(line) else {
            continue;
        };
        let grace = if agent == "codex" {
            match string(&record["payload"], "type") {
                "task_complete" | "turn_aborted" => return false,
                "task_started"
                | "function_call_output"
                | "custom_tool_call_output"
                | "tool_search_output" => 90,
                "function_call" | "custom_tool_call" | "local_shell_call" | "web_search_call"
                | "tool_search_call" => 300,
                _ => continue,
            }
        } else {
            match string(&record, "type") {
                "assistant" => match record["message"]["stop_reason"].as_str() {
                    Some("tool_use") => 300,
                    Some(_) => return false,
                    None => 90,
                },
                "user" => {
                    let content = &record["message"]["content"];
                    let interrupted = content
                        .as_str()
                        .is_some_and(|s| s.contains("[Request interrupted by user"))
                        || content.as_array().is_some_and(|blocks| {
                            blocks
                                .iter()
                                .any(|b| string(b, "text").contains("[Request interrupted by user"))
                        });
                    if interrupted {
                        return false;
                    }
                    90
                }
                _ => continue,
            }
        };
        let timestamp = record["timestamp"]
            .as_str()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|at| SystemTime::from(at.with_timezone(&chrono::Utc)))
            .unwrap_or(modified);
        return now.duration_since(timestamp).unwrap_or_default() <= Duration::from_secs(grace);
    }
    now.duration_since(modified).unwrap_or_default() <= Duration::from_secs(30)
}
