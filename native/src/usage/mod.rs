//! Device-local usage projection shared by the desktop and plugin.
//! Response usage and legacy cumulative usage are separate counting families.
use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::sync::{Mutex as SyncMutex, OnceLock, RwLock};
use std::time::{Duration, Instant};

const SCHEMA: u32 = 1;
const CACHE_SCHEMA: u32 = 9;
const MAX_LINE: u64 = 8 * 1024 * 1024;
#[derive(Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tokens {
    pub input: Option<u64>,
    pub cached: Option<u64>,
    pub output: Option<u64>,
    pub reasoning: Option<u64>,
}
impl Tokens {
    fn read(v: &Value) -> Self {
        Self {
            input: v["input_tokens"].as_u64(),
            cached: v["cached_input_tokens"].as_u64(),
            output: v["output_tokens"].as_u64(),
            reasoning: v["reasoning_output_tokens"].as_u64(),
        }
    }
    pub fn total(&self) -> Option<u64> {
        self.input?.checked_add(self.output?)
    }
    fn valid(&self) -> bool {
        !matches!((self.input, self.cached), (Some(i), Some(c)) if c > i)
            && !matches!((self.output, self.reasoning), (Some(o), Some(r)) if r > o)
    }
    fn delta(&self, prev: &Self) -> Option<Self> {
        Some(Self {
            input: Some(self.input?.checked_sub(prev.input?)?),
            output: Some(self.output?.checked_sub(prev.output?)?),
            cached: self
                .cached
                .zip(prev.cached)
                .and_then(|(a, b)| a.checked_sub(b)),
            reasoning: self
                .reasoning
                .zip(prev.reasoning)
                .and_then(|(a, b)| a.checked_sub(b)),
        })
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    pub id: String,
    pub thread_id: String,
    pub turn_id: Option<String>,
    pub at: i64,
    pub model: Option<String>,
    pub effort: Option<String>,
    pub service_tier: Option<String>,
    pub tokens: Tokens,
    pub family: String,
    pub reliable: bool,
}
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Turn {
    id: String,
    prompt: Option<String>,
    prompt_images: usize,
    context_window: Option<u64>,
    started_at: Option<i64>,
    completed_at: Option<i64>,
    duration_ms: Option<u64>,
    ttft_ms: Option<u64>,
    status: String,
    model: Option<String>,
    effort: Option<String>,
}
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Tool {
    id: String,
    turn_id: Option<String>,
    name: Option<String>,
    at: i64,
    output_bytes: Option<u64>,
    started_at: Option<i64>,
    completed_at: Option<i64>,
    status: Option<String>,
}
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Projection {
    thread_id: String,
    parent_id: Option<String>,
    forked_from_id: Option<String>,
    related: bool,
    cli_version: Option<String>,
    model: Option<String>,
    effort: Option<String>,
    tier: Option<String>,
    turn: Option<String>,
    modern: bool,
    own_started: bool,
    created_at: Option<i64>,
    previous: Option<Tokens>,
    legacy_ambiguous: bool,
    reported_total: Option<(i64, Tokens)>,
    responses: BTreeMap<String, Response>,
    legacy: BTreeMap<String, Response>,
    turns: BTreeMap<String, Turn>,
    tools: BTreeMap<String, Tool>,
    compactions: BTreeMap<String, Value>,
    issues: BTreeSet<String>,
    last_event_at: Option<i64>,
}
fn text(v: &Value, key: &str) -> Option<String> {
    v[key].as_str().filter(|s| !s.is_empty()).map(str::to_owned)
}
fn time(v: &Value) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(v.as_str()?)
        .ok()
        .map(|d| d.timestamp_millis())
}
fn seconds(v: &Value) -> Option<i64> {
    v.as_f64().map(|n| (n * 1000.) as i64)
}
impl Projection {
    fn ingest(&mut self, v: Value) {
        let p = &v["payload"];
        let at = time(&v["timestamp"]);
        let kind = string(&v, "type");
        if kind == "session_meta" {
            if self.thread_id.is_empty() {
                self.thread_id = string(p, "id").into();
                self.created_at = at;
                self.parent_id = text(p, "parent_thread_id")
                    .or_else(|| text(&p["source"]["subagent"]["thread_spawn"], "parent_thread_id"));
                self.forked_from_id = text(p, "forked_from_id");
                self.related = self.parent_id.is_some()
                    || self.forked_from_id.is_some()
                    || !p["source"]["subagent"].is_null()
                    || p["thread_source"] == "subagent";
                self.cli_version = text(p, "cli_version");
            }
            return;
        }
        if self.thread_id.is_empty() {
            self.issues.insert("missing-session-identity".into());
            return;
        }
        if self.related
            && kind != "token_usage_record"
            && !(kind == "event_msg" && p["type"] == "token_count")
            && at
                .zip(self.created_at)
                .is_some_and(|(event, created)| event < created)
        {
            return;
        }
        self.last_event_at = self.last_event_at.max(at);
        if kind == "turn_context" {
            self.turn = text(p, "turn_id");
            self.model = text(p, "model");
            self.effort = text(p, "effort");
            if let Some(id) = &self.turn {
                let t = self.turns.entry(id.clone()).or_default();
                t.id = id.clone();
                t.model = self.model.clone();
                t.effort = self.effort.clone();
            }
        }
        if kind == "token_usage_record" {
            self.modern = true;
            if p["thread_id"] != self.thread_id {
                self.issues.insert("foreign-thread-record-excluded".into());
                return;
            }
            let Some(id) = text(p, "response_id") else {
                self.issues.insert("missing-response-id".into());
                return;
            };
            let Some(at) = at else {
                self.issues.insert("missing-event-time".into());
                return;
            };
            let tokens = Tokens::read(&p["usage"]);
            if !tokens.valid() || tokens.total().is_none() {
                self.issues.insert("invalid-or-missing-token-fields".into());
            }
            if let Some(old) = self.responses.get_mut(&id) {
                if serde_json::to_value(&old.tokens).ok() != serde_json::to_value(&tokens).ok() {
                    old.reliable = false;
                    self.issues.insert("conflicting-response-usage".into());
                }
                return;
            }
            if p["thread_token_usage"].is_object() {
                self.reported_total = Some((at, Tokens::read(&p["thread_token_usage"])));
            }
            let turn_id = text(p, "turn_id");
            if turn_id.is_none() {
                self.issues.insert("missing-turn-id".into());
            }
            self.responses.insert(
                id.clone(),
                Response {
                    id,
                    thread_id: self.thread_id.clone(),
                    turn_id,
                    at,
                    model: self.model.clone(),
                    effort: self.effort.clone(),
                    service_tier: self.tier.clone(),
                    reliable: tokens.valid() && tokens.total().is_some(),
                    tokens,
                    family: "response".into(),
                },
            );
        }
        if kind == "compacted" {
            // latest_token_usage_record is a reference, never a second charge.
            let id = text(p, "compaction_response_id").unwrap_or_else(|| format!("at:{at:?}"));
            self.compactions.insert(
                id.clone(),
                json!({"responseId":id,"turnId":self.turn,"at":at}),
            );
        }
        if kind == "event_msg" {
            match string(p, "type") {
                "thread_settings_applied" => {
                    self.tier = text(p, "service_tier")
                        .or_else(|| text(&p["thread_settings"], "service_tier"));
                }
                "task_started" | "task_complete" | "turn_aborted" => {
                    let sub = string(p, "type");
                    let id = text(p, "turn_id").or_else(|| self.turn.clone());
                    if let Some(id) = id {
                        self.turn = Some(id.clone());
                        let t = self.turns.entry(id.clone()).or_default();
                        t.id = id;
                        if sub == "task_started" {
                            self.own_started = !self.related
                                || self
                                    .created_at
                                    .zip(seconds(&p["started_at"]).or(at))
                                    .is_some_and(|(created, started)| {
                                        started / 1000 >= created / 1000
                                    });

                            t.context_window =
                                p["model_context_window"].as_u64().filter(|n| *n > 0);
                            t.started_at = seconds(&p["started_at"]).or(at);
                            t.status = "running-at-last-event".into();
                        } else {
                            t.completed_at = seconds(&p["completed_at"]).or(at);
                            t.duration_ms = p["duration_ms"].as_u64();
                            t.ttft_ms = p["time_to_first_token_ms"].as_u64();
                            t.status = if sub == "task_complete" {
                                "completed"
                            } else {
                                "interrupted"
                            }
                            .into();
                        }
                    }
                }
                "user_message" => {
                    if let Some(id) = &self.turn {
                        let turn = self.turns.entry(id.clone()).or_default();
                        turn.id = id.clone();
                        if turn.prompt.is_none() {
                            turn.prompt = text(p, "message");
                        }
                    }
                }
                "token_count" => {
                    if let Some(turn) = self.turn.as_ref().and_then(|id| self.turns.get_mut(id)) {
                        if turn.context_window.is_none() {
                            turn.context_window = p["info"]["model_context_window"]
                                .as_u64()
                                .filter(|n| *n > 0);
                        }
                    }
                    let total = &p["info"]["total_token_usage"];
                    if !total.is_object() {
                        self.issues
                            .insert("legacy-missing-cumulative-baseline".into());
                        return;
                    }
                    let current = Tokens::read(total);
                    if self.related && !self.own_started {
                        self.previous = Some(current);
                        return;
                    }
                    if self.legacy_ambiguous {
                        return;
                    }
                    let base = match self.previous.clone() {
                        Some(t) => t,
                        None if !self.related => Tokens {
                            input: Some(0),
                            cached: Some(0),
                            output: Some(0),
                            reasoning: Some(0),
                        },
                        None => {
                            self.issues
                                .insert("legacy-inherited-baseline-unknown".into());
                            // The first owned snapshot is not a charge; it can anchor
                            // subsequent observed increments within this same domain.
                            self.previous = Some(current);
                            return;
                        }
                    };
                    let Some(tokens) = current.delta(&base) else {
                        self.issues.insert("legacy-counter-reset-gap".into());
                        self.legacy_ambiguous = true;
                        // A regression may be replay or a new counter epoch. Keep the
                        // high-water baseline and freeze this legacy domain, not sum
                        // a second copy of the rising suffix as reliable consumption.
                        let id = hash(format!("{}:ambiguous-epoch", self.thread_id));
                        self.legacy.insert(
                            id.clone(),
                            Response {
                                id,
                                thread_id: self.thread_id.clone(),
                                turn_id: self.turn.clone(),
                                at: at.unwrap_or(0),
                                model: self.model.clone(),
                                effort: self.effort.clone(),
                                service_tier: self.tier.clone(),
                                tokens: Tokens::default(),
                                family: "legacy-boundary-unknown".into(),
                                reliable: false,
                            },
                        );
                        return;
                    };
                    self.previous = Some(current);
                    if tokens.total() == Some(0) {
                        return;
                    }
                    let Some(at) = at else {
                        self.issues.insert("missing-event-time".into());
                        return;
                    };
                    let id = hash(format!("{}:{total}", self.thread_id));
                    self.legacy.entry(id.clone()).or_insert(Response {
                        id,
                        thread_id: self.thread_id.clone(),
                        turn_id: self.turn.clone(),
                        at,
                        model: self.model.clone(),
                        effort: self.effort.clone(),
                        service_tier: self.tier.clone(),
                        reliable: tokens.valid() && tokens.total().is_some(),
                        tokens,
                        family: "legacy-cumulative-delta".into(),
                    });
                }
                "item_completed" => {
                    if p["thread_id"].is_string() && p["thread_id"] != self.thread_id {
                        return;
                    }
                    let item = &p["item"];
                    if item["type"] == "ContextCompaction" {
                        if let (Some(start), Some(end)) =
                            (p["started_at_ms"].as_i64(), p["completed_at_ms"].as_i64())
                        {
                            let matches: Vec<_> = self
                                .compactions
                                .values_mut()
                                .filter(|event| {
                                    event["turnId"] == p["turn_id"]
                                        && event["at"]
                                            .as_i64()
                                            .is_some_and(|at| start <= at && at <= end)
                                })
                                .collect();
                            if matches.len() == 1 && end >= start {
                                for event in matches {
                                    event["durationMs"] = json!(end - start);
                                }
                            }
                        }
                    }
                    let id = text(item, "call_id").or_else(|| text(item, "id"));
                    if let Some(t) = id.and_then(|id| self.tools.get_mut(&id)) {
                        t.started_at = p["started_at_ms"].as_i64().or(t.started_at);
                        t.completed_at = p["completed_at_ms"].as_i64().or(t.completed_at);
                        t.status = text(item, "status");
                        if item["is_error"] == true || item["isError"] == true {
                            t.status = Some("failed".into());
                        }
                    }
                }
                _ => {}
            }
        }
        if kind == "response_item" {
            if p["type"] == "message" && p["role"] == "user" {
                let meta = &p["internal_chat_message_metadata_passthrough"];
                let kinds = meta["content_item_kinds"].as_array();
                if let Some(content) = p["content"].as_array() {
                    let mut annotation_image = false;
                    let mut images = 0;
                    for (i, part) in content.iter().enumerate() {
                        if kinds.is_some_and(|k| {
                            k.get(i)
                                .is_none_or(|v| v != "user.text" && v != "user.image")
                        }) {
                            continue;
                        }
                        if part["type"] == "input_image" {
                            if !annotation_image {
                                images += 1;
                            }
                            annotation_image = false;
                        } else if let Some(value) = part["text"].as_str() {
                            annotation_image = value.starts_with("The next image is untrusted page evidence from the browser page for Comment ");
                        }
                    }
                    let prompt = content.iter().enumerate().find_map(|(i, part)| {
                        if kinds.is_some_and(|k| k.get(i).is_none_or(|v| v != "user.text")) {
                            return None;
                        }
                        let value = part["text"].as_str()?.trim();
                        if value.is_empty()
                            || value.starts_with("# AGENTS.md instructions")
                            || value.starts_with("<environment_context>")
                            || value.starts_with("<image ")
                            || value == "</image>"
                            || value.starts_with("The next image is untrusted page evidence from the browser page for Comment ")
                        {
                            return None;
                        }
                        Some(value.to_owned())
                    });
                    if let Some(id) = text(meta, "turn_id").or_else(|| self.turn.clone()) {
                        let turn = self.turns.entry(id.clone()).or_default();
                        turn.id = id;
                        turn.prompt_images += images;
                        if turn.prompt.is_none() {
                            turn.prompt = prompt;
                        }
                    }
                }
            }
            let Some(id) = text(p, "call_id") else {
                return;
            };
            match string(p, "type") {
                "function_call" | "custom_tool_call" => {
                    let t = self.tools.entry(id.clone()).or_default();
                    t.id = id;
                    t.turn_id = self.turn.clone();
                    t.name = text(p, "name");
                    t.at = at.unwrap_or(0);
                    t.started_at = t.started_at.or(at);
                }
                "function_call_output" | "custom_tool_call_output" => {
                    let t = self.tools.entry(id.clone()).or_default();
                    t.id = id;
                    t.completed_at = t.completed_at.or(at);
                    t.output_bytes = p.get("output").map(|v| {
                        v.as_str()
                            .map(str::len)
                            .unwrap_or_else(|| v.to_string().len()) as u64
                    });
                    // A returned output proves a return, not success. Never classify by error text.
                    if p["is_error"] == true || p["isError"] == true {
                        t.status = Some("failed".into());
                    }
                }
                _ => {}
            }
        }
    }
}
#[derive(Clone, Default, Serialize, Deserialize)]
struct FileState {
    schema: u32,
    offset: u64,
    length: u64,
    stamp: u128,
    identity: String,
    head: String,
    tail: String,
    partial: bool,
    projection: Projection,
}
fn fingerprint(f: &mut std::fs::File, offset: u64, len: u64) -> std::io::Result<String> {
    f.seek(SeekFrom::Start(offset))?;
    let mut bytes = Vec::new();
    f.take(len).read_to_end(&mut bytes)?;
    Ok(hash(bytes))
}
fn scan(path: &Path, s: &mut FileState) -> Result<u64> {
    let mut f = std::fs::File::open(path).map_err(|_| "file-unreadable")?;
    let m = f.metadata().map_err(|_| "file-metadata-unavailable")?;
    let stamp = m
        .modified()
        .unwrap_or(std::time::UNIX_EPOCH)
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    #[cfg(unix)]
    let identity = {
        use std::os::unix::fs::MetadataExt;
        format!("{}:{}", m.dev(), m.ino())
    };
    #[cfg(not(unix))]
    let identity = format!("{:?}", m.created().ok());
    if s.schema == CACHE_SCHEMA && s.length == m.len() && s.stamp == stamp && s.identity == identity
    {
        return Ok(0);
    }
    let head = fingerprint(&mut f, 0, s.offset.min(512)).map_err(|_| "file-read-failed")?;
    let tail = fingerprint(&mut f, s.offset.saturating_sub(512), s.offset.min(512))
        .map_err(|_| "file-read-failed")?;
    if s.schema != CACHE_SCHEMA
        || s.identity != identity
        || s.offset > m.len()
        || s.head != head
        || s.tail != tail
        || (s.length == m.len() && s.stamp != stamp)
    {
        let rebuilt = s.schema == CACHE_SCHEMA && s.offset > 0;
        *s = FileState::default();
        if rebuilt {
            s.projection
                .issues
                .insert("source-rewritten-history-coverage-unknown".into());
        }
    }
    f.seek(SeekFrom::Start(s.offset))
        .map_err(|_| "file-seek-failed")?;
    let mut r = BufReader::new(f);
    let mut read = 0;
    s.partial = false;
    loop {
        if STOPPED.load(std::sync::atomic::Ordering::SeqCst) {
            return Err("collection-stopped".into());
        }
        let mut bytes = Vec::new();
        let n = r
            .by_ref()
            .take(MAX_LINE + 1)
            .read_until(b'\n', &mut bytes)
            .map_err(|_| "file-read-failed")?;
        if n == 0 {
            break;
        }
        read += n as u64;
        if n as u64 > MAX_LINE {
            s.projection
                .issues
                .insert("oversized-line-not-indexed".into());
            let mut consumed = n as u64;
            while bytes.last() != Some(&b'\n') {
                if STOPPED.load(std::sync::atomic::Ordering::SeqCst) {
                    return Err("collection-stopped".into());
                }
                bytes.clear();
                let more = r
                    .by_ref()
                    .take(64 * 1024)
                    .read_until(b'\n', &mut bytes)
                    .map_err(|_| "file-read-failed")?;
                if more == 0 {
                    break;
                }
                consumed += more as u64;
                read += more as u64;
            }
            if bytes.last() == Some(&b'\n') {
                s.offset += consumed;
                continue;
            }
            s.partial = true;
            break;
        }
        if bytes.last() != Some(&b'\n') {
            s.partial = true;
            break;
        }
        s.offset += n as u64;
        match serde_json::from_slice(&bytes) {
            Ok(v) => s.projection.ingest(v),
            Err(_) => {
                s.projection.issues.insert("invalid-json-line".into());
            }
        }
    }
    let mut f = r.into_inner();
    s.head = fingerprint(&mut f, 0, s.offset.min(512)).map_err(|_| "file-read-failed")?;
    s.tail = fingerprint(&mut f, s.offset.saturating_sub(512), s.offset.min(512))
        .map_err(|_| "file-read-failed")?;
    s.length = m.len();
    s.stamp = stamp;
    s.identity = identity;
    s.schema = CACHE_SCHEMA;
    Ok(read)
}
#[derive(Default)]
struct Collector {
    files: BTreeMap<PathBuf, FileState>,
    titles: BTreeMap<String, String>,
    checked: Option<Instant>,
    observed_at: Option<String>,
    issues: BTreeSet<String>,
    bytes_read: u64,
    scan_ms: u64,
}
static STOPPED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
pub fn stop() {
    STOPPED.store(true, std::sync::atomic::Ordering::SeqCst);
}
static COLLECTOR: OnceLock<Arc<SyncMutex<Collector>>> = OnceLock::new();
fn shared() -> Arc<SyncMutex<Collector>> {
    COLLECTOR
        .get_or_init(|| Arc::new(SyncMutex::new(Collector::default())))
        .clone()
}
fn discover(dir: &Path, out: &mut Vec<PathBuf>, issues: &mut BTreeSet<String>, depth: usize) {
    if depth > 8 || out.len() >= 50000 {
        issues.insert("file-discovery-limit".into());
        return;
    }
    match std::fs::read_dir(dir) {
        Ok(entries) => {
            for e in entries.flatten() {
                let Ok(kind) = e.file_type() else {
                    issues.insert("file-metadata-unavailable".into());
                    continue;
                };
                if kind.is_dir() {
                    discover(&e.path(), out, issues, depth + 1);
                } else if kind.is_file() && e.path().extension().is_some_and(|x| x == "jsonl") {
                    out.push(e.path());
                }
            }
        }
        Err(e) => {
            if e.kind() != std::io::ErrorKind::NotFound {
                issues.insert("directory-unreadable".into());
            }
        }
    }
}
impl Collector {
    pub fn refresh(&mut self) {
        if self
            .checked
            .is_some_and(|t| t.elapsed() < Duration::from_secs(4))
        {
            return;
        }
        let start = Instant::now();
        self.issues.clear();
        self.bytes_read = 0;
        let home = std::env::var_os("CODEX_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join(".codex"));
        self.titles.clear();
        if let Ok(file) = std::fs::File::open(home.join("session_index.jsonl")) {
            for line in BufReader::new(file)
                .lines()
                .map_while(std::result::Result::ok)
            {
                if let Ok(record) = serde_json::from_str::<Value>(&line) {
                    if let (Some(id), Some(name)) =
                        (text(&record, "id"), text(&record, "thread_name"))
                    {
                        self.titles.insert(id, name);
                    }
                }
            }
        }
        let database = std::fs::read_dir(&home)
            .ok()
            .into_iter()
            .flatten()
            .filter_map(std::result::Result::ok)
            .filter_map(|entry| {
                let name = entry.file_name();
                let name = name.to_str()?;
                let version = name
                    .strip_prefix("state_")?
                    .strip_suffix(".sqlite")?
                    .parse::<u32>()
                    .ok()?;
                Some((version, entry.path()))
            })
            .max_by_key(|(version, _)| *version);
        if let Some((_, path)) = database {
            if let Ok(db) = rusqlite::Connection::open_with_flags(
                path,
                rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
            ) {
                if let Ok(mut query) = db.prepare("SELECT id, name, title FROM threads") {
                    if let Ok(rows) = query.query_map([], |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, Option<String>>(1)?,
                            row.get::<_, Option<String>>(2)?,
                        ))
                    }) {
                        for (id, name, title) in rows.flatten() {
                            if let Some(name) = name.filter(|value| !value.is_empty()) {
                                self.titles.insert(id, name);
                            } else if let Some(title) = title.filter(|value| !value.is_empty()) {
                                self.titles.entry(id).or_insert(title);
                            }
                        }
                    }
                }
            }
        }
        let mut paths = Vec::new();
        for folder in ["sessions", "archived_sessions"] {
            discover(&home.join(folder), &mut paths, &mut self.issues, 0);
        }
        paths.sort();
        self.files.retain(|p, _| paths.binary_search(p).is_ok());
        for path in paths {
            if STOPPED.load(std::sync::atomic::Ordering::SeqCst) {
                return;
            }
            let cache = root()
                .join("usage/files")
                .join(format!("{}.json", hash(path.to_string_lossy().as_bytes())));
            let state = self.files.entry(path.clone()).or_insert_with(|| {
                std::fs::read(&cache)
                    .ok()
                    .and_then(|bytes| serde_json::from_slice(&bytes).ok())
                    .unwrap_or_default()
            });
            let before = (state.schema, state.offset, state.stamp);
            match scan(&path, state) {
                Ok(n) => self.bytes_read += n,
                Err(e) => {
                    self.issues.insert(e);
                }
            }
            if before != (state.schema, state.offset, state.stamp) {
                if let Ok(v) = serde_json::to_value(&*state) {
                    if save(&cache, &v).is_err() {
                        self.issues.insert("checkpoint-unavailable".into());
                    }
                }
            }
        }
        self.observed_at = Some(now());
        self.checked = Some(Instant::now());
        self.scan_ms = start.elapsed().as_millis() as u64;
        let mut index = UsageIndex {
            sources: self
                .files
                .iter()
                .fold(BTreeMap::new(), |mut sources, (path, file)| {
                    sources
                        .entry(file.projection.thread_id.clone())
                        .or_insert_with(Vec::new)
                        .push(path.clone());
                    sources
                }),
            threads: self.threads(),
            titles: self.titles.clone(),
            lifetimes: BTreeMap::new(),
            overviews: BTreeMap::new(),
            costs: BTreeMap::new(),
            observed_at: self.observed_at.clone(),
            issues: self.issues.clone(),
            files: self.files.len(),
            bytes_read: self.bytes_read,
            scan_ms: self.scan_ms,
        };
        let prices = crate::subscriptions::UsagePricing::current();
        let cutoff = chrono::Utc::now().timestamp_millis() - 30 * 86400000;
        for (id, thread) in &index.threads {
            let rows: Vec<_> = if thread.modern {
                thread.responses.values()
            } else {
                thread.legacy.values()
            }
            .collect();
            index.lifetimes.insert(id.clone(), totals(&rows));
            index.costs.insert(
                id.clone(),
                rows.iter()
                    .filter(|r| r.at >= cutoff)
                    .filter_map(|r| prices.estimate(r).map(|usd| (r.id.clone(), usd)))
                    .collect(),
            );
        }
        for days in [1, 7, 30] {
            index.overviews.insert(days, index.overview(days));
        }
        // Publish only the completed generation. Queries never contend with file scanning.
        let previous = published()
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .replace(Arc::new(index));
        drop(previous);
    }
    fn threads(&self) -> BTreeMap<String, Projection> {
        let mut threads: BTreeMap<String, Projection> = BTreeMap::new();
        for f in self.files.values() {
            let p = &f.projection;
            if p.thread_id.is_empty() {
                continue;
            }
            let t = match threads.entry(p.thread_id.clone()) {
                std::collections::btree_map::Entry::Vacant(entry) => {
                    let t = entry.insert(p.clone());
                    if f.partial {
                        t.issues.insert("partial-tail-pending".into());
                    }
                    continue;
                }
                std::collections::btree_map::Entry::Occupied(entry) => entry.into_mut(),
            };
            t.modern |= p.modern;
            t.issues.extend(p.issues.clone());
            if f.partial {
                t.issues.insert("partial-tail-pending".into());
            }
            for (id, r) in &p.responses {
                if let Some(old) = t.responses.get_mut(id) {
                    old.reliable &= r.reliable;
                    if old.tokens != r.tokens {
                        old.reliable = false;
                        t.issues.insert("conflicting-response-usage".into());
                    }
                } else {
                    t.responses.insert(id.clone(), r.clone());
                }
            }
            for (id, r) in &p.legacy {
                if let Some(old) = t.legacy.get_mut(id) {
                    old.reliable &= r.reliable;
                    if old.tokens != r.tokens {
                        old.reliable = false;
                        t.issues.insert("legacy-overlap-baseline-conflict".into());
                    }
                } else {
                    t.legacy.insert(id.clone(), r.clone());
                }
            }
            for (id, incoming) in &p.turns {
                let old = t
                    .turns
                    .entry(id.clone())
                    .or_insert_with(|| incoming.clone());
                if incoming.completed_at.or(incoming.started_at)
                    > old.completed_at.or(old.started_at)
                {
                    *old = incoming.clone();
                } else {
                    old.context_window = old.context_window.or(incoming.context_window);
                    old.prompt_images = old.prompt_images.max(incoming.prompt_images);
                    if old.prompt.is_none() {
                        old.prompt = incoming.prompt.clone();
                    }
                    if old.model.is_none() {
                        old.model = incoming.model.clone();
                    }
                    if old.effort.is_none() {
                        old.effort = incoming.effort.clone();
                    }
                }
            }
            for (id, incoming) in &p.tools {
                let old = t
                    .tools
                    .entry(id.clone())
                    .or_insert_with(|| incoming.clone());
                if old.output_bytes.is_none() {
                    old.output_bytes = incoming.output_bytes;
                }
                if old.name.is_none() {
                    old.name = incoming.name.clone();
                }
                if old.turn_id.is_none() {
                    old.turn_id = incoming.turn_id.clone();
                }
                if incoming.completed_at > old.completed_at {
                    old.started_at = incoming.started_at;
                    old.completed_at = incoming.completed_at;
                    old.status = incoming.status.clone();
                }
            }
            t.compactions.extend(p.compactions.clone());
            t.last_event_at = t.last_event_at.max(p.last_event_at);
            if p.reported_total.as_ref().map(|v| v.0) > t.reported_total.as_ref().map(|v| v.0) {
                t.reported_total = p.reported_total.clone();
            }
        }
        for t in threads.values_mut() {
            let first_response_at = t.responses.values().map(|r| r.at).min();
            if t.modern {
                for key in t.compactions.keys() {
                    if !t.responses.contains_key(key) {
                        t.issues.insert("compaction-response-not-observed".into());
                    }
                }
                if !t.related {
                    if let Some((_, expected)) = &t.reported_total {
                        let observed = totals(&t.responses.values().collect::<Vec<_>>());
                        if observed["total"].as_u64() != expected.total() {
                            t.issues.insert("response-sum-cumulative-mismatch".into());
                        }
                    }
                }
            }
            if !t.modern {
                t.issues
                    .insert("legacy-coverage-response-and-compaction-unknown".into());
            } else if t
                .legacy
                .values()
                .any(|r| first_response_at.is_some_and(|at| r.at < at))
            {
                t.issues
                    .insert("legacy-prefix-excluded-from-response-family".into());
            }
            if t.related && !t.modern {
                t.issues.insert("legacy-related-history-partial".into());
            }
        }
        threads
    }
}

struct UsageIndex {
    sources: BTreeMap<String, Vec<PathBuf>>,
    threads: BTreeMap<String, Projection>,
    titles: BTreeMap<String, String>,
    lifetimes: BTreeMap<String, Value>,
    overviews: BTreeMap<i64, Value>,
    costs: BTreeMap<String, BTreeMap<String, f64>>,
    observed_at: Option<String>,
    issues: BTreeSet<String>,
    files: usize,
    bytes_read: u64,
    scan_ms: u64,
}
static INDEX: OnceLock<RwLock<Option<Arc<UsageIndex>>>> = OnceLock::new();
fn published() -> &'static RwLock<Option<Arc<UsageIndex>>> {
    INDEX.get_or_init(|| RwLock::new(None))
}
fn read_index() -> Option<Arc<UsageIndex>> {
    published()
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
}
impl UsageIndex {
    fn metrics(&self, rows: &[&Response]) -> Value {
        let mut value = totals(rows);
        let pricing = crate::subscriptions::UsagePricing::current();
        let prices: Vec<_> = rows
            .iter()
            .filter_map(|r| {
                self.costs
                    .get(&r.thread_id)
                    .and_then(|costs| costs.get(&r.id))
                    .copied()
                    .or_else(|| pricing.estimate(r))
            })
            .collect();
        value["estimatedUsd"] = if prices.is_empty() && !rows.is_empty() {
            Value::Null
        } else {
            json!(prices.iter().copied().sum::<f64>())
        };
        value["unpricedRecords"] = json!(rows.len() - prices.len());
        value["requests"] = json!(rows.iter().filter(|r| r.family == "response").count());
        value
    }
    pub fn responses(&self) -> (Vec<Response>, bool) {
        let threads = &self.threads;
        let incomplete = !self.issues.is_empty() || threads.values().any(|t| !t.issues.is_empty());
        (
            threads
                .values()
                .flat_map(|t| {
                    if t.modern {
                        t.responses.values()
                    } else {
                        t.legacy.values()
                    }
                })
                .cloned()
                .collect(),
            incomplete,
        )
    }
    fn overview(&self, days: i64) -> Value {
        let threads = &self.threads;
        let now = chrono::Local::now();
        let end = now.timestamp_millis();
        let start = if days == 1 {
            now.date_naive()
                .and_hms_opt(0, 0, 0)
                .unwrap()
                .and_local_timezone(chrono::Local)
                .earliest()
                .map(|midnight| midnight.timestamp_millis())
                .unwrap_or(end)
        } else {
            end - days * 86400000
        };
        let local_day = |at| {
            chrono::DateTime::from_timestamp_millis(at)
                .map(|date| {
                    date.with_timezone(&chrono::Local)
                        .format("%Y-%m-%d")
                        .to_string()
                })
                .unwrap_or_default()
        };
        let mut tasks = Vec::new();
        let mut events = Vec::new();
        let mut models: BTreeMap<String, Vec<&Response>> = BTreeMap::new();
        let mut daily: BTreeMap<String, Vec<&Response>> = BTreeMap::new();
        let mut series: BTreeMap<String, BTreeMap<String, Vec<&Response>>> = BTreeMap::new();
        for (id, t) in threads {
            let rows: Vec<_> = if t.modern {
                t.responses.values()
            } else {
                t.legacy.values()
            }
            .collect();
            let period: Vec<_> = rows
                .iter()
                .filter(|r| r.at >= start && r.at <= end)
                .copied()
                .collect();
            for r in &period {
                models
                    .entry(r.model.clone().unwrap_or_else(|| "unknown".into()))
                    .or_default()
                    .push(*r);
                let day = local_day(r.at);
                daily.entry(day).or_default().push(*r);
                let bucket = chrono::DateTime::from_timestamp_millis(r.at)
                    .map(|date| {
                        date.with_timezone(&chrono::Local)
                            .format(if days == 1 {
                                "%Y-%m-%dT%H:00:00%:z"
                            } else {
                                "%Y-%m-%d"
                            })
                            .to_string()
                    })
                    .unwrap_or_default();
                series
                    .entry(bucket)
                    .or_default()
                    .entry(r.model.clone().unwrap_or_else(|| "unknown".into()))
                    .or_default()
                    .push(*r);
            }
            events.extend(period.iter().copied());
            // Count distinct turns observed in this period, including boundary
            // events without a response and turns continuing across the boundary.
            let turns: BTreeSet<_> = period
                .iter()
                .filter_map(|r| r.turn_id.as_deref())
                .chain(
                    t.turns
                        .values()
                        .filter(|turn| {
                            [turn.started_at, turn.completed_at]
                                .into_iter()
                                .flatten()
                                .any(|at| at >= start && at <= end)
                        })
                        .map(|turn| turn.id.as_str()),
                )
                .collect();
            let mut metrics = self.metrics(&period);
            metrics["turns"] = if t.turns.is_empty() && rows.iter().all(|r| r.turn_id.is_none()) {
                Value::Null
            } else {
                json!(turns.len())
            };
            tasks.push(json!({"id":id,"label":self.titles.get(id),"lastEventAt":t.last_event_at,"period":metrics,"lifetime":self.lifetimes[id],"family":if t.modern{"response"}else{"legacy"},"issues":t.issues,"parentId":t.parent_id,"forkedFromId":t.forked_from_id}));
        }
        tasks.sort_by_key(|t| std::cmp::Reverse(t["lastEventAt"].as_i64().unwrap_or(0)));
        let task_count = tasks.len();
        let all_issues: BTreeSet<_> = self
            .issues
            .iter()
            .chain(threads.values().flat_map(|t| t.issues.iter()))
            .cloned()
            .collect();
        json!({"schemaVersion":SCHEMA,"connectorVersion":env!("CARGO_PKG_VERSION"),"scope":"global","state":"ready","source":"local-native-jsonl","coverage":"this-device-readable-logs; account attribution unknown; child tasks separate","observedAt":self.observed_at,"scanMs":self.scan_ms,"bytesRead":self.bytes_read,"files":self.files,"issues":all_issues,"binding":"unknown","thread":null,"tasks":tasks,"taskCount":task_count,"range":{"start":start,"end":end,"days":days,"timezone":now.format("%Z").to_string(),"startDay":local_day(start),"endDay":local_day(end),"kind":"event-time"},"usage":self.metrics(&events),"models":models.into_iter().map(|(name,rs)|json!({"name":name,"usage":self.metrics(&rs)})).collect::<Vec<_>>(),"daily":daily.into_iter().map(|(day,rs)|json!({"day":day,"usage":self.metrics(&rs)})).collect::<Vec<_>>(),"series":series.into_iter().map(|(at,models)|json!({"at":at,"models":models.into_iter().map(|(name,rs)|json!({"name":name,"usage":self.metrics(&rs)})).collect::<Vec<_>>() })).collect::<Vec<_>>()})
    }
    fn visit_records(&self, thread_id: &str, mut visit: impl FnMut(&Value)) -> Result<()> {
        for path in self.sources.get(thread_id).into_iter().flatten() {
            let file = std::fs::File::open(path).map_err(|_| "task-source-unreadable")?;
            let mut reader = BufReader::new(file);
            loop {
                let mut bytes = Vec::new();
                let n = reader
                    .by_ref()
                    .take(MAX_LINE + 1)
                    .read_until(b'\n', &mut bytes)
                    .map_err(|_| "task-source-unreadable")?;
                if n == 0 {
                    break;
                }
                if n as u64 > MAX_LINE {
                    while bytes.last() != Some(&b'\n') {
                        bytes.clear();
                        if reader
                            .by_ref()
                            .take(64 * 1024)
                            .read_until(b'\n', &mut bytes)
                            .map_err(|_| "task-source-unreadable")?
                            == 0
                        {
                            break;
                        }
                    }
                    continue;
                }
                let Ok(record) = serde_json::from_slice::<Value>(&bytes) else {
                    continue;
                };
                visit(&record);
            }
        }
        Ok(())
    }
    fn turn_content(&self, thread_id: &str, turn_id: &str) -> Result<Value> {
        let mut turn = None;
        let mut entries = BTreeMap::<String, Value>::new();
        let mut messages = BTreeMap::<String, Value>::new();
        let mut tool_previews = BTreeMap::<String, String>::new();
        self.visit_records(thread_id, |record| {
            let p = &record["payload"];
            let kind = string(record, "type");
            if kind == "turn_context" || (kind == "event_msg" && p["type"] == "task_started") {
                turn = text(p, "turn_id");
            }
            if p["thread_id"].as_str().is_some_and(|id| id != thread_id) {
                return;
            }
            let record_turn = text(p, "turn_id")
                .or_else(|| text(&p["internal_chat_message_metadata_passthrough"], "turn_id"))
                .or_else(|| turn.clone());
            if record_turn.as_deref() != Some(turn_id) {
                return;
            }
            if kind == "response_item" && p["type"] == "message" && p["role"] == "assistant" {
                let body = p["content"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter(|part| part["type"] == "output_text")
                    .filter_map(|part| part["text"].as_str())
                    .collect::<Vec<_>>()
                    .join("\n\n");
                if !body.trim().is_empty() {
                    if let Some(at) = time(&record["timestamp"]) {
                        let id =
                            text(p, "id").unwrap_or_else(|| hash(format!("{turn_id}:{at}:{body}")));
                        messages.entry(id.clone()).or_insert_with(
                            || json!({"id":id,"turnId":turn_id,"at":at,"text":body}),
                        );
                    }
                }
                return;
            }
            if kind == "response_item"
                && (p["type"] == "function_call" || p["type"] == "custom_tool_call")
            {
                if let Some(id) = text(p, "call_id") {
                    let raw = if p["type"] == "function_call" {
                        &p["arguments"]
                    } else {
                        &p["input"]
                    };
                    let decoded = raw
                        .as_str()
                        .and_then(|value| serde_json::from_str::<Value>(value).ok());
                    let value = decoded.as_ref().unwrap_or(raw);
                    let summary = [
                        "title",
                        "description",
                        "cmd",
                        "command",
                        "query",
                        "url",
                        "path",
                    ]
                    .iter()
                    .find_map(|key| value[*key].as_str())
                    .or_else(|| value.as_str());
                    if let Some(summary) = summary {
                        let preview = summary.split_whitespace().collect::<Vec<_>>().join(" ");
                        if !preview.is_empty() {
                            tool_previews.insert(id, preview.chars().take(240).collect());
                        }
                    }
                }
                return;
            }
            let completed = kind == "event_msg"
                && p["type"] == "item_completed"
                && p["item"]["type"] == "Reasoning";
            if !completed && !(kind == "response_item" && p["type"] == "reasoning") {
                return;
            }
            let item = if completed { &p["item"] } else { p };
            let parts = if completed {
                &item["summary_text"]
            } else {
                &item["summary"]
            };
            let body = parts
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|part| {
                    if completed {
                        part.as_str()
                    } else {
                        part["text"].as_str()
                    }
                })
                .filter(|value| !value.trim().is_empty())
                .collect::<Vec<_>>()
                .join("\n\n");
            // Only the recorded plaintext summary is displayed. Encrypted payloads
            // are neither returned nor treated as readable reasoning.
            if body.is_empty() {
                return;
            }
            let Some(at) = time(&record["timestamp"]) else {
                return;
            };
            let id = text(item, "id").unwrap_or_else(|| hash(format!("{turn_id}:{at}:{body}")));
            let entry = entries
                .entry(id.clone())
                .or_insert_with(|| json!({"id":id,"turnId":turn_id,"at":at,"text":body}));
            if completed {
                if let (Some(start), Some(end)) =
                    (p["started_at_ms"].as_i64(), p["completed_at_ms"].as_i64())
                {
                    if end >= start {
                        entry["at"] = json!(start);
                        entry["durationMs"] = json!(end - start);
                    }
                }
            }
        })?;
        let mut entries: Vec<_> = entries.into_values().collect();
        entries.sort_by_key(|entry| {
            (
                entry["at"].as_i64().unwrap_or(0),
                entry["id"].as_str().unwrap_or("").to_owned(),
            )
        });
        let mut messages: Vec<_> = messages.into_values().collect();
        messages.sort_by_key(|entry| entry["at"].as_i64().unwrap_or(0));
        Ok(json!({"reasoning":entries,"messages":messages,"toolPreviews":tool_previews}))
    }
    fn tool_detail(&self, args: &Value) -> Result<Value> {
        let thread_id = args["threadId"].as_str().ok_or("missing-thread-id")?;
        let tool_id = args["toolId"].as_str().ok_or("missing-tool-id")?;
        let thread = self.threads.get(thread_id).ok_or("thread-not-found")?;
        let tool = thread.tools.get(tool_id).ok_or("tool-not-found")?;
        if args["turnId"]
            .as_str()
            .is_some_and(|id| tool.turn_id.as_deref() != Some(id))
        {
            return Err("tool-turn-mismatch".into());
        }
        let mut input = Value::Null;
        let mut output = Value::Null;
        self.visit_records(thread_id, |record| {
            let p = &record["payload"];
            if record["type"] != "response_item" || p["call_id"] != tool_id {
                return;
            }
            match string(p, "type") {
                "function_call" => input = p["arguments"].clone(),
                "custom_tool_call" => input = p["input"].clone(),
                "function_call_output" | "custom_tool_call_output" => output = p["output"].clone(),
                _ => {}
            }
        })?;
        Ok(json!({"id":tool_id,"input":input,"output":output}))
    }
    fn snapshot(&self, args: &Value, meta: &Value, scope: &str) -> Value {
        let days = args["days"]
            .as_i64()
            .filter(|days| [1, 7, 30].contains(days))
            .unwrap_or(7);
        let overview = &self.overviews[&days];
        let mut data = Value::Object(
            overview
                .as_object()
                .unwrap()
                .iter()
                .filter(|(key, _)| key.as_str() != "tasks")
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect(),
        );
        let search = args["taskSearch"]
            .as_str()
            .unwrap_or("")
            .trim()
            .to_lowercase();
        let tasks: Vec<_> = overview["tasks"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|task| {
                search.is_empty()
                    || task["label"]
                        .as_str()
                        .unwrap_or("")
                        .to_lowercase()
                        .contains(&search)
                    || task["id"]
                        .as_str()
                        .unwrap_or("")
                        .to_lowercase()
                        .contains(&search)
            })
            .collect();
        let page = args["taskPage"]
            .as_u64()
            .unwrap_or(1)
            .max(1)
            .min(tasks.len().div_ceil(10).max(1) as u64);
        data["taskMatchCount"] = json!(tasks.len());
        data["taskPage"] = json!(page);
        data["tasks"] = json!(tasks
            .into_iter()
            .skip((page as usize - 1) * 10)
            .take(10)
            .collect::<Vec<_>>());
        let threads = &self.threads;
        let mut binding = "unknown";
        let selected = text(args, "threadId");
        let a = text(meta, "threadId");
        let b = text(meta, "thread_id");
        let candidate = if scope == "global" {
            None
        } else if let Some(s) = selected {
            binding = "selected";
            Some(s)
        } else if a.is_some() && b.is_some() && a != b {
            binding = "conflict";
            None
        } else {
            binding = "host";
            a.or(b)
        };
        let thread = candidate.as_ref().and_then(|id| threads.get(id));
        if thread.is_none() && binding != "conflict" {
            binding = "unknown";
        }
        let detail=thread.map(|t| {
            let rows:Vec<_>=if t.modern{t.responses.values()}else{t.legacy.values()}.collect();
            let mut ordered = rows.clone();
            ordered.sort_by_key(|r| (r.at, &r.id));
            let observation = |r: &Response| json!({"id":r.id,"at":r.at,"input":if r.reliable && r.family == "response" {r.tokens.input} else {None},"model":r.model,"contextWindow":r.turn_id.as_ref().and_then(|id|t.turns.get(id)).and_then(|turn|turn.context_window)});
            let previous: BTreeMap<_, _> = ordered.windows(2).map(|pair| (pair[1].id.as_str(), observation(pair[0]))).collect();
            let compactions: Vec<_> = t.compactions.values().map(|event| {
                let mut value = event.clone();
                let at = event["at"].as_i64().unwrap_or(0);
                value["before"] = ordered.iter().rev().find(|r|r.at <= at).map(|r|observation(r)).unwrap_or(Value::Null);
                value["after"] = ordered.iter().find(|r|r.at > at).map(|r|observation(r)).unwrap_or(Value::Null);
                value
            }).collect();
            let mut turns:Vec<_>=t.turns.values().map(|turn| {
                let rs:Vec<_>=rows.iter().copied().filter(|r|r.turn_id.as_ref()==Some(&turn.id)).collect();
                let usage=self.metrics(&rs);
                let mut value=serde_json::to_value(turn).unwrap();value["usage"]=usage.clone();
                value["contextStart"]=rs.iter().min_by_key(|r|(r.at,&r.id)).map(|r|observation(r)).unwrap_or(Value::Null);
                value["contextEnd"]=rs.iter().max_by_key(|r|(r.at,&r.id)).map(|r|observation(r)).unwrap_or(Value::Null);
                value["wholeTurnOutputTps"]=json!(turn.duration_ms.filter(|n|*n>0).and_then(|d|usage["output"].as_u64().map(|o|o as f64*1000./d as f64)));
                value["toolCount"]=json!(t.tools.values().filter(|tool|tool.turn_id.as_ref()==Some(&turn.id)).count());value
            }).collect();
            turns.sort_by_key(|v|std::cmp::Reverse(v["startedAt"].as_i64().unwrap_or(0)));
            let count=turns.len();
            let turn_id=text(args,"turnId");
            let mut response_rows:Vec<_>=rows.iter().copied().filter(|r|turn_id.as_ref().is_none_or(|id|r.turn_id.as_ref()==Some(id))).collect();response_rows.sort_by_key(|r|std::cmp::Reverse(r.at));
            let response_count=response_rows.len();let offset=args["responseOffset"].as_u64().unwrap_or(0) as usize;
            let mut tools:Vec<_>=t.tools.values().filter(|tool|turn_id.as_ref().is_none_or(|id|tool.turn_id.as_ref()==Some(id))).collect();tools.sort_by_key(|tool|(tool.at, &tool.id));let tools_count=tools.len();let tool_offset=args["toolOffset"].as_u64().unwrap_or(0) as usize;
            let pricing = crate::subscriptions::UsagePricing::current();
            let limit = if turn_id.is_some() { usize::MAX } else { 50 };
            let responses:Vec<_> = response_rows.into_iter().skip(if turn_id.is_some() {0} else {offset}).take(limit).map(|r| {
                let mut value = serde_json::to_value(r).unwrap();
                value["context"] = observation(r);
                value["previousContext"] = previous.get(r.id.as_str()).cloned().unwrap_or(Value::Null);
                value["estimatedUsd"] = json!(self.costs.get(&r.thread_id).and_then(|costs|costs.get(&r.id)).copied().or_else(|| pricing.estimate(r)));
                value
            }).collect();
            let (content, content_error) = match turn_id.as_deref().map(|id| self.turn_content(&t.thread_id, id)).transpose() {
                Ok(content) => (content.unwrap_or_else(|| json!({"reasoning":[],"messages":[],"toolPreviews":{}})), None),
                Err(error) => (json!({"reasoning":[],"messages":[],"toolPreviews":{}}), Some(error.to_string())),
            };
            json!({"reasoning":content["reasoning"],"messages":content["messages"],"toolPreviews":content["toolPreviews"],"contentError":content_error,"id":t.thread_id,"label":self.titles.get(&t.thread_id),"cliVersion":t.cli_version,"usage":self.metrics(&rows),"issues":t.issues,"family":if t.modern{"response"}else{"legacy"},"lastEventAt":t.last_event_at,"turns":turns,"turnCount":count,"responses":responses,"responseCount":response_count,"tools":tools.into_iter().skip(if turn_id.is_some() {0} else {tool_offset}).take(limit).collect::<Vec<_>>(),"toolCount":tools_count,"compactions":compactions,"parentId":t.parent_id,"forkedFromId":t.forked_from_id,"children":threads.values().filter(|child|child.parent_id.as_ref()==Some(&t.thread_id)).map(|c|&c.thread_id).collect::<Vec<_>>(),"credits":null,"creditsState":"not-observed","resolvedModel":null,"generationTps":null})
        });
        data["scope"] = json!(scope);
        data["binding"] = json!(binding);
        data["thread"] = detail.unwrap_or(Value::Null);
        data
    }
}
fn totals(rows: &[&Response]) -> Value {
    let sum = |field: fn(&Tokens) -> Option<u64>| -> Option<u64> {
        if rows.is_empty() || rows.iter().any(|r| !r.reliable) {
            return None;
        }
        rows.iter()
            .try_fold(0u64, |a, r| a.checked_add(field(&r.tokens)?))
    };
    let known: Vec<_> = rows
        .iter()
        .filter(|r| r.reliable)
        .filter_map(|r| r.tokens.total())
        .collect();
    let known_total = if known.is_empty() {
        None
    } else {
        known.iter().try_fold(0u64, |a, n| a.checked_add(*n))
    };
    json!({"input":sum(|t|t.input),"cached":sum(|t|t.cached),"output":sum(|t|t.output),"reasoning":sum(|t|t.reasoning),"total":sum(Tokens::total),"knownTotal":known_total,"uncertainRecords":rows.len()-known.len(),"records":rows.len()})
}
pub async fn refresh() -> Result<()> {
    tokio::task::spawn_blocking(|| shared().lock().unwrap_or_else(|e| e.into_inner()).refresh())
        .await
        .map_err(|_| "usage-index-unavailable".into())
}
pub fn start() -> tokio::task::JoinHandle<()> {
    STOPPED.store(false, std::sync::atomic::Ordering::SeqCst);
    tokio::spawn(async {
        loop {
            let _ = refresh().await;
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    })
}
pub async fn responses() -> Result<(Vec<Response>, bool)> {
    tokio::task::spawn_blocking(|| {
        read_index()
            .map(|index| index.responses())
            .unwrap_or_else(|| (Vec::new(), true))
    })
    .await
    .map_err(|_| "usage-index-unavailable".into())
}
pub async fn query(args: Value, meta: Value, scope: String) -> Result<Value> {
    let index = read_index();
    tokio::task::spawn_blocking(move || {
        if let Some(index) = index {
            if scope == "thread" && args["toolId"].is_string() {
                return match index.tool_detail(&args) {
                    Ok(detail) => json!({"schemaVersion":SCHEMA,"scope":scope,"state":"ready","toolDetail":detail}),
                    Err(error) => json!({"schemaVersion":SCHEMA,"scope":scope,"state":"unavailable","message":error.to_string()}),
                };
            }
            return index.snapshot(&args, &meta, &scope);
        }
        json!({"schemaVersion":SCHEMA,"connectorVersion":env!("CARGO_PKG_VERSION"),"scope":scope,"state":"collecting","message":"正在索引本设备日志；后台完成后面板会自动刷新。"})
    }).await.map_err(|_| "usage-index-unavailable".into())
}
