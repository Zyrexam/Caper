
use rustex1_kit::h::{m, Node};

use crate::components::rail_head;
use crate::css;

pub const DEFAULT_ACCENT: &str = "#2563eb";

pub const USER_ICON: &str = "user";
pub const USER_ACCENT: &str = "#d97706";
pub fn user_name(st: &str) -> String { format!("({st}.userName || 'You')") }

pub const CSS: &str = r#"
:is(#sb, #home) .presence { display: flex; align-items: center; gap: 8px; }
:is(#sb, #home) .presence-seats {
  display: flex; align-items: center; flex-wrap: wrap; min-width: 0;
}

/* First-on-top: every seat carries an explicit z-index, highest on the left,
   because DOM order would stack them the other way (later paints on top).
   The value descends from the row seat count in the guide and from
   (100 - i) under x-for - only the direction matters, not the base. */
:is(#sb, #home) .presence-seat { position: relative; display: inline-flex; }
:is(#sb, #home) .presence-seat + .presence-seat { margin-left: -6px; }


:is(#sb, #home) .seat-disc {
  width: 22px; height: 22px; flex: 0 0 auto;
  display: inline-flex; align-items: center; justify-content: center;
  border-radius: 50%;
  background: color-mix(in srgb, var(--agent-accent) 14%, var(--panel-2));
  color: var(--agent-accent);
  box-shadow: 0 0 0 2px var(--panel);
}
:is(#sb, #home) .seat-disc > i { font-size: 12px; line-height: 1; }

/* Unseated: the disc empties and the glyph dims. It stays in the cluster
   because its messages are still attributed to it. Opaque, not transparent:
   a transparent disc would show its neighbour through the overlap. */
:is(#sb, #home) .seat-disc.is-off { background: var(--panel); color: var(--gray-dim); }


:is(#sb, #home) .presence-more {
  position: relative;
  width: 22px; height: 22px; flex: 0 0 auto;
  display: inline-flex; align-items: center; justify-content: center;
  border: 0; border-radius: 50%;
  background: var(--panel-2); color: var(--muted);
  font-family: var(--mono); font-size: 10px; line-height: 1;
  cursor: pointer;
  margin-left: -6px;
  box-shadow: 0 0 0 2px var(--panel);
}
:is(#sb, #home) .presence-more:hover { color: var(--accent-soft); background: var(--accent-glow); }


.presence-demo { display: flex; flex-direction: column; gap: 22px; }
"#;

/// `accent` is wrapped in a fallback here rather than trusting the caller,
/// because the value lands in a `style` attribute: an empty
/// computed-value time, which falls back to *inherited* — not to the `var`
/// fallback — so a seat would wear its neighbour's colour.
/// `handle` is never drawn — it becomes the disc's `title`, so the cluster
/// session editor dialog, so the cluster never has to make room for one.
pub fn seat(handle: &str, icon: &str, accent: &str, off: &str, z: &str) -> Node {
    let style = format!("('--agent-accent: ' + (({accent}) || '{DEFAULT_ACCENT}'))");
    let disc = m("span")
        .class("seat-disc")
        .bind("class", &format!("({off} ? 'is-off' : '')"))
        .bind("style", &style)
        .bind("title", handle)
        .child(Node::icon_dyn(icon));
    m("span")
        .class("presence-seat")
        .bind("style", &format!("('z-index:' + ({z}))"))
        .child(disc)
}

pub fn more(count: &str) -> Node {
    m("button")
        .class("presence-more")
        .attr("type", "button")
        .show(&format!("(({count}) > 0)"))
        .text(&format!("'+' + ({count})"))
}

pub fn seats(kids: Vec<Node>) -> Node {
    m("div").class("presence-seats").kids(kids)
}

/// rail add, with a pencil. It belongs to the session header, not to the
pub fn row(cluster: Node, add: Option<(&str, &str)>) -> Node {
    let mut kids = vec![cluster];
    if let Some((tip, on)) = add {
        kids.push(rail_head::edit(tip, on));
    }
    m("div").class("presence").kids(kids)
}

/// reading an empty conversation is never nobody. All three are expressions,
pub fn empty(handle: &str, icon: &str, accent: &str) -> Node {
    seats(vec![seat(handle, icon, accent, "false", "1")])
}

pub fn demo() -> Node {
    let agents: [(&str, &str, &str); 7] = [
        ("researcher", "flask", "#0091ff"),
        ("joker", "mask-happy", "#be5cff"),
        ("fish", "fish", "#3af89f"),
        ("homie", "house", "#de4a91"),
        ("phony", "phone", "#2bc5c2"),
        ("bowser", "browser", "#96949e"),
        ("dummy", "lego-smiley", "#eeff00"),
    ];
    let inert = "$store.samples.noop()";
    // agents descending numbers never have to know the count.
    let you = |z: &str| seat("'Bob'", &format!("'{USER_ICON}'"), &format!("'{USER_ACCENT}'"), "false", z);
    let built = |n: usize| -> Vec<Node> {
        agents[..n]
            .iter()
            .enumerate()
            .map(|(k, (h, i, c))| {
                seat(
                    &format!("'{h}'"),
                    &format!("'{i}'"),
                    &format!("'{c}'"),
                    "false",
                    &format!("{}", n - k),
                )
            })
            .collect()
    };

    let mut seven = vec![you("6")];
    seven.extend(built(5));
    seven.push(more("2"));

    let mut three = vec![you("4")];
    three.extend(built(3));

    let mut mixed = vec![you("5")];
    mixed.extend(built(3));
    mixed.push(seat("'bowser'", "'browser'", "'#96949e'", "true", "0"));

    m("div").class("presence-demo").kids(vec![
        row(seats(seven), Some(("Edit session", inert))),
        row(seats(three), Some(("Edit session", inert))),
        row(seats(mixed), Some(("Edit session", inert))),
        row(
            empty("'Bob'", &format!("'{USER_ICON}'"), &format!("'{USER_ACCENT}'")),
            Some(("Edit session", inert)),
        ),
    ])
}

pub fn register() {
    css::inject("presence", CSS);
}