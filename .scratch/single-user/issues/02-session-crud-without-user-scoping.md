# 02: Session CRUD without user scoping

**What to build:** A local user can create a new session, list sessions, switch between sessions, and delete sessions; each session holds its own conversation/thread data; deleting a session removes all related threads/messages; no per-user filtering or permission checks apply.

**Blocked by:** None (can start immediately)

**Status:** ready-for-agent

- [ ] Create a new session and start chatting.
- [ ] List all sessions.
- [ ] Switch the active session.
- [ ] Delete a session; `GET` after delete returns 404 and session list count decrements.
- [ ] Deleted session removes all related threads/messages and legacy session files where applicable.
- [ ] No login, owner, grant, capability, admin, or sharing checks are required.
