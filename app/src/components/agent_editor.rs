//! `soul`, `goals` and `memory` are three boxes rather than one, because they
//! because it is the field that decides whether the agent ever speaks, and a

use rustex1_kit::h::{lit, m, Node};

use crate::css;
use crate::components::{dropdown, validation_info};

pub const CSS: &str = r#"
:is(#sb, #home) .agent-editor {
  display: flex; flex-direction: column; gap: 14px;
  font-size: 12px; color: var(--gray);
}
:is(#sb, #home) .agent-editor-head {
  display: flex; align-items: center; gap: 8px;
}
:is(#sb, #home) .agent-editor-title {
  flex: 1; min-width: 0; font-size: 13px; color: var(--fg);
  overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
}
:is(#sb, #home) .ae-field { display: flex; flex-direction: column; gap: 5px; }
:is(#sb, #home) .ae-label {
  font-size: 11px; color: var(--gray-dim); line-height: 16px;

  display: contents;
}
:is(#sb, #home) .ae-label .hint {
  order: 1; display: block; margin-top: 2px;
  color: var(--gray-dim); opacity: .7; font-size: 10px;
}
:is(#sb, #home) .ae-field input,
:is(#sb, #home) .ae-field textarea {
  width: 100%; box-sizing: border-box;
  background: var(--panel-2); color: var(--fg);
  border: 1px solid transparent; border-radius: 8px;
  padding: 7px 9px; font: inherit; line-height: 1.5;
  resize: vertical;
}
:is(#sb, #home) .ae-field textarea { min-height: 64px; font-size: 12px; }
:is(#sb, #home) .ae-mcp-presets { margin: -9px 0 0; color: var(--gray-dim); font-size: 10px; line-height: 1.5; }

:is(#sb, #home) .ae-field input.ae-num { width: 7em; }
:is(#sb, #home) .ae-field input.ae-num::-webkit-outer-spin-button,
:is(#sb, #home) .ae-field input.ae-num::-webkit-inner-spin-button {
  -webkit-appearance: none; margin: 0;
}
:is(#sb, #home) .ae-field input.ae-num { -moz-appearance: textfield; appearance: textfield; }
:is(#sb, #home) .ae-field input:focus,
:is(#sb, #home) .ae-field textarea:focus {
  outline: none; border-color: var(--gray-dim);
}
:is(#sb, #home) .ae-field input::placeholder,
:is(#sb, #home) .ae-field textarea::placeholder { color: var(--gray-dim); }


:is(#sb, #home) .ae-modes { display: flex; gap: 4px; }
:is(#sb, #home) .ae-mode {
  flex: 1; display: inline-flex; align-items: center; justify-content: center; gap: 5px;
  border: 0; border-radius: 7px; background: var(--panel-2); color: var(--gray-dim);
  padding: 6px 4px; cursor: pointer; font: inherit; font-size: 11px; line-height: 18px;
}
:is(#sb, #home) .ae-mode:hover { color: var(--fg); }
:is(#sb, #home) .ae-mode.is-on { color: var(--accent); }
:is(#sb, #home) .ae-mode i { line-height: 18px; }
:is(#sb, #home) .ae-mode-note {
  font-size: 11px; color: var(--gray-dim); line-height: 1.5; min-height: 16px;
}

:is(#sb, #home) .ae-color { display: flex; gap: 8px; align-items: center; }
:is(#sb, #home) .ae-color input[type="color"] {
  width: 38px; height: 30px; padding: 0; flex: 0 0 auto;
  background: var(--panel-2); border: 1px solid transparent; border-radius: 8px;
  cursor: pointer;
}
:is(#sb, #home) .ae-color input[type="color"]::-webkit-color-swatch-wrapper { padding: 3px; }
:is(#sb, #home) .ae-color input[type="color"]::-webkit-color-swatch {
  border: 0; border-radius: 5px;
}
:is(#sb, #home) .ae-color-hex { flex: 1; font-family: var(--mono, monospace); }


:is(#sb, #home) .ae-icon { display: flex; gap: 8px; align-items: center; }
:is(#sb, #home) .ae-icon .ae-icon-preview {
  width: 38px; height: 30px; flex: 0 0 auto;
  display: inline-flex; align-items: center; justify-content: center;
  background: var(--panel-2); border-radius: 8px;

  color: var(--agent-accent, var(--accent));
  font-size: 17px; line-height: 1;
}
:is(#sb, #home) .ae-icon input { flex: 1; font-family: var(--mono, monospace); }

:is(#sb, #home) .ae-actions {
  display: flex; align-items: center; gap: 8px; padding-top: 2px;
}
:is(#sb, #home) .ae-actions .spacer { flex: 1; }
:is(#sb, #home) .ae-btn {
  display: inline-flex; align-items: center; gap: 5px;
  border: 0; border-radius: 7px; background: var(--panel-2); color: var(--gray);
  padding: 6px 11px; cursor: pointer; font: inherit; font-size: 12px; line-height: 18px;
}
:is(#sb, #home) .ae-btn:hover { color: var(--fg); }
:is(#sb, #home) .ae-btn.primary { color: var(--accent); }
:is(#sb, #home) .ae-btn.danger:hover { color: var(--danger, #d9534f); }
:is(#sb, #home) .ae-btn.is-confirm { color: var(--danger, #d9534f); }
:is(#sb, #home) .ae-btn[disabled] { opacity: .5; cursor: default; }


"#;

const ST: &str = "$store.agents";

fn field(label: &str, hint: &str, model: &str, placeholder: &str, rows: usize) -> Node {
    let input = if rows == 0 {
        m("input")
            .attr("type", "text")
            .attr("placeholder", placeholder)
            .model(model)
    } else {
        m("textarea")
            .attr("rows", &rows.to_string())
            .attr("placeholder", placeholder)
            .model(model)
    };
    m("div").class("ae-field").kids(vec![
        m("label").class("ae-label").kids(vec![
            m("span").child(lit(label)),
            m("span").class("hint").child(lit(hint)),
        ]),
        input,
    ])
}

fn mode(value: &str, label: &str, icon: &str) -> Node {
    m("button")
        .class("ae-mode")
        .attr("type", "button")
        .bind(
            "class",
            &format!("({ST}.draft.engagement === '{value}' ? 'is-on' : '')"),
        )
        .on("click", &format!("{ST}.setEngagement('{value}')"))
        .kids(vec![
            m("i").class(&format!("ph {icon}")),
            m("span").child(lit(label)),
        ])
}

pub fn agent_editor() -> Node {
    m("div")
        .class("agent-editor")
        .show(&format!("{ST}.open"))
        .kids(vec![
            m("div").class("agent-editor-head").kids(vec![
                Node::icon_dyn(&format!("({ST}.draft.icon || 'robot')")),
                m("span")
                    .class("agent-editor-title")
                    .text(&format!(
                        "({ST}.creating ? 'New agent' : ({ST}.draft.name || 'Agent'))"
                    )),
            ]),
            validation_info::live(
                &format!("({ST}.error ? 'error' : 'ok')"),
                &format!("({ST}.error ? {ST}.error : {ST}.saved)"),
            ),
            field("Name", "the @handle", &format!("{ST}.draft.name"), "scout", 0),
            field(
                "Description",
                "what it is for",
                &format!("{ST}.draft.description"),
                "an ops engineer who reads deploy logs",
                2,
            ),

            m("div").class("ae-field").kids(vec![
                m("label").class("ae-label").kids(vec![
                    m("span").child(lit("Engagement")),
                    m("span").class("hint").child(lit("when it speaks")),
                ]),
                m("div").class("ae-modes").kids(vec![
                    mode("invoked", "Invoked", "ph-at"),
                    mode("relevant", "Relevant", "ph-funnel"),
                    mode("engaged", "Engaged", "ph-chats-circle"),
                ]),
                // Spelled out, because "relevant" does not say what happens.
                m("p")
                    .class("ae-mode-note")
                    .text(&format!("{ST}.engagementLabel({ST}.draft.engagement)")),
            ]),

            field(
                "Soul",
                "who it is",
                &format!("{ST}.draft.soul"),
                "You are Scout, a terse ops engineer.",
                5,
            ),
            field(
                "Goals",
                "what it is working toward",
                &format!("{ST}.draft.goals"),
                "Keep deploys healthy.",
                3,
            ),
            field(
                "Memory",
                "what it has learned — it may rewrite this",
                &format!("{ST}.draft.memory"),
                "(empty)",
                3,
            ),

            m("div").class("ae-field").kids(vec![
                m("label").class("ae-label").kids(vec![
                    m("span").child(lit("Icon")),
                    m("span")
                        .class("hint")
                        .child(lit("a phosphor name; blank = robot")),
                ]),
                m("div")
                    .class("ae-icon")
                    .bind(
                        "style",
                        &format!(
                            "('--agent-accent: ' + ({ST}.draft.accentColor || '#a78bfa'))"
                        ),
                    )
                    .kids(vec![
                        // does not have draws nothing, which is the only
                        m("span")
                            .class("ae-icon-preview")
                            .child(Node::icon_dyn(&format!(
                                "({ST}.draft.icon || 'robot')"
                            ))),
                        m("input")
                            .attr("type", "text")
                            .attr("spellcheck", "false")
                            .attr("placeholder", "robot")
                            .attr("title", "phosphoricons.com — the name under the glyph")
                            .model(&format!("{ST}.draft.icon")),
                    ]),
            ]),

            m("div").class("ae-field").kids(vec![
                m("label").class("ae-label").kids(vec![
                    m("span").child(lit("Accent colour")),
                    m("span").class("hint").child(lit("how it is drawn")),
                ]),
                m("div").class("ae-color").kids(vec![
                    m("input")
                        .attr("type", "color")
                        .model(&format!("{ST}.draft.accentColor")),
                    // The hex beside the swatch, because a swatch alone
                    m("input")
                        .attr("type", "text")
                        .class("ae-color-hex")
                        .attr("spellcheck", "false")
                        .model(&format!("{ST}.draft.accentColor")),
                ]),
            ]),

            m("div").class("ae-field").kids(vec![
                m("label").class("ae-label").child(lit("Model")),
                dropdown::agent_option("model", "cpu"),
            ]),
            m("div").class("ae-field").kids(vec![
                m("label").class("ae-label").child(lit("Reasoning")),
                dropdown::agent_option("reasoningEffort", "brain"),
            ]),
            m("div").class("ae-field").kids(vec![
                m("label").class("ae-label").kids(vec![
                    m("span").child(lit("Max turns")),
                    m("span")
                        .class("hint")
                        .child(lit("model round-trips per reply; blank = 5")),
                ]),
                m("input")
                    .class("ae-num")
                    .attr("type", "number")
                    .attr("min", "1")
                    .attr("max", "1000")
                    .attr("step", "1")
                    // does, which is why this is per-agent.
                    .attr("placeholder", "5")
                    .model(&format!("{ST}.draft.maxTurns")),
            ]),
            field(
                "Workspace root",
                "file-io and the shell start here; blank = project root \
                (host-only, no effect on sandboxes)",
                &format!("{ST}.draft.cwd"),
                "/workspace/blz",
                0,
            ),
            field(
                "Containerfile",
                "absolute host path; empty = runs on the host",
                &format!("{ST}.draft.containerFile"),
                "/workspace/models/sbox/video/Containerfile",
                0,
            ),
            field(
                "MCP servers",
                "configured names only, one per line",
                &format!("{ST}.draft.mcp"),
                "",
                2,
            ),
            m("p")
                .class("ae-mcp-presets")
                .text(&format!("({ST}.mcpPresets.length ? 'Available: ' + {ST}.mcpPresets.join(', ') : 'No MCP servers are configured')")),
            field(
                "Allowlist",
                "auto-approved tools, one per line. Empty approves nothing",
                &format!("{ST}.draft.allowlist"),
                "file_io__read_file",
                4,
            ),

            m("div").class("ae-actions").kids(vec![
                m("button")
                    .class("ae-btn primary")
                    .attr("type", "button")
                    .bind("disabled", &format!("{ST}.saving"))
                    .on("click", &format!("{ST}.save()"))
                    .kids(vec![
                        Node::icon("check").class("ph ph-check"),
                        m("span").text(&format!("({ST}.saving ? 'Saving…' : 'Save')")),
                    ]),
                m("span").class("spacer"),
                m("button")
                    .class("ae-btn danger")
                    .attr("type", "button")
                    .show(&format!("!{ST}.creating"))
                    .bind("class", &format!("({ST}.confirmDelete ? 'is-confirm' : '')"))
                    .on("click", &format!("{ST}.remove()"))
                    .kids(vec![
                        Node::icon("trash").class("ph ph-trash"),
                        m("span").text(&format!(
                            "({ST}.confirmDelete ? 'Really delete?' : 'Delete')"
                        )),
                    ]),
            ]),
        ])
}

pub fn settings_panel() -> Node {
    let st = "$store.settings";
    m("div")
        .class("agent-editor settings-panel")
        .show(&format!("{st}.open"))
        .init(&format!("{st}.load()"))
        .kids(vec![
            m("div").class("agent-editor-head").kids(vec![
                Node::icon("gear").class("ph ph-gear"),
                m("span").class("agent-editor-title").child(lit("Settings")),
            ]),
        ])
}

pub fn register() {
    css::inject("agent-editor", CSS);
}
