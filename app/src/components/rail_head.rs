
use rustex1_kit::h::{lit, m, Node};

use crate::css;

pub const CSS: &str = r#"
.rail-head {
  display: flex; align-items: center; gap: 6px;
  margin-bottom: 10px;
}

.rail-head .rail-add { margin-left: auto; }

.rail-title {
  margin: 0;
  font-size: 13px; font-weight: 400;
  color: var(--gray-dim);
}

button.rail-title {
  background: transparent; border: 0; padding: 0;
  display: inline-flex; align-items: center; gap: 5px;
  /* Family only: size/weight/ink stay on `.rail-title`, whose lower
     specificity would otherwise lose to this button rule. */
  font-family: inherit; text-align: left;
  cursor: pointer;
}
button.rail-title:hover { color: var(--text); }
button.rail-title .ph { font-size: 11px; line-height: 1; }
button.rail-title:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; border-radius: 4px; }
/* Collapsed: the section keeps only its head — nothing trails it, so the
   head sheds its margin-bottom and the *container gap* alone decides the
   space to the next group. (An earlier version pulled the root up by a
   negative margin to stack heads flush — which dragged the next group up
   over this one and made every gap measurement a liar.) */
.project-tree.is-collapsed > .rail-head,
.agents-menu.is-collapsed > .rail-head,
.sandboxes-menu.is-collapsed > .rail-head { margin-bottom: 0; }
.rail-add {
  width: 22px; height: 22px;
  display: inline-flex; align-items: center; justify-content: center;
  border: 0; border-radius: 6px;
  background: transparent; color: var(--gray-dim);
  cursor: pointer; font: inherit; line-height: 1;
}
.rail-add:hover { color: var(--accent); background: var(--panel-2); }

.rail-head-demo { display: flex; flex-direction: column; gap: 18px; width: 264px; }

/* Narrow shell (see home.rs breakpoints): the rail is a 60px icon strip, so
   the section label goes and the caret toggle stays — a collapsed section
   with no visible toggle could never be expanded again. */
@media (max-width: 900px) {
  .rail-head .rail-title span { display: none; }
  .rail-head { justify-content: center; }
  .rail-head .rail-add { margin-left: 0; }
}
"#;

/// Split out of [`head`] because the same button is the one control beside a
fn button(icon: &str, tip: &str, on: &str) -> Node {
    m("button")
        .class("rail-add")
        .attr("type", "button")
        .attr("title", tip)
        .attr("aria-label", tip)
        .on("click", on)
        .child(Node::icon(icon))
}

pub fn add(tip: &str, on: &str) -> Node {
    button("plus", tip, on)
}

/// different glyph, so the two never drift into two button styles.
pub fn edit(tip: &str, on: &str) -> Node {
    button("pencil-simple", tip, on)
}

/// static because a section never changes what it is. `add` is `(tooltip,
pub fn head(title: &str, icon: &str, add: Option<(&str, &str, &str)>, section: Option<&str>) -> Node {
    let title_node = match section {
        // its click never reaches this button and needs no `.stop`.
        Some(key) => m("button")
            .class("rail-title")
            .attr("type", "button")
            .bind("title", &format!("($store.home.{key}Open ? 'Collapse section' : 'Expand section')"))
            .bind("aria-expanded", &format!("($store.home.{key}Open ? 'true' : 'false')"))
            .on("click", &format!("$store.home.toggleSection('{key}')"))
            .kids(vec![
                Node::icon_dyn(&format!("($store.home.{key}Open ? 'caret-down' : 'caret-right')")),
                Node::icon(icon),
                m("span").child(lit(title)),
            ]),
        None => m("p")
            .class("rail-title")
            .kids(vec![Node::icon(icon), m("span").child(lit(title))]),
    };
    let mut kids = vec![title_node];
    if let Some((tip, on, guard)) = add {
        let b = self::add(tip, on);
        kids.push(if guard.is_empty() { b } else { b.show(guard) });
    }
    m("div").class("rail-head").kids(kids)
}

pub fn demo() -> Node {
    m("div").class("rail-head-demo").kids(vec![
        head("Projects", "folders", Some(("New project", "$store.samples.noop()", "")), Some("projects")),
        head("Agents", "robot", Some(("Define a new agent", "$store.samples.noop()", "")), Some("agents")),
        head("(ungrouped)", "tray", None, None),
    ])
}

pub fn register() {
    css::inject("rail-head", CSS);
}