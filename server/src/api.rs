
use std::sync::Arc;

use angela::http::{Request as ARequest, ResponseSink};
use angela::serve::{AppState, Server as AngelaServer};
use rustex1_devserver::{Request, State, Stream};
use serde_json::Value;

/// Where sessions live. `CAPER_ROOT` overrides; legacy `ANGELA_ROOT` remains
/// legacy `ANGELA_ROOT` remains supported; otherwise the server directory is
pub fn project_root() -> std::path::PathBuf {
    if let Ok(v) = std::env::var("CAPER_ROOT") {
        if !v.trim().is_empty() {
            return std::path::PathBuf::from(v.trim());
        }
    }
    match std::env::var("ANGELA_ROOT") {
        Ok(v) if !v.trim().is_empty() => std::path::PathBuf::from(v.trim()),
        _ => std::env::current_dir().unwrap_or_else(|_| ".".into()),
    }
}

fn migrate_legacy_store(root: &std::path::Path) {
    let legacy = root.join(".angela");
    let current = root.join(".caper");
    if legacy.is_dir() && !current.exists() {
        if let Err(error) = std::fs::rename(&legacy, &current) {
            eprintln!(
                "Caper: could not migrate legacy store {} to {}: {error}",
                legacy.display(),
                current.display()
            );
        }
    }
}

pub(crate) fn angela(port: u16) -> &'static AngelaServer {
    use std::sync::{Mutex, OnceLock};
    // One embedded server per project root. The shipped app only ever uses
    // its one workspace root; keying by root keeps independent callers (and
    // tests) from sharing one `.caper` store. `Box::leak` because callers
    // receive `&'static`.
    static SERVERS: OnceLock<Mutex<std::collections::HashMap<std::path::PathBuf, &'static AngelaServer>>> = OnceLock::new();
    let map = SERVERS.get_or_init(|| Mutex::new(std::collections::HashMap::new()));
    let root = project_root();
    let mut guard = map.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(server) = guard.get(&root) {
        return server;
    }
    let server = build_angela_server(&root, port);
    let leaked: &'static AngelaServer = Box::leak(Box::new(server));
    guard.insert(root.clone(), leaked);
    leaked
}

fn build_angela_server(root: &std::path::Path, port: u16) -> AngelaServer {
    migrate_legacy_store(root);
    let mut state = AppState::new(root, port).expect("angela state");
    // a typed prompt. Wired here because the bridging (the event feed)
    state.prompt_sender = Some(Arc::new(|session: &str, content: String, agent: String, model: String| {
        crate::automations::submit_prompt(session, content, agent, model);
    }));
    state.seat_sender = Some(Arc::new(|_session: &str| {
        crate::events::bump_sessions();
    }));
    angela::sandbox::set_backend(Box::new(crate::sbox::SboxBackend {
        root: root.to_path_buf(),
    }));
    eprintln!(
        "angela embedded · conversations in {}/.caper/conversations.db",
        root.display()
    );
    AngelaServer::new(Arc::new(state))
}

fn chat_messages(store: &angela::conv_store::ConvStore, id: &str) -> Vec<Value> {
    let mut messages = angela::serve::reconstruct_messages(store, id);
    let path = store.path(id);
    let order: std::collections::HashMap<_, _> = path.iter().enumerate().map(|(i, node)| (node.id.as_str(), i)).collect();
    for (index, node) in path.iter().enumerate() {
        if node.kind != "chat_status" { continue; }
        let Some(text) = node.payload.get("text").and_then(Value::as_str) else { continue; };
        let row = serde_json::json!({"id": node.id, "eventId": node.id, "kind": "system", "text": text, "ts": node.ts, "commandNotice": true, "visible": node.visible});
        let position = messages.iter().position(|row| row.get("eventId").and_then(Value::as_str).and_then(|id| order.get(id)).is_some_and(|next| *next > index)).unwrap_or(messages.len());
        messages.insert(position, row);
    }
    for message in &mut messages { crate::assistant_text::normalize_assistant_messages(message); }
    messages
}

struct DevSink<'s, 'a> {
    out: &'s mut Stream<'a>,
    streaming: bool,
    ended: bool,
    chat: bool,
    session_id: String,
    prompt: String,
    baseline: Vec<Value>,
    turn_id: Option<u64>,
    disconnected: bool,
    port: u16,
    assistant_filter: crate::assistant_text::StreamFilter,
}

impl DevSink<'_, '_> {
    fn publish_row(&mut self, value: &Value) {
        if self.chat {
            if let Some(id) = self.turn_id { crate::events::turn_row(id, value); }
        }
        if !self.disconnected {
            let mut line = serde_json::to_vec(value).unwrap_or_else(|_| b"{}".to_vec());
            line.push(b'\n');
            self.disconnected = self.out.chunk(&line).is_err();
        }
    }
}

impl<'s, 'a> ResponseSink for DevSink<'s, 'a> {
    fn json(&mut self, status: u16, body: &Value) {
        let mut body = body.clone();
        if body.get("messages").and_then(Value::as_array).is_some() {
            if let Some(id) = body.pointer("/session/id").and_then(Value::as_str).map(str::to_owned) {
                body["messages"] = Value::Array(chat_messages(&angela(self.port).state.store, &id));
            }
        }
        crate::assistant_text::normalize_assistant_messages(&mut body);
        let _ = self.out.json(status, &body);
        self.ended = true;
    }

    fn begin_ndjson(&mut self) {
        // The dev server adds its own chunked framing, so only the content
        let _ = self.out.start(
            200,
            &[
                ("content-type", "application/x-ndjson"),
                ("cache-control", "no-cache"),
            ],
        );
        self.streaming = true;
        if self.chat && !self.session_id.is_empty() {
            self.turn_id = Some(crate::events::begin_turn(&self.session_id, std::mem::take(&mut self.baseline), self.prompt.clone()));
        }
    }

    fn row(&mut self, value: &Value) -> bool {
        if self.chat {
            if self.turn_id.is_none() && value.get("type").and_then(Value::as_str) == Some("session") {
                if let Some(id) = value.get("sessionId").and_then(Value::as_str) {
                    self.session_id = id.to_string();
                    let mut baseline = chat_messages(&angela(self.port).state.store, id);
                    if baseline.last().is_some_and(|v| v.get("kind").and_then(Value::as_str) == Some("user") && v.get("text").and_then(Value::as_str) == Some(self.prompt.as_str())) { baseline.pop(); }
                    self.turn_id = Some(crate::events::begin_turn(id, baseline, self.prompt.clone()));
                }
            }
        }
        let rows = if self.chat { self.assistant_filter.process(value) } else { vec![value.clone()] };
        for row in rows { self.publish_row(&row); }
        // the server and its SSE subscribers, so do not cancel inference.
        self.chat || !self.disconnected
    }

    fn end(&mut self) {
        if self.ended {
            return;
        }
        self.ended = true;
        if let Some(row) = self.assistant_filter.finish() { self.publish_row(&row); }
        if let Some(id) = self.turn_id { crate::events::end_turn(id); }
        if !self.disconnected { let _ = self.out.finish(); }
    }
}

pub fn models() -> Value {
    let cfg = agl::config::read_user_config();
    let mut default = cfg.get("default_model").and_then(|v| v.to_scalar_string()).unwrap_or_default();
    let mut models = Vec::<String>::new();
    if let Some(providers) = cfg.get("context_windows").and_then(|v| v.as_map()) {
        for (provider, entries) in providers {
        if let Some(entries) = entries.as_map() {
            for name in entries.keys() {
            let id = format!("{provider}:{name}");
            if !models.contains(&id) { models.push(id); }
            }
        }
        }
    }
    for name in agl::opencode::models() {
        let id = format!("opencode:{name}");
        if !models.contains(&id) { models.push(id); }
    }
    if default.is_empty() {
        default = models
            .iter()
            .find(|m| m.ends_with(":deepseek-v4.1-flash-free"))
            .or_else(|| models.first())
            .cloned()
            .unwrap_or_default();
    }
    if !default.is_empty() && !models.contains(&default) { models.push(default.clone()); }
    serde_json::json!({"default_model": default, "models": models})
}

pub(crate) fn sessions(port: u16) -> Value {
    let mut data = angela(port).sessions_list_for_test();
    let path = project_root().join(".caper").join("conversations.db");
    if let Ok(conn) = rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY) {
        add_latest_events(&mut data, &conn);
        add_token_counts(&mut data, &conn);
        add_seat_counts(&mut data, &conn);
    }
    data
}

/// session would reach their sidebar only after an unrelated row edit.
fn add_seat_counts(data: &mut Value, db: &rusqlite::Connection) {
    let Some(rows) = data.get_mut("sessions").and_then(Value::as_array_mut) else { return; };
    let Ok(mut query) = db.prepare_cached(
        "SELECT COUNT(*) FROM participants WHERE conversation_id = ?1 AND active = 1"
    ) else { return; };
    for row in rows {
        let Some(id) = row.get("id").and_then(Value::as_str) else { continue; };
        let n: i64 = query.query_row([id], |r| r.get(0)).unwrap_or(0);
        if let Some(row) = row.as_object_mut() {
            row.insert("seats".into(), Value::from(n));
        }
    }
}

fn add_latest_events(data: &mut Value, db: &rusqlite::Connection) {
    let Some(rows) = data.get_mut("sessions").and_then(Value::as_array_mut) else { return; };
    let Ok(mut query) = db.prepare_cached(
        "SELECT ts FROM nodes WHERE conversation_id = ?1 ORDER BY seq DESC LIMIT 1"
    ) else { return; };
    for row in rows {
        let Some(id) = row.get("id").and_then(Value::as_str) else { continue; };
        let time: Option<String> = rusqlite::OptionalExtension::optional(query.query_row([id], |r| r.get(0))).ok().flatten();
        if let (Some(time), Some(row)) = (time, row.as_object_mut()) {
            row.insert("lastEventAt".into(), Value::String(time));
        }
    }
}

fn add_token_counts(data: &mut Value, db: &rusqlite::Connection) {
    let Some(rows) = data.get_mut("sessions").and_then(Value::as_array_mut) else { return; };
    let Ok(mut query) = db.prepare_cached(
        "SELECT gen_total_tokens FROM nodes WHERE conversation_id = ?1 \
         AND kind = 'gen_info' AND gen_total_tokens IS NOT NULL \
         ORDER BY seq DESC LIMIT 1"
    ) else { return; };
    for row in rows {
        let Some(id) = row.get("id").and_then(Value::as_str) else { continue; };
        let total: Option<i64> = rusqlite::OptionalExtension::optional(query.query_row([id], |r| r.get(0))).ok().flatten();
        if let (Some(total), Some(row)) = (total, row.as_object_mut()) {
            row.insert("totalTokens".into(), Value::from(total));
        }
    }
}

fn delete_empty_session(db: &rusqlite::Connection, id: &str) -> rusqlite::Result<bool> {
    db.execute_batch("PRAGMA foreign_keys = ON")?;
    Ok(db.execute(
        "DELETE FROM conversation WHERE id = ?1 AND NOT EXISTS \
         (SELECT 1 FROM nodes WHERE conversation_id = ?1 AND NOT \
          (kind = 'session' AND COALESCE(json_extract(payload, '$.kind'), '') = 'created'))", [id]
    )? == 1)
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(b) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(b);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn query_str(req: &Request, key: &str) -> String {
    let prefix = format!("{key}=");
    for pair in req.query.split('&') {
        if let Some(v) = pair.strip_prefix(&prefix) {
            return percent_decode(v);
        }
    }
    String::new()
}

/// Create a session filed under a project. `project` (explicit) or, when
/// absent, the active local project names the folder; a local project added
/// from a directory sets `projectRoot` so edits happen in its context.
/// Single local user — no owner, grant, capability, or sharing checks.
fn create_session_in_project(
    store: &angela::conv_store::ConvStore,
    root: &std::path::Path,
    body: &Value,
) -> Result<Value, String> {
    let str_field = |k: &str| body.get(k).and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty()).map(str::to_string);
    let project = str_field("project").or_else(|| crate::local_projects::active_name(root));
    let project_root_dir = str_field("projectRoot")
        .or_else(|| str_field("project_root"))
        .or_else(|| project.as_deref().and_then(|n| crate::local_projects::path_for(root, n).map(|p| p.to_string_lossy().to_string())));
    let s = store.create(angela::session_store::NewSession {
        agent: str_field("agent"),
        model: str_field("model"),
        title: str_field("title"),
        ..Default::default()
    }).map_err(|e| e.to_string())?;
    let id = s.get("id").and_then(Value::as_str).unwrap_or_default().to_string();
    let mut patch = serde_json::Map::new();
    if let Some(p) = project.filter(|p| !p.is_empty()) {
        let _ = store.create_project(&p); // a session inside it implies the folder exists
        patch.insert("project".into(), Value::String(p));
    }
    if let Some(pr) = project_root_dir.filter(|p| !p.is_empty()) { patch.insert("projectRoot".into(), Value::String(pr)); }
    if patch.is_empty() { Ok(s) } else { Ok(store.update_meta(&id, patch).unwrap_or(s)) }
}

pub fn handle_api(state: &Arc<State>, req: &Request, out: &mut Stream<'_>) {
    if req.path == "/api/upload" && req.method == "POST" {
        let name = query_str(req, "name");
        let ext = name.rsplit('.').next().unwrap_or("").to_lowercase();
        if !matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "gif" | "webp") || req.body.is_empty() {
            let _ = out.json(
                400,
                &serde_json::json!({"ok": false, "error": "images only (png/jpg/gif/webp)"}),
            );
            return;
        }
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let file = format!("{nanos:x}.{ext}");
        let dir = project_root().join(".caper").join("media");
        let _ = std::fs::create_dir_all(&dir);
        match std::fs::write(dir.join(&file), &req.body) {
            Ok(()) => {
                let _ = out.json(200, &serde_json::json!({"ok": true, "url": format!("/media/{file}")}));
            }
            Err(e) => {
                let _ = out.json(500, &serde_json::json!({"ok": false, "error": e.to_string()}));
            }
        }
        return;
    }
    if req.method == "GET" && req.path.starts_with("/media/") {
        let f = req.path.trim_start_matches("/media/");
        let path = project_root().join(".caper").join("media").join(f);
        let safe = !f.is_empty() && !f.contains('/') && !f.contains('\\') && !f.contains("..");
        let bytes = if safe { std::fs::read(&path).ok() } else { None };
        match bytes {
            Some(data) => {
                let ctype = match f.rsplit('.').next().unwrap_or("") {
                    "png" => "image/png",
                    "jpg" | "jpeg" => "image/jpeg",
                    "gif" => "image/gif",
                    "webp" => "image/webp",
                    _ => "application/octet-stream",
                };
                let _ = out.start(200, &[("content-type", ctype), ("cache-control", "public, max-age=31536000, immutable")]);
                let _ = out.chunk(&data);
            }
            None => {
                let _ = out.start(404, &[("content-type", "text/plain")]);
            }
        }
        return;
    }


    if req.path == "/api/session/status" && req.method == "POST" {
        let body: Value = serde_json::from_slice(&req.body).unwrap_or(Value::Null);
        let id = body.get("sessionId").and_then(Value::as_str).unwrap_or("");
        let text = body.get("text").and_then(Value::as_str).unwrap_or("");
        let store = &angela(state.config.port).state.store;
        if id.is_empty() || text.trim().is_empty() || store.load(id).is_none() {
            let _ = out.json(400, &serde_json::json!({"ok": false, "error": "Invalid session or status"}));
            return;
        }
        match store.append_event(id, angela::session_store::NewEvent::new("chat_status", serde_json::json!({"text": text}))) {
            Ok(event) => { let _ = out.json(200, &serde_json::json!({"ok": true, "eventId": event.event_id, "ts": event.ts})); }
            Err(_) => { let _ = out.json(500, &serde_json::json!({"ok": false, "error": "Could not save status"})); }
        }
        return;
    }
    if req.path == "/api/session/delete-if-empty" && req.method == "POST" {
        let body: Value = serde_json::from_slice(&req.body).unwrap_or(Value::Null);
        let id = body.get("id").and_then(Value::as_str).unwrap_or_default();
        if id.is_empty() || id.contains("..") || id.contains('/') || id.contains('\\') {
            let _ = out.json(400, &serde_json::json!({"ok": false}));
            return;
        }
        let result = crate::events::if_session_idle(id, || {
            let path = project_root().join(".caper").join("conversations.db");
            let db = rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE)?;
            db.busy_timeout(std::time::Duration::from_secs(2))?;
            delete_empty_session(&db, id)
        });
        match result {
            Some(Ok(true)) => {
                let server = angela(state.config.port);
                // Finish the ordinary deletion cleanup, including legacy files.
                // this path by construction: a session with no events has never
                // had a turn, and a container only exists from a turn on.
                server.state.store.delete(id);
                server.state.tabs.forget_session(id);
                server.state.cancels.forget(id);
                server.state.approvals.forget(id);
                let _ = out.json(200, &serde_json::json!({"ok": true, "deleted": true}));
            }
            Some(Err(_)) => { let _ = out.json(500, &serde_json::json!({"ok": false})); }
            _ => { let _ = out.json(200, &serde_json::json!({"ok": true, "deleted": false})); }
        }
        return;
    }
    if req.path == "/api/sessions" && req.method == "GET" {
        let body = sessions(state.config.port);
        let _ = out.json(200, &body);
        return;
    }
    if req.path == "/api/events" && req.method == "GET" {
        crate::events::stream(out);
        return;
    }
    if req.path == "/api/models" {
        let body = models();
        crate::events::publish_models(body.clone());
        let _ = out.json(200, &body);
        return;
    }
    if req.path == "/api/targets" && req.method == "GET" {
        let root = project_root();
        let mut body = crate::hostgit::everything(&root);
        let store = &angela(state.config.port).state.store;
        let (sandboxes, up) = match angela::sandbox::list_for_menu(store) {
            Ok(rows) => (rows, true),
            Err(_) => (serde_json::json!([]), false),
        };
        if let Value::Object(m) = &mut body {
            m.insert("sandboxes".into(), sandboxes);
            m.insert("sboxUp".into(), serde_json::json!(up));
        }
        let _ = out.json(200, &body);
        return;
    }
    if req.path.starts_with("/api/targets/") && req.method == "POST" {
        let root = project_root();
        let body: Value = serde_json::from_slice(&req.body).unwrap_or(Value::Null);
        let f = |k: &str| {
            body.get(k)
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string()
        };
        let flag = |k: &str, dflt: bool| body.get(k).and_then(Value::as_bool).unwrap_or(dflt);
        let action = req.path.trim_start_matches("/api/targets/").trim_matches('/');
        let result: Result<Value, String> = match action {
            "repo-add" => crate::hostgit::add_repo(&root, &f("path")),
            "repo-remove" => crate::hostgit::remove_repo(&root, &f("id")),
            "branch-add" => {
                crate::hostgit::add_branch(&root, &f("repoId"), &f("name"), &f("from"))
            }
            "worktree-add" => {
                crate::hostgit::add_worktree(&root, &f("repoId"), &f("branch"), flag("create", false))
            }
            "worktree-remove" => crate::hostgit::remove_worktree(&root, &f("id")),
            "sandbox-remove" => crate::sbox::destroy(&root, &f("id")).map(|_| serde_json::json!({})),
            other => Err(format!("unknown target action: {other}")),
        };
        match result {
            // fresh listing, so the caller never has to ask twice.
            Ok(mut v) => {
                if let Value::Object(map) = &mut v {
                    map.insert("ok".into(), serde_json::json!(true));
                    if let Value::Object(all) = crate::hostgit::everything(&root) {
                        for (k, val) in all {
                            map.insert(k, val);
                        }
                    }
                    let store = &angela(state.config.port).state.store;
                    if let Ok(rows) = angela::sandbox::list_for_menu(store) {
                        map.insert("sandboxes".into(), rows);
                    }
                }
                let _ = out.json(200, &v);
            }
            Err(e) => {
                let _ = out.json(400, &serde_json::json!({"ok": false, "error": e}));
            }
        }
        return;
    }

    if req.path == "/api/automations" && req.method == "GET" {
        let body = crate::automations::list(&project_root());
        let _ = out.json(200, &body);
        return;
    }
    if req.path.starts_with("/api/automations/") && req.method == "POST" {
        let root = project_root();
        let body: Value = serde_json::from_slice(&req.body).unwrap_or(Value::Null);
        let f = |k: &str| {
            body.get(k)
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string()
        };
        let action = req.path.trim_start_matches("/api/automations/").trim_matches('/');
        let result: Result<Value, String> = match action {
            "automation-save" => crate::automations::upsert(&root, &body).map(|id| serde_json::json!({ "id": id })),
            "automation-remove" => crate::automations::remove(&root, &f("id")).map(|_| serde_json::json!({})),
            "automation-toggle" => crate::automations::set_enabled(
                &root,
                &f("id"),
                body.get("enabled").and_then(Value::as_bool).unwrap_or(true),
            )
            .map(|_| serde_json::json!({})),
            other => Err(format!("unknown automation action: {other}")),
        };
        match result {
            // listing, so the caller never has to ask twice.
            Ok(mut v) => {
                if let Value::Object(map) = &mut v {
                    map.insert("ok".into(), serde_json::json!(true));
                    if let Value::Object(list) = crate::automations::list(&root) {
                        for (k, val) in list {
                            map.insert(k, val);
                        }
                    }
                }
                let _ = out.json(200, &v);
            }
            Err(e) => {
                let _ = out.json(400, &serde_json::json!({"ok": false, "error": e}));
            }
        }
        return;
    }

    if req.path.starts_with("/api/projects") {
        let root = project_root();
        let Ok(store) = angela::conv_store::ConvStore::new(&root) else {
            let _ = out.json(500, &serde_json::json!({"error": "no conversation store"}));
            return;
        };
        if req.method == "GET" {
            let mut body = store.projects_detail();
            // Local projects (directories on disk) and the active choice,
            // alongside named folders. No per-project sharing scoping.
            let local = crate::local_projects::projects(&root);
            if let Some(Value::Array(rows)) = body.get_mut("projects") {
                for row in rows.iter_mut() {
                    let name = row.get("name").and_then(Value::as_str).unwrap_or_default().to_string();
                    if let Some(p) = local.iter().find(|p| p.name == name) {
                        if let Some(obj) = row.as_object_mut() {
                            obj.insert("path".into(), Value::String(p.path.clone()));
                            obj.insert("local".into(), Value::Bool(true));
                        }
                    }
                }
            }
            match crate::local_projects::active_name(&root) {
                Some(name) => body["activeProject"] = Value::String(name),
                None => body["activeProject"] = Value::Null,
            }
            let _ = out.json(200, &body);
            return;
        }
        if req.method != "POST" {
            let _ = out.json(405, &serde_json::json!({"error": "use GET or POST"}));
            return;
        }
        let body: Value = serde_json::from_slice(&req.body).unwrap_or(Value::Null);
        let field = |k: &str| {
            body.get(k)
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string()
        };
        let action = req
            .path
            .trim_start_matches("/api/projects")
            .trim_matches('/')
            .to_string();
        let result: Result<Value, String> = match action.as_str() {
            "" | "create" => {
                // A named folder, or — with `path` — a local project/repo
                // rooted at an existing local directory (single local user;
                // no sharing scoping). The named folder is registered either
                // way, so sessions can be filed under it.
                let raw_path = field("path");
                if raw_path.trim().is_empty() {
                    store.create_project(&field("name")).map(|n| {
                        serde_json::json!({"name": n, "select": n, "selectKind": "project"})
                    })
                } else {
                    crate::local_projects::add(&root, &field("name"), &raw_path).and_then(|p| {
                        store.create_project(&p.name)?;
                        Ok(serde_json::json!({
                            "name": p.name, "path": p.path,
                            "select": p.name, "selectKind": "project",
                        }))
                    })
                }
            }
            // Choose the active project; sessions created while it is active
            // are created inside it. An empty name clears the choice.
            "select" => {
                let name = { let a = field("name"); if a.trim().is_empty() { field("project") } else { a } };
                crate::local_projects::set_active(&root, &name).map(|p| {
                    if let Some(p) = &p { let _ = store.create_project(&p.name); }
                    match p {
                        Some(p) => serde_json::json!({
                            "activeProject": p.name, "name": p.name,
                            "path": p.path, "select": p.name, "selectKind": "project",
                        }),
                        None => serde_json::json!({"activeProject": Value::Null}),
                    }
                })
            }
            "rename" => store
                .rename_project(&field("from"), &field("to"))
                .map(|n| serde_json::json!({"name": n})),
            "delete" => store
                .delete_project(&field("name"))
                .map(|n| serde_json::json!({"unfiled": n})),
            "assign" => store
                .assign_project(&field("sessionId"), &field("project"))
                .map(|n| serde_json::json!({"project": n})),
            other => Err(format!("unknown project action: {other}")),
        };
        match result {
            Ok(mut v) => {
                if let Value::Object(map) = &mut v {
                    map.insert("ok".into(), serde_json::json!(true));
                    if let Some(Value::Array(list)) = store.projects_detail().get("projects") {
                        map.insert("projects".into(), Value::Array(list.clone()));
                    }
                }
                let _ = out.json(200, &v);
            }
            Err(e) => {
                let _ = out.json(400, &serde_json::json!({"ok": false, "error": e}));
            }
        }
        return;
    }

    if req.path == "/api/composer-settings" && req.method == "POST" {
        let payload: Value = serde_json::from_slice(&req.body).unwrap_or(Value::Null);
        let id = payload.get("sessionId").and_then(Value::as_str).unwrap_or("");
        let root = project_root();
        let result = angela::conv_store::ConvStore::new(root).ok().and_then(|store| {
            let old = store.load(id)?;
            let mut patch = serde_json::Map::new();
            for key in [
                "model",
                "allowlist",
                "allowlistMode",
                "allowlistSource",
                "project",
                "worktreeId",
                "branchId",
            ] {
                if let Some(value) = payload.get(key) { patch.insert(key.into(), value.clone()); }
            }
            if let Some(reasoning) = payload.get("reasoningEffort") {
                let mut config = old.get("config").and_then(Value::as_object).cloned().unwrap_or_default();
                config.insert("reasoningEffort".into(), reasoning.clone());
                patch.insert("config".into(), Value::Object(config));
            }
            store.update_meta(id, patch)
        });
        let (status, body) = match result {
            Some(session) => (200, serde_json::json!({"ok": true, "session": session})),
            None => (404, serde_json::json!({"ok": false, "error": "session not found"})),
        };
        let _ = out.json(status, &body);
        return;
    }
    // Ticket 04: a session created inside a project lands in it. The active
    // local project (or an explicit `project`) names the folder the session
    // is filed under, and — for a local project added from a directory —
    // `projectRoot` points session edits at that directory. Nothing here
    // adds user scoping or sharing.
    if req.path == "/api/session/new" && req.method == "POST" {
        let body: Value = serde_json::from_slice(&req.body).unwrap_or_else(|_| serde_json::json!({}));
        let server = angela(state.config.port);
        match create_session_in_project(&server.state.store, &project_root(), &body) {
            Ok(session) => { let _ = out.json(200, &serde_json::json!({ "ok": true, "session": session })); }
            Err(e) => { let _ = out.json(500, &serde_json::json!({ "error": e })); }
        }
        return;
    }
    let areq = ARequest {
        method: req.method.clone(),
        path: req.path.clone(),
        query: req.query.clone(),
        headers: req
            .headers
            .iter()
            .map(|(k, v)| (k.to_ascii_lowercase(), v.clone()))
            .collect(),
        body: req.body.clone(),
    };
    let chat = req.method == "POST" && req.path == "/api/chat";
    let body: Value = if chat { serde_json::from_slice(&req.body).unwrap_or(Value::Null) } else { Value::Null };
    let session_id = body.get("sessionId").and_then(Value::as_str).unwrap_or_default().to_string();
    let prompt = body.get("content").and_then(Value::as_str).unwrap_or_default().to_string();
    let baseline = if chat && !session_id.is_empty() {
        chat_messages(&angela(state.config.port).state.store, &session_id)
    } else { Vec::new() };
    let mut sink = DevSink {
        out, streaming: false, ended: false, chat, session_id, prompt, baseline,
        turn_id: None, disconnected: false, port: state.config.port,
        assistant_filter: crate::assistant_text::StreamFilter::default(),
    };
    angela(state.config.port).dispatch(&areq, &mut sink);
    sink.end();
}

#[cfg(test)]
mod latest_event_tests {
    use super::*;

    #[test]
    fn roster_statuses_survive_reconstruction_in_branch_order() {
        use angela::session_store::{NewEvent, NewSession};
        let store = angela::conv_store::ConvStore::memory("/tmp/chat-status-test").unwrap();
        let id = store.create(NewSession::default()).unwrap()["id"].as_str().unwrap().to_string();
        let status = store.append_event(&id, NewEvent::new("chat_status", serde_json::json!({"text":"Invited @agent"}))).unwrap();
        store.append_event(&id, NewEvent::new("context_window", serde_json::json!({"role":"user", "content":"hi"}))).unwrap();
        store.append_event(&id, NewEvent::new("context_window", serde_json::json!({"role":"assistant", "content":"hello"}))).unwrap();
        store.append_event(&id, NewEvent::new("chat_status", serde_json::json!({"text":"Removed @agent"}))).unwrap();
        let rows = chat_messages(&store, &id);
        assert_eq!(rows.iter().map(|r| r["text"].as_str().unwrap_or("")).collect::<Vec<_>>(), ["Invited @agent", "hi", "hello", "Removed @agent"]);
        assert_eq!(rows[0]["eventId"], status.event_id);
        assert_eq!(rows[0]["ts"], status.ts);
        assert_eq!(rows[0]["commandNotice"], true);
    }

    #[test]
    fn only_sessions_without_events_are_deleted() {
        let db = rusqlite::Connection::open_in_memory().unwrap();
        db.execute_batch("CREATE TABLE conversation (id TEXT PRIMARY KEY); \
            CREATE TABLE nodes (conversation_id TEXT REFERENCES conversation(id) ON DELETE CASCADE, seq INTEGER, kind TEXT, payload TEXT); \
            INSERT INTO conversation VALUES ('empty'), ('with-events'); \
            INSERT INTO nodes VALUES ('empty', 1, 'session', '{\"kind\":\"created\"}'), \
            ('with-events', 1, 'session', '{}');").unwrap();
        assert!(delete_empty_session(&db, "empty").unwrap());
        assert!(!delete_empty_session(&db, "with-events").unwrap());
        assert!(!delete_empty_session(&db, "missing").unwrap());
        assert_eq!(db.query_row("SELECT COUNT(*) FROM conversation", [], |r| r.get::<_, i64>(0)).unwrap(), 1);
    }

    #[test]
    fn latest_event_is_taken_from_the_latest_sequence_not_metadata() {
        let db = rusqlite::Connection::open_in_memory().unwrap();
        db.execute_batch("CREATE TABLE nodes (conversation_id TEXT, seq INTEGER, ts TEXT);
            INSERT INTO nodes VALUES ('a', 1, '2020-01-01T00:00:00Z');
            INSERT INTO nodes VALUES ('a', 2, '2020-02-01T00:00:00Z');
            INSERT INTO nodes VALUES ('b', 1, '2020-03-01T00:00:00Z');").unwrap();
        let mut sessions = serde_json::json!({"sessions": [
            {"id":"a", "updatedAt":"2025-01-01T00:00:00Z"},
            {"id":"b"}, {"id":"empty"}
        ]});
        add_latest_events(&mut sessions, &db);
        assert_eq!(sessions["sessions"][0]["lastEventAt"], "2020-02-01T00:00:00Z");
        assert_eq!(sessions["sessions"][1]["lastEventAt"], "2020-03-01T00:00:00Z");
        assert!(sessions["sessions"][2].get("lastEventAt").is_none());
    }

    #[test]
    fn token_counts_are_taken_from_the_newest_generation() {
        let db = rusqlite::Connection::open_in_memory().unwrap();
        db.execute_batch("CREATE TABLE nodes (conversation_id TEXT, seq INTEGER, kind TEXT, gen_total_tokens INTEGER);
            INSERT INTO nodes VALUES ('a', 1, 'gen_info', 1200);
            INSERT INTO nodes VALUES ('a', 2, 'gen_info', 4800);
            INSERT INTO nodes VALUES ('a', 3, 'tool_result', NULL);
            INSERT INTO nodes VALUES ('b', 1, 'session', NULL);").unwrap();
        let mut sessions = serde_json::json!({"sessions": [{"id":"a"}, {"id":"b"}]});
        add_token_counts(&mut sessions, &db);
        assert_eq!(sessions["sessions"][0]["totalTokens"], 4800);
        assert!(sessions["sessions"][1].get("totalTokens").is_none());
    }
}

/// Ticket 02: session CRUD is single-user — no login, owner, grant,
/// capability, admin, or sharing checks; delete removes the session and
/// everything hanging off it.
#[cfg(test)]
mod session_crud_tests {
    use super::*;
    use std::io::{Read, Write};

    fn temp_root(tag: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("caper-session-crud-{tag}-{nanos}"));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// One request through the real handler; returns (status, body).
    fn call(state: &Arc<State>, method: &str, path: &str, body: &str) -> (u16, Value) {
        let req = Request {
            method: method.into(),
            path: path.into(),
            query: String::new(),
            body: body.as_bytes().to_vec(),
            ..Default::default()
        };
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let mut client = std::net::TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let mut response = Vec::new();
        {
            let (mut server_side, _) = listener.accept().unwrap();
            let mut stream = Stream::new(&mut server_side);
            handle_api(state, &req, &mut stream);
        }
        client.read_to_end(&mut response).unwrap();
        client.flush().ok();
        let text = String::from_utf8_lossy(&response);
        let status = text
            .split_whitespace()
            .nth(1)
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let payload = text.split_once("\r\n\r\n").map(|(_, rest)| rest).unwrap_or("");
        // Chunked framing: drop the size lines and the 0-length terminator.
        let body = payload.split("\r\n0").next().unwrap_or(payload);
        let body = body.split_once("\r\n").map(|(_, rest)| rest).unwrap_or(body);
        (status, serde_json::from_str(body).unwrap_or(Value::Null))
    }

    fn session_count(state: &Arc<State>) -> usize {
        call(state, "GET", "/api/sessions", "")
            .1
            .get("sessions")
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or(usize::MAX)
    }

    #[test]
    fn session_crud_needs_no_identity_and_delete_removes_everything() {
        let root = temp_root("crud");
        std::env::set_var("CAPER_ROOT", &root);
        let config = rustex1_devserver::Config::discover(&root);
        let state = Arc::new(State::new(config, std::collections::HashMap::new()));

        // Create two sessions; no credentials, owner, grant, or capability sent.
        let first = call(&state, "POST", "/api/session/new", "{}");
        assert_eq!(first.0, 200, "create failed: {first:?}");
        let id = first.1["session"]["id"].as_str().unwrap().to_string();
        let second = call(&state, "POST", "/api/session/new", r#"{"title":"other"}"#);
        assert_eq!(second.0, 200);
        let other = second.1["session"]["id"].as_str().unwrap().to_string();

        // List shows both; each session holds its own data.
        assert_eq!(session_count(&state), 2);
        // Append an event to `id`, leave `other` empty.
        let appended = call(
            &state,
            "POST",
            "/api/session/status",
            &format!(r#"{{"sessionId":"{id}","text":"hello thread"}}"#),
        );
        assert_eq!(appended.0, 200);
        let own = call(&state, "GET", &format!("/api/sessions/{id}"), "");
        assert_eq!(own.0, 200);
        assert_eq!(own.1["messages"].as_array().unwrap().len(), 1);
        assert_eq!(call(&state, "GET", &format!("/api/sessions/{other}"), "").1["messages"].as_array().unwrap().len(), 0);

        // Switch active session: read the other one; cursor moves within `id`.
        assert_eq!(call(&state, "GET", &format!("/api/sessions/{other}"), "").0, 200);
        let switched = call(&state, "POST", &format!("/api/sessions/{id}/cursor"), r#"{"eventId":null}"#);
        assert_eq!(switched.0, 200);

        // Legacy session files for `id`, if any, must go too.
        let legacy = root.join(".caper").join("sessions");
        std::fs::create_dir_all(&legacy).unwrap();
        std::fs::write(legacy.join(format!("{id}.json")), "{}").unwrap();
        std::fs::write(legacy.join(format!("{id}.jsonl")), "").unwrap();

        // Delete `id`: 404 afterwards, list count decrements, nothing hanging off it.
        assert_eq!(call(&state, "DELETE", &format!("/api/sessions/{id}"), "").0, 200);
        assert_eq!(call(&state, "GET", &format!("/api/sessions/{id}"), "").0, 404);
        assert_eq!(session_count(&state), 1);
        assert!(!legacy.join(format!("{id}.json")).exists());
        assert!(!legacy.join(format!("{id}.jsonl")).exists());
        let db = rusqlite::Connection::open(root.join(".caper").join("conversations.db")).unwrap();
        for (table, key) in [("conversation", "id"), ("nodes", "conversation_id"), ("participants", "conversation_id"), ("cursor", "conversation_id"), ("memory_leaves", "conversation_id")] {
            let n: i64 = db.query_row(&format!("SELECT COUNT(*) FROM {table} WHERE {key} = ?1"), [&id], |r| r.get(0)).unwrap();
            assert_eq!(n, 0, "{table} still holds rows for deleted session");
        }
        let _ = std::fs::remove_dir_all(&root);
    }
}

/// Ticket 04: sessions created inside the active local project land in it;
/// edits use the selected project context. Single local user, no sharing.
#[cfg(test)]
mod local_projects_tests {
    use super::*;

    #[test]
    fn session_created_inside_selected_project_uses_its_context() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("caper-session-in-project-{nanos}"));
        let repo = std::env::temp_dir().join(format!("caper-session-repo-{nanos}"));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&repo).unwrap();
        let store = angela::conv_store::ConvStore::memory(&root).unwrap();

        // No active project -> session lands nowhere in particular, root unchanged.
        let plain = create_session_in_project(&store, &root, &serde_json::json!({})).unwrap();
        assert_eq!(plain["project"], "");
        assert_eq!(plain["projectRoot"].as_str().unwrap(), store.project_root.to_string_lossy());

        // Add a local project and choose it; created sessions land inside it
        // with `projectRoot` at the selected directory.
        let added = crate::local_projects::add(&root, "", &repo.to_string_lossy()).unwrap();
        store.create_project(&added.name).unwrap();
        crate::local_projects::set_active(&root, &added.name).unwrap();

        let one = create_session_in_project(&store, &root, &serde_json::json!({})).unwrap();
        let two = create_session_in_project(&store, &root, &serde_json::json!({})).unwrap();
        assert_eq!(one["project"], added.name.as_str());
        assert_eq!(two["project"], added.name.as_str());
        assert_ne!(one["id"], two["id"]);
        let edit_root = one["projectRoot"].as_str().unwrap();
        assert!(std::path::Path::new(edit_root).is_dir());
        assert_eq!(std::path::Path::new(edit_root).file_name().unwrap().to_string_lossy(), repo.file_name().unwrap().to_string_lossy());

        // An explicit `project` on the request overrides the active one.
        let other = create_session_in_project(&store, &root, &serde_json::json!({"project": "elsewhere"})).unwrap();
        assert_eq!(other["project"], "elsewhere");
        // ...and doesn't disturb the persisted active selection.
        assert_eq!(crate::local_projects::active_name(&root).as_deref(), Some(added.name.as_str()));

        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&repo);
    }
}