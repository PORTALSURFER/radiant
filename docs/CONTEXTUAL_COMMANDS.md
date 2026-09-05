# Contextual commands

`radiant::application` provides static command registration, immutable active scope snapshots,
a data-only keymap, presentation queries, and one invocation-to-message mapper. The explicit
host boundary is `SurfaceRuntime::dispatch_command_request`; the application builder's
`.command_registry(registry, dispatcher)` connects committed view scopes to the ordinary reducer.
The advanced `.commands(registry, project, dispatcher)` hook also accepts manual scope snapshots.

The Winit keyboard adapter submits native presses through this boundary for runtimes with a
semantic command host. Automatic menu/toolbar/palette adapters and inheritance
of a parent registry into auxiliary-window bridges remain integration work. Those adapters
must use the same resolver and target checks.

## Ownership and precedence

`CommandRegistry` owns stable `CommandId` values and `TextKey` metadata, default bindings,
and repeat policy. It is immutable, cheap to clone, and contains no callbacks or domain state.
`CommandScope<Context>` owns binding availability, optional checked state, and opaque owned
application context. Applications omit inactive scopes from `CommandSnapshot`. The advanced
builder projection receives current read-only application state and runtime focus.

Resolution checks text consumption, composition, and platform reservation before commands.
Then active scopes resolve in this order: modal, overlay, nearest editor, selection, window,
application. Larger modal/overlay order and editor depth are nearer. Disabled bindings decline.
Two enabled matching bindings at equal precedence produce a conflict; declaration order cannot
choose a winner. Repeat defaults to suppressed, and suppression never permits legacy fallback.

The application registers one `CommandDispatcher`, shared by input and presentation activation.
It maps the selected `CommandInvocation<Context>` to a normal message; the reducer remains the
authority for domain changes. Resolution and presentation queries do not run the reducer.
As with other opaque application data, interior mutability inside `Context` is the application's
responsibility; a scope wrapper does not freeze those values.

## Declarative scopes

Attach `.commands([CommandBinding::new(id, context), ...])` to a view to establish a focused
editor scope. The runtime selects the focused widget's ancestors and derives their depth;
siblings are inactive. Use `.command_scope(prepared_scope)` for explicit selection, window,
application, modal or overlay scopes. Modal and overlay scopes must reside in their declared
layer category; their order comes from the scene, overriding caller-supplied numbers. Passive
tooltips, drag previews, synthesized layer input shields and noninteractive floating content
cannot contribute commands. Nodes omitted from the accepted layout are inactive.

Register one `.command_registry(registry, dispatcher)` and optionally a read-only
`.command_keymap(|state| state.keymap.clone())`. `runtime.command_scopes::<Context>()` queries
current scopes for presentation without running the mapper. Advanced hosts receive the same
borrowed `CommandScopeProjection` through `RuntimeInputHost::resolve_command_with_scopes`;
its default delegates to the existing manual command hook.

Captured context remains UI-local. Frozen reconciliation metadata carries only an incarnation
marker. Rebuilding an attachment forces fresh ownership projection while preserving compatible
widget focus and editing state. Component projection caching and application lowering receipts
conservatively decline scope-bearing content. Explicit scope clones retain target identity;
reconstruct a scope when its captured context changes. Registering either builder router replaces
the other; manual and declarative scope lists are never implicitly combined.

Collection admits 1,024 attachments in at most 65,536 source nodes, and activation admits 64
scopes. Duplicate source identities, incompatible context types, invalid construction and
exceeded capacity fail closed. No truncated subset can dispatch. Automatic attachment construction
diagnostics are available on the active projection. Scope indexing is committed with the runtime
view, so an uncommitted application projection cannot replace active command context.

## Presentation and stale activation

`registry.present(...)` resolves labels, descriptions, category and accessibility text through
the current `ResolvedEnvironment`, and returns checked/enabled state, effective shortcut text,
and an opaque `CommandTarget`. Querying does not call the invocation mapper.

A target is qualified by registry and scope incarnation, not just its command string. Clone an
unchanged scope to retain its incarnation. Reconstruct it when captured context changes. A
queued target for a replaced scope, another registry, or a newly shadowed owner is rejected
before mapping. Unavailable and conflicting targets cannot execute. Runtime shutdown rejects
all requests before invoking the application projection.

## Logical and physical keymaps

Character bindings match exact logical text produced by the current layout, including case.
Named bindings describe logical keys such as `Enter`. Physical bindings explicitly name a
position such as `KeyZ`; the adapter must supply both identities without guessing one from the
other. Physical matches are visibly marked in shortcut presentations. `primary` maps to Command
on macOS and Control elsewhere; physical Control and Meta remain separately expressible.

A version-1 persisted keymap is ordinary JSON:

```json
{"version":1,"entries":[{"command":"document.save","bindings":[{"key":{"kind":"character","value":"s"},"modifiers":{"primary":true}}]}]}
```

An empty binding list explicitly unbinds a command. Removing its override restores registered
defaults. Unknown command IDs and malformed/future entries are retained as inactive JSON values;
valid entries can become active when their command is available. Duplicate entries for one
command are all inactive. Unknown root metadata survives round trips. Preservation is semantic
JSON preservation, not byte-for-byte formatting. Unsupported whole-document versions and
oversized input are rejected rather than rewritten.

`validate_override` reports same-precedence conflicts, visible cross-scope shadowing, unavailable
commands, and host-supplied platform/text reservations without applying a choice. The editor must
present the explicit choices and call `override_bindings` only for the selected change. Actual
input resolution also detects logical/physical overlaps that depend on keyboard layout.

The registry admits 4,096 commands and 32 defaults per command. A resolution admits 64 uniquely
named scopes with 256 bindings each. Persisted keymaps admit 64 KiB and 1,024 entries; a malformed
entry with more than 32 bindings stays inactive. Programmatic overrides reject more than 32.

## Host integration and legacy migration

A keyboard host first gives required editing and IME operations their existing precedence,
then submits `CommandRequest::Input`. Only `CommandDispatchStatus::Unhandled` permits a legacy
`ShortcutCatalog` fallback. Other outcomes are terminal, even when they contain no message.
The existing legacy catalog is unchanged; installing a semantic registry does not silently
reinterpret its physical `KeyCode` values as logical characters.

Menus and other semantic surfaces submit `CommandRequest::Target` with the original target and
source, rather than retaining an already-mapped message that could outlive its context. Observe
the returned `CommandOutcome` using the host's normal repaint and command scheduling path.
The native adapter keeps captured widget-key sequences and their retirement fence ahead of
semantic commands. It preserves logical text and full positional codes independently of the
legacy key subset, applies the registered repeat policy, and reserves required text-editing
keys even when clipboard access fails. A command consumed by the semantic host cancels any
pending legacy chord. Native tests cover the shared routing seam without opening an OS window;
they are not OS-level keyboard, menu, or IME acceptance evidence.

Run `cargo run --example contextual_commands` for a headless application/reducer example.
Core tests cover precedence, disabled fallthrough, conflict rejection, identity retirement,
logical/physical distinctions, repeat/text/IME policy, loss-preserving persistence, localization,
and actual reducer dispatch. Native routing tests additionally cover actual composition
ownership, logical/physical conflicts, text precedence and captured-key priority. They do not
constitute OS keyboard or menu acceptance.
