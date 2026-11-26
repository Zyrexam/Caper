# crates/

Shared Rust crates that both `app/` and `server/` depend on: the wire
contract, shared types, anything that must not be defined twice.

`members = ["crates/*"]` already includes them. Add one with:

```bash
cargo new --lib crates/<name>
```

and it joins the workspace automatically.
