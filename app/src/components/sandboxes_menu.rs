//! type — and no create button, because sandboxes are agent-owned and launch
//! id — the only

use rustex1_kit::h::{lit, m, Node};
use rustex1_kit::{js, mjs};
use wasm_bindgen::JsValue;

use crate::components::rail_head;

pub const CSS: &str = r#"

.sandboxes-menu .item-list { padding-left: 24px; }
.sandboxes-menu .item-row { cursor: pointer; }

.sandboxes-menu .item-name { font-size: 12px; font-weight: 400; }

.sandboxes-menu .item-row > i { font-size: 13px; color: var(--gray-dim); }

.sandbox-empty {
  font-size: 12px;
  color: var(--gray-dim);
  padding: 6px 8px;
  line-height: 1.5;
}
"#;

fn is_live(state: &str) -> bool {
    matches!(state, "ready" | "starting" | "cloning")
}

pub fn group_rows(rows: &JsValue) -> JsValue {
    let out = js_sys::Array::new();
    let mut order: Vec<String> = Vec::new(); // first-seen image order
    let mut members: Vec<js_sys::Array> = Vec::new(); // the stored rows handle per group
    let mut counts: Vec<f64> = Vec::new();
    for row in js_sys::Array::from(rows).iter() {
        let image = js::get_str(&row, "image");
        let key = if image.is_empty() { "unknown".to_string() } else { image };
        let gi = match order.iter().position(|k| *k == key) {
            Some(i) => i,
            None => {
                order.push(key.clone());
                members.push(js_sys::Array::new());
                counts.push(0.0);
                let g = js::obj();
                js::set_str(&g, "image", &key);
                js::set(&g, "count", &JsValue::from_f64(0.0));
                js::set(&g, "rows", members.last().expect("just pushed"));
                out.push(&g);
                order.len() - 1
            }
        };
        if is_live(&js::get_str(&row, "state")) {
            counts[gi] += 1.0;
        }
        members[gi].push(&row);
    }
    for (i, count) in counts.into_iter().enumerate() {
        js::set(&out.get(i as u32), "count", &JsValue::from_f64(count));
    }
    out.into()
}

pub fn toggle_group(this: JsValue, args: js_sys::Array) -> JsValue {
    let image = js::arg_str(&args, 0);
    if image.is_empty() {
        return JsValue::UNDEFINED;
    }
    let map = js::get(&this, "sandboxOpen");
    let open = js::get(&map, &image).as_bool().unwrap_or(true);
    if !map.is_object() {
        js::set(&this, "sandboxOpen", &js::obj());
    }
    let map = js::get(&this, "sandboxOpen");
    js::set(&map, &image, &JsValue::from_bool(!open));
    mjs::redraw();
    JsValue::UNDEFINED
}

fn row(st: &str) -> Node {
    m("li")
        .for_each("r in g.rows")
        .key("r.id")
        .class("item-row")
        .on("click", &format!("{st}.editTarget('sandbox', r.id)"))
        .kids(vec![
            Node::icon("desktop"),
            m("span").class("item-name").text(
                "(r.id.startsWith('sandbox2-') ? r.id.slice(9) : r.id)",
            ),
        ])
}

fn group(st: &str) -> Node {
    m("div")
        .for_each(&format!("g in {st}.sandboxGroups"))
        .key("g.image")
        .kids(vec![
            m("div")
                .class("ptree-row")
                .bind("class", &format!("({st}.sandboxOpen[g.image] === false ? '' : 'is-open')"))
                .bind("title", "g.image")
                .on("click", &format!("{st}.toggleSandboxGroup(g.image)"))
                .kids(vec![
                    m("i").class("ph ph-caret-right ptree-caret"),
                    m("i").class("ph ph-shipping-container ptree-icon"),
                    m("span").class("ptree-name").text("g.image"),
                ]),
            m("ul")
                .class("item-list")
                .show(&format!("{st}.sandboxOpen[g.image] !== false"))
                .child(row(st)),
        ])
}

pub fn sandboxes_menu(st: &str) -> Node {
    m("div")
        .class("sandboxes-menu")
        .bind("class", "($store.home.sandboxesOpen ? '' : 'is-collapsed')")
        .kids(vec![
            rail_head::head("Sandboxes", "cube", None, Some("sandboxes")),
            m("div").show("$store.home.sandboxesOpen").kids(vec![
                m("p")
                    .class("sandbox-empty")
                    .show(&format!("!{st}.sboxUp"))
                    .child(lit("The sandbox service is not answering.")),
                m("p")
                    .class("sandbox-empty")
                    .show(&format!("{st}.sboxUp && {st}.sandboxGroups.length === 0"))
                    .child(lit("No sandboxes yet.")),
                group(st),
            ]),
        ])
}