//! and the nesting is only ever visual.

use rustex1_kit::h::{lit, m, Node};

use crate::css;

pub const CSS: &str = r#"
:is(#sb, #home) .thread-row {
  display: flex; gap: 8px; align-items: flex-start;

  padding-left: calc(var(--d, 0) * 20px);
  margin-bottom: 6px;
}
:is(#sb, #home) .thread-card {
  flex: 1; min-width: 0;
  display: flex; gap: 10px; align-items: flex-start;
  padding: 8px 12px;
  border-radius: 10px;
  background: var(--panel-2);
  /* The rule on the left is the "there is more under this" cue, and it is
     the only thing distinguishing a thread from a message card. */
  border-left: 2px solid var(--gray-dim);
  cursor: pointer;
  text-align: left;
  color: inherit;
  font: inherit;
  width: 100%;
}
:is(#sb, #home) .thread-card:hover { border-left-color: var(--accent); }
:is(#sb, #home) .thread-card.is-open { border-left-color: var(--accent); }
:is(#sb, #home) .thread-caret {
  flex: 0 0 auto; line-height: 18px; color: var(--gray-dim);
  transition: transform 120ms ease;
}
:is(#sb, #home) .thread-card.is-open .thread-caret { transform: rotate(90deg); }
:is(#sb, #home) .thread-body { flex: 1; min-width: 0; }
:is(#sb, #home) .thread-meta {
  display: flex; gap: 8px; align-items: center;
  font-size: 11px; color: var(--gray-dim); margin-bottom: 2px;
}
:is(#sb, #home) .thread-count { font-variant-numeric: tabular-nums; }
:is(#sb, #home) .thread-ref { opacity: .6; font-family: var(--mono, monospace); }
/* A clipped line is a summary the model never managed to shorten, so the
   source was truncated instead. Worth saying, quietly. */
:is(#sb, #home) .thread-clipped { color: var(--warn, #c9a227); }
:is(#sb, #home) .thread-text { font-size: 13px; line-height: 1.5; }
:is(#sb, #home) .thread-text.unbuilt { color: var(--gray-dim); font-style: italic; }
:is(#sb, #home) .thread-spinner { font-size: 11px; color: var(--gray-dim); }


:is(#sb, #home) .memory-bar {
  display: flex; justify-content: flex-end; align-items: center;
  padding: 6px 10px; margin-bottom: 8px;
  font-size: 11px; color: var(--gray-dim);
}
:is(#sb, #home) .memory-bar button {
  display: inline-flex; gap: 5px; align-items: center;
  background: var(--panel-2); color: var(--gray);
  border: 0; border-radius: 7px; padding: 4px 9px;
  font: inherit; cursor: pointer; line-height: 18px;
}
:is(#sb, #home) .memory-bar button:hover { color: var(--fg); }
"#;

pub fn memory_bar(st: &str) -> Node {
    m("div")
        .class("memory-bar")
        .show(&format!("$store.mode.advanced && {st}.activeId"))
        .kids(vec![
            // looking at, so it is asked for, never done underneath them.
            m("button")
                .on("click", &format!("{st}.compact()"))
                .bind("disabled", &format!("{st}.compacting"))
                .attr("title", "Summarise older messages into threads")
                .kids(vec![
                    Node::icon("stack").class("ph ph-stack"),
                    m("span").text(&format!(
                        "({st}.compacting ? 'Compacting…' : 'Compact')"
                    )),
                ]),
        ])
}

pub fn row(st: &str) -> Node {
    m("button")
        .bind("class", "('thread-card ' + (t.expanded ? 'is-open' : ''))")
        .on("click", &format!("{st}.toggleThread(t.id)"))
        .kids(vec![
            Node::icon("caret-right").class("ph ph-caret-right thread-caret"),
            m("div").class("thread-body").kids(vec![
                m("div").class("thread-meta").kids(vec![
                    m("span").class("thread-count").text("t.label"),
                    m("span").class("thread-ref").text("t.ref"),
                    m("span")
                        .class("thread-clipped")
                        .iff("t.clipped")
                        .child(lit("truncated")),
                    m("span")
                        .class("thread-spinner")
                        .iff("t.loading")
                        .child(lit("opening…")),
                ]),
                m("div")
                    .bind("class", "('thread-text md-body ' + (t.built ? '' : 'unbuilt'))")
                    .inner_html("t.html || t.text"),
            ]),
        ])
}

pub fn register() {
    css::inject("thread", CSS);
}