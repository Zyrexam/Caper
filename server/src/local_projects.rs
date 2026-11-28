//! Local projects — a single local user adds directories on disk (a plain
//! project or a git repo), picks one as active, and sessions created while
//! it is active live inside it. No users, owners, or per-project sharing:
//! the registry is one JSON file under `.caper`, beside the conversations.
//!
//! A project's *name* is its identity everywhere (`conversation.project`);
//! `path` is where its files are. `path` is optional: a named project with
//! no directory keeps session edits in the server root.

use std::path::{Path, PathBuf};

use serde_json::{json, Value};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalProject {
    pub name: String,
    pub path: String,
}

impl LocalProject {
    fn to_json(&self) -> Value {
        json!({ "name": self.name, "path": self.path })
    }
}

fn registry_path(root: &Path) -> PathBuf {
    root.join(".caper").join("local-projects.json")
}

fn load_raw(root: &Path) -> Value {
    let text = std::fs::read_to_string(registry_path(root)).unwrap_or_default();
    let v: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
    if v.is_object() {
        v
    } else {
        json!({})
    }
}

fn save(root: &Path, projects: &[LocalProject], active: Option<&str>) -> Result<(), String> {
    let p = registry_path(root);
    if let Some(dir) = p.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let body = json!({
        "projects": projects.iter().map(LocalProject::to_json).collect::<Vec<_>>(),
        "active": active,
    });
    let text = serde_json::to_string_pretty(&body).map_err(|e| e.to_string())?;
    // Written whole; a half-written registry would lose every project.
    let tmp = p.with_extension("json.tmp");
    std::fs::write(&tmp, text).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &p).map_err(|e| e.to_string())
}

pub fn projects(root: &Path) -> Vec<LocalProject> {
    load_raw(root)
        .get("projects")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(|r| {
                    let name = r.get("name").and_then(Value::as_str)?.trim();
                    let path = r.get("path").and_then(Value::as_str).unwrap_or_default();
                    if name.is_empty() {
                        None
                    } else {
                        Some(LocalProject {
                            name: name.to_string(),
                            path: path.to_string(),
                        })
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

pub fn active_name(root: &Path) -> Option<String> {
    load_raw(root)
        .get("active")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

pub fn active(root: &Path) -> Option<LocalProject> {
    let name = active_name(root)?;
    // A chosen name needs no registry row: named-only projects are active
    // too. `path` is attached when the project was added from a directory.
    let path = projects(root)
        .iter()
        .find(|p| p.name == name)
        .map(|p| p.path.clone())
        .unwrap_or_default();
    Some(LocalProject { name, path })
}

/// Register a local project rooted at `raw_path`. The path must be an
/// existing local directory; it need not be a git repository. `name`
/// defaults to the directory's own name. Adding an existing project
/// refreshes its path rather than duplicating it.
pub fn add(root: &Path, raw_name: &str, raw_path: &str) -> Result<LocalProject, String> {
    let canon = crate::hostgit::check_path(raw_path)?;
    let canon_s = canon.to_string_lossy().to_string();
    let name = if raw_name.trim().is_empty() {
        canon
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| canon_s.clone())
    } else {
        raw_name.trim().to_string()
    };
    let name = angela::projects::normalize(&name)?;
    let project = LocalProject {
        name,
        path: canon_s,
    };
    let mut list = projects(root);
    match list.iter_mut().find(|p| p.name == project.name) {
        Some(slot) => *slot = project.clone(),
        None => list.push(project.clone()),
    }
    save(root, &list, active_name(root).as_deref())?;
    Ok(project)
}

/// Choose the active project. An empty name clears it. Choosing a name
/// that is not registered locally still selects it (it may be a named
/// folder with no directory); `path` is then empty.
pub fn set_active(root: &Path, name: &str) -> Result<Option<LocalProject>, String> {
    let name = name.trim();
    let list = projects(root);
    if name.is_empty() {
        save(root, &list, None)?;
        return Ok(None);
    }
    let name = angela::projects::normalize(name)?;
    save(root, &list, Some(&name))?;
    Ok(active(root))
}

/// Where a named project's files are, when it was added from a directory.
pub fn path_for(root: &Path, name: &str) -> Option<PathBuf> {
    let p = projects(root)
        .into_iter()
        .find(|p| p.name == name && !p.path.is_empty())?;
    let dir = PathBuf::from(&p.path);
    dir.is_dir().then_some(dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(tag: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("caper-local-projects-{tag}-{nanos}"));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn add_choose_and_resolve_a_local_project() {
        let root = temp_root("add");
        let repo = temp_root("repo");
        // A directory need not be a git repo.
        let p = add(&root, "", &repo.to_string_lossy()).unwrap();
        assert_eq!(p.name, repo.file_name().unwrap().to_string_lossy());
        assert!(!p.path.is_empty());
        // Relative paths are refused; nothing on disk is touched.
        assert!(add(&root, "x", "relative/dir").is_err());
        assert!(add(&root, "x", &root.join("nope").to_string_lossy()).is_err());

        // Choosing active persists; sessions resolve the directory.
        let chosen = set_active(&root, &p.name).unwrap().unwrap();
        assert_eq!(chosen.name, p.name);
        assert_eq!(active_name(&root).as_deref(), Some(p.name.as_str()));
        assert!(path_for(&root, &p.name).is_some());

        // Re-adding the same name refreshes rather than duplicates.
        let other = temp_root("other");
        add(&root, &p.name, &other.to_string_lossy()).unwrap();
        assert_eq!(projects(&root).len(), 1);
        // Active selection survives re-adding.
        assert_eq!(active_name(&root).as_deref(), Some(p.name.as_str()));

        // Clearing active works.
        assert!(set_active(&root, "").unwrap().is_none());
        assert!(active_name(&root).is_none());

        // A name with no directory can still be chosen; no path resolves.
        set_active(&root, "named-only").unwrap();
        assert_eq!(active_name(&root).as_deref(), Some("named-only"));
        assert!(path_for(&root, "named-only").is_none());

        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&repo);
        let _ = std::fs::remove_dir_all(&other);
    }
}
