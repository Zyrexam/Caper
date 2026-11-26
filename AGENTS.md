# AGENTS.md — how to work in Caper

Caper is the chat app. The one rule everything else follows:

> **The UI is a composable hierarchy of reusable parts. Every component is
> defined exactly once. There are no one-off derivations.**

If a card changes on the transcript, it changes in the design guide too — not
because the two were kept in sync, but because there is only one of it.

## The rule in practice

**A component owns one definition of itself.** Each component file owns, for
one thing:

| | |
|---|---|
| its markup | one builder, taking **expressions**, never baked-in text |
| its CSS | a `CSS` const beside the markup, injected once |
| its condition | the `kind` of row it answers to, as its own check |
| its demo | the same builder, over the samples store |

Nothing about a card lives anywhere else — not in a global stylesheet, not
inlined into whichever parent happens to draw it today.

**Builders take the store, not the data.** Thread a store expression down from
the page:

```
parts(store, row)
//    "$store.live" or "$store.samples"
//                 the expression naming the row
```

The app passes the live store; the design guide passes the samples store. A
builder that hardcodes the live store cannot be shown in the guide without
being copied, and a copy is the bug. Side effects follow the same rule: a
dropdown that persists its pick takes the store it persists to.

**Composition, not re-implementation.** A parent lists its children; it does
not draw them. One function decides *which* cards a transcript can hold; each
card decides what it looks like. Collapse-and-click is owned once by the fold
card; the reasoning, tool-call, prompt and decision cards supply only a glyph,
a label, a note and a body.

**The samples store is the guide's data** — one conversation with a row of
every kind, a session list, a model list. Build its content with the app's own
renderers, not hand-written HTML. Its actions are inert, except where being
clickable is the point. When you add a row kind, add a sample for it and a
route: a brick with no page is a brick nobody can find.

## Checks before calling UI work done

- **No duplicate selectors.** A selector appears in exactly one place — the
  shared skin *or* one component.
- **No orphan CSS.** A rule whose classes nothing renders is dead. When the
  markup changes, its old rules go with it.
- **The shared stylesheet is the skin, not the components' look.** Tokens, the
  page frame and shared utilities belong there; anything dressing one component
  does not.
- **Nothing hardcodes the live store below the page level.**
- **The console is clean.** No template-lint warnings on boot, and no
  `could not set ...` errors — those mean a template is writing to a field that
  no longer exists, and they fail silently in the UI.
- **The build is warning-free.** An unused component function usually means a
  second definition was left behind.

## Build and run

- Frontend: Rust → wasm. Needs the `wasm32-unknown-unknown` target and the
  pinned `wasm-bindgen` CLI; `dev.sh` adds or installs either if missing.
- The kit lives at `vendor/rustex1` as a real directory, not a symlink.
- Prefer **blaze** when it is on `PATH`, cargo otherwise. After a big
  structural change, a wild crash in safe code is suspect stale incremental
  state first: wipe the cache, rebuild, then read stack traces.
- Path dependencies outside the workspace are not watched. After changing one,
  restart the server or it keeps serving the old binary.

## Template gotchas

- **Class bindings clobber; class calls replace.** Two class bindings on one
  element are one binding — the later wins and the earlier silently disappears.
  An icon's `.class()` replaces its classes, it does not append. Use one
  binding (or one `.class()`) carrying the **full** class list per state.
- **`model` syncs on `change`, not `input`.** A handler that reads the store in
  the same event that typed the text sees the **previous** value. Read the
  event target's value at the moment of the event.
- **`for_each` repeats the element it is bound to**, not its children. Bind it
  to the row; binding it to a list container clones the whole container once
  per item.

## Verification

- **Measure the visible tree, after it settles.** Hidden duplicate DOM exists
  (a mobile overlay shares class names with the desktop rail). Filter by
  visible geometry, and read computed styles after transitions finish — a
  colour read at t=0 reports the old colour.
- **Look with your own eyes.** Screenshots catch what probes miss.
- **Parallel tests need unique temp dirs** — pid plus an atomic counter, not a
  timestamp alone.

## Where things live

```
Caper/
├── app/      the Rust frontend: components, stores, routes
├── server/   the Rust server: dev server + /api/*
├── crates/   shared Rust crates (wire contract, types)
├── ui/       the document shell and the shared skin
├── docs/     design notes and decisions
├── archive/  frozen reference material (not built, not imported)
├── vendor/   vendored dependencies (kit, harness, model layer)
└── AGENTS.md
```

New server routes go in `server/src`; the one-definition rule applies there
too — one handler per route, one schema per contract. `archive/` is frozen:
read it, never build it or import from it.

## House rules

- Don't commit unless asked.
- Always write professional, production-quality commit messages using conventional commit types (feat, fix, chore, refactor, etc.). Keep the description concise (6–10 words), clearly explaining what was implemented or changed. Never include internal references or labels such as T1, M1, ticket, milestone, task-1, or similar identifiers.
- Use `rg`, never `grep`.
- Don't create or translate MCP servers — run the ones that exist.
- **No real names in committed files.** The placeholders are **Bob** and
  **Alice**, including all-caps variants. Databases may keep real names (they
  are ignored); the repo may not.
- The running app is **authoritative** for how something should look. If the
  design guide disagrees, the guide is wrong — and the fix is never to edit the
  guide's copy, because there should not be one.
- When a citation points at a DOM element (a pasted xpath and bounds), verify
  against *that* element's geometry and behaviour, not something that merely
  looks similar in a screenshot.
- Say so when a request seems off, and propose the correction instead of
  building it. A wrong premise caught early is cheaper than a wrong build.
