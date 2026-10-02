//! Device-local, content-free usage projection shared by the desktop and plugin.
//! Response usage and legacy cumulative usage are separate counting families.
use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::sync::{Mutex as SyncMutex, OnceLock};
use std::time::{Duration, Instant};

const SCHEMA: u32 = 1;
const CACHE_SCHEMA: u32 = 5;
const MAX_LINE: u64 = 8 * 1024 * 1024;
#[derive(Clone, Default, Serialize, Deserialize)]
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
                "token_count" => {
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
                    let id = text(item, "call_id").or_else(|| text(item, "id"));
                    if let Some(t) = id.and_then(|id| self.tools.get_mut(&id)) {
                        t.started_at = p["started_at_ms"].as_i64();
                        t.completed_at = p["completed_at_ms"].as_i64();
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
                }
                "function_call_output" | "custom_tool_call_output" => {
                    let t = self.tools.entry(id.clone()).or_default();
                    t.id = id;
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
pub struct Collector {
    files: BTreeMap<PathBuf, FileState>,
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
pub fn shared() -> Arc<SyncMutex<Collector>> {
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
        let mut paths = Vec::new();
        for folder in ["sessions", "archived_sessions"] {
            discover(&home.join(folder), &mut paths, &mut self.issues, 0);
        }
        paths.sort();
        self.files.retain(|p, _| paths.contains(p));
        for path in paths {
            if STOPPED.load(std::sync::atomic::Ordering::SeqCst) {
                return;
            }
            let cache = root()
                .join("usage/files")
                .join(format!("{}.json", hash(path.to_string_lossy().as_bytes())));
            let state = self.files.entry(path.clone()).or_insert_with(|| {
                load(&cache)
                    .ok()
                    .and_then(|v| serde_json::from_value(v).ok())
                    .unwrap_or_default()
            });
            let before = (state.offset, state.stamp);
            match scan(&path, state) {
                Ok(n) => self.bytes_read += n,
                Err(e) => {
                    self.issues.insert(e);
                }
            }
            if before != (state.offset, state.stamp) {
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
    }
    fn threads(&self) -> BTreeMap<String, Projection> {
        let mut threads: BTreeMap<String, Projection> = BTreeMap::new();
        for f in self.files.values() {
            let p = &f.projection;
            if p.thread_id.is_empty() {
                continue;
            }
            let t = threads
                .entry(p.thread_id.clone())
                .or_insert_with(|| p.clone());
            t.modern |= p.modern;
            t.issues.extend(p.issues.clone());
            if f.partial {
                t.issues.insert("partial-tail-pending".into());
            }
            for (id, r) in &p.responses {
                if let Some(old) = t.responses.get_mut(id) {
                    old.reliable &= r.reliable;
                    if serde_json::to_value(&old.tokens).ok()
                        != serde_json::to_value(&r.tokens).ok()
                    {
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
                    if serde_json::to_value(&old.tokens).ok()
                        != serde_json::to_value(&r.tokens).ok()
                    {
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
            if t.modern {
                for key in t.compactions.keys() {
                    if !t.responses.contains_key(key) {
                        t.issues.insert("compaction-response-not-observed".into());
                    }
                }
                if !t.related {
                    if let Some((_, expected)) = &t.reported_total {
                        let observed = totals(&t.responses.values().cloned().collect::<Vec<_>>());
                        if observed["total"].as_u64() != expected.total() {
                            t.issues.insert("response-sum-cumulative-mismatch".into());
                        }
                    }
                }
            }
            if !t.modern {
                t.issues
                    .insert("legacy-coverage-response-and-compaction-unknown".into());
            } else if t.legacy.values().any(|r| {
                t.responses
                    .values()
                    .map(|r| r.at)
                    .min()
                    .is_some_and(|at| r.at < at)
            }) {
                t.issues
                    .insert("legacy-prefix-excluded-from-response-family".into());
            }
            if t.related && !t.modern {
                t.issues.insert("legacy-related-history-partial".into());
            }
        }
        threads
    }
    pub fn responses(&self) -> (Vec<Response>, bool) {
        let threads = self.threads();
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
    pub fn snapshot(&self, args: &Value, meta: &Value, scope: &str) -> Value {
        let threads = self.threads();
        let end = chrono::Utc::now().timestamp_millis();
        let days = args["days"]
            .as_i64()
            .filter(|d| [1, 7, 30].contains(d))
            .unwrap_or(7);
        let start = end - days * 86400000;
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
        let mut tasks = Vec::new();
        let mut events = Vec::new();
        let mut models: BTreeMap<String, Vec<Response>> = BTreeMap::new();
        let mut daily: BTreeMap<String, Vec<Response>> = BTreeMap::new();
        for (id, t) in &threads {
            let rows: Vec<_> = if t.modern {
                t.responses.values()
            } else {
                t.legacy.values()
            }
            .cloned()
            .collect();
            let period: Vec<_> = rows
                .iter()
                .filter(|r| r.at >= start && r.at <= end)
                .cloned()
                .collect();
            for r in &period {
                models
                    .entry(r.model.clone().unwrap_or_else(|| "unknown".into()))
                    .or_default()
                    .push(r.clone());
                let day = chrono::DateTime::from_timestamp_millis(r.at)
                    .map(|d| d.format("%Y-%m-%d").to_string())
                    .unwrap_or_default();
                daily.entry(day).or_default().push(r.clone());
            }
            events.extend(period.clone());
            tasks.push(json!({"id":id,"label":format!("任务 {}",id.chars().take(8).collect::<String>()),"lastEventAt":t.last_event_at,"period":totals(&period),"lifetime":totals(&rows),"family":if t.modern{"response"}else{"legacy"},"issues":t.issues,"parentId":t.parent_id,"forkedFromId":t.forked_from_id}));
        }
        tasks.sort_by_key(|t| std::cmp::Reverse(t["period"]["total"].as_u64().unwrap_or(0)));
        let task_count = tasks.len();
        tasks.truncate(500);
        let detail=thread.map(|t| {
            let rows:Vec<_>=if t.modern{t.responses.values()}else{t.legacy.values()}.cloned().collect();
            let mut turns:Vec<_>=t.turns.values().map(|turn| {
                let rs:Vec<_>=rows.iter().filter(|r|r.turn_id.as_ref()==Some(&turn.id)).cloned().collect();
                let usage=totals(&rs);
                let mut value=serde_json::to_value(turn).unwrap();value["usage"]=usage.clone();
                value["wholeTurnOutputTps"]=json!(turn.duration_ms.filter(|n|*n>0).and_then(|d|usage["output"].as_u64().map(|o|o as f64*1000./d as f64)));
                value["toolCount"]=json!(t.tools.values().filter(|tool|tool.turn_id.as_ref()==Some(&turn.id)).count());value
            }).collect();
            turns.sort_by_key(|v|std::cmp::Reverse(v["startedAt"].as_i64().unwrap_or(0)));
            let turn_offset=args["turnOffset"].as_u64().unwrap_or(0) as usize;let count=turns.len();
            let turns:Vec<_>=turns.into_iter().skip(turn_offset).take(20).collect();
            let turn_id=text(args,"turnId");
            let mut response_rows:Vec<_>=rows.iter().filter(|r|turn_id.as_ref().is_none_or(|id|r.turn_id.as_ref()==Some(id))).cloned().collect();response_rows.sort_by_key(|r|std::cmp::Reverse(r.at));
            let response_count=response_rows.len();let offset=args["responseOffset"].as_u64().unwrap_or(0) as usize;
            let tools:Vec<_>=t.tools.values().filter(|tool|turn_id.as_ref().is_none_or(|id|tool.turn_id.as_ref()==Some(id))).collect();let tools_count=tools.len();let tool_offset=args["toolOffset"].as_u64().unwrap_or(0) as usize;
            json!({"id":t.thread_id,"cliVersion":t.cli_version,"usage":totals(&rows),"issues":t.issues,"family":if t.modern{"response"}else{"legacy"},"lastEventAt":t.last_event_at,"turns":turns,"turnCount":count,"responses":response_rows.into_iter().skip(offset).take(50).collect::<Vec<_>>(),"responseCount":response_count,"tools":tools.into_iter().skip(tool_offset).take(50).collect::<Vec<_>>(),"toolCount":tools_count,"compactions":t.compactions.values().collect::<Vec<_>>(),"parentId":t.parent_id,"forkedFromId":t.forked_from_id,"children":threads.values().filter(|child|child.parent_id.as_ref()==Some(&t.thread_id)).map(|c|&c.thread_id).collect::<Vec<_>>(),"credits":null,"creditsState":"not-observed","resolvedModel":null,"generationTps":null})
        });
        let all_issues: BTreeSet<_> = self
            .issues
            .iter()
            .chain(threads.values().flat_map(|t| t.issues.iter()))
            .cloned()
            .collect();
        json!({"schemaVersion":SCHEMA,"connectorVersion":env!("CARGO_PKG_VERSION"),"scope":scope,"state":"ready","source":"local-native-jsonl","coverage":"this-device-readable-logs; account attribution unknown; child tasks separate","observedAt":self.observed_at,"scanMs":self.scan_ms,"bytesRead":self.bytes_read,"files":self.files.len(),"issues":all_issues,"binding":binding,"thread":detail,"tasks":tasks,"taskCount":task_count,"range":{"start":start,"end":end,"days":days,"timezone":"UTC","kind":"event-time"},"usage":totals(&events),"models":models.into_iter().map(|(name,rs)|json!({"name":name,"usage":totals(&rs)})).collect::<Vec<_>>(),"daily":daily.into_iter().map(|(day,rs)|json!({"day":day,"usage":totals(&rs)})).collect::<Vec<_>>()})
    }
}
fn totals(rows: &[Response]) -> Value {
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
pub async fn query(args: Value, meta: Value, scope: String) -> Result<Value> {
    tokio::task::spawn_blocking(move || {
        let collector = shared();
        if let Ok(collector) = collector.try_lock() {
            if collector.observed_at.is_some() { return collector.snapshot(&args, &meta, &scope); }
        }
        json!({"schemaVersion":SCHEMA,"connectorVersion":env!("CARGO_PKG_VERSION"),"scope":scope,"state":"collecting","message":"正在索引本设备日志；后台完成后面板会自动刷新。"})
    }).await.map_err(|_| "usage-index-unavailable".into())
}
