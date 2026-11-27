//! opencode — delegate a turn to the local `opencode` CLI.
//!
//! OpenCode's free models are gated to the OpenCode client itself: the same
//! request sent straight to the inference API comes back `429
//! FreeUsageLimitError`. Driving the CLI is therefore the supported path, and
//! it also brings the CLI's own tools and provider config for free.
//!
//! The CLI is spawned per turn with the rendered conversation on stdin and
//! `--format json` on stdout. Text parts arrive whole rather than token by
//! token, so the caller emits each part as one delta.

use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::OnceLock;

use serde_json::Value;

use crate::provider::Message;

/// `opencode` is a `.cmd` shim on Windows, which `CreateProcess` will not
/// resolve on its own, so it goes through `cmd /C`.
fn command() -> Command {
    #[cfg(windows)]
    {
        let mut c = Command::new("cmd");
        c.args(["/C", "opencode"]);
        c
    }
    #[cfg(not(windows))]
    {
        Command::new("opencode")
    }
}

/// Run one turn. `model` is the bare id after the `opencode:` prefix.
pub async fn run(model: &str, messages: &[Message]) -> Result<String, String> {
    let prompt = render(messages);
    let model = model.to_string();
    tokio::task::spawn_blocking(move || run_blocking(&model, &prompt))
        .await
        .map_err(|e| format!("opencode task failed: {e}"))?
}

fn run_blocking(model: &str, prompt: &str) -> Result<String, String> {
    // `cmd /C` is a shell boundary on Windows, so the model id must not carry
    // shell metacharacters.
    if !valid_model(model) {
        return Err(format!("invalid opencode model id: {model}"));
    }
    let mut child = command()
        .args(["run", "--archived", "--format", "json", "-m", &format!("opencode/{model}")])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("could not start `opencode`: {e}"))?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(prompt.as_bytes()).map_err(|e| e.to_string())?;
    }
    let out = child.wait_with_output().map_err(|e| e.to_string())?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        return Err(format!("opencode failed: {}", err.trim()));
    }
    let mut text = String::new();
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        let Ok(event) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if event.get("type").and_then(Value::as_str) == Some("text") {
            if let Some(t) = event.pointer("/part/text").and_then(Value::as_str) {
                text.push_str(t);
            }
        }
    }
    Ok(text)
}

/// The models the installed CLI offers, bare ids (`deepseek-v4.1-flash-free`).
/// Cached for the process: the call is a subprocess and `/api/models` is hot.
pub fn models() -> Vec<String> {
    static CACHE: OnceLock<Vec<String>> = OnceLock::new();
    CACHE
        .get_or_init(|| {
            let Ok(out) = command().arg("models").output() else {
                return Vec::new();
            };
            if !out.status.success() {
                return Vec::new();
            }
            String::from_utf8_lossy(&out.stdout)
                .lines()
                .filter_map(|line| line.trim().strip_prefix("opencode/"))
                .map(|id| id.to_string())
                .collect()
        })
        .clone()
}

/// Model ids come from config, so they reach a shell on Windows.
fn valid_model(model: &str) -> bool {
    model.chars().all(|c| c.is_ascii_alphanumeric() || "._/-".contains(c))
}

/// Flatten the provider-facing messages into one prompt. The CLI keeps its own
/// agent loop, so this is the transcript, not an OpenAI request body.
fn render(messages: &[Message]) -> String {
    let mut out = String::new();
    for m in messages {
        let text = match &m.content {
            Some(Value::String(s)) => s.clone(),
            Some(v) => v.to_string(),
            None => String::new(),
        };
        if text.trim().is_empty() {
            continue;
        }
        out.push_str(&format!("[{}]\n{}\n\n", m.role, text));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_keeps_roles_and_skips_empty_content() {
        let messages = vec![
            Message::system("sys"),
            Message::user("hi"),
            Message {
                role: "assistant".into(),
                content: None,
                tool_calls: None,
                tool_call_id: None,
                reasoning_content: None,
            },
        ];
        assert_eq!(render(&messages), "[system]\nsys\n\n[user]\nhi\n\n");
    }

    #[test]
    fn model_ids_are_restricted_to_safe_characters() {
        assert!(valid_model("deepseek-v4.1-flash-free"));
        assert!(valid_model("anthropic/claude-sonnet-4-5"));
        assert!(!valid_model("x & calc"));
        assert!(!valid_model("x\"y"));
    }
}
