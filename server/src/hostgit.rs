//! ## Why these live here and not in angela
//! ## Why there is no scan
//! A repo becomes selectable only when someone registers its absolute path.
//! every clone on the machine — including ones you would never mount into a
//! shell, so a path or a branch name is an argument and can never be parsed

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{json, Map, Value};

const GIT_TIMEOUT_SECS: u64 = 30;


fn registry_path(root: &Path) -> PathBuf {
    root.join(".caper").join("hosts.json")
}

pub fn load(root: &Path) -> Value {
    let text = std::fs::read_to_string(registry_path(root)).unwrap_or_default();
    let v: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
    if v.is_object() {
        v
    } else {
        json!({ "repos": [], "worktrees": [] })
    }
}

fn save(root: &Path, v: &Value) -> Result<(), String> {
    let p = registry_path(root);
    if let Some(dir) = p.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let text = serde_json::to_string_pretty(v).map_err(|e| e.to_string())?;
    // registration, and this file is only ever read whole.
    let tmp = p.with_extension("json.tmp");
    std::fs::write(&tmp, text).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &p).map_err(|e| e.to_string())
}

fn list_of<'a>(reg: &'a Value, key: &str) -> &'a [Value] {
    reg.get(key).and_then(Value::as_array).map(|v| &v[..]).unwrap_or(&[])
}

fn id_for(prefix: &str, path: &str) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in path.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x1000_0000_01b3);
    }
    format!("{prefix}{h:012x}")
}


/// `:` and `,` are refused because sbox-b's `--mount` takes a
pub fn check_path(raw: &str) -> Result<PathBuf, String> {
    let t = raw.trim();
    if t.is_empty() {
        return Err("a repo needs an absolute path".into());
    }
    if !t.starts_with('/') {
        return Err(format!("path must be absolute: {t}"));
    }
    if t.contains(':') || t.contains(',') {
        return Err("path cannot contain ':' or ',' — a mount spec is delimited by both".into());
    }
    let p = PathBuf::from(t);
    let canon = p
        .canonicalize()
        .map_err(|_| format!("no such directory: {t}"))?;
    if !canon.is_dir() {
        return Err(format!("not a directory: {t}"));
    }
    let s = canon.to_string_lossy().to_string();
    if s.contains(':') || s.contains(',') {
        return Err("resolved path contains ':' or ','".into());
    }
    Ok(canon)
}

fn is_git_dir(p: &Path) -> bool {
    let g = p.join(".git");
    g.is_dir() || g.is_file()
}

pub fn is_safe_ref(s: &str) -> bool {
    let t = s.trim();
    !t.is_empty()
        && t.len() <= 200
        && !t.starts_with('-')
        && !t.starts_with('/')
        && !t.ends_with('/')
        && !t.contains("..")
        && !t.contains("//")
        && !t.contains('\\')
        && !t.contains('~')
        && !t.contains('^')
        && !t.contains(':')
        && !t.contains('?')
        && !t.contains('*')
        && !t.contains('[')
        && !t.chars().any(|c| c.is_whitespace() || c.is_control())
}


/// Run git in `dir`. Never through a shell: every argument is passed as one.
fn git(dir: &Path, args: &[&str]) -> Result<String, String> {
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(dir);
    for a in args {
        cmd.arg(a);
    }
    cmd.env("GIT_TERMINAL_PROMPT", "0");
    cmd.env("GIT_OPTIONAL_LOCKS", "0");
    let out = run_with_timeout(cmd)?;
    if out.status_ok {
        Ok(out.stdout)
    } else {
        let msg = out.stderr.lines().next().unwrap_or("git failed").to_string();
        Err(msg)
    }
}

struct Output {
    status_ok: bool,
    stdout: String,
    stderr: String,
}

fn run_with_timeout(mut cmd: Command) -> Result<Output, String> {
    use std::process::Stdio;
    use std::time::{Duration, Instant};

    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    let mut child = cmd.spawn().map_err(|e| format!("could not run git: {e}"))?;
    let deadline = Instant::now() + Duration::from_secs(GIT_TIMEOUT_SECS);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {
                if Instant::now() > deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!("git timed out after {GIT_TIMEOUT_SECS}s"));
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(e) => return Err(e.to_string()),
        }
    }
    let out = child
        .wait_with_output()
        .map_err(|e| format!("could not read git output: {e}"))?;
    Ok(Output {
        status_ok: out.status.success(),
        stdout: String::from_utf8_lossy(&out.stdout).to_string(),
        stderr: String::from_utf8_lossy(&out.stderr).to_string(),
    })
}

pub fn branches(dir: &Path) -> Vec<String> {
    git(
        dir,
        &["for-each-ref", "--format=%(refname:short)", "refs/heads"],
    )
    .map(|s| {
        s.lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(str::to_string)
            .collect()
    })
    .unwrap_or_default()
}

fn current_branch(dir: &Path) -> String {
    git(dir, &["rev-parse", "--abbrev-ref", "HEAD"])
        .map(|s| s.trim().to_string())
        .map(|s| if s == "HEAD" { String::new() } else { s })
        .unwrap_or_default()
}


pub fn add_repo(root: &Path, raw_path: &str) -> Result<Value, String> {
    let path = check_path(raw_path)?;
    if !is_git_dir(&path) {
        return Err(format!("not a git repository: {}", path.display()));
    }
    let path_s = path.to_string_lossy().to_string();
    let id = id_for("r", &path_s);
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path_s.clone());

    let mut reg = load(root);
    let mut repos: Vec<Value> = list_of(&reg, "repos").to_vec();
    repos.retain(|r| r.get("id").and_then(Value::as_str) != Some(id.as_str()));
    repos.push(json!({ "id": id, "path": path_s, "name": name }));
    reg["repos"] = Value::Array(repos);
    reg.as_object_mut()
        .map(|m| m.entry("worktrees").or_insert_with(|| json!([])));
    save(root, &reg)?;
    // What the branch picker should land on. Registering a clone is only
    let head = current_branch(&path);
    let select = if head.is_empty() {
        String::new()
    } else {
        format!("{id}:{head}")
    };
    Ok(json!({ "id": id, "select": select, "selectKind": "branch" }))
}

pub fn remove_repo(root: &Path, id: &str) -> Result<Value, String> {
    let mut reg = load(root);
    let repos: Vec<Value> = list_of(&reg, "repos")
        .iter()
        .filter(|r| r.get("id").and_then(Value::as_str) != Some(id))
        .cloned()
        .collect();
    let worktrees: Vec<Value> = list_of(&reg, "worktrees")
        .iter()
        .filter(|w| w.get("repoId").and_then(Value::as_str) != Some(id))
        .cloned()
        .collect();
    reg["repos"] = Value::Array(repos);
    reg["worktrees"] = Value::Array(worktrees);
    save(root, &reg)?;
    Ok(json!({ "removed": id }))
}

pub fn add_branch(root: &Path, repo_id: &str, name: &str, from: &str) -> Result<Value, String> {
    let repo = find(root, "repos", repo_id).ok_or("no such repo")?;
    let dir = PathBuf::from(repo.get("path").and_then(Value::as_str).unwrap_or_default());
    if !is_safe_ref(name) {
        return Err(format!("not a usable branch name: {name}"));
    }
    let base = from.trim();
    if !base.is_empty() && !is_safe_ref(base) {
        return Err(format!("not a usable start point: {base}"));
    }
    let mut args = vec!["branch", "--", name];
    if !base.is_empty() {
        args.push(base);
    }
    git(&dir, &args)?;
    Ok(json!({
        "branch": name,
        "select": format!("{repo_id}:{name}"),
        "selectKind": "branch",
    }))
}

fn find(root: &Path, key: &str, id: &str) -> Option<Value> {
    list_of(&load(root), key)
        .iter()
        .find(|r| r.get("id").and_then(Value::as_str) == Some(id))
        .cloned()
}

fn worktree_dir(root: &Path, repo_name: &str, branch: &str) -> PathBuf {
    let safe: String = branch
        .chars()
        .map(|c| if c == '/' { '-' } else { c })
        .collect();
    root.join(".caper")
        .join("worktrees")
        .join(format!("{repo_name}-{safe}"))
}


pub fn add_worktree(
    root: &Path,
    repo_id: &str,
    branch: &str,
    create: bool,
) -> Result<Value, String> {
    let repo = find(root, "repos", repo_id).ok_or("no such repo")?;
    let repo_path = repo.get("path").and_then(Value::as_str).unwrap_or_default();
    let repo_name = repo.get("name").and_then(Value::as_str).unwrap_or("repo");
    let dir = PathBuf::from(repo_path);
    if !is_safe_ref(branch) {
        return Err(format!("not a usable branch name: {branch}"));
    }
    let target = worktree_dir(root, repo_name, branch);
    if target.exists() {
        return Err(format!(
            "a worktree directory already exists at {}",
            target.display()
        ));
    }
    std::fs::create_dir_all(target.parent().unwrap_or(&target)).map_err(|e| e.to_string())?;
    let target_s = target.to_string_lossy().to_string();
    if target_s.contains(':') || target_s.contains(',') {
        return Err("the worktree path would contain ':' or ','".into());
    }

    let args: Vec<&str> = if create {
        vec!["worktree", "add", "-b", branch, &target_s]
    } else {
        vec!["worktree", "add", &target_s, branch]
    };
    git(&dir, &args)?;

    let id = id_for("w", &target_s);
    let mut reg = load(root);
    let mut list: Vec<Value> = list_of(&reg, "worktrees").to_vec();
    list.retain(|w| w.get("id").and_then(Value::as_str) != Some(id.as_str()));
    list.push(json!({
        "id": id,
        "repoId": repo_id,
        "repoName": repo_name,
        "path": target_s,
        "branch": branch,
        "sandboxId": "",
    }));
    reg["worktrees"] = Value::Array(list);
    save(root, &reg)?;
    Ok(json!({
        "id": id,
        "path": target.to_string_lossy(),
        "select": id,
        "selectKind": "worktree",
    }))
}

/// `--force` because the point of removing it is that it is going away, and
pub fn remove_worktree(root: &Path, id: &str) -> Result<Value, String> {
    let Some(wt) = find(root, "worktrees", id) else {
        return Err("no such worktree".into());
    };
    let repo_id = wt.get("repoId").and_then(Value::as_str).unwrap_or_default();
    let path = wt.get("path").and_then(Value::as_str).unwrap_or_default();
    let mut note = String::new();
    if let Some(repo) = find(root, "repos", repo_id) {
        let dir = PathBuf::from(repo.get("path").and_then(Value::as_str).unwrap_or_default());
        if let Err(e) = git(&dir, &["worktree", "remove", "--force", path]) {
            note = e;
        }
    }
    let mut reg = load(root);
    let list: Vec<Value> = list_of(&reg, "worktrees")
        .iter()
        .filter(|w| w.get("id").and_then(Value::as_str) != Some(id))
        .cloned()
        .collect();
    reg["worktrees"] = Value::Array(list);
    save(root, &reg)?;
    Ok(json!({ "removed": id, "note": note }))
}

pub fn claim_worktree(root: &Path, id: &str, sandbox: &str) {
    let mut reg = load(root);
    let list: Vec<Value> = list_of(&reg, "worktrees")
        .iter()
        .map(|w| {
            let mut w = w.clone();
            if w.get("id").and_then(Value::as_str) == Some(id) {
                w["sandboxId"] = json!(sandbox);
            }
            w
        })
        .collect();
    reg["worktrees"] = Value::Array(list);
    let _ = save(root, &reg);
}

/// the checkout bound into it, so a `DELETE` there would otherwise leave the
pub fn forget_for_sandbox(root: &Path, sandbox: &str) {
    if sandbox.is_empty() {
        return;
    }
    let ids: Vec<String> = list_of(&load(root), "worktrees")
        .iter()
        .filter(|w| w.get("sandboxId").and_then(Value::as_str) == Some(sandbox))
        .filter_map(|w| w.get("id").and_then(Value::as_str).map(str::to_string))
        .collect();
    for id in ids {
        let _ = remove_worktree(root, &id);
    }
}

/// or a directory removed by hand, otherwise leaves a row the UI offers and
pub fn sweep(root: &Path) {
    let reg = load(root);
    let before = list_of(&reg, "worktrees").len();
    let kept: Vec<Value> = list_of(&reg, "worktrees")
        .iter()
        .filter(|w| {
            w.get("path")
                .and_then(Value::as_str)
                .map(|p| Path::new(p).is_dir())
                .unwrap_or(false)
        })
        .cloned()
        .collect();
    if kept.len() != before {
        let mut reg = reg;
        reg["worktrees"] = Value::Array(kept);
        let _ = save(root, &reg);
    }
}


/// Flattened rather than grouped because the control shows one line, and two
pub fn branch_items(root: &Path) -> Value {
    let reg = load(root);
    let mut out = Vec::new();
    for repo in list_of(&reg, "repos") {
        let id = repo.get("id").and_then(Value::as_str).unwrap_or_default();
        let name = repo.get("name").and_then(Value::as_str).unwrap_or_default();
        let path = PathBuf::from(repo.get("path").and_then(Value::as_str).unwrap_or_default());
        if !path.is_dir() {
            continue;
        }
        for b in branches(&path) {
            out.push(json!({
                "id": format!("{id}:{b}"),
                "name": b,
                "hint": name,
                "icon": "git-branch",
            }));
        }
    }
    Value::Array(out)
}

pub fn repo_items(root: &Path) -> Value {
    let reg = load(root);
    Value::Array(
        list_of(&reg, "repos")
            .iter()
            .map(|r| {
                let path = r.get("path").and_then(Value::as_str).unwrap_or_default();
                json!({
                    "id": r.get("id").cloned().unwrap_or(Value::Null),
                    "name": r.get("name").cloned().unwrap_or(Value::Null),
                    "hint": current_branch(Path::new(path)),
                    "path": path,
                    "icon": "folder-open",
                })
            })
            .collect::<Vec<_>>(),
    )
}

pub fn worktree_items(root: &Path) -> Value {
    let reg = load(root);
    Value::Array(
        list_of(&reg, "worktrees")
            .iter()
            .map(|w| {
                json!({
                    "id": w.get("id").cloned().unwrap_or(Value::Null),
                    "name": w.get("branch").cloned().unwrap_or(Value::Null),
                    "hint": w.get("repoName").cloned().unwrap_or(Value::Null),
                    "path": w.get("path").cloned().unwrap_or(Value::Null),
                    "icon": "tree-structure",
                })
            })
            .collect::<Vec<_>>(),
    )
}

/// because the caller has to know whether there is anything to reap.
pub fn mount_for(root: &Path, worktree_id: &str, branch_id: &str) -> Option<(String, String)> {
    if !worktree_id.is_empty() {
        if let Some(w) = find(root, "worktrees", worktree_id) {
            let p = w.get("path").and_then(Value::as_str).unwrap_or_default();
            if Path::new(p).is_dir() {
                return Some((p.to_string(), worktree_id.to_string()));
            }
        }
    }
    // `<repoId>:<branch>` — only the repo half names a directory. The branch
    // does not switch it, which is exactly why worktrees exist.
    let repo_id = branch_id.split(':').next().unwrap_or_default();
    if repo_id.is_empty() {
        return None;
    }
    let repo = find(root, "repos", repo_id)?;
    let p = repo.get("path").and_then(Value::as_str).unwrap_or_default();
    if Path::new(p).is_dir() {
        return Some((p.to_string(), String::new()))
    }
    None
}

pub fn everything(root: &Path) -> Value {
    let mut m = Map::new();
    m.insert("repos".into(), repo_items(root));
    m.insert("repoBranches".into(), branch_items(root));
    m.insert("worktrees".into(), worktree_items(root));
    Value::Object(m)
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_branch_name_that_could_be_an_option_is_refused() {
        assert!(is_safe_ref("main"));
        assert!(is_safe_ref("feature/login"));
        assert!(!is_safe_ref("--upload-pack=touch /tmp/pwned"));
        assert!(!is_safe_ref("-b"));
        assert!(!is_safe_ref("has space"));
        assert!(!is_safe_ref("a..b"));
        assert!(!is_safe_ref(""));
        assert!(!is_safe_ref("x:y"));
    }

    #[test]
    fn a_path_that_would_break_a_mount_spec_is_refused() {
        assert!(check_path("relative/path").is_err());
        assert!(check_path("/no/such/dir/anywhere").is_err());
        assert!(check_path("/tmp/a:b").is_err());
        assert!(check_path("/tmp/a,b").is_err());
        assert!(check_path("  ").is_err());
        assert!(check_path("/tmp").is_ok());
    }

    #[test]
    fn ids_are_derived_so_registering_twice_is_idempotent() {
        assert_eq!(id_for("r", "/a/b"), id_for("r", "/a/b"));
        assert_ne!(id_for("r", "/a/b"), id_for("r", "/a/c"));
        assert_ne!(id_for("r", "/a/b"), id_for("w", "/a/b"));
    }

    #[test]
    fn a_branch_with_a_slash_becomes_one_directory() {
        let d = worktree_dir(Path::new("/root"), "chat", "feature/login");
        assert!(d.ends_with("chat-feature-login"));
    }
}