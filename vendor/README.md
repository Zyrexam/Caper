# vendor/

Third-party code, kept inside the repo so the build never reaches outside it.

| path | what | source |
|---|---|---|
| `rustex1/` | the kit and dev server: static files, HMR, wasm build | `github.com/mikesmullin/rustex1` |
| `angela-rs/` | the conversation harness: sessions, agents, memory, MCP | `github.com/mikesmullin/angela`, branch `rust` |
| `agl-rs/` | the model layer: providers, `provider:model` resolution | `github.com/mikesmullin/agl`, branch `rust` |

`vendor/` is excluded from the Cargo workspace; each entry is its own package.

## Windows patches in the vendored crates

- `angela-rs/src/environment.rs` upstream calls `libc::timegm`, `tm_zone`,
  `gmtime_r` and `mktime`, none of which exist on Windows. The copy here
  parses ISO timestamps with pure-Rust calendar math and reads the local clock
  through `localtime_s` (no zone abbreviation on Windows).
- `angela-rs/src/identity.rs` read `/dev/urandom`; on Windows that failed
  silently and every device token became 32 zero bytes. It now uses
  `getrandom`.
- `agl-rs/src/config.rs` looked only at `HOME`; it now falls back to
  `USERPROFILE` so `~/.config/agl/config.yaml` resolves on Windows.

Re-apply these when updating a vendored crate, or the Windows build breaks
again.

## OpenCode free models

`agl-rs/src/opencode.rs` adds an `opencode` provider that shells out to the
installed `opencode` CLI (`opencode run --format json`). The free models are
gated to the OpenCode client — the same request sent straight to the inference
API answers `429 FreeUsageLimitError` — so the CLI is the supported path.

- `opencode:<model>` is accepted like any other `provider:model`.
- The provider keeps its own agent loop and tools; Caper's tool registry is not
  passed through.
- The CLI lists what it can reach, and Caper adds those ids to `/api/models`
  automatically. `--archived` keeps each turn out of the user's session list.
- Text parts arrive whole rather than token by token, so a reply lands as one
  delta.

## Updating the kit

Replace `vendor/rustex1/` with a fresh checkout of the same repo. Keep the
pinned `wasm-bindgen` version in `rustex1/kit/Cargo.toml` in step with the CLI
that `dev.sh` installs - a mismatch produces a wasm module the browser refuses
to instantiate.
