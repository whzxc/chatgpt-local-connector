use crate::control::Control;
use crate::*;
use std::collections::BTreeSet;
pub async fn list(c: &Control) -> Result<Vec<Value>> {
    let mut out = Vec::new();
    let mut cursor = Value::Null;
    let mut seen = BTreeSet::new();
    loop {
        let page = c
            .request("project/list", json!({"cursor":cursor}), 60000)
            .await?;
        out.extend(
            page["data"]
                .as_array()
                .ok_or("invalid project list")?
                .iter()
                .cloned(),
        );
        cursor = page["nextCursor"].clone();
        if cursor.is_null() {
            return Ok(out);
        }
        if !seen.insert(cursor.to_string()) {
            return Err("project pagination did not advance".into());
        }
    }
}
pub async fn resolve(c: &Control, id: &str) -> Result<Value> {
    let (root, pid, name) = if Path::new(id).is_absolute() {
        (
            PathBuf::from(id),
            id.to_owned(),
            Path::new(id)
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
        )
    } else {
        let catalog = list(c).await?;
        let exact = catalog.iter().find(|p| p["id"] == id);
        let candidates = if let Some(p) = exact {
            vec![p]
        } else {
            catalog.iter().filter(|p| p["name"] == id).collect()
        };
        if candidates.len() != 1 {
            return Err("项目不存在或名称不唯一".into());
        }
        let p = candidates[0];
        let roots = p["roots"]
            .as_array()
            .filter(|r| r.len() == 1)
            .ok_or("多目录项目请指定绝对目录")?;
        (
            PathBuf::from(string(&roots[0], "path")),
            string(p, "id").into(),
            string(p, "name").into(),
        )
    };
    let root = std::fs::canonicalize(root).map_err(|e| e.to_string())?;
    if !root.is_dir() {
        return Err("project must be a directory".into());
    }
    Ok(json!({"id":pid,"name":name,"root":root}))
}
async fn git(root: &str, args: Vec<String>) -> Result<String> {
    let null = if cfg!(windows) { "NUL" } else { "/dev/null" };
    let missing_ref_check =
        args.first().is_some_and(|s| s == "show-ref") && args.iter().any(|s| s == "--quiet");
    let mut cmd = command("git");
    cmd.kill_on_drop(true);
    cmd.args([
        "--no-optional-locks",
        "--literal-pathspecs",
        "-c",
        "core.fsmonitor=false",
        "-c",
        &format!("core.hooksPath={null}"),
        "-c",
        &format!("core.attributesFile={null}"),
        "-c",
        "diff.external=",
        "-c",
        "color.ui=false",
        "-C",
        root,
    ])
    .args(args)
    .env_clear();
    for k in ["PATH", "HOME", "SYSTEMROOT", "TEMP"] {
        if let Some(v) = std::env::var_os(k) {
            cmd.env(k, v);
        }
    }
    cmd.env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", null)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_PAGER", "cat")
        .env("LC_ALL", "C");
    let o = tokio::time::timeout(std::time::Duration::from_secs(12), cmd.output())
        .await
        .map_err(|_| "GIT_TIMEOUT")?
        .map_err(|e| e.to_string())?;
    if !o.status.success() || o.stdout.len() > 16 * 1024 * 1024 {
        return Err(
            if String::from_utf8_lossy(&o.stderr).contains("not a git repository")
                && !Path::new(root)
                    .ancestors()
                    .any(|p| p.join(".git").symlink_metadata().is_ok())
            {
                "NOT_GIT_REPOSITORY".into()
            } else if missing_ref_check && o.status.code() == Some(1) {
                "GIT_REF_MISSING".into()
            } else {
                "GIT_QUERY_FAILED".into()
            },
        );
    }
    String::from_utf8(o.stdout).map_err(|e| e.to_string())
}
fn strings(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}
async fn state(p: &Value) -> Result<Value> {
    let root = string(p, "root");
    match git(root, strings(&["rev-parse", "--git-dir"])).await {
        Ok(_) => (),
        Err(e) if e == "NOT_GIT_REPOSITORY" => {
            return Ok(
                json!({"project":p["id"],"root":root,"git":false,"sha":null,"statusHash":null,"observedAt":now()}),
            )
        }
        Err(e) => return Err(e),
    }
    // An unborn repository is Git, but has no HEAD yet.
    let sha = match git(root, strings(&["rev-parse", "--verify", "HEAD"])).await {
        Ok(s) => Some(s.trim().to_owned()),
        Err(e) => {
            let symbolic = git(root, strings(&["symbolic-ref", "HEAD"])).await?;
            if git(
                root,
                strings(&["show-ref", "--verify", "--quiet", symbolic.trim()]),
            )
            .await
            .is_err_and(|e| e == "GIT_REF_MISSING")
            {
                None
            } else {
                return Err(e);
            }
        }
    };
    let branch = git(root, strings(&["branch", "--show-current"])).await?;
    let raw = git(
        root,
        strings(&[
            "status",
            "--porcelain=v1",
            "-z",
            "--untracked-files=all",
            "--ignore-submodules=all",
        ]),
    )
    .await?;
    let mut changes = Vec::new();
    let mut parts = raw.split('\0');
    while let Some(s) = parts.next() {
        if s.len() < 3 {
            continue;
        }
        let code = &s[..2];
        let old = if code.contains('R') || code.contains('C') {
            parts.next()
        } else {
            None
        };
        changes.push(json!({"status":code,"path":&s[3..],"originalPath":old}));
    }
    Ok(
        json!({"project":p["id"],"root":root,"git":true,"sha":sha,"branch":branch.trim(),"dirty":!raw.is_empty(),"statusHash":hash(raw),"changesTruncated":changes.len()>200,"changes":changes.into_iter().take(200).collect::<Vec<_>>(),"observedAt":now()}),
    )
}
fn walk(root: &Path, dir: &Path, out: &mut BTreeSet<String>) -> Result<()> {
    for entry in std::fs::read_dir(dir).map_err(|e| e.to_string())? {
        let e = entry.map_err(|e| e.to_string())?;
        if e.file_type().map_err(|e| e.to_string())?.is_dir() {
            walk(root, &e.path(), out)?
        } else {
            out.insert(
                e.path()
                    .strip_prefix(root)
                    .map_err(|e| e.to_string())?
                    .to_string_lossy()
                    .replace('\\', "/"),
            );
        }
        if out.len() > 100000 {
            return Err("目录过大，请缩小范围".into());
        }
    }
    Ok(())
}
async fn files(root: &str, prefix: &str, ignored: bool) -> Result<Vec<String>> {
    let mut args = strings(&["ls-files", "--cached", "--others", "-z"]);
    if !ignored {
        args.push("--exclude-standard".into())
    }
    let entries = match git(root, args).await {
        Ok(s) => s
            .split('\0')
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .collect::<BTreeSet<_>>(),
        Err(e) if e == "NOT_GIT_REPOSITORY" => {
            let mut out = BTreeSet::new();
            walk(Path::new(root), Path::new(root), &mut out)?;
            out
        }
        Err(e) => return Err(e),
    };
    Ok(entries
        .into_iter()
        .filter(|s| prefix.is_empty() || s == prefix || s.starts_with(&format!("{prefix}/")))
        .collect())
}
fn read(root: &str, file: &str) -> Result<String> {
    let path = Path::new(root).join(file);
    let before = std::fs::metadata(&path).map_err(io_reason)?;
    if !before.is_file() {
        return Err("NOT_REGULAR_FILE".into());
    }
    if before.len() > 16 * 1024 * 1024 {
        return Err("FILE_TOO_LARGE".into());
    }
    let bytes = std::fs::read(&path).map_err(io_reason)?;
    let after = std::fs::metadata(&path).map_err(io_reason)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if before.ino() != after.ino()
            || before.ctime() != after.ctime()
            || before.ctime_nsec() != after.ctime_nsec()
        {
            return Err("FILE_CHANGED_DURING_READ".into());
        }
    }
    if before.modified().ok() != after.modified().ok() || before.len() != after.len() {
        return Err("FILE_CHANGED_DURING_READ".into());
    }
    if bytes.contains(&0) {
        return Err("BINARY_FILE".into());
    }
    String::from_utf8(bytes).map_err(|_| "NON_UTF8_FILE".into())
}
fn io_reason(e: std::io::Error) -> String {
    match e.kind() {
        std::io::ErrorKind::NotFound => "NOT_FOUND",
        std::io::ErrorKind::PermissionDenied => "PERMISSION_DENIED",
        _ => "READ_ERROR",
    }
    .into()
}
fn sensitive(path: &Path) -> bool {
    path.components().any(|c| {
        let n = c.as_os_str().to_string_lossy().to_lowercase();
        n == ".env"
            || n.starts_with(".env.")
            || n.starts_with("id_rsa")
            || n.starts_with("id_ed25519")
            || n.starts_with("id_ecdsa")
            || n.starts_with("id_dsa")
            || n.starts_with("service-account")
            || [
                ".ssh",
                ".aws",
                ".azure",
                ".config",
                ".gnupg",
                ".cloudflared",
                ".git",
                ".codex",
                ".clc",
                ".npmrc",
                ".netrc",
                "_netrc",
                ".git-credentials",
                "credentials.json",
                "secrets.json",
                "auth.json",
                "cookies",
                "cookies.sqlite",
            ]
            .contains(&n.as_str())
            || [
                ".pem",
                ".key",
                ".p12",
                ".pfx",
                ".keystore",
                ".jks",
                ".keychain",
                ".keychain-db",
            ]
            .iter()
            .any(|x| n.ends_with(x))
    })
}
fn review_path(root: &str, file: &str) -> Result<()> {
    let root = Path::new(root);
    if sensitive(root) {
        return Err("SENSITIVE_WORKSPACE".into());
    }
    let private_root = crate::root().canonicalize().ok();
    let path = root.join(file);
    let relative = path.strip_prefix(root).map_err(|_| "OUTSIDE_WORKSPACE")?;
    if sensitive(relative) {
        return Err("SENSITIVE_PATH".into());
    }
    // Resolve existing ancestors too: deleted Git paths may still traverse a symlink.
    let mut ancestor = path.as_path();
    loop {
        match std::fs::canonicalize(ancestor) {
            Ok(actual) => {
                if private_root.as_ref().is_some_and(|p| actual.starts_with(p)) {
                    return Err("PRIVATE_RUNTIME_STATE".into());
                }
                let rel = actual.strip_prefix(root).map_err(|_| "OUTSIDE_WORKSPACE")?;
                if sensitive(rel) {
                    return Err("SENSITIVE_PATH".into());
                }
                break;
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                ancestor = ancestor.parent().ok_or("OUTSIDE_WORKSPACE")?;
            }
            Err(e) => return Err(io_reason(e)),
        }
    }
    if relative
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err("PARENT_PATH_RESTRICTED".into());
    }
    Ok(())
}
fn filter_paths(
    root: &str,
    paths: Vec<String>,
    review: bool,
    skipped: &mut std::collections::BTreeMap<String, usize>,
) -> Vec<String> {
    paths
        .into_iter()
        .filter(|p| {
            if review {
                if let Err(e) = review_path(root, p) {
                    *skipped.entry(e).or_default() += 1;
                    return false;
                }
            }
            true
        })
        .collect()
}
fn filter_state(
    root: &str,
    source: &mut Value,
    review: bool,
    skipped: &mut std::collections::BTreeMap<String, usize>,
) {
    if !review {
        return;
    }
    if let Some(changes) = source["changes"].as_array_mut() {
        changes.retain(|c| {
            for key in ["path", "originalPath"] {
                if let Some(p) = c[key].as_str() {
                    if let Err(e) = review_path(root, p) {
                        *skipped.entry(e).or_default() += 1;
                        return false;
                    }
                }
            }
            true
        });
    }
}
async fn revision(root: &str, rev: &str) -> Result<String> {
    Ok(git(
        root,
        vec![
            "rev-parse".into(),
            "--verify".into(),
            "--end-of-options".into(),
            format!("{rev}^{{commit}}"),
        ],
    )
    .await?
    .trim()
    .into())
}
pub async fn query(c: &Control, action: &str, args: &Value) -> Result<Value> {
    let p = resolve(c, string(args, "project")).await?;
    let root = string(&p, "root");
    let started = now();
    let review = args["view"] == "review";
    let mut skipped = std::collections::BTreeMap::new();
    if review {
        review_path(root, "")?;
    }
    let mut source = state(&p).await?;
    let mut source_skipped = std::collections::BTreeMap::new();
    filter_state(root, &mut source, review, &mut source_skipped);
    let file = string(args, "path");
    if file.contains('\0') {
        return Err("invalid path".into());
    }
    if review && !file.is_empty() {
        review_path(root, file)?;
    }
    let mut data = match action {
        "overview" => {
            let paths = filter_paths(root, files(root, "", false).await?, review, &mut skipped);
            json!({"listingHash":hash(paths.join("\0")),"truncated":paths.iter().filter_map(|s|s.split('/').next()).collect::<BTreeSet<_>>().len()>100 || paths.iter().filter(|s|s.ends_with("README.md")).count()>30,"name":p["name"],"topLevel":paths.iter().filter_map(|s|s.split('/').next()).collect::<BTreeSet<_>>().into_iter().take(100).collect::<Vec<_>>(),"documentEntrypoints":paths.iter().filter(|s|s.ends_with("README.md")).take(30).collect::<Vec<_>>()})
        }
        "tree" => {
            let prefix = file.trim_end_matches('/');
            let paths = filter_paths(
                root,
                files(root, prefix, args["includeIgnored"] == true).await?,
                review,
                &mut skipped,
            );
            let depth = num(args, "depth", 2)
                + if prefix.is_empty() {
                    0
                } else {
                    prefix.split('/').count()
                };
            let items = paths
                .iter()
                .map(|s| {
                    let parts: Vec<_> = s.split('/').collect();
                    format!(
                        "{}{}",
                        parts
                            .iter()
                            .take(depth)
                            .copied()
                            .collect::<Vec<_>>()
                            .join("/"),
                        if parts.len() > depth { "/" } else { "" }
                    )
                })
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();
            let offset = num(args, "offset", 0);
            let limit = num(args, "limit", 100);
            json!({"path":file,"entries":items.iter().skip(offset).take(limit).collect::<Vec<_>>(),"total":items.len(),"listingHash":hash(items.join("\0")),"nextOffset":if offset+limit<items.len(){Some(offset+limit)}else{None}})
        }
        "read" => {
            let rev = if args["revision"].is_string() {
                Some(revision(root, string(args, "revision")).await?)
            } else {
                None
            };
            let content = if let Some(ref rev) = rev {
                let entry = git(
                    root,
                    vec!["ls-tree".into(), rev.clone(), "--".into(), file.into()],
                )
                .await?;
                if !entry.starts_with("100644 blob ") && !entry.starts_with("100755 blob ") {
                    return Err("提交中没有可读普通文件".into());
                }
                let oid = entry
                    .split_whitespace()
                    .nth(2)
                    .ok_or("invalid git object")?;
                if git(root, strings(&["cat-file", "-s", oid]))
                    .await?
                    .trim()
                    .parse::<u64>()
                    .unwrap_or(u64::MAX)
                    > 16 * 1024 * 1024
                {
                    return Err("文件过大".into());
                }
                git(root, strings(&["cat-file", "blob", oid])).await?
            } else {
                read(root, file)?
            };
            if content.contains('\0') {
                return Err("需要文本文件".into());
            }
            let lines: Vec<_> = content.split('\n').collect();
            let start = num(args, "startLine", 1);
            let count = num(args, "lineCount", 160);
            json!({"path":file,"revision":rev.unwrap_or_else(||"working".into()),"sha256":hash(&content),"startLine":start,"totalLines":lines.len(),"nextLine":if start+count<=lines.len(){Some(start+count)}else{None},"text":lines.iter().skip(start-1).take(count).enumerate().map(|(i,s)|format!("{}: {s}",start+i)).collect::<Vec<_>>().join("\n"),"truncated":false})
        }
        "search" => {
            let paths = filter_paths(
                root,
                files(root, file, args["includeIgnored"] == true).await?,
                review,
                &mut skipped,
            );
            let needle = string(args, "query");
            let sensitive = args["caseSensitive"] == true;
            let needle = if sensitive {
                needle.into()
            } else {
                needle.to_lowercase()
            };
            let mut results = Vec::new();
            let mut index = num(args, "offset", 0);
            let (mut bytes, mut scanned, mut skipped_count) = (0, 0, 0);
            while index < paths.len()
                && results.len() < num(args, "limit", 30)
                && scanned < 1500
                && bytes < 256 * 1024 * 1024
            {
                let path = &paths[index];
                index += 1;
                match read(root, path) {
                    Ok(text) => {
                        bytes += text.len();
                        scanned += 1;
                        let total_matches = text
                            .lines()
                            .filter(|s| {
                                if sensitive {
                                    s.contains(&needle)
                                } else {
                                    s.to_lowercase().contains(&needle)
                                }
                            })
                            .count();
                        let matches:Vec<_>=text.lines().enumerate().filter(|(_,s)|if sensitive{s.contains(&needle)}else{s.to_lowercase().contains(&needle)}).take(5).map(|(i,s)|json!({"line":i+1,"text":s.chars().take(500).collect::<String>()})).collect();
                        if !matches.is_empty()
                            || (if sensitive {
                                path.clone()
                            } else {
                                path.to_lowercase()
                            })
                            .contains(&needle)
                        {
                            results
                                .push(json!({"path":path,"sha256":hash(&text),"matches":matches,"matchesComplete":total_matches<=5 && text.lines().filter(|s| if sensitive { s.contains(&needle) } else { s.to_lowercase().contains(&needle) }).all(|s|s.chars().count()<=500),"totalMatches":total_matches,"readNext":{"tool":"read","arguments":{"project":args["project"],"path":path,"view":if review{"review"}else{"raw"}}}}));
                        }
                    }
                    Err(e) => {
                        skipped_count += 1;
                        *skipped.entry(e).or_default() += 1;
                    }
                }
            }
            json!({"query":args["query"],"results":results,"scanned":scanned,"skipped":skipped_count,"listingHash":hash(paths.join("\0")),"nextOffset":if index<paths.len(){Some(index)}else{None}})
        }
        "git" => {
            let op = string(args, "operation");
            if op == "status" {
                skipped = source_skipped.clone();
                source.clone()
            } else if op == "show" {
                let rev = revision(root, args["revision"].as_str().unwrap_or("HEAD")).await?;
                let raw = git(
                    root,
                    vec![
                        "diff-tree".into(),
                        "--root".into(),
                        "--no-commit-id".into(),
                        "--name-only".into(),
                        "--no-renames".into(),
                        "-r".into(),
                        "-z".into(),
                        rev.clone(),
                    ],
                )
                .await?;
                let paths: Vec<_> = raw.split('\0').filter(|s| !s.is_empty()).collect();
                let paths = filter_paths(
                    root,
                    paths.iter().map(|s| s.to_string()).collect(),
                    review,
                    &mut skipped,
                );
                json!({"revision":rev,"text":if review { String::new() } else {git(root, vec!["show".into(),"-s".into(),"--format=%H%n%cs%n%s".into(),rev.clone()]).await?},"truncated":false,"paths":paths.iter().take(200).collect::<Vec<_>>(),"truncatedPaths":paths.len()>200,"hiddenPaths":skipped.values().sum::<usize>()})
            } else {
                let mut cmd = match op {
                    "log" => vec![
                        "log".into(),
                        format!("-{}", num(args, "limit", 10)),
                        "--format=%H %cs %s".into(),
                        revision(root, args["revision"].as_str().unwrap_or("HEAD")).await?,
                    ],
                    "diff" => {
                        if args["baseRevision"].is_string() && !args["revision"].is_string() {
                            return Err("baseRevision requires revision".into());
                        }
                        if args["staged"] == true && args["revision"].is_string() {
                            return Err("staged cannot be combined with revision".into());
                        }
                        let mut v = strings(&[
                            "diff",
                            "--no-ext-diff",
                            "--no-textconv",
                            "--no-renames",
                            "--ignore-submodules=all",
                            "--unified=3",
                        ]);
                        if args["staged"] == true {
                            v.push("--cached".into())
                        }
                        for key in ["baseRevision", "revision"] {
                            if let Some(rev) = args[key].as_str() {
                                v.push(revision(root, rev).await?)
                            }
                        }
                        v
                    }
                    _ => return Err("未知 Git 查询".into()),
                };
                cmd.push("--".into());
                if !file.is_empty() {
                    cmd.push(file.into())
                }
                if review && op == "log" {
                    for arg in &mut cmd {
                        if arg.starts_with("--format=") {
                            *arg = "--format=%H %cs".into();
                        }
                    }
                    json!({"text":git(root,cmd).await?,"metadataOnly":true,"omitted":"commit-messages","truncated":false})
                } else if review {
                    let mut names = cmd.clone();
                    names.insert(1, "--name-status".into());
                    names.insert(2, "-z".into());
                    names.retain(|s| s != "--no-renames");
                    names.insert(3, "--find-renames".into());
                    // Discover globally so a path selector cannot hide a sensitive rename side.
                    names.truncate(names.iter().position(|s| s == "--").unwrap() + 1);
                    let raw = git(root, names).await?;
                    let mut parts = raw.split('\0').filter(|s| !s.is_empty());
                    let mut candidates = Vec::new();
                    let mut restricted_deletion = false;
                    while let Some(status) = parts.next() {
                        let first = parts.next().ok_or("INVALID_GIT_PATH_METADATA")?;
                        let mut pair = vec![first.to_owned()];
                        if status.starts_with('R') || status.starts_with('C') {
                            pair.push(parts.next().ok_or("INVALID_GIT_PATH_METADATA")?.to_owned());
                        }
                        let allowed = filter_paths(root, pair.clone(), true, &mut skipped);
                        if allowed.len() == pair.len() {
                            candidates.push((status.to_owned(), pair));
                        } else if status.starts_with('D') {
                            restricted_deletion = true;
                        }
                    }
                    let mut safe = Vec::new();
                    for (status, paths) in candidates {
                        // Git similarity detection cannot pair a heavily rewritten rename.
                        if restricted_deletion && status.starts_with('A') {
                            *skipped
                                .entry("POSSIBLE_SENSITIVE_RENAME".into())
                                .or_default() += paths.len();
                        } else {
                            safe.extend(paths);
                        }
                    }
                    safe.retain(|p| {
                        file.is_empty() || p == file || p.starts_with(&format!("{file}/"))
                    });
                    cmd.truncate(cmd.iter().position(|s| s == "--").unwrap() + 1);
                    cmd.extend(safe.clone());
                    json!({"text":if safe.is_empty(){String::new()}else{git(root,cmd).await?},"truncated":false,"paths":safe})
                } else {
                    json!({"text":git(root,cmd).await?,"truncated":false})
                }
            }
        }
        _ => return Err("未知项目查询".into()),
    };
    let after = state(&p).await?;
    let changed = source["sha"] != after["sha"] || source["statusHash"] != after["statusHash"];
    let paged = !data["nextLine"].is_null() || !data["nextOffset"].is_null();
    let snippets_partial = data["results"]
        .as_array()
        .is_some_and(|r| r.iter().any(|r| r["matchesComplete"] == false));
    data["coverage"] = json!({"state":if paged || snippets_partial || !skipped.is_empty() || data["changesTruncated"]==true || data["truncatedPaths"]==true || data["truncated"]==true || data["state"]=="restricted" {"partial"}else{"complete"},"paged":paged,"snippetsComplete":!snippets_partial,"skippedByReason":skipped});
    source["consistency"] = json!("best-effort");
    source["statusHashScope"] = json!("git-status-text-not-workspace-content");
    source["observationStartedAt"] = json!(started);
    source["observationEndedAt"] = json!(now());
    source["view"] = json!(if review { "review" } else { "raw" });
    source["coverage"] = json!({"state":if source["changesTruncated"]==true || !source_skipped.is_empty(){"partial"}else{"complete"},"skippedByReason":source_skipped});
    if args["expectedHash"].is_string() {
        let actual = if action == "read" {
            &data["sha256"]
        } else {
            &data["listingHash"]
        };
        if actual != &args["expectedHash"] {
            return Err("CONTENT_CHANGED_RESTART_PAGINATION".into());
        }
    }
    Ok(json!({"source":source,"changedDuringRead":changed,"consistency":"best-effort","data":data}))
}

pub async fn native_id(c: &Control, cwd: &str) -> Result<Option<String>> {
    let target = Path::new(cwd).canonicalize().map_err(|e| e.to_string())?;
    let mut matches = BTreeSet::new();
    for project in list(c).await? {
        for root in project["roots"].as_array().into_iter().flatten() {
            if Path::new(string(root, "path")).canonicalize().ok().as_ref() == Some(&target) {
                matches.insert(string(&project, "id").to_owned());
            }
        }
    }
    Ok(if matches.len() == 1 {
        matches.into_iter().next()
    } else {
        None
    })
}
