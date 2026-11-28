//! imperative class toggle, because the grid track has to shrink with it —

use wasm_bindgen::JsValue;

use rustex1_kit::h::{lit, m, Node};
use rustex1_kit::mjs::Def;
use rustex1_kit::{js, mjs};

use crate::components::{
    agent_editor, agents_menu, automations_menu, lightbox, mode_toggle, perf_badge,
    project_tree, sandboxes_menu, session_editor, session_panel, tab, target_form,
};
use crate::css;

const LIVE: &str = session_panel::LIVE_STORE;

const STORE: &str = "home";

pub const CSS: &str = r#"

#home {
  display: grid;
  grid-template-columns: auto 1fr auto;
  height: 100vh;
  height: 100dvh;
  min-height: 0;
  background: var(--bg);
  color: var(--text);
  font-family: var(--sans, ui-sans-serif, system-ui, sans-serif);
}

#home.aside-collapsed { grid-template-columns: auto 1fr 0px; }
#home.aside-collapsed .home-aside { display: none; }
#home.rail-collapsed { grid-template-columns: 0px 1fr auto; }
#home.rail-collapsed .home-rail { display: none; }
#home.rail-collapsed .home-col { margin-left: 8px; }
#home.rail-collapsed.aside-collapsed { grid-template-columns: 0px 1fr 0px; }
.home-rail { grid-column: 1; }


.home-rail { --group-gap: 8px; }
.home-col { grid-column: 2; }
.home-aside { grid-column: 3; }


/* shortcut: resized widths reset on reload; persist them in the home store
   when that starts to annoy. Limits are the min/max below. */
.home-rail {
  width: 264px;
  min-width: 216px;
  max-width: 420px;
  min-height: 0;
  display: flex;
  flex-direction: column;
  gap: var(--group-gap);
  overflow: hidden;
  resize: horizontal;
  padding: 8px 0 8px 4px;

  margin-right: 8px;
}
.home-rail-content {
  flex: 1 1 auto;
  min-height: 0;
  display: flex;
  flex-direction: column;
  gap: var(--group-gap);
  overflow-y: auto;
}
.home-agents { padding: 0 14px; }
.home-sandboxes { padding: 0 14px; }
.home-automations { padding: 0 14px; }

/* ---- centre column ----------------------------------------------
   A plain flex column holding the tab strip and, beneath it, the card.
   The column itself has no surface — only `.home-main` does. */
.home-col {
  min-width: 0;
  min-height: 0;
  display: flex;
  flex-direction: column;
  margin-right: 8px;
}

/* the tab strip: on the black page, ABOVE the card it labels. A row now:
   a toggle for each side column flanking the tabs, so both stay reachable
   when their column is gone. The strip itself scrolls, never the buttons. */
.home-tabs {
  flex: 0 0 auto;
  display: flex; align-items: center; gap: 6px;
  padding: 8px 2px 6px;
}
.home-tabs .tab-bar {
  flex: 1 1 auto; min-width: 0;
  display: flex; overflow-x: auto; overflow-y: hidden;
}

.tabs-toggle {
  flex: 0 0 auto;
  width: 28px; height: 28px;
  display: inline-flex; align-items: center; justify-content: center;
  border: 1px solid transparent;
  border-radius: 7px;
  background: transparent;
  color: var(--muted);
  font-size: 16px;
  cursor: pointer;
  transition: color .13s, border-color .13s, background-color .13s;
}
.tabs-toggle:hover { color: var(--text); background: var(--panel-2); border-color: var(--border); }
.tabs-toggle:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
#home.aside-collapsed .tabs-toggle.for-aside .ph { transform: scaleX(-1); }
#home.rail-collapsed .tabs-toggle.for-rail .ph { transform: scaleX(-1); }

.home-main,
.home-aside {
  min-width: 0;
  min-height: 0;
  background: var(--panel);
  border: 1px solid var(--border);
  border-radius: 12px;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.home-main { flex: 1 1 auto; margin-bottom: 8px; }
.home-aside {
  width: 288px;
  min-width: 240px;
  max-width: 520px;
  resize: horizontal;
  margin: 8px 8px 8px 0;
}

.home-empty {
  align-items: center;
  justify-content: center;
  gap: 10px;
  padding: 24px;
  text-align: center;
}
.home-empty-title {
  margin: 0;
  font-size: 20px;
  font-weight: 600;
  letter-spacing: -0.01em;
  color: var(--text);
}
.home-empty-note {
  margin: 0;
  max-width: 320px;
  font-size: 13.5px;
  line-height: 1.5;
  color: var(--muted);
}
.home-empty-new {
  margin-top: 6px;
  padding: 8px 14px;
  border: 0;
  border-radius: 9px;
  background: var(--text);
  color: var(--bg);
  font: 600 13px/1 var(--sans);
  cursor: pointer;
  transition: background-color .13s;
}
.home-empty-new:hover { background: #fff; }
.home-empty-new:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }

.home-main .mjs-chat { height: 100%; background: transparent; border-left: 0; }


.home-main .composer-inner { padding: 0 10px 10px; }
.home-main .messages { padding-left: 10px; padding-right: 10px; }


.home-aside-body { flex: 1 1 auto; min-height: 0; overflow-y: auto; padding: 12px 12px 12px; }
.home-aside-toggle-row { display: none; }



.home-projects { padding: 0 14px; }

.home-settings {
  flex: 0 0 auto; display: flex; flex-direction: column;
  gap: 0; padding-top: 0;
}
.home-settings-row { display: flex; align-items: center; gap: 6px; padding-right: 14px; }


.home-rail-content > div { flex: 0 0 auto; }
.rail-settings {
  display: inline-flex; align-items: center; gap: 8px;
  flex: 1 1 auto; min-width: 0; border: 0; border-radius: 8px;
  background: transparent; color: var(--gray-dim);
  padding: 7px 8px; cursor: pointer; font: inherit; font-size: 13px;
  text-align: left;
}
.rail-settings:hover { background: var(--panel-2); color: var(--fg); }
.rail-settings i { font-size: 15px; line-height: 18px; }
#home.aside-collapsed .home-aside-body { display: none; }

/* ---- responsive: the shell gives way in stages; content never scrolls
   sideways. Breakpoints are tuned for tablet widths (a Kindle Fire HD
   reports roughly 960-1280 CSS px in landscape, 600-800 in portrait).
   These rules only narrow tracks and, at the last step, lift the aside out
   of flow — no component's look is redefined here, so each component keeps
   owning its own rules (its narrow mode lives in its own file). */
@media (max-width: 1180px) {
  #home { grid-template-columns: 248px minmax(0, 1fr) 288px; }
  #home.aside-collapsed { grid-template-columns: 248px minmax(0, 1fr) 0px; }
}
@media (max-width: 1020px) {
  #home { grid-template-columns: 216px minmax(0, 1fr) 256px; }
  #home.aside-collapsed { grid-template-columns: 216px minmax(0, 1fr) 0px; }
}

@media (max-width: 900px) {

  #home,
  #home.aside-collapsed { grid-template-columns: 60px minmax(0, 1fr); }
  #home.rail-collapsed,
  #home.rail-collapsed.aside-collapsed { grid-template-columns: 0px minmax(0, 1fr); }
  .home-rail { padding: 10px 0; margin-right: 8px; gap: 12px; }
  .home-rail-content { gap: 12px; }
  .home-projects { padding: 10px 6px 4px; }
  .home-agents { padding: 0 6px 10px; }
  .home-sandboxes { padding: 0 6px 10px; }
  .home-automations { padding: 0 6px 10px; }

  .home-col { margin-right: 8px; isolation: isolate; }
  .home-aside-toggle-row {
    display: flex; justify-content: flex-end; flex: 0 0 auto;
    padding: 6px 8px 0;
  }
  .home-aside {
    position: fixed; top: 10px; right: 10px; bottom: 10px;
    width: min(340px, calc(100vw - 100px));
    margin: 0; z-index: 60;
    box-shadow: 0 16px 48px rgba(0, 0, 0, .55);
  }
  .rail-settings { justify-content: center; padding-left: 0; padding-right: 0; }
  .rail-settings span { display: none; }
}
"#;

fn rail() -> Node {
    m("aside")
        .class("home-rail")
        .attr("aria-label", "Projects, agents and automations")
        .kids(vec![
            m("div").class("home-rail-content").kids(vec![
                // The Sandboxes section never has a `+`
                m("div").class("home-projects").child(project_tree::tree(
                    LIVE,
                    &format!("{LIVE}.previewSession({{}})"),
                    &format!("{LIVE}.createTarget('project')"),
                )),
                m("div").class("home-agents").child(agents_menu::agents_menu_live()),
                // the (session, agent) it serves, grouped per image. Read-only
                m("div").class("home-sandboxes")
                    .show(&format!("{LIVE}.sandboxGroups.length > 0"))
                    .child(sandboxes_menu::sandboxes_menu(LIVE)),
                m("div")
                    .class("home-automations")
                    .child(automations_menu::automations_menu_live()),
            ]),
            // never move as the lists above it grow.
            m("div").class("home-settings").kids(vec![
                m("div").class("home-settings-row").child(
                    m("button")
                        .class("rail-settings")
                        .attr("type", "button")
                        .attr("aria-label", "Settings")
                        .on("click", "$store.settings.openPanel()")
                        .kids(vec![
                            Node::icon("gear").class("ph ph-gear"),
                            m("span").child(lit("Settings")),
                        ]),
                ),
                m("div").class("home-settings-row").kids(vec![
                    mode_toggle::button(),
                ]),
            ]),
        ])
}

fn aside_toggle() -> Node {
    m("button")
        .class("tabs-toggle for-aside")
        .attr("type", "button")
        .attr("title", "Toggle inspector")
        .attr("aria-label", "Toggle inspector")
        .attr("aria-controls", "home-aside")
        .bind("aria-expanded", "($store.home.asideOpen ? 'true' : 'false')")
        .on("click", "$store.home.toggleAside()")
        .child(Node::icon("sidebar-simple"))
}

fn main_col() -> Node {
    m("div").class("home-col").kids(vec![
        m("div").class("home-tabs").kids(vec![
                m("button")
                    .class("tabs-toggle for-rail")
                    .attr("type", "button")
                    .attr("title", "Toggle left sidebar")
                    .attr("aria-label", "Toggle left sidebar")
                    .bind("aria-expanded", "($store.home.railOpen ? 'true' : 'false')")
                    .on("click", "$store.home.toggleRail()")
                    .child(Node::icon("sidebar-simple")),
                tab::tab_bar(LIVE),
                // why it writes its own text instead of binding it.
                perf_badge::badge().show("$store.mode.advanced"),
                aside_toggle(),
            ]),
        m("section")
            .class("home-main")
            .show("$store.live.tabs.length > 0")
            // guide mounts too — see that module for why the tree is not
            .child(session_panel::session_panel()),
        m("section")
            .class("home-main home-empty")
            .show("$store.live.tabs.length === 0")
            .kids(vec![
                m("h1").class("home-empty-title").child(lit("No session open")),
                m("p")
                    .class("home-empty-note")
                    .child(lit("Start a session to talk with an agent.")),
                m("button")
                    .class("home-empty-new")
                    .attr("type", "button")
                    .on("click", "$store.live.newSession()")
                    .child(lit("New session")),
            ]),
    ])
}

fn aside() -> Node {
    m("aside")
        .class("home-aside")
        .attr("id", "home-aside")
        .attr("aria-label", "Details")
        .kids(vec![
            m("div").class("home-aside-toggle-row").child(aside_toggle()),
            m("div").class("home-aside-body").kids(vec![
                agent_editor::agent_editor(),
                agent_editor::settings_panel(),
                target_form::panel(),
                session_editor::panel(),
            ]),
        ])
}

pub fn shell() -> Node {
    m("div")
        .attr("id", "home")
        .class("home")
        .init("$store.live.loadProjects(); $store.live.loadSessions(); $store.live.loadTargets(); $store.live.loadAutomations()")
        .bind(
            "class",
            &format!(
                "(($store.{STORE}.asideOpen ? '' : 'aside-collapsed') + ($store.{STORE}.railOpen ? '' : ' rail-collapsed'))"
            ),
        )
        .kids(vec![rail(), main_col(), aside(), lightbox::lightbox()])
}

fn toggle_aside(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let open = js::get_bool(&s, "asideOpen");
    js::set_bool(&s, "asideOpen", !open);
    mjs::redraw();
    JsValue::UNDEFINED
}


fn toggle_rail(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let open = js::get_bool(&s, "railOpen");
    js::set_bool(&s, "railOpen", !open);
    mjs::redraw();
    JsValue::UNDEFINED
}

fn section_open(key: &str) -> bool {
    let storage = js::get(&js::global(), "localStorage");
    let value = js::call(&storage, "getItem", &[JsValue::from_str(&format!("chat.rail.{key}"))]);
    value.as_string().as_deref() != Some("false")
}

/// than in the data stores because collapsing a section hides a view, it
fn toggle_section(_this: JsValue, args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let key = match js::arg_str(&args, 0).as_str() {
        "projects" => "projectsOpen",
        "agents" => "agentsOpen",
        "sandboxes" => "sandboxesOpen",
        "automations" => "automationsOpen",
        _ => return JsValue::UNDEFINED,
    };
    let open = !js::get_bool(&s, key);
    js::set_bool(&s, key, open);
    let storage = js::get(&js::global(), "localStorage");
    js::call(&storage, "setItem", &[JsValue::from_str(&format!("chat.rail.{key}")), JsValue::from_str(if open { "true" } else { "false" })]);
    mjs::redraw();
    JsValue::UNDEFINED
}

pub fn register() {
    css::inject("home", CSS);
    let mut d = Def::new().named("store home");
    d.bool("asideOpen", false)
        .bool("railOpen", true)
        .bool("projectsOpen", section_open("projectsOpen"))
        .bool("agentsOpen", section_open("agentsOpen"))
        .bool("sandboxesOpen", section_open("sandboxesOpen"))
        .bool("automationsOpen", section_open("automationsOpen"))
        .method("toggleAside", toggle_aside)
        .method("toggleRail", toggle_rail)
        .method("toggleSection", toggle_section);
    mjs::store(STORE, &d.finish());
}