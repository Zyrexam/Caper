//! ## Why a builder, not an `x-component`
//! straight from one dropdown to the other does not race: the button's own

use wasm_bindgen::JsValue;

use rustex1_kit::h::{lit, m, Node};
use rustex1_kit::mjs::Def;
use rustex1_kit::{js, mjs};

use crate::css;

const STORE: &str = "dropdown";

pub const CSS: &str = r#"

.dropdown { position: relative; display: inline-flex; flex: 0 0 auto; }
.dropdown-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 4px 6px;
  border: 1px solid transparent;
  border-radius: 7px;
  background: transparent;
  color: var(--text);
  /* The page's own face, not mono: these are labels, and the only reason
     they were mono was that the reference screenshot happened to be. */
  font-family: var(--sans);
  font-size: 12px;

  line-height: 18px;
  cursor: pointer;
  white-space: nowrap;
  transition: border-color .13s, background-color .13s;
}
.dropdown-btn:hover { background: var(--panel-2); border-color: var(--border); }
.dropdown-btn:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
.dropdown.open .dropdown-btn { background: var(--panel-2); border-color: var(--border); }

/* the icon carries the role colour, and only while the dropdown is
   hovered — at rest it is muted, like the caret. */

.dropdown-icon {
  font-size: 14px;
  line-height: 18px;
  flex: 0 0 auto;
  color: var(--muted);
  transition: color .13s;
}
.dropdown:hover .dropdown-icon.tone-green { color: var(--pastel-green); }

.dropdown:hover .dropdown-icon.tone-accent { color: var(--accent); }
/* Policy: muted while the allowlist is enforced, warning-red once every
   tool is permitted. Unlike the other two this is NOT hover-only — an
   unrestricted session must announce itself at rest. */
.dropdown:hover .dropdown-icon.tone-policy { color: var(--muted); }
.dropdown .dropdown-icon.tone-policy.unrestricted,
.dropdown:hover .dropdown-icon.tone-policy.unrestricted { color: var(--pastel-red); }

.dropdown-value { overflow: hidden; text-overflow: ellipsis; }
.dropdown-caret {
  font-size: 11px;
  line-height: 18px;
  color: var(--gray-dim);
  flex: 0 0 auto;
  transition: transform .13s;
}

.dropdown.open .dropdown-caret { transform: rotate(180deg); }


.dropdown-menu {
  position: absolute;
  bottom: calc(100% + 6px);
  left: 0;
  z-index: 40;
  min-width: 100%;
  width: max-content;
  max-width: 260px;
  padding: 4px;
  display: flex;
  flex-direction: column;
  gap: 1px;
  background: var(--panel-2);
  border: 1px solid var(--border);
  border-radius: 10px;
  box-shadow: 0 10px 28px rgba(0, 0, 0, .55);
}
.dropdown-item {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  padding: 6px 8px;
  border: 0;
  border-radius: 6px;
  background: transparent;
  color: var(--muted);
  font-family: var(--sans);
  font-size: 12px;
  line-height: 1.4;
  text-align: left;
  cursor: pointer;
  white-space: nowrap;
}
.dropdown-item:hover { background: var(--accent-glow); color: var(--text); }
.dropdown-item:focus-visible { outline: 2px solid var(--accent); outline-offset: -2px; }
.dropdown-item.on { color: var(--accent-soft); background: var(--accent-tint); }
.dropdown-item-label { flex: 1 1 auto; }
.dropdown-model-tools { display: flex; justify-content: flex-end; }
.dropdown-model-tools .model-refresh { width: auto; padding: 5px 8px; }
.dropdown-harnesses { padding: 5px 8px 6px; border-bottom: 1px solid var(--border); }
.dropdown-harnesses-title { margin-bottom: 3px; color: var(--gray-dim); font-size: 10px; font-weight: 600; }
.dropdown-harness-row { display: flex; justify-content: space-between; gap: 18px; padding: 2px 0; color: var(--muted); font-size: 10px; }
.dropdown-harness-row .ready { color: var(--pastel-green); }
.dropdown-harness-row .unavailable { color: var(--gray-dim); }
.dropdown-item-check {
  font-size: 12px;
  color: var(--accent);
  flex: 0 0 auto;
}


.dropdown-demo { display: flex; align-items: center; gap: 18px; }
"#;

/// something rather than only record a choice.
fn item(key: &str, value: &str, after: Option<&str>) -> Node {
    let is_on = format!("$store.{STORE}.{key} === '{value}'");
    let mut click = format!("$store.{STORE}.pick('{key}', '{value}')");
    if let Some(expr) = after {
        click.push_str("; ");
        click.push_str(&expr.replace("{}", value));
    }
    m("button")
        .class("dropdown-item")
        .attr("type", "button")
        .bind("class", &format!("({is_on} ? 'on' : '')"))
        .on("click", &click)
        .kids(vec![
            m("span").class("dropdown-item-label").child(lit(value)),
            m("i").class("ph ph-check dropdown-item-check").show(&is_on),
        ])
}

fn model_item(st: &str) -> Node {
    m("button").class("dropdown-item")
        .attr("type", "button")
        .bind("class", "($store.dropdown.model === modelOption ? 'on' : '')")
        .on("click", &format!("{st}.setModel(modelOption)"))
        .kids(vec![m("span").class("dropdown-item-label").text("modelOption"),
                   m("i").class("ph ph-check dropdown-item-check").show("$store.dropdown.model === modelOption")])
}

fn harness_readiness() -> Node {
    let row = m("div").class("dropdown-harness-row")
        .for_each("harness in $store.live.harnessCatalog.harnesses")
        .kids(vec![
            m("span").bind("title", "harness.reason").text("harness.label"),
            m("span")
                .bind("class", "(harness.ready ? 'ready' : 'unavailable')")
                .text("(harness.ready ? 'Ready' : 'Adapter needed')"),
        ]);
    m("div").class("dropdown-harnesses")
        .show("$store.live.harnessCatalog.harnesses.length > 0")
        .kids(vec![m("div").class("dropdown-harnesses-title").child(lit("Harnesses")), row])
}

/// - `tone`   — `green` / `accent`; the icon tints on hover/open only
pub fn dropdown(key: &str, icon: &str, tone: &str, extra: &str, options: &[&str], st: &str) -> Node {
    dropdown_ex(key, icon, tone, extra, options, None, st)
}

pub fn dropdown_ex(
    key: &str,
    icon: &str,
    tone: &str,
    extra: &str,
    options: &[&str],
    after: Option<&str>,
    st: &str,
) -> Node {
    let is_open = format!("$store.{STORE}.open === '{key}'");
    let value_expr = if key == "model" { format!("{st}.model") } else { format!("$store.{STORE}.{key}") };
    m("div")
        .class(&format!("dropdown {extra}"))
        .bind("class", &format!("({is_open} ? 'open' : '')"))
        .on("click.outside", &format!("$store.{STORE}.closeIf('{key}')"))
        .kids(vec![
            m("button")
                .class("dropdown-btn")
                .attr("type", "button")
                .on("click", &format!("$store.{STORE}.toggle('{key}')"))
                .kids(vec![
                    m("i")
                        .class(&format!("ph ph-{icon} dropdown-icon tone-{tone}"))
                        .bind(
                            "class",
                            &format!(
                                "($store.{STORE}.{key} === 'Unrestricted' ? 'unrestricted' : '')"
                            ),
                        ),
                    m("span")
                        .class("dropdown-value")
                        .text(&value_expr),
                    m("i").class("ph ph-caret-down dropdown-caret"),
                ]),
            m("div")
                .class("dropdown-menu")
                .show(&is_open)
                .kids(if key == "model" {
                    vec![m("div").class("dropdown-model-tools").child(m("button").class("dropdown-item model-refresh").attr("type", "button")
                        .attr("title", "Refresh models").attr("aria-label", "Refresh models")
                        .on("click", &format!("{st}.refreshModels()"))
                        .child(m("i").class("ph ph-arrows-clockwise"))),
                        harness_readiness(),
                        model_item(st).for_each(&format!("modelOption in {st}.models"))]
                } else {
                    options.iter().map(|o| item(key, o, after)).collect()
                }),
        ])
}

pub const LEVELS: &[&str] = &["low", "medium", "high", "xhigh", "max"];

pub fn agent_option(field: &str, icon: &str) -> Node {
    let key = format!("agent-{field}");
    let current = format!("$store.agents.draft.{field}");
    let open = format!("$store.dropdown.open === '{key}'");
    let row = |value: &str, label: &str| {
        let selected = format!("{current} === '{value}'");
        m("button").class("dropdown-item").attr("type", "button")
            .bind("class", &format!("({selected} ? 'on' : '')"))
            .on("click", &format!("$store.agents.pickOption('{field}', '{value}')"))
            .kids(vec![m("span").class("dropdown-item-label").child(lit(label)),
                       m("i").class("ph ph-check dropdown-item-check").show(&selected)])
    };
    let mut rows = vec![row("", "default")];
    if field == "model" {
        rows.push(m("button").class("dropdown-item").attr("type", "button")
            .for_each("agentModel in $store.live.models")
            .bind("class", "($store.agents.draft.model === agentModel ? 'on' : '')")
            .on("click", "$store.agents.pickOption('model', agentModel)")
            .kids(vec![m("span").class("dropdown-item-label").text("agentModel"),
                m("i").class("ph ph-check dropdown-item-check").show("$store.agents.draft.model === agentModel")]));
    } else {
        rows.extend(LEVELS.iter().map(|v| row(v, v)));
    }
    m("div").class("dropdown")
        .bind("class", &format!("({open} ? 'open' : '')"))
        .on("click.outside", &format!("$store.dropdown.closeIf('{key}')"))
        .kids(vec![m("button").class("dropdown-btn").attr("type", "button")
            .on("click", &format!("$store.dropdown.toggle('{key}')"))
            .kids(vec![m("i").class(&format!("ph ph-{icon} dropdown-icon")),
                m("span").class("dropdown-value").text(&format!("{current} || 'default'")),
                m("i").class("ph ph-caret-down dropdown-caret")]),
            m("div").class("dropdown-menu").show(&open).kids(rows)])
}

pub fn model(extra: &str, st: &str) -> Node {
    dropdown("model", "cpu", "green", extra, &[], st)
}

pub fn level(extra: &str, st: &str) -> Node {
    dropdown_ex(
        "level", "brain", "accent", extra, LEVELS,
        Some(&format!("{st}.setReasoning('{{}}')")),
        st,
    )
}

pub const POLICIES: &[&str] = &["Allowlist", "Unrestricted"];

/// shield turns pastel-red while unrestricted, because "every tool is
pub fn policy(extra: &str, st: &str) -> Node {
    dropdown_ex(
        "policy",
        "shield-check",
        "policy",
        extra,
        POLICIES,
        Some(&format!("{st}.setPolicy('{{}}')")),
        st,
    )
}

pub fn demo() -> Node {
    m("div")
        .class("dropdown-demo")
        .kids(vec![
            model("", crate::components::samples::EXPR),
            level("", crate::components::samples::EXPR),
        ])
}

fn toggle(_this: JsValue, args: js_sys::Array) -> JsValue {
    let key = js::arg_str(&args, 0);
    let s = mjs::store_ref(STORE);
    let now = js::get_str(&s, "open");
    js::set_str(&s, "open", if now == key { "" } else { &key });
    mjs::redraw();
    JsValue::UNDEFINED
}

fn pick(_this: JsValue, args: js_sys::Array) -> JsValue {
    let key = js::arg_str(&args, 0);
    let value = js::arg_str(&args, 1);
    if key.is_empty() {
        return JsValue::UNDEFINED;
    }
    let s = mjs::store_ref(STORE);
    js::set_str(&s, &key, &value);
    js::set_str(&s, "open", "");
    mjs::redraw();
    JsValue::UNDEFINED
}

/// `closeIf(key)`: dismiss, but only when *this* dropdown is the open one —
fn close_if(_this: JsValue, args: js_sys::Array) -> JsValue {
    let key = js::arg_str(&args, 0);
    let s = mjs::store_ref(STORE);
    if js::get_str(&s, "open") == key {
        js::set_str(&s, "open", "");
        mjs::redraw();
    }
    JsValue::UNDEFINED
}

pub fn register() {
    css::inject("dropdown", CSS);
    let mut d = Def::new().named("store dropdown");
    d.str("open", "")
        .str("model", "")
        .str("level", "medium")
        .str("policy", "Allowlist")
        .method("toggle", toggle)
        .method("pick", pick)
        .method("closeIf", close_if);
    mjs::store(STORE, &d.finish());
}
