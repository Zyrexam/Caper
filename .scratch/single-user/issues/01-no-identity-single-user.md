# 01: No-identity single-user gate

**What to build:** Caper behaves as one local user with no login, accounts, auth, ACL, sharing, capabilities, admin, masquerade, shields, or bans; loopback is the only transport boundary; `/login` does not render a login screen.

**Blocked by:** None (can start immediately)

**Status:** ready-for-agent

- [ ] No `/api/me`, `/api/users`, `/api/admin`, `/login`, `/masquerade`, logout, or share endpoints are exposed by Caper.
- [ ] No app identity store or identity UI remains; create affordances are unconditional.
- [ ] Non-loopback peers are disconnected without a reply; loopback requests are served without cookies.
- [ ] `GET /login` returns 404.
- [ ] `GET /api/me` returns 404.
- [ ] No schema changes or destructive migration; vendored identity tables are untouched but unread by Caper.
