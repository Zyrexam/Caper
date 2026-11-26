# vendor/

Third-party code, kept inside the repo so the build never reaches outside it.

| path | what | source |
|---|---|---|
| `rustex1/` | the kit and dev server: static files, HMR, wasm build | `github.com/mikesmullin/rustex1` |
| `angela-rs/` | the conversation harness: sessions, agents, memory, MCP | `github.com/mikesmullin/angela`, branch `rust` |
| `agl-rs/` | the model layer: providers, `provider:model` resolution | **not present** |

`vendor/` is excluded from the Cargo workspace; each entry is its own package.

## agl-rs is required

Both `angela-rs` and `server/` depend on the Rust `agl` crate, which is not
vendored here yet. The server will not resolve until it is:

1. put the crate at `vendor/agl-rs/` (its package name must be `agl`);
2. `cargo build -p caper-server`.

## Updating the kit

Replace `vendor/rustex1/` with a fresh checkout of the same repo. Keep the
pinned `wasm-bindgen` version in `rustex1/kit/Cargo.toml` in step with the CLI
that `dev.sh` installs - a mismatch produces a wasm module the browser refuses
to instantiate.
