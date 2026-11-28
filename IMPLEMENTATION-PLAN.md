# Caper implementation plan

## Goal

Caper is the local control plane for coding harnesses. It keeps projects,
sessions, transcripts, and the interface in one place while letting each turn
run through a deliberately selected harness, provider, and model. Claude Code,
Codex, and OpenCode are the first external harnesses. Caper's existing Angela
and AGL path remains available during the transition.

The selected target belongs to the session. A provider or model failure stays
visible in that session. Caper never retries through a different provider or
model without an explicit user choice.

## Current shape and constraints

- The Rust/Wasm frontend renders through rustex1 components. A component owns
  its markup and CSS, and the storybook uses the same builders.
- `POST /api/chat` currently enters Angela's dispatcher. AGL handles its model
  providers, while OpenCode is currently called as an AGL model backend.
- Angela's conversation store owns Caper's sessions, messages, participants,
  and session metadata. This remains the source for the Caper transcript and
  session list.
- Different harnesses have different continuation, cancellation, event, and
  approval protocols. Their integrations must keep those details local.
- Existing single-user, loopback-only behavior and completed session/project
  work are baseline. Do not add identity, sharing, or multi-user logic.
- Diagnostic work uses a separate temporary `CAPER_ROOT`. Never delete a
  session from the user's existing `.caper` store during diagnosis.

## Recommended shape

Store a validated session target with distinct fields:

```rust
enum HarnessId { Angela, OpenCode, ClaudeCode, Codex }

struct SessionTarget {
    harness: HarnessId,
    provider: Option<String>,
    model: Option<String>,
}
```

The server owns one explicit dispatcher that matches the selected harness.
Each integration owns its process or protocol, native continuation id,
provider/model catalog, cancellation, approval handling, and error details.
The shared boundary carries only the target, Caper session context, and
normalized transcript events. It does not pretend every harness has the same
capabilities.

The UI asks the selected harness for its available providers and models.
Provider selection appears where that harness supports it. Claude Code and
Codex expose only choices their official integrations report. The server
reports whether each harness is installed and ready. An unavailable target
stays selected and shows an actionable error; it does not resolve to another
target.

Caper stores each harness's native continuation id in namespaced session
metadata. Caper's event store remains the visible transcript. Adapters map
text, tool activity, approvals, errors, and completion into that transcript,
while retaining harness-specific details needed to resume or cancel work.

## UI direction

Keep the existing Rust/Wasm component system. Use Apple's sidebar guidance for
clear top-level groups and disclosure behavior. Use Coss UI and Align UI as
visual references for spacing, hierarchy, and control states. Do not add a
React component runtime or Tailwind build to copy their components.

The Vercel UI SDK has a Web Components adapter, so test one accessibility
behavior in an isolated prototype before deciding whether it fits Caper's
builders. Keep the Rust builder as the sole owner of a component's markup and
CSS. Adopt the SDK only if it can own behavior without duplicating that
component definition. Its beta status makes a broad dependency premature.
The Vercel AI SDK remains a reference for streamed chat states, not a
replacement for Caper's Rust event path.

## Implementation order

Each numbered item is one review checkpoint. Implement it, inspect the diff,
run only its relevant verification, report the result, and then begin the next
item. Do not commit or push.

1. [x] **New favicon mark.** Design a distinctive Caper berry mark with a
   transparent background and no enclosing app tile. Regenerate the source,
   favicon PNG sizes, ICO, and Apple touch icon. Inspect the actual raster files
   at 16, 32, and 180 pixels.
2. [x] **Usable agent for new sessions.** Seat a configured, usable agent when
   creating a session. If no configured agent is usable, seat the built-in
   assistant with no MCP dependencies. Start new agent definitions with no MCP
   defaults, and reject MCP names that are not configured.
3. **Target persistence and selection.** Add the session target fields and
   expose a server-owned catalog for harnesses, providers, models, and
   readiness. Preserve a session's choice across reloads.
4. **Explicit failure behavior.** Keep the selected target if it becomes
   unavailable. Surface provider status and error text, including
   `429 FreeUsageLimitError`. Remove fallback behavior that silently changes
   provider or model.
5. **OpenCode harness integration.** Move OpenCode out of the AGL model
   fallback path into its explicit harness adapter. Preserve streaming,
   continuation, cancellation, and errors supported by its installed CLI.
6. **Claude Code integration.** Add the official local harness using its
   supported structured streaming interface. Preserve its session and approval
   behavior in the adapter.
7. **Codex integration.** Add the Codex app-server protocol for persistent
   threads, streamed events, and approvals. Avoid reducing it to a one-shot
   command when the app-server can own the live conversation.
8. **Isolated diagnostic.** In a temporary root, create a diagnostic test
   agent and session, send `Say OK`, verify the response and event stream, then
   remove only the temporary test data. Never delete or mutate a user's
   existing sessions.
9. **Remove Automations from the UI.** Remove the section and its navigation
    routes from the user-facing flow. Keep stored automation data and server
    APIs untouched in this UI cleanup unless a separate removal is requested.
10. **Sidebar hierarchy.** Show clear Projects, Agents, and Sessions groups.
    Put each collapse control inline between its groups, keep each `+` action,
    and remove the bottom `>` collapse control. Move the dev profile to the
    sidebar bottom.
11. **Settled sessions and deletion.** Keep settled sessions visible in their
    own section. Ensure each remains selectable and deletable through
    `DELETE /api/sessions/{id}`. Verify deletion only against an isolated test
    session.
12. **Final review.** Check the component/skin boundary, template warnings,
    clean console, startup, `/api/config`, `/api/agents`, `/api/sessions`, and
    `/api/models`. Run the server tests and frontend check after the final
    structural changes. Do not push.

## UI library choice

Use Apple Human Interface Guidelines for the sidebar structure. Use Coss UI
and Align UI only as design references; their component setups target React
and Tailwind, which Caper does not use. HeroUI and Ant Design also depend on
React. DaisyUI would add a Tailwind dependency and a second styling system.
Evaluate Vercel UI SDK's Web Components adapter on one control because it can
provide accessible behavior without requiring React. Keep it only if the
prototype preserves Caper's single component owner. Caper's builders remain
the implementation either way.

## Alternatives considered

- A generic harness registry with a large shared capability interface would
  hide routing but expose every harness difference as flags and optional
  methods. That is likely to spread protocol rules across callers.
- Keeping Claude and Codex as model adapters inside AGL would force their
  native session, approval, and tool behavior through a model-only interface.
  OpenCode's current one-shot adapter shows why that surface is too small.
- Replacing the Rust UI with a React component library would add a second
  runtime and violate Caper's one-definition rule.

The recommended design keeps a small shared event boundary and an explicit
match over separate harness integrations. It follows candidate B's distinct
adapter shape and candidate A's typed target and per-session native thread id.
An independent cross-vendor design judge was unavailable in this environment.

## Reference material

- [T3 Code architecture](https://t3code.org/what-is-t3-code) describes the
  control-plane model over official harnesses.
- [Codex app-server docs](https://learn.chatgpt.com/docs/app-server) describe
  streamed events, approvals, and persistent conversations for rich clients.
- [OpenAI's Codex harness overview](https://openai.com/index/unlocking-the-codex-harness/)
  explains its JSON-RPC app-server boundary.
- [Apple sidebar guidance](https://developer.apple.com/design/human-interface-guidelines/sidebars)
  informs the navigation hierarchy.
- [Vercel UI SDK Web Components](https://ui-sdk.dev/docs/web-components) offers
  an unstyled custom-element adapter for interaction behavior; its docs mark
  the SDK as beta.
- [Coss UI setup](https://coss.com/ui/docs/get-started) confirms its React and
  Tailwind integration model.
- [HeroUI setup](https://heroui.com/en/docs/react/getting-started) and
  [Ant Design setup](https://ant.design/docs/react/getting-started/) document
  their React requirements.
- [Vercel AI SDK](https://vercel.com/ai-sdk) provides AI interaction patterns;
  it is not a drop-in dependency for this frontend.
