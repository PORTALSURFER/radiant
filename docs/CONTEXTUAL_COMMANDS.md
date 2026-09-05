# Contextual commands

`radiant::application` provides static command registration, immutable active scope snapshots,
a data-only keymap, presentation queries, and one invocation-to-message mapper. The explicit
host boundary is `SurfaceRuntime::dispatch_command_request`; the application builder's
`.command_registry(registry, dispatcher)` connects committed view scopes to the ordinary reducer.
The advanced `.commands(registry, project, dispatcher)` hook also accepts manual scope snapshots.

The Winit keyboard adapter submits native presses through this boundary for runtimes with a
semantic command host. Menu, toolbar and palette controls consume the shared presentation
and revalidate their opaque targets on activation. Native auxiliary windows inherit the
registered resolver and keymap. Native adapters query current command presentations and
submit opaque activations through the same runtime boundary.

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

## Native presentation queries

`runtime.command_presentations(&ids, platform)` reads one current keymap, committed scope
projection and resolved environment for up to 256 items, in request order. It needs no
application context type, raw focus inspection, mapper call or reduction. Unknown commands,
invalid scopes, exceeded capacity and unavailable services reject the whole batch through
`CommandPresentationError`. Disabled or conflicting bindings retain static metadata without
an enabled activation. Repeated IDs are allowed for commands appearing in several menus.

Native adapters use each presentation's label, checked/enabled state, accessibility text and
platform shortcut forms, then associate `activation(CommandSource::Menu)` with the native
item. Submit its `request()` through `dispatch_command_request` when activated. Never retain
an already-mapped domain message. This is the shared platform projection boundary; the
embedding native adapter owns OS menu objects and event delivery. The command module does
not install OS menus or establish OS-level menu acceptance. Exported `CommandService` values
also provide `presentations` for custom hosts, with the same bounds and target qualification.

## Command controls

A `CommandPresentation` builds `toolbar_button`, `menu_item`, `palette_item`, or passive
`shortcut_help` views without another label, binding or enabled callback. The first three
retain only a qualified activation; the ordinary runtime input path resolves it through the
registered mapper and then reduces one message. They require no `Message: Clone` bound.
Disabled presentations remain visible but cannot focus or dispatch. Checked state uses the
control's selected treatment; accessibility uses the registered accessible label and expanded
shortcut description. Help lists all effective bindings with expanded platform key names.

Project the presentation again when locale, keymap, availability or captured scope changes.
An old visible control cannot bypass current scope validation: a replaced or newly shadowed
target is rejected. Changing a target or presentation source while its control is pressed
cancels that press; release cannot invoke the replacement context. `activation(source)`
returns an owned, opaque `CommandActivation` whose
`request()` can be submitted by a native adapter through `dispatch_command_request`; it does
not install an OS menu or run the mapper itself. A standalone `UiSurface` does not execute
semantic activations because it has no registered application command host.

Moving focus from an editor to command controls preserves that editor's command context.
The runtime keeps actual keyboard focus on the control and qualifies the retained editor
against the current tree. Normal focus changes, window focus clearing, removal, incompatible
replacement and omitted layout retire that retained context. Traversing between command
controls preserves it; visiting an unrelated control replaces it. Selection/window scopes
remain explicitly application-owned and do not become implicit selections.

## Child surfaces

Native auxiliary windows receive the parent's declarative `CommandService` with each child
projection. A child resolves its own committed scopes, focus ancestry and layers, then queues
the mapped message for the existing owner-qualified parent reduction path. It does not inherit
the parent's editor focus or scope list. Service/keymap updates are installed with the new
child surface; retired or unavailable native children retain the existing input admission fences.

`runtime.command_service()` exports the current resolver and keymap snapshot while the runtime
accepts work. Advanced embedders can also construct `CommandService::new(registry, dispatcher,
keymap)` and use `resolve(request, projection)` from `RuntimeInputHost::resolve_command_with_scopes`.
Cloning the service requires neither cloneable context nor cloneable messages. The service is a
UI-local value, not a parent lifecycle lease: an embedding host must admit input and forward
messages through its own lifecycle boundary. Refresh its snapshot when the parent's keymap
changes. The advanced manual `.commands(registry, project, dispatcher)` hook does not export
parent scope callbacks by default; custom hosts can explicitly supply a service through
`RuntimeInputHost::command_service`.

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

To migrate a `ShortcutCatalog` caller:

1. Register each semantic action once with a stable `CommandId`, `TextKey` metadata and
   explicit logical or physical defaults. Use physical defaults when preserving positional
   legacy behavior is intentional.
2. Move dynamic availability, checked state and context into view `.commands(...)` bindings
   or prepared named scopes. Keep the reducer's domain checks and undo policy.
3. Register `.command_registry(registry, dispatcher)` with one invocation-to-message mapper.
4. Build menus, toolbar controls, palettes and help from shared presentations; load optional
   persisted overrides through `.command_keymap(...)`.
5. Remove the corresponding legacy bindings after validating their semantic replacements.
   During migration only `Unhandled` reaches the old catalog, preventing duplicate actions.

Run `cargo run --example contextual_commands` for a headless application/reducer example.
Core tests cover precedence, disabled fallthrough, conflict rejection, identity retirement,
logical/physical distinctions, repeat/text/IME policy, loss-preserving persistence, localization,
and actual reducer dispatch. Native routing tests additionally cover actual composition
ownership, logical/physical conflicts, text precedence and captured-key priority. They do not
constitute OS keyboard or menu acceptance.
