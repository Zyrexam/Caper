# 03: Show user as `dev` always

**What to build:** The local user is displayed as `dev` in user bylines and roster presence, regardless of stored `from`/`userName`; no account name or `You` fallback appears; message kind and agent names are unchanged.

**Blocked by:** None (can start immediately)

**Status:** ready-for-agent

- [ ] User message byline shows `dev`.
- [ ] Roster/local-user presence shows `dev`.
- [ ] No `You` fallback remains for the local user.
- [ ] `kind === 'user'` own-message detection is unchanged.
- [ ] Stored message data is not rewritten.
- [ ] `cargo check -p caper-app --target wasm32-unknown-unknown` passes.
