# Caper

An agentic chat app: multi-agent conversations with a branching history, one
Rust server, one Rust-wasm client. Caper is our own build of this app; the UI
is composed from reusable parts, each defined exactly once.

## Status

The base application has been copied in and the workspace scaffolded. The
frontend builds against the vendored kit. The server still needs the model
layer: drop `agl-rs` into `vendor/agl-rs` (see `vendor/README.md`) before
`cargo build -p caper-server` can resolve.

## Layout

```
Caper/
├── app/        Rust -> wasm frontend: components, stores, routes
├── server/     Rust server: dev server + /api/*
├── crates/     shared Rust crates (wire contract, types)
├── ui/         document shell + shared skin (index.html, styles.css)
├── docs/       design notes and decisions
├── archive/    frozen reference material (not built)
├── vendor/     vendored dependencies (kit, harness, model layer)
├── dev.sh      build + run
└── AGENTS.md   how to work here
```

## Run

```bash
./dev.sh                # -> http://127.0.0.1:3030
./dev.sh 4000           # another port
./dev.sh --rebuild      # force a fresh wasm build on start
```

`dev.sh` adds the `wasm32-unknown-unknown` target and installs the pinned
`wasm-bindgen` CLI into `tools/` if either is missing, exports
`MUSE_API_KEY` from `.angela/.env` if it is not already in the environment,
builds `caper-server`, then hands the process to the rustex1 dev server, which
builds the frontend before answering the first request.

It uses **blaze** when that is on `PATH` and `cargo` otherwise; `CAPER_BUILD`
forces either.

## Where to add things

| you are adding | it goes in |
|---|---|
| a card, widget or panel | `app/src/components/`, with its `CSS` const, its `parts()` builder and its `demo()` |
| a route | `app/src/pages/` (app shell) or `app/src/pages/storybook.rs` (design guide) |
| an API handler | `server/src/api.rs`, one handler per route |
| a shared type or the wire contract | `crates/<name>/`, then use it from `app/` and `server/` |
| a static asset | `ui/` |

When you add a row kind, add a sample for it and a storybook route. A brick
with no page is a brick nobody can find.

## Stores

| store | owns |
|---|---|
| `live` | the open conversation: sessions, tabs, transcript, streaming text, participants, branches, allowlist, metrics |
| `agents` | the agent rail and the editor draft |
| `settings` | project settings, persisted server-side |
| `home` | the shell's aside collapse |
| `samples` | the design guide's conversation, in the same shape as `live` |
| `dropdown`, `tabbook`, `automationDraft` | widget-local state |

## Checks

Before calling UI work done: no duplicate selectors, no orphan CSS, nothing
hardcodes the live store below the page level, the console is clean and the
build is warning-free. See `AGENTS.md` for the full list.
