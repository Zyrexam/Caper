//! every HMR swap). The shared builder `symbol` is also called by

use rustex1_kit::h::{m, Node};
use rustex1_kit::mjs;
use rustex1_kit::mjs::Def;

use crate::css;

pub const NAME: &str = "uiSessionState";

const CSS: &str = r#"

.state-demo { display: flex; align-items: center; gap: 36px; }
/* the widget root must be a flex container so its fixed-size child (the
   14px pie ring) sizes correctly — a plain inline root collapses it to a
   zero-width inline box. */
.ui-state-root {
  display: inline-flex;
  align-items: center;
  flex: 0 0 auto;
}
"#;

pub fn ctx_fraction(tokens: &str, window: &str) -> String {
    format!("Math.min(1, ({tokens} || 0) / Math.max(1, ({window} || 128000)))")
}

pub fn fraction_of(row: &str) -> String {
    ctx_fraction(
        &format!("{row}.totalTokens"),
        &format!("{row}.contextWindow"),
    )
}

pub fn hint_of(row: &str) -> String {
    format!(
        "{tone} + ' - ' + {used} + '/' + {cap}",
        tone = tone_of(row),
        used = tokens(&format!("({row}.totalTokens || 0)")),
        cap = tokens(&format!("({row}.contextWindow || 128000)")),
    )
}

fn tokens(value: &str) -> String {
    format!(
        "(({value}) >= 1e6 ? ((({value}) / 1e6).toFixed(1).replace('.0', '') + 'M') \
         : ({value}) >= 1e3 ? ((({value}) / 1e3).toFixed(1).replace('.0', '') + 'K') \
         : ('' + ({value})))"
    )
}

pub fn symbol(state: &str) -> Node {
    let tone = match state {
        "done" => "done",
        "working" => "working",
        _ => "idle",
    };
    Node::host(NAME, &[]).bind("tone", &format!("'{tone}'"))
}

pub fn tone_of(row: &str) -> String {
    format!(
        "({row}.status === 'running' ? 'working' : {row}.turns === 0 ? 'idle' : 'done')"
    )
}

pub fn register() {
    css::inject(NAME, CSS);
    mjs::component(NAME, &def().finish());
}

fn def() -> Def {
    let mut d = Def::new().named(NAME);
    d.prop_str("tone", "done")
        .prop_num("fraction", 0.68)
        .prop_str("title", "")
        .tree(
            m("span")
                .class("ui-state-root")
                .bind("title", "title")
                .child(
                    Node::host("uiPieRing", &[])
                        .bind(
                            "color",
                            "(tone === 'done' ? 'var(--accent)' : \
                             tone === 'working' ? 'var(--pastel-amber)' : \
                             'var(--gray-dim)')",
                        )
                        .bind("fraction", "fraction"),
                ),
        );
    d
}