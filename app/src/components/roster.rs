//! A chat of two is you and one disc, right-aligned and otherwise out of

use rustex1_kit::h::{m, Node};

use crate::components::{notification_toggle, presence};
use crate::css;

pub const CSS: &str = r#"
:is(#sb, #home) .roster {
  display: flex; gap: 6px; align-items: center; flex-wrap: wrap;
  padding: 6px 10px; font-size: 11px;
}
/* The cluster shows you plus the first five seats; the rest live behind `+N`
   in the session editor. Cut in CSS, not with `x-show` on the `x-for` row:
   two directives fighting over one element visibility is exactly the kind
   of bind-time surprise this repo documents to avoid. Scoped to `.roster`
   so the editor full list — same markup under `.se-participants` — is
   never cut. `nth-child` counts every sibling, so the user disc in first
   position is what makes this seven and not six. */
:is(#sb, #home) .roster .presence-seat:nth-child(n+7) { display: none; }

:is(#sb, #home) .roster .notify-toggle { margin-left: auto; }
"#;

/// You first: the reader is never nobody, even alone, and the fallback says
pub fn roster(st: &str) -> Node {
    m("div")
        .class("roster")
        .show(&format!("{st}.activeId"))
        .child(notification_toggle::button(st))
        .child(
            presence::row(
                presence::seats(vec![
                    presence::seat(
                        &presence::user_name(st),
                        &format!("'{}'", presence::USER_ICON),
                        &format!("'{}'", presence::USER_ACCENT),
                        "false",
                        &format!("({st}.activeOthers.length + 100)"),
                    ),
                    // and its bylines can never disagree, and a seated
                    presence::seat(
                        "p.handle",
                        &format!("{st}.iconOf(p.handle)"),
                        &format!("{st}.accentOf(p.handle)"),
                        "!p.active",
                        "(100 - i)",
                    )
                    .for_each(&format!("(p, i) in {st}.activeOthers"))
                    .key("p.id"),
                    presence::more(&format!("{st}.activeOthers.length - 5")),
                ]),
                Some(("Edit session", &format!("{st}.openSessionEditor()"))),
            ),
        )
}

pub fn register() {
    css::inject("roster", CSS);
}