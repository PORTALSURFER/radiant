# Radiant Core API

Radiant is a reusable declarative GUI library. Host applications own domain
state and business logic; Radiant owns view-tree identity, layout, input
routing, focus, style resolution, invalidation, renderer-facing paint plans,
typed platform services, and business-work scheduling.
Radiant targets macOS, Windows, and Linux through native Wayland. The current
native implementation is macOS-first, while backend-neutral GUI contracts and
host boundaries are designed for cross-platform use across all three in-scope
platforms. X11 is an explicit non-goal. See `docs/TARGET.md` for the
modern-system matrix and the
CI-versus-native acceptance boundary. See the
[Platform Acceptance and Evidence Policy](PLATFORM_ACCEPTANCE.md) for the
canonical lane, outcome, session, and artifact rules.
For a contributor-facing map of subsystem ownership, rendering/text/platform
boundaries, and validation lanes, see `docs/ARCHITECTURE.md`. For the preferred
shape of application-facing APIs, examples, and cleanup tickets, see
`docs/API_STYLE.md`. For custom-host timer integration after the opaque-wake
change, see [the timer host API migration](migrations/TIMER_API_MIGRATION.md).

## Document Status And Authority

This document describes the currently shipped application-facing API and calls
out compatibility or migration paths where they remain public. It is not a
claim that Radiant already implements the target architecture. For new
architecture and API decisions, `docs/DESIGN_DIRECTION.md` is the normative
target-state contract; `docs/TARGET.md` remains the broader product boundary
and incremental-delivery direction. When a target example uses a name that is
not yet shipped, this document's current API spelling and migration note take
precedence for code that must compile today.

Only canonical merged source counts for shipped status; branch, draft,
acceptance-only, and unverified evidence do not. X11 and product-specific
behavior remain explicit non-goals for Radiant.

## Dependency Boundary

The dependency direction is host application to Radiant. Radiant default builds
must not depend on host crates, host modules, product assets, or product-domain
model names. In short: host -> Radiant, never Radiant -> host. Radiant now
exposes only generic GUI and native runtime APIs; host-shaped compatibility
facades and native-shell composition trees belong in the consuming application.
Boundary tests prove that dependency direction, public exports, examples, and
runtime behavior stay generic; they intentionally avoid enforcing product
neutrality through lists of forbidden host-domain words.

Radiant exposes one public API with progressive control. Application builders
and explicit runtime objects are part of the same API surface:

- `radiant::prelude` collects the common imports for readable application code.
- `radiant::window("Title").size(...).run(text("Hello"))` for no-state apps.
- `radiant::app(State::default()).view(...).update(...).run()` for small
  stateful apps, with `.handle_message(...)` when handlers need
  `UiUpdateContext`.
- `radiant::runtime`, `radiant::widgets`, `radiant::layout`, `radiant::theme`,
  and `radiant::gui` expose the same model with more explicit control over
  projection, explicit runtime commands, sizing, layout, styling, input, invalidation, and
  backend integration.

Radiant's cleanup target is message-first, non-blocking application code: views
emit explicit messages, update handlers own durable state changes, and any
business work must be scheduled through Radiant. Reducer-style aliases remain
available for advanced lifecycle code during the breaking migration, but
ordinary application code should stay message-first. See `docs/API_STYLE.md`.

## Application API

### Controlled scrolling

`scroll(content)` owns its live logical offset in the runtime. Use
`ScrollPolicy` for axis, locking, scrollbar placement and visibility, and use
`initial_offset` only to seed a newly mounted container. `controlled_offset`
accepts a strictly newer `Controlled<Vector2>` generation; `scroll_request`
consumes each generation once after resolving a materialized key, rectangle, or
edge. `on_offset_settled` is called once after an accepted offset settles, so
applications can persist the resulting value without driving every wheel
update through application state.

```rust
use radiant::layout::{ScrollAxis, ScrollPolicy, ScrollbarPlacement, Vector2};
use radiant::prelude::*;

scroll(content)
    .scroll_policy(
        ScrollPolicy::default()
            .axes(ScrollAxis::Vertical)
            .scrollbar_placement(ScrollbarPlacement::Reserved),
    )
    .initial_offset(Vector2::new(0.0, 96.0))
    .on_offset_settled(Message::ScrollSettled);
```

Configure the raw layout and chrome axes with `.axes(...)`, and inspect that
selection with `ScrollPolicy::configured_axes()`; the default retains legacy
horizontal offset mutation authority separately.

The runtime validates generations, finite geometry, mount identity, and
current committed layout evidence before mutating scroll state. A stale,
malformed, unavailable, or no-op request is consumed silently. Focus reveal,
wheel chaining, keyboard page/Home/End commands, and horizontal or vertical
scrollbar interaction use the same policy and committed geometry.

Explicit `ScrollPolicy` axes constrain both declarative offsets and retained
runtime state: disabled components are silently projected to zero for mount
seeds, newer controlled values, and policy changes. `ScrollPolicy::default()`
keeps the legacy two-axis declaration behavior; re-enabling an axis does not
restore a component discarded by an earlier policy without a newer input
generation. Policy-only normalization preserves accepted generations; a newer
valid controlled value consumes its generation even when projection leaves the
effective offset unchanged.

Radiant's application API is designed to be easy to read without hiding the
runtime model. Application code imports `radiant::prelude::*`, declares view
structure, emits explicit messages from widgets, and mutates durable state in
the update handler. The helper and export inventory is documented later in
[Prelude And Helper Reference](#prelude-and-helper-reference) so this section
can stay focused on the canonical reader path.

No-state apps can launch without naming `NativeRunOptions`, `RuntimeBridge`,
`UiSurface`, `SurfaceNode`, `SurfaceChild`, or `WidgetSizing`:

```rust
use radiant::prelude::*;

fn main() -> radiant::Result {
    radiant::window("Radiant Hello World")
        .size(320, 120)
        .run(text("Hello, world!"))
}
```

Tests, automation, and embedded previews that only need to inspect one view can
prepare layout and paint frames directly from any `IntoView` value with
`view_layout(...)`, `view_layout_at_size(...)`, `view_frame(...)`,
`view_frame_at_size(...)`, `view_frame_with_default_theme(...)`, or
`view_frame_at_size_with_default_theme(...)`.
This keeps simple app-facing checks on the declarative view path without
manually wrapping views in `UiSurface`.
Focused widget and mapper tests can also call
`view_dispatch_widget_output(...)` and `view_dispatch_widget_input(...)`
directly on an `IntoView` value when the test only needs to verify one projected
view's widget-message mapping or input behavior.

`IntoView::into_projection()` is the lossless stateful-application boundary. It
returns a `ViewProjection<Message>` containing the lowered `UiSurface` together
with Scene frame-clock, transient-overlay, and shortcut bindings. Custom
wrappers must delegate this required method to their wrapped value:

```rust
use radiant::prelude::*;

struct WrappedView<Message>(ViewNode<Message>);

impl<Message: 'static> IntoView<Message> for WrappedView<Message> {
    fn into_projection(self) -> ViewProjection<Message> {
        self.0.into_projection()
    }
}
```

A stateful app may also return `ViewProjection<Message>` directly when it wants
to lower a Scene before returning from `.view(...)`. Bare `SurfaceNode` and
`UiSurface` values do not implement `IntoView`; low-level adapters must make
metadata rejection explicit with `ViewProjection::from_surface(...)`. Calling
`IntoView::into_node()`, `IntoView::into_surface()`, or
`ViewProjection::into_surface()` intentionally strips application-only Scene
lifecycle bindings for layout, paint-frame, test, and low-level host use; do not
round-trip a Scene through those surface-only methods before returning it from a
stateful app projection.

Small stateful apps should use the same message-first model as larger apps.
Widgets emit explicit messages, and the update handler owns durable state
changes. The normal `.view(...)` projection receives `&State`; prepare derived
host state before launch or in the update path, because view construction cannot
mutate it:

```rust
use radiant::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq)]
enum Message {
    Increment,
}

#[derive(Default)]
struct State {
    count: usize,
}

fn main() -> radiant::Result {
    radiant::app(State::default())
        .title("Counter")
        .size(320, 120)
        .view(|state: &State| {
            column([
                text(format!("Count: {}", state.count)),
                button("Increment").message(Message::Increment),
            ])
        })
        .update(|state, message| match message {
            Message::Increment => state.count += 1,
        })
        .run()
}
```

This message-first shape is the canonical style for new examples and host
applications. Use `.handle_message(...)` when an update handler needs
`UiUpdateContext<Message>` to emit follow-up messages, request repaint, move
focus, schedule business work, request typed platform services, schedule delayed
messages, or request runtime exit. Reducer-style aliases remain available for
advanced lifecycle control during the breaking migration, but ordinary
application code should stay message-first.

Ordinary application `Message` values are owned and reduced on the UI thread, so
they do not need to implement `Send` or `Sync` and may contain UI-local `Rc` or
`RefCell` state. Cross-thread APIs instead require their request, payload, or
worker result to implement `Send`; their completion mapper stays on the UI owner
and may produce a UI-only application message.

Immutable application labels use `TextContent`, which normal builders accept
through `Into<TextContent>`. Literals such as `text("Ready")` and
`button("Play")` remain in static storage through paint-plan construction.
Owned `String` values move into shared storage once, while an existing
`Arc<str>` is cloned without copying its bytes:

```rust
use radiant::prelude::*;
use std::sync::Arc;

let status: Arc<str> = Arc::from("Ready");
let view: View<()> = column([
    text("Static label"),
    text(String::from("Owned label")),
    text(Arc::clone(&status)),
]);
```

This applies to normal text, button, badge, toggle, selectable, menu,
dropdown, option-list, details-list, panel, and other immutable display
content. Text-input values remain owned `String` state because editing requires
mutable value and selection ownership; immutable placeholders and completion
suffixes use `TextContent`. A non-static borrowed `&str` must be converted to an
owned `String` or shared `Arc<str>` before it enters an owned view tree.

### Non-Blocking App Contract

Radiant application handlers are UI reducers. They run on the UI/event/render
path and must stay short. A handler may mutate durable host state, apply a
business or platform-service result, emit follow-up messages, request repaint or
paint-only repaint, move focus, schedule timers or debounced messages, request
typed platform services, and schedule host-owned business work through
`context.business()`.

Handlers must not run business work directly. Filesystem and database access,
audio/image/data decoding or loading, cache hydration, network or process
work, sleeps, blocking waits or joins, thread creation, long CPU transforms,
and helper calls that hide those operations must leave the UI path through
`context.business().interactive(...)`, `.background(...)`, `.blocking_io(...)`,
or `.idle(...)`. When host policy already resolved a `TaskPriority`, use
`context.business().priority(name, priority)` instead of repeating a local
priority-to-lane match in app code.
Worker closures receive `radiant::runtime::BusinessWorkContext` as an explicit
runtime capability so helper signatures can inspect cooperative cancellation
without importing it from the normal app prelude or constructing it in UI code.

The qualified one-shot `BusinessRequest::run_for_owner_with_receipt(owner, work, map)`
resolves the explicit `DeclarativeEffectOwner` only against one current
accepted keyed-node or overlay owner after fresh-surface reconciliation. Absent,
ambiguous, unkeyed, stale, retired, or incompatible handles are rejected without
spawning or mapping. Accepted work is fenced to the current private owner
generation, and `BusinessWorkContext` cancellation checks plus late UI
mapper/reducer fencing apply. The matching
`BusinessLatestRequest::run_for_owner_with_receipt(owner, work, map)` reuses the
existing latest transaction, effect identity/generation, receipt, and owner
ledger. Failed owner or host admission rejects without retry or fallback and
restores the eligible predecessor ticket; accepted completion maps once only
while the latest ticket and owner generation remain current. These are the
shipped one-shot owner-worker routes. The qualified
`CancellableBusinessRequest::run_for_owner_with_receipt(owner, work, map)` is the
token-cancellable ordinary owner one-shot. Callers clone `request.token()` before
consuming the request; its explicit token and declarative owner retirement probes
are OR-composed fences for cooperative work, deferred mapping, and reduction.
Only this token-cancellable owner one-shot defers mapping until UI drain;
application-owned and non-cancellable owner one-shots retain eager mapping. Its
`BusinessTaskAdmissionReceipt` is admission-only and its UI-local mapper need not
be `Send` or `Sync`. Invalid, removed, ambiguous, unkeyed, incompatible, stale,
same-update, host, capacity, and closing admissions reject without spawn, mapping,
retry, or `Application` fallback. The qualified
`BusinessRequest::latest_for(&mut keyed_tasks, key).run_for_owner_with_receipt(owner, work, map)`
route is the application-owned keyed-latest one-shot variant. It retains the
exact host key, keyed latest ticket, replacement transaction, declarative owner
generation, and receipt; its mapper receives exactly one
`KeyedTaskCompletion<Key, Output>` only while both keyed-latest and owner
generation remain current. Owner retirement and keyed supersession are
OR-composed cancellation and late-publication fences. Invalid owner, lifecycle,
host, or capacity admission rejects without spawning, mapping, reducing,
retrying, or falling back to `Application`, and restores the eligible
predecessor only for the affected key. The capability-specific keyed wrapper
does not exist on `latest_for_resource(...)` or `exclusive_for(...)`; shared
`ResourceTasks` remains application-owned. The qualified
`BusinessRequest::latest_for(&mut keyed_tasks, key).stream_for_owner_with_receipt(owner, work, map_event, map_final)`
route is the shipped ordered application-owned keyed-latest stream. Every
accepted intermediate and final mapper receives the exact host key and keyed
ticket in `KeyedTaskCompletion<Key, Event>` or
`KeyedTaskCompletion<Key, Output>`; intermediate events remain FIFO and the
final maps once after the last accepted event. Keyed supersession and owner
retirement independently fence worker, mapping, and reduction, while invalid
owner, lifecycle, host, or capacity admission restores only the affected
key's predecessor without fallback. The coalesced keyed-latest owner stream
retains the exact host key, keyed ticket, replacement transaction, owner
generation, and receipt; it retains only the newest pending intermediate
payload before UI drain and delivers the uncoalesced final exactly once after
that event. Its exact `KeyedTaskCompletion<Key, _>` mappers remain UI-local/
non-`Send`; keyed supersession and owner retirement independently fence worker,
mapping, and reduction. Invalid, removed, ambiguous, unkeyed, incompatible,
stale, host, capacity, closing, and same-update admissions fail closed without
`Application` fallback and restore only the affected key's predecessor; sibling
keys remain unchanged. The qualified
`BusinessRequest::stream_for_owner_with_receipt(owner, work, map_event,
map_final)` is the shipped ordinary ordered owner-scoped stream route. The
qualified `BusinessRequest::stream_latest_for_owner_with_receipt(owner, work,
map_event, map_final)` is the shipped ordinary coalesced owner-scoped stream
route: before UI drain it keeps at most one pending intermediate payload and one
queued latest marker, replaces a pending event with a newer event, and records
the existing coalescing diagnostic. Separately accepted events may map when the
UI drains between emissions. Both routes map the final exactly once after the
last accepted intermediate event and pass the controller-composed owner
cancellation probe into `BusinessWorkContext`; event and final mappers remain
UI-local and may capture `Rc`/`RefCell`. The shipped
`BusinessLatestRequest::stream_for_owner_with_receipt(owner, work, map_event,
map_final)` route adds the same ordered event/final behavior to one latest-task
ticket: both mappers receive the exact `TaskCompletion` ticket, and owner
retirement or latest supersession fences late mapping and reduction. Invalid
owner or host admission rolls back the predecessor latest ticket without
spawning or mapping. The shipped
`BusinessLatestRequest::stream_latest_for_owner_with_receipt(owner, work,
map_event, map_final)` route composes the same latest-task ticket with the
existing coalesced intermediate-event slot: both mappers receive the exact
`TaskCompletion` ticket, the final is delivered once after the retained event,
and latest supersession or owner retirement fences late mapping and reduction.
Invalid owner or host admission rolls back the predecessor latest ticket without
spawning or mapping. The cancellable ordinary ordered owner-stream route is
available on `CancellableBusinessRequest`: callers clone `request.token()` before
consuming the request, and the admission-only receipt retains the ordinary FIFO
event/final contract. The explicit token and declarative owner probe are
OR-composed, so token cancellation and owner retirement independently fence
cooperative work, later events, final mapping, and reduction, including later
entries queued for one UI drain. The
`CancellableBusinessRequest::stream_latest_for_owner_with_receipt(owner, work,
map_event, map_final)` route uses the same owner generation and cancellation
fences with the existing bounded latest-wins ingress: before UI drain it keeps
one pending intermediate payload and one queued marker, replaces older pending
events, records the existing coalescing diagnostic, and maps the uncoalesced
final exactly once after the retained event. Events separated by a UI drain map
separately. Invalid, removed, ambiguous, unkeyed,
incompatible, stale, same-update, host, capacity, and closing admissions reject
atomically without spawning, mapping, retry, or `Application` fallback; event and
final mappers remain UI-local/non-`Send`.

The cancellable latest-task owner routes are
`CancellableBusinessLatestRequest::run_for_owner_with_receipt(owner, work, map)`,
`CancellableBusinessLatestRequest::stream_for_owner_with_receipt(owner, work,
map_event, map_final)`, and
`CancellableBusinessLatestRequest::stream_latest_for_owner_with_receipt(owner,
work, map_event, map_final)`. They reuse the existing latest ticket and
replacement transaction and compose the explicit token with the declarative
owner-generation fence for cooperative work, delivery, mapping, and reduction.
The one-shot maps one completion; the ordered stream preserves FIFO events and
final delivery; the coalesced stream keeps only the newest pending intermediate
event before UI drain and delivers the final uncoalesced. Each receipt is
admission-only and the UI-local mappers remain non-`Send` where permitted.
Invalid, removed, ambiguous, unkeyed, incompatible, stale, same-update, host,
capacity, and closing admissions fail closed without spawn, mapping, retry, or
`Application` fallback; failed latest admission restores the predecessor ticket.
Resource ownership beyond the application-owned `KeyedLatestTasks` route,
including `ResourceTasks`, and platform ownership remain separate and deferred;
owner timers remain a separate shipped consumer.

Radiant runs interactive, background, blocking-IO, and idle business work on separate
runtime-owned lanes so user-visible interactive work is not queued behind
background, blocking-IO, or idle jobs that are already running. Use
`blocking_io(...)` for explicit filesystem, database, process, cache-hydration,
and other blocking IO work that should run on a limited lane instead of sharing
ordinary background capacity. Long workers should call
`BusinessWorkContext::checkpoint()`, `check_cancelled()`,
`yield_if_elapsed(...)`, or `fail_if_over_budget(...)` at natural chunk
boundaries so cancellation and checkpoint diagnostics stay meaningful.
Resource-scoped work should use `ResourceKey` with `ResourceTasks` and the
business request policies `latest_for_resource(...)` or `exclusive_for(...)`.
Build keys with `ResourceKey::scoped(scope, identity)` for stable host-owned
classes such as documents, cache entries, folders, devices, or viewports, and
`ResourceKey::path(scope, path)` when the host has already chosen path display
text as the resource identity. `ResourceKey::new(...)` remains available for
advanced hosts that already own a complete opaque key.
Use latest resource work when the newest request for a file, document, cache
entry, device, or viewport should win; use exclusive resource work when
duplicate loads for the same key should be rejected until the active request
finishes or is cancelled. Keyed streaming workers tag intermediate and final
messages with both the resource key and task ticket so stale progress,
preview-ready, playback-ready, and final messages can be ignored without
app-local ticket plumbing. Use `LatestTask::is_active_completion(...)`,
`LatestTask::finish_completion(...)`,
`KeyedLatestTasks::is_active_completion(...)`,
`KeyedLatestTasks::finish_completion(...)`,
`ResourceTasks::is_active_completion(...)`, and
`ResourceTasks::finish_completion(...)` when reducers receive
`TaskCompletion` or `KeyedTaskCompletion` values; these helpers keep ticket
validation and output extraction in one generic task API.
Platform interactions such as file dialogs, reveal/open, clipboard text and
file-list reads/writes, confirmation prompts, and native handoffs must use
typed Radiant platform services instead of direct blocking calls from handlers.

Host applications should enforce this boundary with a static guardrail test.
Use `radiant::guardrails::NonBlockingGuardrail::app_update_paths()` over the
application's UI/update/action/view roots, add host-specific forbidden tokens
with `.forbid_token(...)`, and keep `.allow_path_fragment(...)` entries limited
to explicit worker, business-runtime, or typed platform-adapter modules. The
report includes file and line numbers and points developers back to
`UiUpdateContext::business()` or typed platform services.
Runtime slow-handler diagnostics are the second line of defense for work that
static scans cannot see, such as heavy CPU loops, lock contention, or helpers
with innocent names. Test and development harnesses can call
`SurfaceRuntime::set_update_handler_diagnostics_policy(...)` with
`UiUpdateHandlerDiagnosticsPolicy::panic_at(threshold)` to fail when an update
handler exceeds a controlled threshold. The default policy is warn-only in
debug/test builds and disables the timing read in release builds unless a host
explicitly opts in.

This contract is mandatory for normal Radiant applications. During the current
breaking migration, older command-returning or generic command-injection paths
may still exist for compatibility, tests, or embedders, but they are not the
target app-facing architecture and are scheduled for removal or isolation behind
advanced-only surfaces. Wavecrate is the current consumer, so compatibility
with old app-facing task/spawn/command APIs is not a design constraint.

Application builders generate deterministic structural IDs during projection and
provide default widget sizing. Production apps and tests can opt back into
explicit control with `.id(...)`, `.sizing(...)`, `.size(...)`, `.fixed(...)`,
`.min_size(...)`, `.preferred_size(...)`, `.baseline(...)`, and `.spacing(...)`.
Use `empty()` for optional branches that must return a view without
contributing visible layout size; use `spacer()` when the view should reserve a
non-painting fixed or flexible gap. Use `fixed_slot_opt(...)` or
`fixed_slot_if(...)` when optional content should keep a fixed-width and
fixed-height control slot while absent. Use
`text_input(value).clear_button(message)` when a search/filter input needs a
reserved clear-button slot without app-local row assembly; `.id(...)` or
`.key(...)` on that builder identifies the text input and Radiant derives the
clear affordance identity. Use
`text_input(value).revision(TextInputRevision::new(n))` when a host-controlled
single-line value needs explicit authority evidence across reprojection; import
`TextInputRevision` from `radiant::widgets` because it is intentionally not in
the common prelude. A strictly newer revision applies the projected value and
selection, while an equal or older revision preserves retained editing state.
This is a single-line authority prerequisite only; composition, multiline,
clipboard, undo, and native accessibility remain separate capabilities. The
target text contract nevertheless requires platform adapters to translate
pre-edit, commit, and cancellation into backend-neutral composition state.
Use
`text_line(label, height)` for
fixed-height single-line labels that should fill their parent width and truncate
rather than wrap. Use `children().push(...).push_opt(...).push_if(...)` when a
row, column, grid, stack, or similar container has a short declarative child
list with optional branches. This keeps conditional children at the container
composition site without app-local temporary vectors or optional layout widgets.
Rows, columns, and fixed-column grids use intrinsic main-axis child sizing by
default, so list rows and grid tiles do not stretch just because there are only
a few items. Apps can request
stretch behavior explicitly with `.fill()`, `.fill_width()`, `.fill_height()`,
and `.grow(...)`, add container padding with `.padding(...)`, `.padding_x(...)`,
and `.padding_y(...)`, and use `.primary()`, `.danger()`, `.subtle()`,
`.wrap()`, `.truncate()`, or `.align_text(TextAlign::Center)` for common style
and text policies. Use `resizable(content).resize_handle(Message::Resize)` when
a content region should own its trailing resize drag handle instead of adding an
adjacent `drag_handle()` sibling by hand. Use
`resizable(content).subtle_resize_handle("stable-key", Message::Resize)` for a
standard subtle hover-only resize handle with stable identity. Stateful
examples should use stable keys or explicit IDs for controls whose focus or
input state must survive list edits. The launch builders expose `.options(...)`
for callers that need the full `NativeRunOptions` surface. Normal apps should
use `.message(...)` on widgets plus `.update(...)` for state-only handlers or
`.handle_message(...)` when they need `UiUpdateContext` capabilities. Native OS
file-drop targets should be declared on the view subtree that owns the
interaction:
`.accepts_native_file_drop().on_native_file_drop(Message::FileDrop)`.
Radiant routes hover, cancel, and drop events to the topmost accepting target
using the normal surface traversal and attaches `NativeFileDrop::target_widget`
before emitting the host message. Use `NativeFileDropPhase` to distinguish the
event phase. Both callback payload types are part of the common prelude. The
app-builder `.on_native_file_drop(...)` hook remains available
as an advanced compatibility fallback for hosts that intentionally handle
targeted drops outside the view tree. Interactive row and badge builders can use
`InteractiveRowActions` when they only need common activation, secondary-click,
drag, drop, or hover-drop routing without hand-written enum filtering. Use
`InteractiveRowBuilder::tracked_drag_source(...)` when host-owned row drag
state should configure the common draggable, drag-active, drag-source, and
pointer-motion policy together. Use
`InteractiveRowBuilder::tracked_drag_source_with_motion(...)` when the active
source is retained from host state and should keep emitting pointer movement
after projection. Retained tracked rows automatically clear stale pressed and
drag state when host synchronization moves them from an active drag/source state
to idle or non-source, so apps do not need to churn row identity after drag
cancellation just to reset transient input paint. Use
`InteractiveRowUnderlayBuilder::tracked_drop_target(...)` when arbitrary
visible row content should keep its own paint tree while the transparent
interactive-row underlay owns standard tracked drop-target behavior. Use
`InteractiveRowUnderlayBuilder::tracked_drop_candidate(...)` for the same
conditional drop-target lifecycle through an underlay row without dropping to
`.row(|row| ...)`. Use
`InteractiveRowBuilder::tracked_drop_candidate(...)` with
`InteractiveRowActions::tracked_drop_candidate_key(...)` when host-owned
candidate validation needs Radiant to route both target hover and stale-target
clear intents without app-local hover filtering.
Context-aware app code should use `.handle_message(...)` with an
`UiUpdateContext<Message>` to emit messages, request repaint, move focus, schedule
business work, request typed platform services, schedule delayed messages, or
request runtime exit. Radiant does not keep compatibility aliases for this hook
on the normal app-facing path; use `.handle_message(...)` so the UI context
capability boundary is explicit at the call site. Use
`.repaint_policy(...)` with `RepaintPolicy` only when ordinary app messages
need custom automatic repaint behavior. Ordinary app messages request a
surface repaint by default unless the handler explicitly requests surface or
paint-only repaint. Frame-clock messages use their `FrameClock::repaint_scope`
policy first, so apps do not need to exclude frame messages from
`RepaintPolicy`.

## Prelude And Helper Reference

Normal application code should start with `radiant::prelude::*`. The prelude is
a grouped facade over application builders, backend-neutral GUI helpers,
runtime commands/resources, widgets, layout signatures, and theme tokens. It is
not a separate framework: every builder lowers into the same `UiSurface`,
`SurfaceNode`, `SurfaceChild`, `WidgetSizing`, and `RuntimeBridge` contracts
available through the explicit `radiant::runtime`, `radiant::widgets`,
`radiant::layout`, `radiant::theme`, and `radiant::gui` modules.

The prelude boundary is intentionally conservative. Its focused source groups
are reviewed allowlists, and a source guardrail bounds their combined named
surface so splitting an oversized group into more files cannot bypass review.
It contains app-facing types that ordinary declarative UI code reaches for
repeatedly: builders, messages, core widget contracts, geometry used in
signatures, theme tokens, the paint and overlay types required by common trait
signatures, and typed runtime commands. Named-parts constructors, specialist
details-list and virtual-tree manipulation, low-level paint construction, raw
platform protocols, external-drag models, advanced host control, paint-plan
inspection, GPU/custom-shader surfaces, native diagnostics and run reports,
retained projection machinery, and specialist visualization geometry require
explicit imports from their owning modules.
Advanced host-control APIs, renderer or windowing implementation details, and
platform-specific adapters never enter the common wildcard surface.

The reviewed inventory currently contains 433 named exports. The guardrail cap
is 477, leaving 44 exports (10.2% of the current surface) for genuinely common
future API without forcing local reshuffles. Source-quality tests compute and
verify both the aggregate and this per-subsystem inventory:

| Prelude subsystem | Named exports | Ordinary caller role |
| --- | ---: | --- |
| Application | 245 | Canonical app/view/control builders and required signature types |
| GUI | 102 | Common state/update models, geometry, input, text, and list policies |
| Layout | 1 | Layout signature output |
| Runtime | 32 | Common commands, resources, platform-service inputs/results, and callback signature types |
| Theme | 3 | Theme signature tokens |
| Widgets | 50 | Common widget contracts, messages, sizing, and style models |

| API family | Prelude disposition | Explicit owner when excluded |
| --- | --- | --- |
| Application/view builders and their signature support | Included | `radiant::application` |
| Basic controls, layout, overlays, lists, menus, theme, and geometry | Included | Owning `radiant::application`, `radiant::gui`, `radiant::layout`, or `radiant::theme` module |
| Core custom-widget contracts plus `PaintPrimitive` and `TransientOverlayContext` signature types | Included | `radiant::widgets` and `radiant::runtime` |
| Named-parts constructors and `_from_parts` entry points | Excluded | The owning `radiant::application`, `radiant::gui`, `radiant::runtime`, or `radiant::widgets` module |
| Details-list drag, resize, placement, sortable-list, and virtual-tree helpers | Excluded | `radiant::application` |
| Raw platform request/response protocols and external-drag models | Excluded | `radiant::runtime` |
| Generic normalized color ramps | Included | `radiant::gui::visualization` |
| Native options, diagnostics, run errors, and run reports | Excluded | `radiant::runtime` |
| GPU surfaces, retained canvases, custom shaders, and window manifests | Excluded | `radiant::runtime` |
| Paint plans, SVG parsing errors, paint emitters, and primitive query helpers | Excluded | `radiant::runtime` |
| Timelines, grids, axes, canvas selection, and other specialist visualization geometry | Excluded | `radiant::gui::visualization` |
| Concrete low-level widgets and widget construction parts | Excluded | `radiant::widgets` |
| Badge/flow geometry and dense-row paint helpers beyond builder signature support | Excluded | The owning `radiant::gui` module |

Examples may import `radiant::runtime`, `radiant::widgets`, `radiant::layout`,
`radiant::theme`, or `radiant::gui` beside the prelude when they demonstrate
custom widgets, tests, retained surfaces, diagnostics, or other advanced
control; that explicit import is the signal that the example has moved beyond
the common app import set. For example:

```rust
use radiant::gui::visualization::TimelineViewport;
use radiant::prelude::*;
use radiant::runtime::{NativeFrameDiagnostics, SurfacePaintPlan};
```

| Area | Common prelude entries |
| --- | --- |
| Application setup | `window`, `app`, `IntoView`, `View`, `UiUpdateContext`, `EmbeddedFont` |
| Basic views | `text`, `button`, `button_row`, `toolbar`, `row`, `column`, `scroll`, `scroll_column`, `list`, `list_row`, `empty`, `spacer`, `toggle`, `text_input`, `dropdown_trigger`, `custom_widget` |
| Widget authoring | `Widget`, `WidgetCommon`, `WidgetSizing`, `WidgetInput`, `WidgetOutput`, `WidgetPaintContext`, `PointerButton`, `FocusBehavior`, `ActivationInputPolicy`, `ColorMarkerProps`, `ColorMarkerAlign`, `handle_activation_input` |
| Common row and list policy | `TreeGuideRow`, `TreeGuideMetrics`, `TreeGuideStyle`, `StyledTreeGuideStyle`, `DenseRowPalette`, `DenseRowMarkerStyle`, `DenseRowOutlineStyle`, `VirtualListWindow` |
| Geometry and theme | `Rect`, `Point`, `Vector2`, `LayoutOutput`, `ImageRgba`, `ImageRgbaError`, `Rgba8`, `ThemeTokens` |
| Generic chrome and feedback | `StatusSegments`, `StatusLineLog`, `StatusLineEntry`, `ContentViewChrome` |
| Input and scroll payloads | `NativeFileDrop`, `NativeFileDropPhase`, `ScrollUpdate`, `ScrollUpdateMetadata` |
| Shortcut routing | `KeyPress`, `ShortcutResolution`, `FocusSurface` |
| Runtime drag requests | `DragPreview`, `DragPreviewTextSizing`, `DragRequest` |
| Platform-service inputs/results | `FileDialogRequest`, `FileDialogFilter`, `ConfirmDialogRequest`, `ConfirmationLevel`, `ConfirmationButtons`, `ConfirmationResponse`, `PlatformResult`, `PlatformResultExt` |
| Auxiliary windows | `AuxiliaryWindow`, `AuxiliaryWindowClosePolicy` |
| Presentation callbacks | `Presentation`, `TransientOverlay`, `TransientOverlayContext` |
| Assets and paint helpers | `SvgIcon`, `SvgIconTintCache`, `SvgIconTintPalette`, `horizontal_progress_fill_rect`, `horizontal_line_rect`, `vertical_line_rect` |
| Paint callback signature | `PaintPrimitive` |

Advanced pointer admission is qualified through `radiant::widgets::PointerPressAdmission`;
it is intentionally absent from `radiant::prelude`. `Widget::preflight_pointer_press`
defaults to `PointerPressAdmission::Legacy`, so existing custom widgets keep their
source-compatible press, focus, double-click, capture, and release behavior. A
widget may return `ManagedCapture` or `Blocked` from the immutable preflight hook,
and may report continued managed ownership through
`Widget::retains_managed_pointer_capture`; both hooks are object-safe defaults.

`KeyboardModifiers` is intentionally excluded from `radiant::prelude::*`; use
`radiant::widgets::KeyboardModifiers` for the normalized keyboard modifier
payload.

Custom widgets can use `Rgba8::new`, `Rgba8::with_alpha`,
`Rgba8::with_alpha_if`, `Rgba8::blend_toward`, and
`Rgba8::blend_opaque_toward` for common color manipulation. Use
`Rect::from_size(width, height)` for origin-based widget, viewport, and test
bounds, or `Rect::from_xy_size(x, y, width, height)` for positioned widget
bounds, instead of repeating `Point` plus `Vector2` construction. Dense
visualizations can use `ColorRamp` and `ColorRampStop` for normalized heatmap
and intensity palettes without local interpolation helpers.

Layout admission keeps these public geometry signatures unchanged while applying
one private validation policy: coordinates are finite (negative origins are
valid), sizes are finite and non-negative (zero is valid), and only an explicit
positive-infinity constraint maximum remains unbounded. Invalid minima normalize
to zero; invalid, negative, or contradictory maxima normalize to the minimum.
Non-finite or overflowing rounded placement is omitted with a stable layout
diagnostic, and omitted nodes do not dispatch widget or explicit-overlay paint.
This private geometry boundary uses existing layout diagnostics and adds no
public `LayoutDiagnosticCode` variant.
The production-path deterministic runtime host regression covers malformed
custom-policy widget placement and malformed explicit-overlay geometry; invalid
bounds are absent from the published snapshot and produce no paint primitives.

Focus-loss veto is an additive advanced widget contract. Import
`radiant::widgets::{FocusLossDecision, Widget}` explicitly when a custom widget
needs to participate in focus-release validation. `Widget::prepare_focus_loss`
defaults to `FocusLossDecision::Allow`; returning `Veto` keeps the exact
controller-owned focus target and focused state, suppresses both focus-change
inputs and host terminal output for the rejected transition, and requests
repaint. The controller validates a proposed target before asking the current
owner to release focus. An allowed transition installs the proposed owner
before routing `FocusChanged(false)`, so focus-loss output sees the new owner
during synchronous reprojection, and `FocusChanged(true)` is delivered only if
the target remains live, focusable, and authoritative afterward. Direct
`focus_widget` requests for missing or non-focusable targets are invalid and
leave controller focus unchanged. Stale or removed-widget cleanup bypasses the
hook. The synchronous, allocation-free contract is
intentionally excluded from `radiant::prelude::*`, while the default preserves
existing custom-widget behavior.

Custom canvas, image, GPU surface, and overlay widgets can explicitly import
their advanced contracts from `radiant::widgets` and `radiant::runtime`, then use
`WidgetCommon::fixed(...)` when a fixed-size custom widget can declare identity
and intrinsic size together, then chain `WidgetCommon::without_default_chrome()`
when it still needs Radiant's sizing, focus, hit testing, and style contracts
but draws its own focus and state affordances. Use
`WidgetCommon::is_hovered()`, `is_pressed()`, `is_focused()`,
`is_selected()`, `is_active()`, `is_disabled()`, and `is_read_only()`, or the
matching `WidgetState` helpers, when tests, custom widgets, or automation need
to query shared interaction state without reading raw state fields. Use
`InteractiveRowWidget::paints_interaction_fill()` when custom dense-row
painters need hover/pressed fills to follow Radiant's hover suppression and
active-drag policy. Use `Widget::paint_plan(...)` or
`paint_plan_with_defaults(...)` when focused custom-widget tests or previews
need the same `SurfacePaintPlan` query helpers available from full view frames.

Dynamic custom widgets and row input layers can use `stable_widget_id(...)` to
derive deterministic widget IDs from host-owned scopes and durable text app
keys instead of duplicating local hashing helpers. Use
`stable_widget_id_u64(...)` when dynamic rows or controls are keyed by durable
numeric app IDs or enum indexes and projection should avoid allocating
temporary strings. `interactive_row_underlay(content)` can use
`.input_key(...)`, `.stable_input_id(scope, key)`, or
`.stable_u64_input_id(scope, key)` to bind caller-owned identity directly to
the backing interactive row. Use
`.stable_row_identity(scope, row_key)` when one durable row key should identify
both the composed row subtree and the backing input widget. Use
`.dense_chrome()`, `.selected(...)`, `.active_target(...)`, `.candidate(...)`,
or `.visual_state(...)` when arbitrary visible row content should keep
Radiant's standard dense-row hover, pressed, selected, and drop-target chrome
without an app-local transparent hit-target widget. Use
`.dense_chrome_palette(...)`, `.leading_marker(...)`, `.trailing_marker(...)`,
and `.outline(...)` when app-owned row state needs custom fills, edge markers,
or outlines while Radiant still owns generic row input and dense-state
projection. Custom matrix or heatmap widgets can explicitly import `DenseGridLayout` and
`DenseGridCell` for reusable row/column cell projection and hit testing.

For paint-plan emission, explicitly import `WidgetPaint`, `push_fill_rect`,
`push_fill_rect_batch`, `push_stroke_rect`, `push_stroke_rect_batch`,
`push_fill_polygon`, `push_stroke_polyline`, `push_text`,
`PaintTextMetrics`, and `push_text_run_with_metrics` provide the reusable
primitive construction path used by complex examples and custom widgets. Dense
custom widgets can use `push_visible_fill_rect` when derived or clipped
geometry should only enter the paint plan if it has finite positive area. Use
`WidgetPaint::new(...)` when several primitives are emitted for the same custom
widget and local code would otherwise thread the same primitive buffer and
widget id through every helper call.

Timeline, waveform, progress, and scrubber-style custom widgets can explicitly import
`push_horizontal_progress_fill`,
`push_horizontal_value_range_fill`,
`push_horizontal_value_range_edge_fills`,
`push_horizontal_value_cursor_fill`,
`push_horizontal_value_cursor_fills`, or the matching `WidgetPaint` methods,
to append guarded progress fills, normalized range fills, range edges, and
single or repeated cursor fills without repeating local geometry-to-paint
boilerplate. Editor-style
widgets that draw sampled curves such as EQ responses, automation curves, fade
curves, and analysis overlays can use `SampledCurveStrokeParts`,
`sampled_curve_points`, and `push_sampled_curve_stroke` to keep finite-point
filtering, bounds clamping, point-buffer allocation, and stroke emission on
Radiant's generic paint path while the host owns the curve math. Use
`SampledCurveAreaFillParts`, `SampledCurveAreaBaseline`, and
`push_sampled_curve_area_fill` when the sampled curve also needs a filled area
against the top, bottom, or a caller-supplied Y baseline. The helper emits one
`PaintFillPath` regardless of sample count, splits missing samples into separate
closed subpaths, and accepts either `PaintBrush::solid(...)` or a
`PaintLinearGradient`. `PaintLinearGradient::vertical(...)` is the compact path
for translucent curve fills that fade toward their baseline. Import the brush
and path types explicitly from `radiant::runtime`; they remain specialist
custom-paint APIs rather than common prelude entries.

Run `cargo run --example curve_area_fill` to inspect the shared area-fill path,
vertical gradient brush, and missing-sample segmentation in a native window.

Native hosts that already own a platform view and event loop, such as audio
plugin hosts, should use `EmbeddedVelloSurfaceHandle` and
`EmbeddedVelloRenderer`. The host keeps lifecycle and input-event ownership,
while Radiant creates the WGPU surface and reuses the same Vello scene encoder
as `run_native_vello_runtime`. Call `resize(...)` when the host view or backing
scale changes and call `Renderer::render(...)` from the host redraw path.
Trait-based renders use elapsed monotonic time automatically for animated paint
such as focused text-input carets; use `render_at(...)` when the host needs to
supply an explicit deterministic animation time.
Portable or sandboxed hosts should create the renderer with
`EmbeddedVelloRenderer::new_with_text_options(...)` and pass
`NativeTextOptions` containing embedded fonts or host-approved font paths;
`EmbeddedVelloRenderer::new(...)` keeps the system-fallback default.
Embedded rendering supports the normal vector, gradient, clip, text, image, and
SVG paint plan; retained GPU/custom surfaces fail explicitly because those need
the standalone runtime's additional compositing and host callbacks.
Headless capture hosts can use `OffscreenVelloCapture::new_with_text_options(...)`
with the same `NativeTextOptions` policy; `OffscreenVelloCapture::new(...)`
retains the default system-fallback behavior.

Tests, automation, and embedded hosts that inspect paint plans should import
`SurfacePaintPlan` from `radiant::runtime`, then use
`SurfacePaintPlan::text_runs()`, `text_labels()`, `text_label_strings()`,
`first_text_run(...)`, `contains_text(...)`, `first_text_run_after_x(...)`,
`contains_text_after_x(...)`, `first_text_rect(...)`,
`first_text_color(...)`, `text_inputs()`, `first_text_input()`,
`contains_text_input()`, `paint_primitives()`,
`contains_paint_primitives()`, `clip_starts()`, `rects()`,
`contains_rect_matching(...)`, `paint_rects()`,
`contains_paint_rect_matching(...)`, `fill_rects()`, `stroke_rects()`,
`fill_polygons()`, `stroke_polylines()`, `svgs()`, and `render_canvases()`.
Widget-specific query helpers such as `fill_rects_for_widget(...)`,
`visible_fill_rects_for_widget(...)`,
`contains_visible_fill_rect_for_widget(...)`,
`fill_polygons_for_widget(...)`, `visible_fill_polygons_for_widget(...)`,
`contains_visible_fill_polygon_for_widget(...)`, `svgs_for_widget(...)`, and
`first_svg_rect_for_widget(...)` cover common automation assertions without
app-local primitive filtering. Transient overlays can use
`first_widget_rect(...)` or `first_widget_rect_by_priority(...)` to anchor
frame-time paint to a cached paint plan. Use `PaintPrimitive::text_run()`,
`text_input()`, `clip_start()`, `fill_rect()`, `stroke_rect()`,
`fill_path()`, `fill_polygon()`, `stroke_polyline()`, `svg()`, and `render_canvas()` to query
common paint primitives without app-local exhaustive primitive matches.

## Large Virtual Lists

The future keyed virtualization/materialization contract is defined in
[`VIRTUAL_LAYOUT_DESIGN.md`](VIRTUAL_LAYOUT_DESIGN.md). Current shipped virtual
layout includes the qualified query-only `radiant::layout::VirtualLayoutPolicy`
and `VirtualLayoutQueryExecutor` APIs, the public declarative attachment, and
mounted `SurfaceRuntime` registration. They remain without a public constructor
or prelude entry. A crate-private coordinator now provides bounded
accepted-window, key-continuity, fallback, and anchor evidence internally. The
crate-private runtime bridge
inside `SurfaceRuntime` now contains private
semantic-demand/provider-attempt/retention, semantic/materialization
classification, and atomic whole-surface logical publication/composition
evidence. That kernel remains private and does not itself expose a
provider-registration API or serve product collections. The shipped generic
logical session below is the public semantic selection boundary and exposes
only the bounded selected snapshot through explicit `SurfaceRuntime` operations.
A separate crate-private materialization/recycling correctness kernel is shipped
for accepted-commit evidence and explicit private projector/lifecycle tests; it
does not expose a public constructor or prelude entry, register through
`LayoutCapabilities`, project product surfaces, own focus/accessibility pins,
schedule work, or serve product collections. The first-class production
consumer/collection family remains future work, sequenced by OPT-1362 and then
OPT-1400, OPT-1398, OPT-1397, OPT-1399, and OPT-1401. In this slice, an explicit
anchor is corrected only when its same key is present in both accepted bounded
windows; bounded absence leaves it unresolved without deletion or
successor/predecessor inference. Authoritative required-key found/not_found
evidence for removal replacement remains a later prerequisite. The APIs in
this section are the currently shipped fixed-row host projection path and
retain their existing ownership and compatibility behavior.

### Next production consumer: semantic automation session (normative; generic logical implementation shipped)

The private semantic-demand/provider-attempt/retention and atomic
whole-surface-publication kernels above are shipped. They are now consumed by
one generic backend-neutral semantic automation session, not a native adapter
or product integration. The caller/host owns session intent and MUST explicitly
open, refresh, retry, and close the session. `SurfaceRuntime` owns bounded
session state, demand membership, cancellation/supersession, selected
publication, and publication lifetime. Mounted virtual-layout runtime owns
provider registration. Callers MUST NOT infer demand from paint order,
visibility, viewport/overscan, item count, provider availability, diagnostics,
or snapshot reads. Session/container identity is opaque and runtime-issued;
callers cannot fabricate provider identity or authority.

The shipped operations are `open_semantic_automation_session`,
`semantic_automation_containers`, `refresh_semantic_automation_session`,
`retry_semantic_automation_session`, `selected_semantic_automation_snapshot`,
and `close_semantic_automation_session`, with the corresponding opaque demand,
handle, result, status, and fallback types under `radiant::runtime`. Ordinary
`automation_snapshot(&self)` and `automation_target_snapshot(&self)` remain
pure ordinary reads. Explicit refresh and retry are the only provider-calling
operations: refresh atomically replaces the complete demand set, while retry
reattempts the unchanged set. Opening and closing perform provider-free
lifecycle mutation. A separate pure selected semantic snapshot read returns
the last accepted session publication or the conservative ordinary baseline
plus a typed status. This contract invents no public
provider-registration API.

Opening establishes one bounded empty session and an exact session generation.
The first explicit refresh supplies any initial demand members, which start at
attempt one. Refresh atomically replaces the whole session demand set and
supersedes/cancels prior work. An unchanged retry increments only the attempt.
Closing cancels before retiring the generation and clears selected publication
and demand. The first implementation allows one
active semantic session per `SurfaceRuntime`, one contiguous logical range per
mounted container plus the existing independent one-item pin, at most 64
registrations, per-registration and 1024-entry caps, aggregate range length
1024, and at most one provider call per container/attempt. Automatic
retry/backoff and a scheduler are not part of this slice; `Deferred` returns to
the caller and only explicit retry reattempts.

Selection/publication carries session generation, demand generation, attempt,
request/range or pin, mount/container/policy identity, registration identity and
generation, data/policy/measurement/semantic revisions, provider identity/
generation, coordinate, budget, cancellation, materialization/classification
authority, ordinary projection generation, and complete-demand-set generation.
For a custom coordinate it also carries exact transform identity, transform
revision, runtime resolver generation/token, destination context, and a private
transform witness. Acceptance requires exact equality of every required field;
stale, superseded, and cancelled results are inert. Provider attempts are
non-reentrant and cannot publish or mutate runtime state directly.

The consumer stages the complete selected snapshot and status under the exact
fence and swaps only after every active demand member resolves or has an
eligible exact-fence fallback. It never publishes a partial subset. `Found` and
authoritative `NotFound` may participate in a complete publication.
`DataUnavailable` and `Deferred` retain only an eligible last-complete selection
for unchanged exact demand/fence; without that exact fallback they expose the
ordinary baseline and a typed non-success status. `NoProvider`/`Unsupported`
are terminal. `Rejected`/malformed, provider panic, and collision outcomes use
the conservative ordinary baseline even when an older selection exists. Stale,
cancelled, and superseded results are inert. Changed demand, close,
mount/identity/provider/revision/coordinate/budget changes invalidate the old
selection. Materialization/ordinary-projection changes may reclassify retained
exact provider evidence without provider reentry when fences permit it.
`Unmaterialized`/`materialized = false` never authorizes materialization,
scrolling, focus, action, paint, hit testing, scheduling, or renderer work.

The generic consumer admits `Logical` unchanged and admits `Custom` only from
the qualified application-owned transform attachment. The synchronous `Rc`
resolver receives the finite complete source rectangle, runtime-validated
ordinary anchor, effective destination clip, host revisions, and exact
transform revision. It returns a conservative AABB directly; no matrix,
inverse, point mapping, hit testing, or materialization assumption exists.
Runtime admission validates the complete provider output first, invokes the
resolver at most once per accepted entry during explicit refresh/retry, clips
against the exact surface/anchor/scroll/ancestor context, and fences the
private witness through publication. Missing context, replacement or stale
resolver evidence, panic/reentry, unsupported/singular/ambiguous output,
invalid/overflowing output, and fully clipped output use the ordinary baseline.
The private primary-window AppKit consumer now admits `Logical` registrations
unchanged and qualified `Custom(identity)` registrations only when the current
runtime transform attachment and provider-free admission authority match. It
consumes only the compositor's complete normalized logical-window bounds and
the sidecar's exact transform witness/publication fences, then performs the
existing logical content-view to AppKit screen conversion. It never invokes or
reconstructs the custom resolver, assumes an affine mapping, maps corners,
inverts, or uses identity fallback. Product consumer implementations,
scheduler/backoff/fairness, and multiple active ranges per container remain
outside this evidence point. Automated AppKit boundary evidence remains shipped;
exact fresh-bundle activated Computer Use/AppKit evidence verifies discoverability
and numeric action, bounded set-value, and restart acceptance for this bounded
primary-window consumer. VoiceOver-specific acceptance remains unperformed;
repeated negative-geometry AppKit runtime diagnostics remain a separate
unverified follow-up if reproducible.

### Native semantic accessibility query consumer (normative; private primary-window macOS/AppKit consumer)

The first native semantic consumer is the private primary-window macOS/AppKit
production path over the shipped generic logical semantic automation session.
One private native-window
adapter MAY acquire one runtime-issued semantic-session lease. The adapter and
lease remain private: neither owns provider registration, mount identity, provider
generations, demand fences, cancellation, or publication. The existing bound of
one active semantic session per `SurfaceRuntime` remains. A native lease MUST NOT
evict, supersede, or silently reuse an externally active session; contention
returns the one private typed unavailable result `Unavailable(SessionContended)`.
Multi-consumer arbitration is a later contract.

Lease acquisition is lazy: passive root construction, ordinary native-tree
observation, exact count reads, registration/cardinality synchronization, and
ordinary property reads never acquire the lease or create demand. Only an
explicit item or child-range query reaching the owned runtime turn may acquire
it.

Accessibility enablement, native tree-root construction, accessibility-state
observation, ordinary native events, repaint, and ordinary property reads are
observation/capability only. Only an explicit bounded native item or child-range
query MAY become `SemanticAutomationDemand`. Each query MUST translate to exactly
one current runtime-issued semantic-session lease and one current runtime-issued
container handle, plus either one stable required-item key or one finite
contiguous logical range. Missing, stale, ambiguous, duplicate, oversized, or
unrepresentable evidence is unavailable and MUST cause no provider call.

The adapter submits intent only through the existing explicit
`refresh_semantic_automation_session(session, demands)` and
`retry_semantic_automation_session(session)` operations. It never invokes a
provider directly or causes a second call for one container/attempt. Native
callbacks MUST NOT synchronously re-enter a provider or mutate `SurfaceRuntime`
through observational access. Native-to-runtime handoff enters one owned runtime
turn and is bounded transport only; it does not add scheduler, retry, or fairness
policy.

Native publication exposes only a complete selected snapshot under the existing
exact fence. It MUST NOT expose partial virtual subtrees, mix generations, or
repair malformed or colliding evidence. `DataUnavailable` and `Deferred` MAY
retain only an exact eligible complete selection. Missing provider, unsupported,
rejected, panic, malformed, collision, stale, or cancelled evidence uses the
existing typed conservative baseline behavior; stale and cancelled completions
are inert and MUST NOT mutate or publish native state.

The root is attached to the primary content view only through AppKit's supported
`accessibilityChildren`/`setAccessibilityChildren:` property, and installation
is accepted only after a bounded exact one-root identity readback. A nil or
wrong root/host, unsupported selector, Objective-C exception, or mismatched
readback leaves the adapter unattached, posts no layout/value notification, and
attempts to clear any attempted host state. Retirement releases pre-commit
allocations silently only after nil/empty clear readback; if that readback
cannot be verified, callback state and object ivars become inert but objects
remain quarantined, with no release or destruction notification while the host
may retain a stale root. Previously committed objects receive exactly one
destruction notification only after verified clear. Replacement, recovery,
close, and drop clear `accessibilityChildren` symmetrically before releasing
committed native objects; the root reports the same content view as `AXParent`.
Automated AppKit boundary evidence remains shipped. Exact fresh-bundle
activated Computer Use/AppKit evidence verifies discoverability for this
bounded primary-window consumer: the activated window exposed the Radiant
container and a settable stepper at `42.00`; Increment and Decrement produced
`43.00` and `42.00`, bounded `SetValueText` produced `55.50` and `57.25` with
fresh reads showing normal app-owned Begin/Update/Commit events, and a fresh
restarted instance exposed the same tree. VoiceOver-specific acceptance remains
unperformed. Repeated negative-geometry AppKit runtime diagnostics remain a
separate unverified follow-up if reproducible.

The first native consumer accepts `Logical` registrations unchanged and admits
`Custom(identity)` only with the matching current transform attachment, exact
cardinality/provider/anchor evidence, and runtime-owned transform
revision/generation/token. Native publication consumes only the compositor's
complete normalized logical-window bounds plus the matching sidecar witness and
publication fences. Native conversion MUST identify source surface space,
destination window/screen accessibility space, DPI, window/display generation,
orientation, clipping, and a finite non-inverted conversion. Stale, unsupported,
missing, or mismatched authority withholds the complete custom projection; no
resolver is invoked or reconstructed and no affine, corner-mapping, inversion,
or identity fallback is permitted.

The private primary-window macOS/AppKit consumer also consumes the passive
split-pane projection. It admits a separator only when one unique current
target with materialized authority matches the exact node ID, semantic path,
`AutomationRole::Separator`, normalized ratio value, finite bounds, orientation,
and actionless/non-focusable state. A valid separator publishes one `AXSplitter`
between the two pane children with current bounds, ratio value, and
`AXOrientation`; stable native token/object identity is retained across an
admitted ratio update. Static/controlled, stale, unmaterialized,
duplicate/mismatched, malformed, focused, or actionful evidence leaves the
ordinary native tree in place without provider calls, runtime refresh, partial
topology, or interaction authority. Manual macOS/VoiceOver acceptance remains
unverified. A separate crate-private runtime focus-owner foundation and
committed mixed-order sidecar may retain exact current mounted separator
identity and behavior evidence, but this passive native consumer does not
consume it. The explicit backend-neutral sequential traversal consumer does,
using exact private separator stops without exposing them through public/native
focus. Private pointer ownership remains a separate divider interaction path.
The crate-private generic native plain `Tab`/`Shift-Tab` consumer is shipped:
focused-key/text input gets first refusal; modified `Tab` is unchanged;
repeats/releases do not retraverse; focus loss/regain clears the latch; and only
`NoDestination` feeds the existing host-first/widget fallback. Public/native
focus, spatial traversal, keyboard/arrow-key resizing, semantic accessibility
actions, pointer/collapse mapping, and paint/cursor/renderer work remain future
slices.

Activation/opening is provider-free. Explicit native queries refresh, and an
explicit repeated query MAY retry. Deactivation, window retirement, recovery
replacement, and close cancel and retire the lease before native objects drop.
`materialized = false` remains authoritative: native semantics cannot materialize,
scroll, focus, execute actions, paint, hit-test, schedule, render, or claim
provider authority. Virtual/provider nodes therefore never acquire native
numeric action authority. For an ordinary runtime node only, the private
adapter pairs the pure ordinary `automation_target_snapshot` with the native
semantic tree and admits exactly an enabled, editable, focusable,
`AutomationRole::TextInput` target with current `value_text`, materialized
authority, and both neutral increment/decrement actions; it need not already be
runtime-focused. The exact ordinary
ID/path/role/authority is captured with one native token; geometry is never an
authority fence. A qualified node publishes `AXIncrementor`, the exact label when present
and NSString value, `AXDescription`/`AXHelp` when present, enabled true,
`AXFocused` and modern `isAccessibilityFocused` agree for the current
ordinary focus contract; a value is settable only for an eligible current
ordinary materialized target, and exactly `AXIncrement` and `AXDecrement` are
accepted. Modern
increment/decrement selectors use `BOOL c@:` and the
deprecated action selector uses `void v@:@`; only those exact action names are
accepted.
The modern `accessibilityValue` getter uses `id @@:`, while modern and legacy AXValue setters use `v@:@` and `v@:@@`; only a bounded `NSString` (at most 1,024 UTF-16 units and 4,096 UTF-8 bytes) is accepted and translated to the existing neutral `SetValueText` path. Runtime fences, codec validation, atomic publication, and inert stale/failure behavior remain authoritative; no native focus or virtual/provider mutation is added. Each native action enqueues one bounded primary-window,
adapter-generation, token, target, and neutral-action event. The running
event-loop validates current window/generation/token/identity/authority and
delegates to existing `SurfaceRuntime` admission once; that admission may
perform the ordinary runtime focus transition. Non-focusable, focus-vetoed,
blocked, stale, unsupported, disabled, read-only, recovery, close, borrow,
panic, and transport failures are inert and never retarget or mutate. Foundation
exceptions from callback-supplied native objects are caught inside the private
Objective-C boundary and map to the same inert no-event result; they never cross
into Rust. A stable
value-only
change retains the native object, installs the new queryable value before one
`AXValueChanged`, and posts no layout notification; unchanged, no-change,
typed-failure, stale, and enqueue-failure paths post none. Native requests are
not provider demand, materialization, virtual/provider mutation, slider,
range, orientation, percentage, or scalar conversion. The adapter and lease are
not public imperative provider-registration APIs. `automation_snapshot(&self)`, `automation_target_snapshot(&self)`, and
`selected_semantic_automation_snapshot(&self)` remain pure reads.

Read-only ordinary focus exposure is shipped in this slice under a separate
ownership contract. `SurfaceRuntime` and its controller remain the sole
logical-focus authority; the current focused state of the primary Winit window
is the platform eligibility fence; and the AppKit adapter is only a consumer.
At most one ordinary, enabled, focusable, materialized node in the current
runtime/window generation is focused when exactly one controller-owned target
matches the current ID, path, role, and focus evidence. Missing, stale,
ambiguous, mismatched, provider/virtual, unmaterialized, or inactive-window
evidence exposes no native focus. The root's legacy `AXFocusedUIElement` and
modern `accessibilityFocusedUIElement` return the same object or nil. Stable
A-to-B changes retain both native objects and post one
`AXFocusedUIElementChanged` notification on B after it is queryable; unchanged
and clearing transitions post no gained-focus notification, and focus-only
changes post no layout, value, or destruction notification. Focus publication
and update are provider-free and invoke no providers. The adapter never
implements `setAccessibilityFocused`, transfers native focus, or publishes
auxiliary-window/virtual focus. Explicit virtual item/range queries retain the
provider-demand exception described above.

This focus slice has automated Rust/Objective-C boundary coverage only. No live
VoiceOver or live AppKit focus acceptance was performed for it; prior numeric
action and host-attachment evidence is a separate acceptance boundary.

#### Provider-free semantic cardinality (qualified, shipped declaration foundation; native consumer shipped privately)

The shipped declaration foundation exposes the qualified public value
`radiant::application::virtual_layout::VirtualLayoutSemanticCardinality`. It
contains exactly an `usize` logical item count and a separate `u64` cardinality
revision, conceptually `{ logical_item_count, cardinality_revision }`. The
shipped `VirtualLayoutParts<Message>` contract carries an optional cardinality
field and the qualified builder
`VirtualLayoutParts::with_semantic_cardinality(...)`; the field and builder ship
outside the common prelude. The exact private registration/live-fence
invalidation foundation, normalized sidecar, native topology, bounded AppKit
queries, and private primary-window platform consumer are implemented.
Automated AppKit boundary evidence remains shipped; exact fresh-bundle activated
Computer Use/AppKit evidence verifies discoverability and numeric action,
bounded set-value, and restart acceptance for this bounded primary-window
consumer. VoiceOver-specific acceptance remains unperformed; repeated
negative-geometry AppKit runtime diagnostics remain a separate unverified
follow-up if reproducible. No public API is added.

Cardinality is immutable declaration evidence. It is not a callback, demand, or
provider availability signal. `None` means unknown or unsupported; an exact zero
is supported. The count is not capped at `VIRTUAL_LAYOUT_MAX_QUERY_ENTRIES` (1024)
and must not cause storage proportional to the count. Count reads, declaration
updates, mounting, and enumeration are provider-free and never create demand.

The runtime fences the exact `(logical_item_count, cardinality_revision)` pair
with registration identity and generation, container identity, mount generation,
the existing data/policy/measurement/semantic revisions, coordinate space, budget,
and source-qualified provider generations. Fence equality is exact; no latest-
revision ordering or partial match is valid. A count or cardinality-revision
change invalidates affected semantic and native state without calling a provider.
Replacing a provider preserves the cardinality declaration but invalidates its
provider publication. Unmount, native recovery, deactivation, and session close
retire all cardinality, token, and publication state.

The native adapter does not vend a virtual child container when cardinality is
unknown. A positive count without a range provider is unsupported for native child
traversal and is not vended; an exact zero may be represented without a provider.
An AppKit count read returns the exact declared count. A bounded child-range query
normalizes `index` and `maxCount` by checked subtraction from that count: zero
count or zero maximum is empty, an out-of-range index is empty, subtraction and
end arithmetic must not overflow, and the normalized length must fit the declared
budget, the 1024 per-query cap, and the remaining aggregate budget. A provider
must supply the stable key; the adapter never synthesizes one from an index.

#### Compositor-owned normalized native sidecar and private topology

The compositor produces one crate-private normalized native sidecar from the
same staged `entries_by_container` union that produces
`VirtualLayoutAutomationComposition`. Each retained member carries container,
mount, and registration authority; the exact cardinality fence; logical index;
stable `VirtualLayoutItemKey`; provider `AutomationNodeId`; final normalized
node/path; materialization authority; and the publication fence. Exact same-key,
same-index overlaps coalesce only under the existing full-evidence equality.
Raw range or pin members are never reconstructed by the native adapter.

Conflicting, ambiguous, duplicate, unstable, colliding, ordinary-ID, or
aggregate-budget failures reject the whole publication. The sidecar is stored
atomically with `RuntimeSemanticAutomationSelection`'s composition, status,
ordinary projection, and native projection. There is no parallel native
reconstruction and no mixed native/public selection.

The primary content view/window exposes one private root. Each accepted virtual
anchor exposes one private read-only virtual container, and each normalized
logical item is a direct child of that container; duplicate placement elsewhere
is suppressed. Container identities and monotonic item tokens are runtime-issued
and private. Tokens are never derived from an index, pointer, provider ID,
serialized ID, or bounds. Token continuity requires exact lease, container,
mount, cardinality-fence, and stable-key equality; a cardinality change retires
the tokens. Foreign, stale, retired, duplicate, ambiguous, or colliding tokens
return `nil`/`NSNotFound` without a provider call.

The existing dynamic native class registers exactly
`accessibilityIndexOfChild:` with Objective-C encoding `Q@:@`; its callback ABI
returns `usize`/`NSUInteger`. It returns `NSNotFound`, defined as
`isize::MAX as usize` (never `usize::MAX`), for any index equal to that sentinel.
The callback resolves receiver and child uniquely by opaque object identity in
one current immutable native projection and requires exact direct-parent token
equality. For `Root`, `Ordinary`, and `Item`, it searches only the receiver's
ordered direct `children` vector and returns the unique compact zero-based
position. For `Container`, it searches only `logical_children` against
`logical_count` and returns the retained logical index itself, including
sparse/nonzero indices such as 100; a child present only in ordinary `children`
is absent. Nil, foreign, stale, retired, indirect, sibling, ancestor, self,
wrong-parent, wrong-kind, out-of-count, missing, malformed, duplicate,
ambiguous, or sentinel-colliding evidence, borrow conflict, and panic all return
`NSNotFound`. The callback does not message or dereference the supplied child,
allocates no storage proportional to declared count, and performs no
provider/query/runtime/action/lease/notification mutation.

The same dynamic native class registers exactly
`accessibilityNotifiesWhenDestroyed` with Objective-C encoding `c@:`. Its
crate-private Rust callback ABI returns `ObjcBool` and returns `YES`
unconditionally for every instance, including when the callback-state ivar has
already been cleared during retirement. It is contained by the existing
`ffi_boundary`/unwind boundary but does not consult or borrow callback state,
inspect projection/tokens/view/window, allocate, lock, access provider/runtime,
enqueue events or notifications, or mutate anything. It does not post
notifications. Existing retirement order clears callback state and each state
ivar before posting exactly one `AXUIElementDestroyed` notification and
releasing each object; repeated retirement is idempotent for already-retired
objects.

The root, virtual containers, and all non-text items map to
`NSAccessibilityGroupRole`; only `Text` and `Readout` map to
`NSAccessibilityStaticTextRole`. The native surface exposes only role, exact
parent/children, finite frame, label, description/help, and static-text value.
It omits checked, selected, enabled, read-only, focusable, focused, tab, live,
and action metadata for this virtual/provider path. Focus is always false,
actions are empty/no-op for virtual/provider objects, and buttons, toggles,
sliders, tables, and text inputs in that path are never mapped to actionable
roles. Defunct objects return conservative empty or zero values. Ordinary
materialized TextInput numeric nodes use the separate native action contract
above; they are never synthesized from virtual/provider evidence.

AppKit callbacks are non-blocking and never call or synchronously mutate the
runtime or a provider. A valid explicit item or range query enqueues/coalesces
one owned runtime turn. While it is pending, the count remains exact; item and
range reads return only an exact eligible retained result under the same fence,
otherwise empty/`nil`, with no placeholder or mixed tree. Identical in-flight
queries coalesce. An explicit repeated query after `Deferred` may retry; an
ordinary read is never a retry.

An accepted publication installs the complete normalized native projection
atomically. Retention requires exact equality of the semantic fence and native
coordinate/cardinality fence. `DataUnavailable` or `Deferred` without an exact
fallback exposes only an empty/baseline result; terminal failures clear virtual
native publication; stale and cancelled results are inert. A changed visible
state posts exactly one `NSAccessibilityLayoutChangedNotification` only after
the complete state is queryable on the main thread. Unchanged, pending, stale,
cancelled, and rejected work posts no layout notification. Retired custom
objects follow the `UIElementDestroyed` notification lifecycle.

This extension preserves the one-session bound, opaque private handles, explicit
refresh/retry-only demand, one range plus one required-item slot, 64
registrations, 1024 per-query and aggregate caps, one provider call per
container/attempt, exact publication/fallback, `materialized = false`,
normalized logical conservative coordinates, and pure snapshots. It excludes
native focus transfer and virtual/provider focus exposure, native actions for
virtual/provider targets, selection mutation, scroll/materialize, scheduler/retry policy, render,
product, direct native custom-resolver invocation/reconstruction, Wayland/Windows, auxiliary,
multi-consumer, and public registry behavior.

This contract is limited to the private primary-window macOS/AppKit consumer.
Automated AppKit boundary evidence remains shipped, covering projection
construction, supported host attachment, exact-root readback, failure cleanup,
and symmetric retirement. Exact fresh-bundle activated Computer Use/AppKit
evidence verifies discoverability and numeric action, bounded set-value, and
restart acceptance for this bounded primary-window consumer. VoiceOver-specific
acceptance remains unperformed. Repeated negative-geometry AppKit runtime
diagnostics remain a separate unverified follow-up if reproducible. Wayland,
Windows, non-qualified/virtual native actions, native focus
setter/transfer or focus exposure beyond the ordinary materialized-target
contract, scrolling, product policy,
direct native custom-resolver invocation/reconstruction, scheduler, and
renderer behavior remain excluded.

### Public declarative provider attachment (normative; custom attachment bounded)

The existing semantic-session methods above are the only current public
provider-calling intent. The one public declarative attachment path is
`radiant::application::VirtualLayoutParts<Message>` with
`virtual_layout_from_parts`; it may carry optional semantic item and contiguous
range providers. `radiant::runtime::VirtualLayoutRevisions`,
`VirtualLayoutSemanticProvider`, `VirtualLayoutSemanticRangeProvider`,
read-only item/range requests, `VirtualLayoutSemanticEntry`, and generic
`VirtualLayoutSemanticProviderOutcome<T>` are qualified shipped vocabulary. The
custom-coordinate vocabulary is separately qualified under
`radiant::runtime::virtual_layout`: `VirtualLayoutSemanticCoordinateTransform`,
`VirtualLayoutSemanticCoordinateTransformRequest`, and
`VirtualLayoutSemanticCoordinateTransformOutcome`. The builder
`VirtualLayoutParts::with_semantic_coordinate_transform(identity, revision, Rc)`
declares `Custom(identity)`; without it the existing declaration remains
`Logical`. None of these transform types are in the prelude.

The shipped declarative foundation exposes the qualified public
`radiant::application::virtual_layout::VirtualLayoutSemanticCardinality` value
field on `VirtualLayoutParts<Message>` and the qualified builder
`VirtualLayoutParts::with_semantic_cardinality(...)`. The value contains the
exact `usize` logical item count and separate `u64` cardinality revision. The
optional field and builder now ship outside the common prelude, and the exact
private registration/live-fence invalidation foundation, normalized sidecar,
native topology, bounded AppKit query path, and private primary-window platform
consumer are implemented. Automated AppKit boundary evidence remains shipped;
exact fresh-bundle activated Computer Use/AppKit evidence verifies discoverability
and numeric action, bounded set-value, and restart acceptance for this bounded
primary-window consumer. VoiceOver-specific acceptance remains unperformed;
repeated negative-geometry AppKit runtime diagnostics remain a separate
unverified follow-up if reproducible.
Cardinality is immutable
declaration evidence, not a callback or demand; its exact count is independent
of the one-range and one-required-item slots and of the 1024 per-query/aggregate
budgets.

The shipped boundary preserves the existing 64-registration limit, one
contiguous range and one required-item slot per mounted container, 1024 entries
per query and in aggregate, and exact runtime-issued source tickets. Callbacks
are read-only synchronous `Rc` capabilities; reentry is rejected and provider
panic maps to the conservative baseline. There is no `Send`/`Sync`, worker, or
scheduler promise.

The mounted registration, removal, provider replacement, registration/mount/
provider generations, lifetime cancellation, and exact source tickets are
runtime-owned. There is no public imperative registration API or
application-owned mount generation. The first boundary is synchronous,
single-threaded `Rc`, with no `Send`/`Sync`/worker/scheduler promise. Provider
`Unavailable` reasons are `DataUnavailable` and `Unsupported`; bounded
`Deferred` reasons are `DataPending`, `SemanticPending`, and `Retry`; missing
slots produce runtime-synthesized `NoProvider`.

Only explicit `refresh_semantic_automation_session(session, demands)` and
`retry_semantic_automation_session(session)` call providers; an attached
custom transform is invoked only inside those explicit turns, after complete
provider output validation and destination-context validation. Registration,
opening, enumeration, ordinary snapshot/target reads, repaint,
viewport/visibility/overscan, diagnostics, item count, provider availability,
and IME/native events do not create demand. The full exact-fence, validation,
fallback, lifecycle, native-boundary, non-goal, and acceptance-matrix contract
is in [`VIRTUAL_LAYOUT_DESIGN.md`](VIRTUAL_LAYOUT_DESIGN.md). Custom output is
clipped to the exact logical destination context and carries a private exact
transform witness through publication; failure uses the ordinary baseline and
never a partial subtree. The private primary-window macOS/AppKit native semantic accessibility
consumer translates explicit platform queries only through the same backend-neutral
semantic-session model; it is not the hidden provider-registration or demand
owner. The custom transform is not a native resolver API: the native consumer
consumes only normalized logical bounds and validated sidecar authority. The
non-goals are direct native custom-resolver invocation/reconstruction, native
accessibility action dispatch, focus,
scrolling/materialization,
scheduler/backoff/fairness, renderer/paint/hit-testing/cache policy, product
policy, multiple ranges, and prelude export.

Large list, table, tree, browser, and picker surfaces should use Radiant's
virtual-list contract instead of constructing hidden rows. Host applications own
the logical item collection, stable row keys, selection, and domain state.
Radiant owns the bounded viewport math, focus-follow policy, row hit-test scope,
scrollbar mapping, and retained overlay invalidation primitives.

Use `VirtualListController` or `resolve_virtual_list_window(...)` with the total
logical item count, visible viewport length, explicit overscan, requested
viewport start, and optional focus. Then construct row widgets only for the
returned `window_start..window_end` range. The wider logical count is metadata
for scrollbars and clamping, not permission to build offscreen widgets. Stable
row identity should come from host-owned IDs through `VirtualListItemKey`,
`stable_widget_id(...)`, or explicit widget IDs, so focus, hover, drag,
selection, and retained overlays survive sorting, filtering, insertion, and
scroll-window changes.
`VirtualListWindow::viewport_contains(...)` tests the visible viewport, while
`contains(...)` tests the wider materialized window. Use `overscan()`,
`leading_overscan()`, and `trailing_overscan()` when app-owned state needs to
retain the runtime's materialization policy without hand-computing it from
window bounds. Use `reconcile_total_items(...)` when host-owned data changes
after a materialized window was cached and the current viewport should be
clamped to the new logical count without app-local window validity checks.
After a runtime-originated window change, `VirtualListController` records the
runtime viewport length. Use `runtime_viewport_len_or(fallback)` when the next
projection should prefer the runtime viewport over an estimated host viewport,
and `runtime_viewport_contains_index(...)` when only a known runtime viewport
should suppress focus-follow scrolling. Use
`configure_projection_and_focus_changed_unless_visible_optional(...)` when a
changed selection key should follow only if the selected item is outside that
runtime-reported viewport.

Hit testing should use the materialized row slice, such as with
`virtual_list_stacked_item_at_point(...)`, so hidden rows are never needed to
route normal pointer input. Repaint and invalidation should stay scoped to one
list window: structure/window changes rebuild materialized geometry, while
item-state changes are overlay-only through `VirtualListInvalidation`. Keep one
`VirtualListController` per scrollable list surface; sharing a controller is an
explicit host decision and otherwise one large list must not move another list's
viewport or force its rows to be rebuilt.

## Message-Handler Helper Reference

The normal application path remains `.update(...)` for simple message handlers
and `.handle_message(...)` for handlers that need `UiUpdateContext`. The helpers
in this section support more explicit runtime work, background task ownership,
platform-service decoding, text-input event handling, and secondary windows.

`PlatformResponse` exposes helpers such as
`path()`, `into_path()`, `into_path_or_canceled()`, `is_canceled()`,
`is_completed()`, `into_completed()`, `confirmation()`, and
`into_confirmation()`, while the `PlatformResultExt` prelude trait provides the
same common decoders directly on platform-service callback results so reducers
can propagate platform errors and reject wrong response shapes without local
adapter code. Use `context.business()` for host-owned business work that must
not run on the UI/event/render path. The business builder exposes
`interactive(...)`, `background(...)`, `blocking_io(...)`, and `idle(...)`
lanes, plus `priority(name, TaskPriority)` when a host-owned scheduler policy
has already selected the lane, then optional policies such as
`latest(&mut LatestTask)`,
`latest_for(&mut KeyedLatestTasks<_>, key)`,
`latest_for_resource(&mut ResourceTasks, ResourceKey)`,
`exclusive_for(&mut ResourceTasks, ResourceKey)`,
`resource(&mut ResourceSlot<_>)`, and `cancellable()` before
`.run(work, map)`.
When a host needs to observe bounded admission without adding a callback or
retry queue, use `.run_with_receipt(work, map)`. It returns a UI-local
`BusinessTaskAdmissionReceipt`; poll it for `BusinessTaskAdmission::Pending`,
`Accepted`, `Rejected`, or `Closed`. The receipt is resolved only after the
controller has attempted actual host admission, and dropping it releases the
weak controller-side state. The additive `latest(...).run_with_receipt(...)`
variant preserves the latest transaction and output ordering; existing
`.run(...)` behavior is unchanged.
For an owner-scoped cancellable ordered stream, create the cancellable request,
clone `let token = request.token()` before consuming it, and call
`request.stream_for_owner_with_receipt(owner, work, map_event, map_final)`.
The route keeps the ordinary bounded FIFO event stream and one final output while
the explicit token and declarative owner retirement independently fence later
cooperative work, event/final mapping, and reduction. Its admission receipt is
admission-only; event and final mappers remain UI-local/non-`Send`.
Use `.stream(work, map_event, map_final)` when one worker should report
progressive results, such as progress, preview-ready, and final-ready states,
without exposing UI state to the worker or using an app-local message channel.
Streaming workers receive a `BusinessEventSink<Event>` and emitted events are
mapped back through the normal message queue in FIFO order. Use the explicit
`.stream_latest(...)` variant only when intermediate visual/progress events are
safe to discard: its bounded ingress is latest-wins and replaces retained
intermediate work while the UI catches up. The final completion is never
coalesced and remains ordered after all retained events. `latest(...).stream(...)`
tags both intermediate events and the final output with the same
`TaskCompletion` ticket; keyed/latest resource streams tag events and final
output with a `KeyedTaskCompletion<Key, Output>` so hosts can keep stale-result
protection while adopting staged loading designs.
Long-running workers should use `BusinessWorkContext::checkpoint()` when a
chunk completes, `check_cancelled()` when they can stop promptly,
`yield_if_elapsed(duration)` when CPU work should periodically yield, and
`fail_if_over_budget(duration)` when an interactive worker must enforce a hard
checkpoint budget.
Latest completions receive a `TaskCompletion<Output>` or
`KeyedTaskCompletion<Key, Output>`; call the matching `LatestTask::finish(...)`
or `KeyedLatestTasks::finish(...)` before applying the output so stale work is
rejected consistently without host-specific task-id plumbing. Resource-keyed
completions should call `ResourceTasks::finish_key(...)` or
`ResourceTasks::is_active_key(...)` with the carried `ResourceKey` and
`TaskTicket` before applying progress, preview, playback, or final output. Use
`ResourceKey::scoped(...)` or `ResourceKey::path(...)` to keep resource classes
explicit instead of hand-concatenating scope prefixes in app code. Use
`Command::after(...)` and `UiUpdateContext::after(...)` schedule one delayed
UI-owned mapper. The host timer lane carries only an opaque wake; it never
constructs or transports the application message. `UiUpdateContext::after_latest(...)`
uses a caller-owned `LatestTask` to replace a pending debounce, and the UI
runtime invokes the mapper only when its ticket is still active. Keep one
`LatestTask` with the host's UI state for each logical resource, and call
`LatestTask::finish(...)` (or its completion helper) before applying work
results. `Subscription::interval(...)` uses the same opaque-wake lane for
recurring ticks; its message factory also runs on the UI owner.

For custom hosts, implement `RuntimeTaskHost::schedule_timer(...)` and
`RuntimeQueueHost::map_runtime_timer_wake(...)` together. Hosts with one shared
worker/platform/timer ingress should also override
`RuntimeQueueHost::drain_runtime_queue_item_batch_into(...)` and emit
`RuntimeQueueItem` values in admission order. The host stores or forwards
`RuntimeTimerWake` values only; the UI runtime owns FIFO ordering,
generation/epoch validation, mapper invocation, and message reduction. The
legacy `take_runtime_timer_wakes(...)` default remains useful for simple hosts
without a combined ingress. No application message crosses the timer thread,
and controller-owned wakes must remain available to the runtime controller
rather than being mapped by the host.
Worker/platform payloads whose mapper must also respect that total order can be
wrapped in `RuntimeQueueDelivery` and emitted as
`RuntimeQueueItem::Delivery`; implement
`RuntimeQueueHost::map_runtime_queue_delivery(...)` to downcast and map them
only when the controller reaches that FIFO item.
Text inputs can use `.message(...)` for value-only routing or
`.message_event(...)` when the host needs to distinguish edits from submissions.
Inline edit flows can seed caret and selection state with `.selection(...)` or
`.select_all()` while staying on the application-builder path. Autocomplete and
inline suggestion flows can use `.completion_suffix(...)` to paint a suffix
after the current value without app-local floating text overlays or text-offset
math. Reducers that receive full `TextInputMessage` values can use `value()`,
`into_value()`, `kind()`, `parts()`, `is_changed()`, `is_submitted()`, and
`is_completion_requested()` instead of repeating exhaustive variant matches
when they only need the event kind or carried text value. Use `parts()` when a
reducer needs both `TextInputMessageKind` and the borrowed value without
cloning or consuming the message.
Applications with several mutually exclusive transient surfaces, such as
dropdowns, popovers, or inspector subpanels, can use `ExclusiveOpen<T>` to keep
one typed item open at a time and centralize toggle/close behavior. Use
`open_changed(...)`, `close_changed()`, and `toggle_changed(...)` when retained
rows, overlays, or drag/drop targets need to request invalidation only when the
exclusive item actually changed.
Stateful apps can project secondary top-level windows with
`.auxiliary_windows(...)` and the common-prelude
`AuxiliaryWindow::utility(...)` constructor. Use
`.on_close(message)` to route native close requests back into the host reducer.
Frequently reopened utility windows such as settings panels and inspectors can
also call `.cache_on_close()` so native close hides and retains the prepared
window; a later projection with the same key updates and shows the cached
window instead of recreating the native window and renderer state.
Windows that require advanced native configuration can explicitly import
`NativeRunOptions` and call `AuxiliaryWindow::new(...)` instead.
The standalone native Vello multi-window path keeps one event-loop-confined
WGPU context, device, queue, and device-loss callback owner for the whole run.
The primary window selects that owner; auxiliary windows borrow it and create
only their own compatible surface and renderer. An auxiliary
`NativeGpuBackend::Auto` policy inherits the selected primary backend, while an
explicit policy must be compatible with that backend and the selected adapter
must support the auxiliary surface. Auxiliary child runners return messages to
the parent, which remains responsible for projection and synchronization.
Applications that need lightweight UI-cadence diagnostics can explicitly import
`FrameCadenceMonitor` with `FrameCadenceConfig` to classify first-frame,
warning-spike, error-spike, periodic, and normal frame deltas while keeping
application-specific context in the host log payload.
Higher-level application helpers follow the same logical-coordinate sizing
model as view modifiers: fixed details-list columns use `f32` logical widths
through `DetailsColumn::fixed(...)`, matching `.size(...)`, `.fixed(...)`, and
other layout builders instead of introducing a separate integer sizing model.
Details-list state, drag, resize, placement, sortable-list, and virtual-tree
APIs are specialist contracts. Import the names used by a details surface from
`radiant::application`; they intentionally do not enter `radiant::prelude::*`.
Sortable details lists can use `SortDirection::apply_ordering(...)` after
computing an ascending domain ordering, so hosts keep column-specific sort keys
while Radiant owns the common ascending/descending direction policy.
Custom details-list rows can use `compact_details_row(...)` and
`compact_details_cell(...)` to share Radiant's compact row chrome, 20px cell
height, fixed-width cell sizing, and flexible fill-cell sizing while still
composing app-specific cell contents. Use
`compact_details_anchored_cell(...)` when a compact cell needs a fixed-size
anchored child such as a badge, status marker, or compact action without
rebuilding the anchored-layer and cell-sizing composition locally. Keep
`CompactDetailsAnchoredCellParts` with
`compact_details_anchored_cell_from_parts(...)` for advanced named-field
construction.
Custom details-list headers can use
  `compact_details_header_row(...)`, `compact_resizable_details_header_cell(...)`,
  and `details_sort_label(...)` to share Radiant's compact header chrome,
  sortable click-or-drag behavior, resize handles, and sort marker copy while
  still composing app-specific menus or column policies. Dynamic header cells
  should assign one stable header-cell id to the returned cell with
  `.id(stable_widget_id(scope, column_key))`; Radiant derives the internal
  sort/reorder and resize child identities under that parent. Use `.key(...)`
  only when repeated static header structure needs a scoped key but not an
  external numeric id. Use `compact_details_header_sort_drag_id(...)`
  or `compact_details_header_resize_id(...)` only in tests, automation, or host
  integrations that need to address those child affordances directly. Use
  `compact_resizable_details_header_cell_with_ids(...)` with
  `CompactDetailsHeaderCellIds` when dynamic header cells need stable explicit
  externally reserved widget ids for retained focus, drag, or resize state; use
  `CompactDetailsHeaderCellIds::from_cell_id(...)` to derive the default child
  ids from a stable parent cell id, or
  `CompactDetailsHeaderCellIds::from_stable_key(...)` only when preserving an
  existing two-scope external id contract.
Resizable and reorderable details headers can keep interaction state in
`DetailsColumnResizeDrag` and `DetailsColumnReorderDrag`, using
`update_details_column_resize_drag(...)`,
`update_details_column_reorder_drag(...)`,
`details_column_drag_content_left(...)`, `details_column_reorder_index(...)`,
`details_column_drag_feedback(...)`, `reorder_details_columns_by_id(...)`,
`reorder_visible_details_columns_by_id(...)`, and
`update_visible_details_column_reorder_drag(...)` for stable framework-owned
column geometry and drag-lifecycle behavior. Use the visible-subset helpers
when durable column preferences include hidden columns but the rendered header
only exposes a filtered subset.
`DetailsColumnReorderDrag` retains the current pointer position and exposes
`current_feedback(...)` so host applications can render drag previews and local
insertion markers without duplicating the generic drag lifecycle or marker
projection math.
The details-column resize helper is a concise layout-interaction projection: an
active `Cancelled` message returns one `DetailsColumnWidthUpdate` for the
captured starting width and original column id before clearing the active drag,
so a host can durably restore a width already applied from a move. An orphaned
resize cancellation returns no update. Reorder cancellation still clears the
active drag without producing a reorder. These helpers are not typed
`EditEvent` boundaries or runtime interaction handlers. The qualified
`radiant::layout::{LayoutCapabilities, LayoutInteraction}` contract provides
UI-local registration, exact/conservative revision evidence, and validated
normalized hit-region declaration/projection for generic surface containers.
Version 3 additionally admits typed pointer input through
`LayoutEventContext<Message>`; the runtime owns separate layout capture and
offers a fresh topmost compatible target before widget fallback. Version 2 is
retained as a projection/query-only contract. Version 4 adds the optional
`ContainerStateDeclaration` / `LayoutContainerStateContext` surface: a
declaration carries an opaque typed `ContainerStateId`, schema version, and
explicit initializer, while the runtime owns a bounded UI-local slot keyed by
mounted container identity, concrete type, and schema. The state surface
supports UI-local values such as `Rc<Cell<_>>`; it does not expose `Any` or
`TypeId`, and state-only mutation does not request work, repaint, or a message.
Version 3 continues to delegate through the unchanged
`handle_layout_input` entrypoint. `SurfaceRuntime::layout_hit_target_at(...)`
continues to expose the resulting read-only container/region target and
projected bounds. The qualified runtime-owned split-pane consumer adds one
clipped built-in divider target from final child geometry and keeps static and
controlled-ratio splits inert; this generic capability contract remains
product-neutral outside that consumer and does not claim virtualization
completion.
Custom row painters can compose `InteractiveRowWidget` directly for shared
dense-row hover, activation, drag-source, drag-active, drop-target, and retained
hover synchronization behavior while keeping domain-specific row visuals in the
host widget. Implement `EmbeddedInteractiveRowWidget` when the custom widget is
primarily an app-painted wrapper around an embedded `InteractiveRowWidget`; the
trait supplies the standard `Widget` implementation for common contract
delegation, input routing, pointer-motion policy, and retained state
synchronization while the host instance provides action routing and paint. Use
`EmbeddedInteractiveRowWidget::interactive_row_actions(...)` when the host can
route standard row interactions through `InteractiveRowActions`; override
`map_interactive_row_message(...)` only when the host needs custom filtering or
nonstandard event mapping. Use
`InteractiveRowWidget::dense_visual_state(...)` with
`InteractiveRowVisualStateParts` when custom row paint needs the generic dense
row state model without reading widget internals. Use
`DenseRowVisualState::emphasizes_label()` when custom row labels should switch
to a higher-contrast color for selected rows, committed operation targets, or
hovered operation candidates without repeating dense-row state predicates. Use
`InteractiveRowWidget::handle_input_mapped(...)` and
`synchronize_from_previous_embedded(...)` when a custom row widget embeds an
interactive row for generic input behavior but exposes host-specific messages
and custom paint outside the trait shape. Interactive-row synchronization
preserves ordinary pressed state between frames, but clears stale pressed and
drag state when a retained host-tracked drag row is no longer active or no
longer the drag source. Use `InteractiveRowWidget::id()`, `common()`, and
`common_mut()` when custom row wrappers need paint identity or widget-contract
delegation without reading the embedded row field layout. Use
`InteractiveRowWidget::push_dense_fill(...)` when a custom row painter should
use the row's retained hover/pressed state plus host-owned selection or target
state to append standard dense-row feedback. Use
`InteractiveRowWidget::dense_chrome_parts(...)` and
`push_dense_chrome(...)` when the custom row needs standard dense-row fill,
markers, or outlines while keeping row identity and retained input-state
projection inside Radiant. Use `push_dense_labeled_chrome(...)` when the custom
row needs that standard chrome followed by one centered dense-row label. Use
`InteractiveRowMessage::activation_provenance()`,
`activation_modifiers()`, `single_activation_modifiers()`,
`is_activation()`, `is_single_activation()`, `is_double_activation()`,
`secondary_position()`, `drag_message()`, `hover_drop_position()`,
`clear_drop_position()`, and `is_drop()` when custom row widgets need to map
Radiant row interactions into host-specific row messages without repeating
exhaustive event-shape matches.
The shared `InteractionSource` and `InteractionProvenance` enums are owned by
`widgets::interaction` and exported explicitly from `radiant::widgets` and the
common `radiant::prelude`; both are
`Clone + Copy + Debug + PartialEq + Eq + Hash` and
intentionally have no `Default`. `InteractionProvenance::source()` returns the
explicit `InteractionSource` category, so missing native evidence is not
inferred as `Programmatic`.

### Shared edit-event lifecycle

The consumer-free shared edit lifecycle is available through the qualified
`radiant::widgets` import:

```rust
use radiant::widgets::{EditEvent, EditPhase, EditTransaction, InteractionProvenance};
```

`EditPhase` has `Begin`, `Update`, `Commit`, and `Cancel` variants;
`is_terminal()` is true only for `Commit` and `Cancel`. `EditTransaction` is an
opaque process-local identity allocated once by `EditEvent::begin(...)`. Copies
remain equal and hash-equivalent, but the identity has no raw-ID accessor or
constructor and has no ordering, persistence, timestamp, serialization, or
cross-process meaning. `transaction.source()` is selected at `Begin` and stays
fixed for the lifecycle.

`EditEvent<T>` is non-exhaustive and exposes readable `transaction`, `phase`,
`start_value`, `value`, and `provenance` fields. `begin(start_value,
provenance)` creates a `Begin` event with equal starting and current values.
`update(value, provenance)` and `commit(value, provenance)` preserve the
transaction and starting value. `cancel(provenance)` creates a terminal event
whose current value is restored to the starting value. A transition returns
`None` when its predecessor is terminal or when the new provenance's source
does not match the fixed transaction source; native metadata may change between
phases within the same source, and missing metadata never becomes
`Programmatic`.

The lifecycle foundation performs no dispatch, callbacks, or event coalescing.
`Begin`, `Commit`, and `Cancel` are delivery boundaries; a future delivery
policy may keep only the latest `Update`. For `Copy` values such as `f32`, the
bounded transitions are allocation-free after the process-local transaction
identity is allocated.

`SliderWidget` is a shipped production consumer. Its qualified
`SliderEditBatch` message contains one to three ordered `EditEvent<f32>` values
in fixed-capacity copy-only storage; `events()` exposes the used slice and every
non-empty batch shares one transaction. Pointer press, move, release, focus
loss, and capture cancellation preserve their accepted provenance rules and
emit deterministic `Begin`, `Update`, `Commit`, or `Cancel` boundaries.
Focused keyboard changes are atomic `Begin`/`Update`/`Commit` batches. The
concise `SliderMessage::ValueChanged` and `on_change` paths project only
effective value changes, while `WidgetMessageMapper::slider_edits`,
`SurfaceNode::slider_edits_mapped`, `SliderBuilder::on_edit`, and
`application::slider_edit_mapped` receive the complete ordered batch. These
lifecycle APIs are qualified and are not exported through the common prelude.
The public `SliderState` remains the source-compatible one-field
`SliderState { value }` model, and `SliderWidget` retains its public
`{ common, props, state }` fields. Official Slider constructors lower a
crate-private retained adapter that owns the active transaction; a bare public
`SliderWidget` keeps the concise `handle_input(...) -> Option<SliderMessage>`
contract and does not carry typed lifecycle state. `Knob` is also a shipped
shared-edit adopter, and `PanelResizeState` is also a shipped shared-edit
consumer. The shared edit-event adopters currently shipped are `Slider`,
`Knob`, `PanelResizeState`, and the public generic `NumericInput`; remaining
continuous controls and separately unshipped native/product boundaries remain
follow-up work.

### Additive Slider domain mapping

The executable additive domain consumer is the qualified
`radiant::application::slider_domain(value, adjustment)` constructor:

```rust
use radiant::application::{slider_domain, SliderDomainBuilder};
use radiant::widgets::{
    NumericAdjustment, SliderDomainError, SliderDomainMessage, ValueFormat,
};

// `DomainAdjustment` and `AdjustmentError` are application-owned.
let slider: SliderDomainBuilder<DomainAdjustment> =
    slider_domain(state.volume, DomainAdjustment::new())?;
let view = slider
    .format(ValueFormat::decimal(1))
    .message(|message: SliderDomainMessage<AdjustmentError>| Message::VolumeDomain(message));
```

The full generic signature is
`slider_domain(value: f32, adjustment: A) ->
Result<SliderDomainBuilder<A>, SliderDomainError<A::Error>>` where
`A: NumericAdjustment<f32>`. `SliderDomainBuilder` and `slider_domain` are
qualified exports from `radiant::application`; `SliderDomainMessage` and
`SliderDomainError` are qualified exports from
`radiant::widgets::interaction` and are also re-exported from
`radiant::widgets`. None of these domain-specific names are in the common
prelude. `SliderDomainBuilder::message(...)` additionally requires
`A: 'static` and `A::Error: Clone + 'static`; the `slider_domain(...)`
constructor remains available with only the weaker `A: NumericAdjustment<f32>`
bound.

Construction is checked and finite. The supplied domain value must be finite;
the adjustment's `value_to_normalized` inverse must succeed and return a finite
normalized value in `0.0..=1.0`. The constructor returns
`SliderDomainError::ValueToNormalized`, `NonFiniteValue`,
`NonFiniteNormalized`, or `NormalizedOutOfRange` as applicable. It never
silently clamps an invalid inverse result.

The retained Slider still performs its existing normalized interaction
lifecycle. An accepted normalized candidate is then checked and passed once to
`NumericAdjustment::normalized_to_value`. A successful finite result emits
`SliderDomainMessage::ValueChanged { value }`. An adjustment error, a
nonfinite domain result, or an invalid normalized candidate emits
`SliderDomainMessage::MappingFailed { normalized, error }` with the distinct
`SliderDomainError::NormalizedToValue`, `NonFiniteValue`,
`NonFiniteNormalized`, or `NormalizedOutOfRange` error. Mapping failure restores
the previous normalized value and leaves the `domain_value` unchanged for every
input. For nonterminal input it also restores the prior retained interaction
state and active edit. For terminal `PointerRelease` and `FocusChanged(false)`
(including capture cancellation), it retains the normalized handler's cleanup
instead of resurrecting `pressed` or the active edit. The failed candidate is
not committed or clamped.

`SliderDomainBuilder::format(ValueFormat)` is display-only and formats the
mapped domain value. It never formats or exposes the normalized fraction, and
it does not parse text or change interaction policy. The existing normalized
`slider(...)`, `slider_mapped(...)`, `slider_edit_mapped(...)`,
`SliderMessage`, and `SliderEditBatch` contracts remain source-compatible and
unchanged.

This bounded consumer intentionally does not add a domain edit batch or
`on_edit` path, numeric text editing, `NumericAdjustment` step/scrub/wheel
dispatch, or domain mapping for `Knob`. Those are separate non-goals rather
than implicit behavior of `slider_domain`.

### Additive Knob domain mapping

The executable additive domain consumer for radial knobs is the qualified
`radiant::application::knob_domain(value, adjustment)` constructor:

```rust
use radiant::application::{knob_domain, KnobDomainBuilder};
use radiant::widgets::{KnobDomainMessage, NumericAdjustment, ValueFormat};

let knob: KnobDomainBuilder<DomainAdjustment> =
    knob_domain(state.cutoff, DomainAdjustment::new())?
        .default_value(20.0)?;
let view = knob
    .format(ValueFormat::frequency())
    .message(|message: KnobDomainMessage<AdjustmentError>| Message::CutoffDomain(message));
```

The full generic signature is
`knob_domain(value: f32, adjustment: A) ->
Result<KnobDomainBuilder<A>, KnobDomainError<A::Error>>` where
`A: NumericAdjustment<f32>`. `KnobDomainBuilder` and `knob_domain` are
qualified exports from `radiant::application`; `KnobDomainMessage`,
`KnobDomainError`, `KnobDomainMappingAttempt`, and
`KnobDomainCancellationReason` are qualified exports from
`radiant::widgets::interaction` and are also re-exported from
`radiant::widgets`. These names are not in the common prelude.
Construction and `default_value(...)` each validate a finite domain value and
call the checked inverse exactly once. They reject an inverse error, a
nonfinite normalized result, or a normalized result outside `0.0..=1.0` with
the corresponding typed `KnobDomainError`; neither path silently clamps.
`KnobDomainBuilder::message(...)` additionally requires `A: 'static` and
`A::Error: Clone + 'static`, while construction and `default_value(...)` keep
the weaker `A: NumericAdjustment<f32>` bound.

The domain adapter caches both current and reset domain/normalized pairs. It
projects the full pointer lifecycle through `KnobDomainMessage`: start,
accepted update, end, and explicit `GestureCancelled` boundaries for focus
loss, pointer-capture loss, or disabled/read-only state. Keyboard and wheel
changes are atomic three-event `KnobDomainKeyboardGesture` and
`KnobDomainWheelGesture` values using the existing Knob metadata types. Reset
emits one `Reset` message even when it is a no-op and uses its cached default
without interaction-time remapping.

Each accepted pointer, keyboard, or wheel candidate is finite and in range
before one call to `NumericAdjustment::normalized_to_value`. A forward error,
nonfinite domain result, or invalid normalized candidate emits one typed
`MappingFailed` message with its attempt, normalized candidate, retained
domain value, and full `InteractionProvenance`; it emits no partial gesture.
Nonterminal pointer failures and atomic keyboard/wheel failures restore the
complete pre-input state, while terminal cleanup is never resurrected.
`format(ValueFormat)` is display-only and formats the mapped domain value. The
builder has no domain `on_edit`, edit-batch, numeric-text, or adjustment
step/scrub/wheel path. Existing normalized Knob, Slider, and common-prelude
APIs remain source-compatible and unchanged.

`KnobWidget` follows the same contract through the
qualified `KnobEditBatch` message. It carries one to four ordered
`EditEvent<f32>` values in fixed-capacity copy-only storage and exposes the
shared transaction plus concise value projection. Official Knob builders lower
a crate-private retained adapter that owns the active pointer transaction;
pointer relative-motion boundaries preserve their exact provenance, focused
keyboard, wheel, and reset inputs emit atomic `Begin`/`Update`/`Commit`
batches, and official retained focus-loss or pointer-capture interruption emits
a typed `Cancel`, including for a no-op active gesture. A meaningful changed
gesture projects a rollback through `value_change()`. Wheel input is ignored during an active
captured pointer gesture, and fresh same-ID projections remain authoritative
for value while compatible retained interaction state continues. The concise
`KnobBuilder::message`, `WidgetMessageMapper::knob`, and
`SurfaceNode::knob_mapped` paths project typed batches back to the existing
`KnobMessage` lifecycle. Those legacy projections preserve focus-loss
`GestureEnded` with the last value, including a no-op gesture, while pointer
capture cancellation emits no legacy message. `WidgetMessageMapper::knob_edits`,
`SurfaceNode::knob_edits_mapped`, `KnobBuilder::on_edit`, and
`application::knob_edit_mapped` receive complete ordered batches. These Knob
lifecycle APIs are qualified and are not exported through the common prelude;
the public `KnobWidget { common, props, state }` shape and legacy automation
gesture types remain source-compatible. `PanelResizeState` is also a shipped
shared-edit consumer alongside Slider and Knob. Its qualified
`resize_edit(...)` and `resize_collapsible_edit(...)` methods return one
accepted `EditEvent<f32>` boundary for each drag-handle input while the state
owns the active transaction beside its existing drag state. `Begin`, `Update`,
and `Commit` preserve the transaction and start size; `Cancelled` restores the
start size and emits `Cancel` with pointer provenance and no native evidence.
The concise `resize(...)` and `resize_collapsible(...)` methods remain the
compatibility projections: changed cancellation returns the restored size,
no-op cancellation remains lifecycle-only, and collapsible double activation
stays a discrete collapse/restore command outside the edit stream while
clearing active state. These APIs do not add `numeric_input` or controlled-ratio
`split_pane` runtime behavior. Runtime-owned split ratios use the qualified
layout capability registration and the same controller-owned capture/edit
lifecycle: a valid divider press begins one `PanelResizeState` edit, effective
captured motion updates the mounted ratio with a bounded current-surface
relayout, and release commits while interruption rolls back once. The
optional `SplitPaneBuilder::on_ratio_settled(...)` mapper emits exactly one
finite normalized mounted ratio after a meaningful successful runtime-owned
commit; it is silent for press, intermediate motion, no-op commits,
cancellation, capture loss, incompatible refresh, unmount, static mode, and
controlled-ratio mode. Mounted state mutation and terminal capture cleanup
complete before the mapped host message is reduced, and an ordinary
same-identity reprojection retains the captured interaction authority.
qualified layout capability registration, revision, and read-only normalized
hit-region declaration/projection contract is available separately. Version-3 capabilities may receive typed pointer
input and request runtime-owned layout capture; version 2 remains query-only.
This slice does not provide semantic/keyboard behavior or implement
`VirtualLayoutPolicy`.

These types are intentionally not exported through the common prelude.

### Numeric edit sessions

`NumericEditSession<T>` is the shipped, parser-agnostic editing-session
foundation under the qualified `radiant::widgets::interaction` module. The same
type is re-exported from `radiant::widgets`, but intentionally not from the
common prelude:

```rust
use radiant::widgets::interaction::{
    EditPhase, InteractionProvenance, NumericEditSession,
};

let provenance = InteractionProvenance::Keyboard { timestamp: None };
let mut session = NumericEditSession::begin(0.5_f32, "0.", provenance);
session.replace_draft("0.");
assert_eq!(session.draft(), "0.");

let event = match session.commit(0.75_f32, provenance) {
    Ok(event) => event,
    Err(_) => panic!("matching source should commit"),
};
assert_eq!(event.phase, EditPhase::Commit);
```

`begin(...)` creates one `EditEvent::begin` event. `draft()` and
`replace_draft(...)` preserve text verbatim, including empty, incomplete, and
invalid text, and draft changes emit no typed Update. `begin_event()` exposes
the initial event by reference. `commit(...)` accepts a caller-certified typed
value without parsing, validation, mapping, clamping, or other numeric policy;
same-source provenance with changed native metadata is accepted, while a
foreign source returns the unchanged session. `cancel(...)` similarly restores
the Begin value and returns a terminal `Cancel` event. Successful terminal
transitions consume the session and preserve the shared transaction and start
value.

This session deliberately does not provide a parser, validator, locale, range,
clamping, quantization, stepping, or `ValueMapping`/`ValueFormat` policy inside
the session. The separate official application Slider and Knob builders now
accept `ValueFormat` for display-only automation text; that attachment does not
add parsing or change emitted interaction values/events.

### Numeric codec contract

Radiant ships the qualified generic `NumericCodec<T>` and
`NumericParseResult<T>` contract under `radiant::widgets::interaction`; both
are also re-exported from `radiant::widgets` and are intentionally excluded
from the common prelude. Applications provide the concrete codec for their
domain type; Radiant does not expose a concrete public `f32` codec.

```rust
use std::fmt;
use radiant::widgets::interaction::{NumericCodec, NumericParseResult};

struct DomainValue;
struct CodecError;
struct MyCodec;

impl NumericCodec<DomainValue> for MyCodec {
    type Error = CodecError;

    fn parse(&self, text: &str) -> NumericParseResult<DomainValue> {
        # let _ = text;
        # NumericParseResult::Incomplete
    }

    fn format_editable(
        &self,
        value: &DomainValue,
        output: &mut dyn fmt::Write,
    ) -> Result<(), Self::Error> {
        # let _ = (value, output);
        # Ok(())
    }
}
```

`parse(...)` owns the codec grammar and domain validation, returning
`Incomplete`, `Invalid`, `OutOfRange`, or `Valid(T)`. `format_editable(...)`
borrows `&T`, writes canonical editable text into caller-owned `fmt::Write`
storage, and returns the codec's associated error type. Codecs never consult
ambient locale, and display-only `ValueFormat` is never used for parsing.

The codec remains a policy boundary: it does not own edit sessions, focus,
dispatch, or typed output. The bounded public text consumer that composes this
contract is documented below. Non-valid drafts remain inside that consumer
until an application supplies a valid terminal value.

### Numeric text input consumer

For the target numeric interaction set, the first actual text mutation is the
TextEdit admission boundary. The shipped consumer now uses the crate-private
shared gate: it acquires TextEdit only when the incumbent is None, and a
different pending or active owner denies text admission before parsing,
formatting, focus transfer, or edit lifecycle mutation. Complete-mode
explicit-policy KeyboardAdjustment, PointerScrub, and NumericInput wheel
consumption also use the gate; the NumericInput IME/composition consumer,
the widget-local accessibility policy consumer, and generic runtime
accessibility dispatch are shipped. Other native adapters, virtual
materialization/scrolling, scheduler/cache/renderer policy, repeat behavior,
and product policy remain separate unshipped boundaries.

Radiant ships a bounded public, text-first numeric consumer through the explicit
`radiant::application::{numeric_input, NumericInputBuilder}` exports. The
qualified `NumericInputConstructionError<CodecError, AdjustmentError>`,
`NumericInputEditBatch<T>`, and the codec/adjustment contracts are available
from `radiant::widgets` (and their qualified `interaction` module), but none of
these numeric-input-specific types are exported through the common prelude.

`NumericInputEditBatch<T>` is the shipped bounded incremental carrier. Its
private inline storage has capacity three and accepts exactly the non-empty
fragments `[Update]`, `[Commit]`, `[Cancel]`, `[Begin, Update]`,
`[Begin, Commit]`, `[Begin, Cancel]`, and `[Begin, Update, Commit]`; a fragment
must preserve one transaction. The text-first widget emits `[Begin, Commit]`
and `[Begin, Cancel]` for its terminal lifecycle; complete-mode keyboard and
wheel adjustment consume incremental and atomic shapes, including the
three-event wheel fragment. Replacement teardown of an active TextEdit remains
one of the `[Begin, Cancel]` boundaries.

Construction requires both application policies:

```rust
let input = radiant::application::numeric_input(value, codec, adjustment)
    .expect("codec formatting and adjustment validation should succeed")
    .on_edit(|batch| Message::NumericEdit(batch));
```

The constructor writes the initial value with `NumericCodec::format_editable`
and validates `NumericAdjustment::value_to_normalized`. A formatting or inverse
mapping failure returns the corresponding typed construction error; the
consumer does not invent fallback text or a fallback range.

The current consumer begins a `NumericEditSession<T>` on the first actual text
mutation after focus. Draft text remains verbatim. Each changed draft is parsed
once and its `Incomplete`, `Invalid`, `OutOfRange`, or `Valid(T)` classification
is retained for the active session. Invalid, incomplete, or out-of-range drafts
remain visible without typed output. The synchronous
`Widget::prepare_focus_loss` seam reads that retained classification without
calling the arbitrary codec, so it is
allocation-free: invalid focus loss vetoes and keeps focus, while valid focus
loss commits one typed `NumericInputEditBatch<T>` containing exactly `Begin`
then `Commit`. Enter has the same two-event commit boundary. Escape cancels an
active edit with exactly `Begin` then `Cancel` and restores the starting value,
draft, caret, and selection.

During refresh reconciliation, the numeric consumer also consumes
`Widget::prepare_replacement(...)`. It preserves an active TextEdit only for an
exact `NumericInputWidget` successor with the same stable ID and external value
that remains enabled and non-read-only. Removal, incompatible type, changed ID or
value, disabled successor, and read-only successor conservatively publish one
`NumericInputEditBatch<T>` rollback with ordered `Begin` then `Cancel` through the
retiring widget's mapper, restore the starting value/draft/caret/selection, and
release TextEdit ownership. The transaction identity is retained and the
teardown cancellation uses keyboard provenance without fabricated timestamp
metadata. Repeated teardown after cleanup is silent; invalid, incomplete, and
out-of-range drafts cancel from their retained snapshot without a new parse,
format, step, or adjustment-policy call.

Retained text, caret, selection, and session state cross a same-ID
reprojection only when the previous widget has an active edit and the fresh
value remains compatible. With no active session, the current projection's
canonical codec-formatted text and caret remain authoritative; stale
noncanonical committed text is not retained.

This text-edit path intentionally stops at generic text editing,
replacement teardown, and the explicit complete-mode keyboard adjustment
contract. Normalized `KeyRelease` plumbing is shipped across the runtime/native
boundary. Complete-mode PointerScrub and NumericInput wheel consumption are
separate shipped consumers, and NumericInput IME/composition plus the
widget-local accessibility policy and generic runtime accessibility dispatch
are now shipped consumers; `Slider` and `Knob` adoption is complete, while
native unit/phase adapters, scheduler/renderer integration, and product numeric
policy remain follow-up slices. The supplied
`NumericAdjustment<T>` step, scrub, and wheel methods are consumed only by
complete mode with their explicit policies.

The first acceptance fixtures are exact and intentionally small: a `u32` count
over `0..=100` with ASCII-digit text and base/fine/coarse steps `1/1/10`; a
linear `Percent` value over `0..=1` with invariant decimal text, no editable
`%` suffix, and steps `0.01/0.001/0.1`; and a logarithmic `FrequencyHz` value
over `20..=20_000` with invariant decimal text plus exactly one ASCII-space
`Hz` suffix and normalized steps `0.01/0.001/0.1`. Canonical output is
shortest round-tripping text; adjustment clamps only at declared boundaries,
while typed text never silently clamps. Decibel, tempo, arbitrary-unit, and
product-specific locale codecs remain application-supplied. The current
consumer exercises generic construction and the `u32` text lifecycle; the
other adjustment-consuming behavior remains outside this slice.

### Numeric interaction ownership and admission (TextEdit admission and teardown shipped)

This is one shared, backend-neutral contract for the numeric
interaction set. The crate-private gate is shipped for TextEdit admission,
terminal cleanup, replacement teardown, and compatible reprojection in the
generic text consumer, and complete-mode explicit-policy KeyboardAdjustment is
shipped; complete-mode PointerScrub and NumericInput wheel consumers plus their
wheel-sequence routing foundation are shipped, and NumericInput IME composition
plus the widget-local accessibility policy and generic runtime accessibility
dispatch are shipped. Other native adapters, virtual materialization/scrolling,
scheduler/cache/renderer policy, repeat behavior, and product policy remain
separate unshipped boundaries. The gate is not a public Rust API, native
adapter, storage shape, or product policy.
The contract applies to each stable numeric-input identity and is the common
arbitration boundary for all six interaction kinds.

The owner vocabulary is conceptual and consists exactly of TextEdit,
ImeComposition, KeyboardAdjustment, PointerScrub, WheelSequence,
AccessibilityEdit, and None. These names do not prescribe public Rust variants,
handles, fields, or storage.

#### Target incumbent-owner gate

At every admission boundary, each stable numeric-input identity has exactly one
incumbent owner: one of the six interaction owners above or None. A pending
owner counts as an owner even before it has emitted Begin; this includes a
pending wheel sequence or pointer capture. None means that no interaction is
pending or active.

An interaction may acquire ownership only when the current incumbent is None.
Once an interaction is admitted, its owner is established before its first
numeric operation or lifecycle publication. For every ordered pair of distinct
owners, the incumbent owner wins: the new interaction is denied and cannot
preempt, queue behind, or transfer ownership from the incumbent. A matching
sample for the incumbent may continue only through that interaction's own
identity, authority, capture, and continuity rules; it is not a new admission
and cannot join guessed history.

The shared gate is checked before a denied interaction parses, formats, steps,
scrubs, wheel-adjusts, commits, cancels, transfers focus, or mutates any
interaction state. Denial produces no partial lifecycle. The incumbent retains
its exact draft/value, caret/selection, capture/continuity, transaction
identity, authority, and interaction-specific routing. A gate denial never
turns an input into a cancellation or terminal boundary for the incumbent.

Ownership ends only when the owning interaction reaches its own defined
terminal, cancel, authority, identity, focus, disable, or read-only boundary.
That interaction's required rollback or terminal result is published before
its cleanup completes; the shared owner becomes None only after cleanup is
complete. No later input is admitted during cleanup, and no input joins the
prior transaction or infers continuity from history. This gate does not choose
native cancellation semantics and never invents a preemption boundary.

Stable identity and current authority are required admission evidence. Missing,
stale, malformed, ambiguous, or otherwise observational evidence cannot create,
transfer, or terminate shared ownership. If an already admitted interaction's
own contract defines a malformed sample as a conservative cancellation
boundary, that remains its interaction-specific rule; the shared gate itself
does not invent that boundary or use the sample to preempt another owner.
Timestamps, sequence ranges, pointer geometry, diagnostics, snapshots, and
other metadata remain observational and cannot authorize scheduling, cache
admission, reuse, renderer resources, materialization, scrolling, execution,
or any other authority.

The gate preserves each interaction's existing fallback and ordering rules:

- An ineligible or conflicting wheel sample remains unhandled for widget or
  scroll-container fallback wherever the wheel contract permits that fallback.
- An unmodified primary pointer remains ordinary text caret/selection input;
  a blocked scrub attempt creates no scrub lifecycle.
- Keyboard host-shortcut first refusal occurs only at an uncaptured initial
  boundary. Captured matching repeats and release stay with their owner, and a
  gate denial does not broaden host routing.
- Accessibility admission reports Blocked { owner } with the incumbent owner
  at its pre-focus or post-focus check; it does not cancel or mutate that owner.
- Matching IME commit/key suppression remains ahead of ordinary text routing.
  IME preedit is never sent to numeric parsing; numeric parsing may begin only
  after an accepted committed replacement.

Keyboard adjustment and IME composition therefore use the same gate as every
other numeric interaction. Neither KeyboardAdjustment nor ImeComposition may
start while any different owner is pending or active. An IME Start must pass
the gate before it captures composition identity/ranges, and an IME preedit
remains composition text rather than numeric input until Commit. These rules do
not select platform cancellation behavior or allow either interaction to
interrupt an incumbent.

#### Target shared-owner acceptance fixtures

The target contract is accepted only when this matrix holds. The shipped text
consumer covers the TextEdit admission, cleanup, replacement teardown, and
compatible reprojection foundation, and complete-mode explicit-policy
KeyboardAdjustment, PointerScrub, NumericInput wheel, and NumericInput
IME/composition consumption are shipped; generic runtime accessibility dispatch
is shipped. Other native adapters, virtual materialization/scrolling,
scheduler/cache/renderer policy, repeat behavior, and product policy remain
separate unshipped boundaries:

| Fixture | Expected target behavior |
| --- | --- |
| 1. Keyboard press during PointerScrub, WheelSequence, ImeComposition, or AccessibilityEdit | The uncaptured keyboard attempt follows only the existing initial host-shortcut refusal; if it reaches numeric admission, the shared gate denies KeyboardAdjustment. No numeric step, parse, format, lifecycle, focus transfer, or incumbent mutation occurs. |
| 2. IME Start during TextEdit, KeyboardAdjustment, PointerScrub, WheelSequence, or AccessibilityEdit | The shared gate denies ImeComposition before composition capture. No preedit, numeric parse, focus/identity transfer, cancellation, or incumbent mutation occurs. |
| 3. Wheel and pointer attempts during every other owner | A wheel attempt during TextEdit, ImeComposition, KeyboardAdjustment, PointerScrub, or AccessibilityEdit, and a scrub attempt during TextEdit, ImeComposition, KeyboardAdjustment, WheelSequence, or AccessibilityEdit, are denied by the incumbent-owner gate. Ineligible wheel input retains permitted scroll fallback; unmodified pointer input remains text selection; no partial lifecycle is emitted. |
| 4. Accessibility pre-focus and post-focus checks | With an incumbent before focus transfer, accessibility returns Blocked { owner } without transferring focus. If an owner appears after an otherwise allowed transfer, the post-focus check returns Blocked { owner } before numeric mutation and performs no further focus or interaction mutation. |
| 5. Terminal cleanup then independent admission | After an owner reaches its own terminal/cancel/authority boundary and cleanup completes, the owner is None; a later eligible interaction is admitted with a fresh transaction identity and does not join prior capture, continuity, or history. |
| 6. Same-boundary IME commit and matching key suppression | An accepted IME Commit wins at the shared delivery boundary; matching-key suppression remains a deferred adapter boundary, while ordinary keyboard/character routing otherwise remains unchanged. |
| 7. Stale or observational evidence | Missing, stale, malformed, ambiguous, timestamp, sequence, geometry, snapshot, or diagnostic evidence cannot create, transfer, or terminate ownership and cannot authorize execution or fallback changes. |
| 8. Denied admission preserves the incumbent | A denied candidate performs no parse, format, step, scrub, wheel adjustment, commit, cancel, focus transfer, or partial lifecycle. The incumbent's exact draft/value, caret/selection, capture/continuity, transaction identity, authority, and routing remain unchanged. |
| 9. None admits one interaction | With None, one eligible interaction acquires its owner before its first operation; a second competing interaction at the same boundary observes that incumbent and is blocked without joining or replacing it. |

### Target IME/composition lifecycle (TextInputWidget, NumericInputWidget, and native Winit consumers shipped)

For a numeric input, the shared owner gate is checked after the focused stable
identity is resolved and before Start captures composition state. Start may
acquire ImeComposition only when the incumbent is None; a different pending or
active owner denies Start without preedit, parsing, cancellation, focus
transfer, or incumbent mutation.

This subsection describes the shipped, qualified backend-neutral foundation,
the single-line `TextInputWidget` and `NumericInputWidget` consumers, and the
first native consumer in `src/gui_runtime/native_vello/generic_runtime/ime.rs`.
The same Winit normalizer/router is called by the primary and auxiliary Vello
window loops. `radiant::widgets::interaction` provides the validated
`CompositionRange`, `CompositionSample`, `CompositionPhase`, and typed
validation-error vocabulary. `Widget` provides default-compatible,
object-safe composition hooks, `SurfaceRuntime` provides the private
fixed-size focused ownership kernel, and the widgets own captured committed
value/range, transient preedit, scalar selection, and lifecycle terminal
behavior. The public normalized lifecycle samples remain exactly `Start`,
visible `Update { preedit, selection }`, `Commit { text }`, and `Cancel`. A
native adapter with an explicitly hidden preedit selection uses the additive
object-safe `Widget::handle_hidden_composition_update(preedit, timestamp)` hook
instead of adding a public sample variant. Its default conservatively routes
through existing cancel behavior so legacy custom widgets do not retain stale
visible selection. Built-in text consumers keep actual focus true while hidden,
zero the existing caret/selection colors, and rely on the native encoder to
skip zero-alpha caret/selection geometry; no public text-state or paint fields
are added.

The native Winit adapter handles exactly `Ime::Enabled`, `Ime::Preedit`,
`Ime::Commit`, and `Ime::Disabled`. `Enabled` reports platform capability only;
it never starts composition. The first `Preedit`, including an empty preedit,
or a direct `Commit` queries the current authoritative focused widget for its
exact committed-value scalar replacement range and selection, then starts the
existing owner. A Winit preedit cursor pair is an adapter-local byte range and
is converted to Unicode-scalar coordinates only when ordered, in bounds, and
on UTF-8 character boundaries. `None` remains hidden; no `0..0`, end-of-
preedit, or previous selection is fabricated. Invalid evidence cancels the
active owner, or retains/cancels without mutation when no owner can be admitted.
Winit IME events carry no native timestamp here, so normalized samples retain
`None` and fabricate no sequence metadata.

The bounded native Winit candidate-area publication is shipped: the adapter
projects a finite logical caret area from exactly one focused `PaintTextInput`
and publishes it through the actual per-window Winit cursor-area call, with
conservative invalid/ambiguous evidence and repeat suppression fenced by
`WindowId`, `NativeTargetGeneration`, and the actual native `DpiScale`.
Native Japanese/Chinese IME acceptance remains unperformed. The checked
`macos_text_input_ime_acceptance` target provides the primary-window live
procedure and production-projection tests for that boundary, but no live
Japanese IME or AppKit candidate-panel run is evidence here. Matching-key
suppression, candidate behavior beyond this bounded
caret-area publication, multiline editing, product integration, and other
platform adapters remain separate boundaries.

`NumericInputWidget` consumes the same lifecycle through the shared owner gate:
preedit updates remain local and do not parse or publish; valid committed text
goes through `TextInputState` sanitization and `NumericCodec` once to emit one
`[Begin, Commit]` batch; invalid or incomplete commits remain correctable as
text editing, and Cancel or focus loss restores the captured edit state.
Compatible refresh retains the composition owner; incompatible replacement
cancels it.

All generic ranges in this contract are Unicode-scalar ranges. Native adapters
own platform offsets and translate only validated evidence into the generic
contract. The generic contract does not prescribe UTF-16, byte, grapheme, or
any other backend-specific offset convention.

Every composition replacement range and every visible `Update.selection` is a
bounded half-open Unicode-scalar interval `[start, end)`. Both endpoints lie
in `0..=scalar_len` and `start <= end`. For a replacement range, `scalar_len`
is the captured committed text scalar length; for a visible `Update.selection`,
it is that update's preedit scalar length. Hidden preedit delivery has no range and
`start == end` means a collapsed caret only when a range is present. Malformed,
inverted (`start > end`), or out-of-bounds endpoints are invalid evidence and
follow the conservative cancel/retain/no-committed-mutation outcome below.

Ownership is explicit:

- The application owns committed text and its durable `TextInputRevision` and
  value.
- The widget owns transient pre-edit text, the captured scalar replacement
  range, scalar selection/caret, and the composition lifecycle.
- The runtime pins composition to one focused widget with a stable identity.
- The native adapter owns platform IME behavior and translates only validated
  native range evidence into the generic contract; the Winit adapter shares one
  router across primary and auxiliary Vello windows.

Composition is text-input metadata, not numeric edit provenance. It adds no
new `InteractionSource`, `InteractionProvenance`, `NumericEditSession`, or
`EditEvent` phase.

#### Start, update, commit, and cancel

`Start` is accepted only for the currently focused stable widget. The shipped
`TextInputWidget` consumer captures the focused widget identity, committed
text, scalar replacement range, and scalar selection at the beginning of the
composition; runtime ownership and the widget's revision-aware refresh seam
bind later samples to that identity and compatible authority.

`Update` replaces the preedit verbatim; it never appends to the previous
preedit. A visible `Update` carries an explicit, scalar-indexed selection
inside the preedit, including a collapsed caret. Hidden selection is delivered
through the additive hook and explicitly hides the native cursor/selection.
Empty preedit is valid and remains visible; hidden selection never becomes a
collapsed range or reuses a prior selection.
`Update` never mutates committed text, emits ordinary `Changed`, invokes a
`NumericCodec`, or creates a `NumericEditSession`, `EditEvent`, or numeric edit
output.

`Commit` atomically replaces exactly the captured scalar replacement range with
its `text` and then clears composition. The target post-commit selection is a
collapsed scalar caret immediately after the inserted text, or at the captured
replacement start when `text` is empty. An accepted `Commit` emits exactly one
ordinary committed text change after the atomic replacement; a stale,
malformed, or otherwise rejected `Commit` emits none. A `Start` followed
directly by `Commit` is valid; no `Update` is required. A separate numeric
consumer may parse or convert only after this committed replacement, never
while preedit is active.

`Cancel` clears preedit and restores the original committed text, captured
replacement range, and scalar selection from `Start`. It emits no committed
text change.

Native IME delivery reaches the focused composition owner before any ordinary
keyboard/character handling changes. Matching-key suppression is deliberately
deferred: this slice does not invent platform matching evidence or suppress a
later ordinary key. Non-IME keyboard and character routing remains on its
existing path.

#### Authority, identity, and malformed evidence

A compatible same-ID reprojection with an equal or older external
`TextInputRevision` preserves composition, preedit, and scalar selection. A
newer external authority cancels the old composition before replacing committed
state and applying the newer revision/value. Identity loss or change,
incompatible value or capability, disablement, and read-only state cancel
composition. Uncommitted focus loss cancels rather than commits; this contract
adds no implicit ordinary-text terminal, so only explicit composition `Commit`
commits. If explicit `Commit` and focus loss are observed at the same boundary,
`Commit` is ordered first and wins. If focus loss wins an earlier boundary,
the old composition is cancelled and its later terminal sample is stale.

Every `Start`, `Update`, `Commit`, and `Cancel` from an old widget identity or
captured revision is stale and ignored. Such a sample cannot mutate the new
widget's committed text, selection, or revision.

Malformed native ranges and interval endpoints are unknown evidence, not a
reason to guess. Malformed or inverted (`start > end`) intervals, out-of-bounds
endpoints, invalid scalar ranges, invalid UTF-16-to-scalar mappings, and other
malformed native range evidence must not be clamped, appended, silently
accepted, or converted by an invented convention. The conservative target
outcome for an invalid `Start`, `Update`, or `Commit` is to cancel composition,
retain committed text and current scalar selection, and make no committed
mutation. A typed diagnostic may record the rejection, but that diagnostic is
not a shipped public API.

#### Timestamp and synthetic-input rules

The exact native sample timestamp is preserved through each lifecycle sample;
an event with no native timestamp retains no timestamp. Synthetic and
backend-neutral constructors omit timestamps. Composition does not create a
new interaction source or numeric provenance, and no keyboard sequence range
is fabricated.

#### Acceptance fixtures

The target contract is accepted only when these fixtures hold:

Every interval in these fixtures uses the contract-wide bounded half-open
Unicode-scalar convention above.

| Fixture | Expected result |
| --- | --- |
| 1. Start on committed `"a"` with captured replacement range `0..1` and captured scalar selection `0..1`; `Update { preedit: "あ", selection: 1..1 }`; then `Update { preedit: "あい", selection: 1..2 }` | Committed text remains exactly `"a"`; final preedit is exactly `"あい"` with final selection exactly `1..2`; the second update replaces rather than appends; no ordinary `Changed`, `NumericCodec` call, or numeric edit output occurs. |
| 2. Empty preedit | Empty preedit is a valid visible state, not an implicit cancel. |
| 3. Commit `"あい"` | Exactly one atomic captured-range replacement and one committed text change; parsing/value conversion is permitted only after commit. |
| 4. Cancel | Original committed text, captured range, and selection are restored with no committed text change. |
| 5. Direct commit with no update | `Start` followed directly by `Commit` is valid and produces one committed change. |
| 6. Native Winit delivery and hidden cursor | `Enabled` alone does not start composition; a first `Preedit` or direct `Commit` captures the focused scalar context; `Preedit(..., None)` remains hidden and never becomes `0..0`, end-of-preedit, or a previous selection. |
| 7. Native commit followed by ordinary key text | The native commit is handled by the composition owner; matching-key suppression remains deferred and ordinary keyboard/character routing otherwise remains unchanged. |
| 8. Reprojection and stale samples | Same-ID compatible equal/older revision preserves composition/preedit/selection; newer authority cancels/replaces; stale old `Start`/`Update`/`Commit`/`Cancel` is ignored. |
| 9. Identity and focus boundaries | Identity change, disable/read-only, and uncommitted focus loss cancel/restore; explicit commit wins before focus loss when both share a boundary. |
| 10. Malformed native range | Invalid byte endpoints, inverted endpoints, out-of-bounds endpoints, and non-character boundaries conservatively cancel and retain committed text/selection; no clamp, guess, append, or mutation occurs. |
| 11. Metadata and window parity | Winit timestamps remain absent, no sequence range is fabricated, and primary and auxiliary Vello loops produce identical owner/output behavior. |

### Metadata-aware focused-key ownership and preemption (generic kernel and complete numeric consumer shipped)

This additive, backend-neutral kernel now routes metadata-aware focused keys
across generic `SurfaceRuntime` dispatch and native adapters. Complete-mode
numeric input consumes this kernel for explicit `KeyboardAdjustment` ownership.
`Widget` exposes two defaulted,
object-safe opt-in queries, `participates_in_focused_key_routing()` and
`captured_focused_key() -> Option<WidgetKey>`; existing widgets retain the
key-only compatibility path. The controller owns one private fixed-size capture
record, host ordering, owner cancellation, stale/orphan decisions, and refresh
reconciliation. Native Vello only normalizes evidence and delegates to that
authority. No public focused-key phase/route enum, event field, generation, or
token is added.

The conceptual outcomes remain illustrative internal decision names:
`FocusedKeyPhase::{InitialPress, RepeatPress, Release}` and
`FocusedKeyRoute::{HostFirst, FocusedOwner, Ignore}`. They describe routing
decisions, not public Rust types or output messages. The shipped kernel defines
routing using existing focus, interaction, and authority state; it does not
decide which widget owns a key.

Routing evidence carries the exact key, an explicit phase (press, repeat, or
release), lossless `KeyboardModifiers`, and an optional timestamp. Release is a
distinct phase and is never treated as another press. Capture comes only from
current interaction state: timestamp, repeat, modifiers, diagnostics, and
history cannot create authority. Capture pins the stable focused widget
identity. Replacement, focus change or loss, and authority loss make a
continuation stale or unavailable; they never rebase it to a successor, a new
focus, or the host.

The route order is:

- `HostFirst` is available only for an uncaptured initial press. Host
  resolution runs once. A handled input never reaches the widget. An
  unhandled input may be delivered once only after current focus and authority
  are revalidated; it is not retried through the host.
- `FocusedOwner` requires the exact current focused identity together with
  interaction-specific pending or active ownership, or an owner-defined
  cancellation key for that current owner. It bypasses host resolution and
  delivers the evidence once to the owner. Owner-first routing has no host
  fallback even when the owner emits no output. Modified repeats and
  cancellation keys remain eligible when the owner contract permits them.
- `Ignore` applies to orphan repeats or releases, stale capture, an
  owner-defined competing sample, and an unavailable continuation. Ignored
  input is delivered neither to a rebased focus nor to the host.

Generic, native, and synthetic paths use the same routing matrix. A native
adapter may translate physical keys and modifier representations into this
lossless evidence, but it may not add precedence rules. Synthetic and
backend-neutral samples exercise the same decisions; the allowlisted
controller, native, and public-API fixtures cover these shipped outcomes.

#### Target focused-key routing acceptance fixtures

| Fixture | Expected target decision |
| --- | --- |
| Handled uncaptured initial press | `HostFirst` resolves the host once; the handled input reaches the widget zero times. |
| Unhandled uncaptured initial press | `HostFirst` resolves the host once; after focus/authority revalidation, the input reaches the current focused owner once. |
| Captured matching repeat and release | `FocusedOwner` delivers each matching sample to the pinned owner once; the host is never resolved. Release remains a release. |
| Modifier change preserved | An owner-eligible repeat or cancellation sample preserves its exact changed `KeyboardModifiers` and remains owner-first; modifiers do not silently create or remove authority. |
| Active contract-defined Escape | An active owner-defined Escape cancellation sample is `FocusedOwner`, delivered once to the owner, and never resolved by the host. |
| Competing or orphan sample | An owner-defined competing key, orphan repeat, or orphan release is `Ignore`; it reaches neither widget nor host and does not alter capture. |
| Stale identity or authority | A sample whose pinned identity or authority is no longer current is `Ignore`; it is not rebased, sent to a fallback host path, or delivered to a successor/new focus. |
| Native, backend-neutral, and synthetic equivalence | Equivalent evidence produces the same route, host-call count, and owner-delivery count on all three paths; native translation adds no precedence. |

### Numeric interaction output mapping (TextEdit, complete keyboard, and generic routing shipped)

This is the shipped, backend-neutral TextEdit and complete-mode keyboard output
mapping. It defines one selected public mapper and one host dispatch per input or
teardown boundary. The current implementation ships `on_interaction`,
crate-private output-mode storage, mode-specific encoders, TextEdit terminal
validation, explicit-policy keyboard stepping, typed failures and rollback,
complete mapping, and the generic metadata-aware focused-key routing kernel.

The complete binding on `NumericInputBuilder<T, C, A>` is:

```rust
pub fn on_interaction<Message: 'static>(
    self,
    map: impl Fn(NumericInputInteractionBatch<T, A::Error, C::Error>) -> Message + 'static,
) -> ViewNode<Message>
where A::Error: 'static, C::Error: 'static;
```

The associated-error order is fixed to the existing
`NumericInputInteractionBatch<T, StepError, FormatError>` order: `A::Error` is
the step/adjustment error and `C::Error` is the codec/format error. The
signature has only the shown `'static` requirements; it does not add
`Clone`, `Send`, or `Sync` requirements. Construction errors remain
constructor errors and are not delivered through this mapper.

`on_interaction` is the sole complete mapping boundary. Complete mode emits
exactly one `NumericInputInteractionBatch<T, A::Error, C::Error>` payload type
for the shipped TextEdit and keyboard lifecycles; it never alternates a bare
`NumericInputEditBatch<T>` with an interaction batch. One accepted TextEdit
input or teardown boundary produces at most one interaction batch, invokes the
selected mapper at most once, and dispatches at most one host message. The
interaction parts are not separately reduced or dispatched. Keyboard production
uses the same one-batch boundary; in particular, a repeat
rollback `[Edit([Cancel]), failure]` remains one ordered batch with rollback
before the exact typed failure and no interleaving mapper or host dispatch.

TextEdit `[Begin, Commit]` and `[Begin, Cancel]` terminal fragments, including
replacement teardown, are represented in complete mode as exactly one outer
`Edit(NumericInputEditBatch<T>)` part. The inner batch and both inner events
retain their original transaction, value, phase, provenance, and timestamp
without rewriting. The validator accepts these two TextEdit terminal shapes in
addition to the existing keyboard shapes. The capacities remain
`NumericInputEditBatch::MAX_EVENTS == 3` and
`NumericInputInteractionBatch::MAX_INTERACTIONS == 2`.

`on_edit` remains the exact TextEdit-only compatibility binding. It maps the
existing bare `NumericInputEditBatch<T>`, does not enable KeyboardAdjustment,
and does not change the current text lifecycle. In this mode, `on_edit` plus
arrows is a no-op: there is no step, format call, capture, transaction, typed
failure, mapper invocation, or value mutation. `step_modifiers` remains inert
in compatibility mode. A builder selects one binding mode, so complete and
compatibility mappers are never broadcast or duplicated.

TextEdit ownership is established by current stable identity, interaction
state, and the shared admission boundary; it is never inferred from
`InteractionProvenance` alone. A replacement teardown uses the selected
retiring mapper mode: complete mode wraps its rollback through
`on_interaction`, while compatibility mode retains the bare `on_edit`
rollback. A denied, unchanged, stale, orphaned, or blocked input emits no
batch, mapper call, or host message. An invalid text draft emits no interaction
failure. Typed step and format errors remain their exact UI-local typed parts:
they never panic, become a string, log, no-op, bare edit, or fallback output.

The validator preserves the existing keyboard shapes: one keyboard `Edit`
containing `[Begin, Update]`, `[Update]`, `[Commit]`, or `[Cancel]`, one initial
typed `StepFailed` or `FormatFailed`, or one ordered `[Edit([Cancel]), failure]`
repeat rollback. It also accepts exactly the TextEdit terminal shapes above;
all retain the existing capacity and illegal-shape rejection rules. TextEdit
mapping, typed-failure production, numeric stepping, and mapper exclusivity are
shipped. The widget-local accessibility policy and generic runtime accessibility
dispatch are shipped; other native adapters, virtual materialization/scrolling,
scheduler/cache/renderer policy, repeat behavior, and product policy remain
separate unshipped boundaries; the
generic PointerScrub, NumericInput wheel, and NumericInput IME/composition consumers,
wheel-sequence routing foundation, and metadata-aware routing kernel are shipped
in the preceding sections.

#### Numeric interaction output mapping acceptance fixtures (shipped TextEdit and complete keyboard)

| Fixture | Expected target behavior |
| --- | --- |
| 1. Existing `on_edit` commit | One bare `NumericInputEditBatch<T>` containing `[Begin, Commit]` is mapped once and produces one host message. No interaction envelope is emitted. (Shipped.) |
| 2. Existing `on_edit` cancel or teardown | One bare `NumericInputEditBatch<T>` containing `[Begin, Cancel]` is mapped once, including replacement teardown, and produces one host message. (Shipped.) |
| 3. `on_edit` plus arrows | Arrows perform no step, format call, capture, transaction, typed failure, mapper invocation, or mutation; `step_modifiers` remains inert. (Shipped.) |
| 4. Complete-mode TextEdit commit | One `NumericInputInteractionBatch<T, A::Error, C::Error>` contains exactly one outer `Edit(NumericInputEditBatch<T>)` with the unchanged `[Begin, Commit]` inner events; the mapper and host are each used once. (Shipped.) |
| 5. Complete-mode TextEdit cancel or teardown | One interaction batch contains exactly one outer `Edit(NumericInputEditBatch<T>)` with the unchanged `[Begin, Cancel]` inner events; the selected retiring mapper and host are each used once. (Shipped.) |
| 6. Initial keyboard step | One interaction batch contains one keyboard `Edit` with `[Begin, Update]`, and one mapper invocation and host message occur for the accepted boundary. (Shipped.) |
| 7. Keyboard repeat and release | Each accepted repeat or release produces one interaction batch containing `Edit([Update])` or `Edit([Commit])`, respectively, with one mapper invocation and one host message. (Shipped.) |
| 8. Initial typed step or format failure | The one interaction batch contains the exact typed `StepFailed` or `FormatFailed` part only; no edit, transaction, capture, or fallback output occurs. (Shipped.) |
| 9. Repeat typed failure | One interaction batch is ordered `Edit([Cancel])` then the exact typed failure, and one mapper invocation and host message occur with no interleaving. (Shipped.) |
| 10. Denied, unchanged, stale, orphaned, or competing input | No interaction batch, mapper invocation, host message, or mutation is emitted. (Shipped.) |
| 11. Associated-error contract | The complete mapper uses `NumericInputInteractionBatch<T, A::Error, C::Error>` in that order, with only `A::Error: 'static` and `C::Error: 'static`; no `Clone`, `Send`, or `Sync` bound is introduced. (Shipped.) |
| 12. Mapper exclusivity | Each builder selects exactly one compatibility or complete binding mode; it never broadcasts to both mappers or duplicates a host dispatch, and `on_edit` remains TextEdit-only. (Shipped.) |
| 13. Current-runtime truth | TextEdit mapping, complete-mode explicit-policy `KeyboardAdjustment`, `PointerScrub`, `NumericInput` wheel adjustment, NumericInput IME/composition, the widget-local NumericInput accessibility policy, generic runtime accessibility dispatch, terminal validation, both binding modes, the generic wheel-sequence routing foundation, and the generic metadata-aware focused-key routing kernel are shipped. Other native adapters and product-policy consumers remain unshipped. |

### Complete-mode numeric keyboard adjustment (explicit policy shipped; other native adapters and product policy remain target-only)

Keyboard admission uses the shared incumbent-owner gate before any numeric step
or keyboard transaction. KeyboardAdjustment may start only when the stable
numeric identity has owner None; a different pending or active owner wins and
the keyboard attempt does not parse, format, step, commit, cancel, transfer
focus, or mutate that incumbent. The existing host-shortcut first refusal
remains limited to an uncaptured initial boundary.

The preceding `numeric_input` section documents the shipped text-first and
complete-mode consumer. Normalized `Event::KeyRelease { key, modifiers,
timestamp }` and `WidgetInput::KeyRelease { key, modifiers, timestamp }`
plumbing is shipped, and complete mode consumes the contract below only when an
explicit `NumericStepModifiers` policy is attached. Generic runtime
accessibility dispatch is shipped; other native adapters, virtual
materialization/scrolling, scheduler/cache/renderer policy, repeat behavior,
and product policy remain separate target-only boundaries. NumericInput
IME/composition, generic PointerScrub, NumericInput wheel consumption, and wheel
routing foundations are shipped.

Radiant ships the pure, qualified `KeyboardModifier` and
`NumericStepModifiers` selector foundation. Both are also re-exported from
`radiant::widgets` and are intentionally absent from the common prelude.
`NumericStepModifiers::new(fine, coarse)` stores explicit semantic selectors;
`fine()` and `coarse()` expose them, and `select_step(modifiers)` recomputes
`Base`, `Fine`, or `Coarse` from one lossless `KeyboardModifiers` sample with
Fine precedence. The associated `MACOS_DEFAULT` and `WINDOWS_LINUX_DEFAULT`
constants provide explicit Shift/Command and Shift/Control policies; Radiant
does not resolve either constant from the host platform.

The qualified public output envelope below is the fixed-capacity complete-mode
keyboard contract. Its exact interaction-part shape is:

```rust
use std::rc::Rc;

use radiant::widgets::interaction::NumericInputEditBatch;

pub enum NumericStepAttempt {
    Initial,
    Repeat,
}

pub enum NumericInputInteraction<T, StepError, FormatError> {
    Edit(NumericInputEditBatch<T>),
    StepFailed {
        attempt: NumericStepAttempt,
        direction: NumericStepDirection,
        step: NumericStep,
        provenance: InteractionProvenance,
        error: Rc<StepError>,
        cancelled: bool,
    },
    FormatFailed {
        attempt: NumericStepAttempt,
        direction: NumericStepDirection,
        step: NumericStep,
        provenance: InteractionProvenance,
        error: Rc<FormatError>,
        cancelled: bool,
    },
}

pub enum KeyboardModifier {
    Shift,
    Command,
    Control,
    Alt,
}

pub struct NumericStepModifiers {
    fine: KeyboardModifier,
    coarse: KeyboardModifier,
}
```

`NumericInputInteractionBatch<T, StepError, FormatError>` is the qualified
public fixed-capacity envelope around these parts. Its private inline storage
has `MAX_INTERACTIONS == 2`; `from_interactions(...)` accepts exactly one
keyboard `Edit` containing `[Begin, Update]`, `[Update]`, `[Commit]`, or
`[Cancel]`, one initial `StepFailed` or `FormatFailed` with
`cancelled: false`, or `[Edit([Cancel]), failure]` for a repeat failure with
matching keyboard provenance and `cancelled: true`. `parts()`, `events()`,
`len()`, and `is_empty()` expose the validated ordered slice. `step_error()`
and `format_error()` borrow the typed error without copying it. Complete-mode
numeric input produces and consumes these shapes; the batch remains bounded
storage and shape validation, not a platform or product policy.

`KeyboardModifier` is a semantic normalized selector, not a native key name.
The public target attachment is
`NumericInputBuilder::step_modifiers(NumericStepModifiers::new(fine, coarse))`.
It stores the policy on the numeric widget; an unconfigured builder retains
`None`, and compatibility `on_edit` remains inert. Complete mode reads the
explicit policy and recomputes the selected step for every sample. Fine wins
when both configured selectors are held. The exact target storage for the
shipped `NumericInputEditBatch<T>` is fixed at three events so a complete-mode
wheel atomic gesture can carry `[Begin, Update, Commit]`; keyboard and TextEdit
shapes retain their existing bounded forms.

Only a focused, enabled, non-read-only input without an active text mutation
may step. `ArrowUp` selects `Increase` and `ArrowDown` selects `Decrease`;
`ArrowLeft`, `ArrowRight`, `Home`, and `End` remain text navigation. An active
text mutation blocks stepping and the numeric path neither parses nor
commits/cancels the draft. Before an uncaptured initial Up/Down press, host
shortcut routing runs first: a handled result prevents capture, while an
unhandled press may begin numeric adjustment. A captured sequence owns matching
repeats and its matching release, which bypass host routing. Orphan repeats or
releases and competing arrow keys are ignored.

The first effective step starts one physical-sequence transaction with
`Begin(start)` followed by `Update(candidate)`. Every accepted matching repeat
performs one step and emits at most one `Update`; the matching release emits
`Commit(current)`. Escape, capture loss, focus loss, disable, and read-only
transition emit cancellation and restore the start value. A successful
unchanged initial step creates no transaction, capture, or publication; an
unchanged repeat emits no update but does not end an existing capture, so a
later matching release can commit after an earlier effective update. The
adjustment policy owns clamp, wrap, and quantization.

While an active numeric text or keyboard transaction receives Escape, the
numeric consumer handles it before host Escape routing, including when any
modifiers are held; no host Escape action is invoked. With no active numeric
transaction, ordinary host Escape routing remains in force.

Initial step or formatting failures return the typed `StepFailed` or
`FormatFailed` context with `attempt: Initial` and `cancelled: false`; they
produce no transaction, capture, `Edit`, or `Cancel`. For an accepted matching
repeat after `Begin` and at least one effective `Update`, suppress the failed
candidate `Update`, restore the transaction start, and publish exactly one
terminal `Edit` containing `Cancel(start)` with the existing transaction
identity and `InteractionProvenance::Keyboard { timestamp: failing_timestamp }`.
Only after that terminal `Edit(Cancel(start))`, publish the corresponding typed
`StepFailed` or `FormatFailed` with `attempt: Repeat`, the attempted
`NumericStepDirection`, modifier-selected `NumericStep`, the same failing-repeat
keyboard provenance/timestamp, and `cancelled: true`. The timestamp may be
absent when the failing repeat has no timestamp. This cancel ends capture; a
later matching release is orphaned. The rollback `Edit` is mandatory; no
failed or partial candidate `Edit` or `Update` is published. Errors never panic
and never become successful no-ops.

The shipped normalized release boundary is carried through both
`Event::KeyRelease { key, modifiers, timestamp }` and
`WidgetInput::KeyRelease { key, modifiers, timestamp }`; complete-mode
semantic keyboard adjustment consumes it. It preserves shipped normalized
`Event::KeyPress { key, modifiers, repeat, timestamp }` and
`WidgetInput::KeyPress { key, modifiers, repeat, timestamp }`; a release is not
another press.
Every edit phase carries keyboard provenance. The initial press timestamp is
used for `Begin` and its first `Update`, each repeat uses its own timestamp, and
the matching release supplies the `Commit` timestamp. Missing timestamps stay
absent, keyboard events receive no fabricated sequence ranges, and synthetic
inputs use default modifiers, `repeat: false` for a press, and no timestamp.
Repeat cadence, delay, and rate are outside the contract.

Deterministic target fixtures:

| Fixture | Expected target behavior |
| --- | --- |
| `7`, `ArrowUp` press at `t1`, matching repeat at `t2`, matching release at `t3` | `Edit`: `Begin(7, t1)`, `Update(8, t1)`, `Update(9, t2)`, `Commit(9, t3)` for a base step of `1`. |
| Explicit policy selectors | An explicitly attached policy selects Base when neither configured selector matches, Fine for its configured selector, and Coarse for its configured selector; the provided macOS and Windows/Linux constants encode Shift/Command and Shift/Control respectively. An explicitly attached custom policy changes those selectors; if Fine and Coarse both match, Fine wins, including after a per-sample modifier change. |
| Text navigation and active text mutation | Left/Right/Home/End stay with text navigation. While text is actively mutating, Up/Down produce no numeric step and do not parse, commit, or cancel the draft. |
| Host-handled versus unhandled initial press | A handled initial Up/Down produces no numeric capture or edit. An unhandled eligible initial press can capture only after an effective step; matching captured repeats and release bypass host routing. |
| Orphan or competing key | A repeat/release without capture, or the opposite arrow during an active capture, is ignored; the current capture and transaction remain unchanged. |
| Escape, capture loss, focus loss, disable, or read-only | The active keyboard transaction is cancelled and restored to its start value; it does not commit. |
| Initial no-op and boundary after prior updates | An unchanged initial candidate produces no transaction, capture, or publication. After an earlier update, a boundary repeat produces no `Update`; release still commits the current value. |
| Initial/repeat errors and delayed release | Initial failure returns typed `StepFailed`/`FormatFailed` with `attempt: Initial` and `cancelled: false`, with no transaction, capture, `Edit`, or `Cancel`. For `Begin(7,t1)`, `Update(8,t1)`, a failing matching repeat at `t2` suppresses its candidate `Update`, then publishes exactly one terminal `Edit`: `Cancel(7,t2)` with the same transaction identity and `InteractionProvenance::Keyboard { timestamp: t2 }`; only after that publishes typed `StepFailed` or `FormatFailed` with `attempt: Repeat`, `direction: Increase`, `step: Base`, the same keyboard provenance/timestamp, and `cancelled: true`. No failed `Update` is published, capture ends, and a later matching release is orphaned. Separately, a successful sequence with a delayed matching release still commits at the release timestamp; no timeout is implied by the delay. |
| Metadata and synthetic defaults | `Begin`/first `Update` preserve the initial press timestamp, repeat updates preserve their own timestamps, and `Commit` preserves release metadata through keyboard provenance. No keyboard sequence range is fabricated; synthetic press/release defaults have no modifiers and no timestamp, with `repeat: false` on the press. |

### Numeric pointer scrubbing

Pointer scrub admission uses the shared incumbent-owner gate before focus,
capture, or any scrub operation. PointerScrub may start only when the stable
numeric identity has owner None; a different pending or active owner blocks the
scrub without changing the incumbent. The existing unmodified-primary
text-selection fallback remains unchanged.

The complete-mode `NumericInput` consumer now performs this primary-pointer
numeric scrub lifecycle through the merged managed-capture admission hook. It
composes the shipped `NumericAdjustment::scrub` policy boundary and pointer
metadata/capture with the existing numeric text lifecycle. IME/composition,
wheel continuity, and accessibility ownership remain outside this slice.

The `NumericInputBuilder::scrub_policy(...)` attachment and complete
backend-neutral default are:

```rust
// Public backend-neutral API shape.
enum NumericScrubActivation {
    PrimaryButtonHorizontalDrag {
        modifier: KeyboardModifier,
    },
}

struct NumericScrubPolicy {
    activation: NumericScrubActivation,
}

impl NumericScrubPolicy {
    fn default() -> Self {
        Self {
            activation: NumericScrubActivation::PrimaryButtonHorizontalDrag {
                modifier: KeyboardModifier::Alt, // Option on macOS
            },
        }
    }
}

numeric_input(value, codec, adjustment)
    .scrub_policy(NumericScrubPolicy::default());
```

The default is Alt/Option plus a primary-button horizontal drag. An unmodified
primary input remains ordinary text caret/selection behavior. `Alt` is the
semantic normalized activation modifier, not a platform-specific key name.
The activation chord is latched at admission and remains latched through the
matching release or cancellation even if the live Alt/Option modifier changes.

#### Ownership and admission

The application owns the durable `T` value and supplies `NumericCodec<T>` and
`NumericAdjustment<T>`. The numeric input owns its draft, caret, selection,
focus, and edit lifecycle. Its retained scrub state owns the start snapshot,
anchor, selected step, and pointer-capture association; the runtime owns the
actual capture and stable identity. The adjustment policy remains the owner of
domain mapping, quantization, and scrub errors.

Admission requires an enabled, non-read-only numeric input and a shared
incumbent owner of None. A different pending or active owner blocks the scrub.
A blocked scrub is not admitted and does not parse, commit, or cancel an active
interaction. Unmodified primary input remains on ordinary
text caret/selection routing.

On an admitted primary press, the target first focuses the input and latches a
stable identity, then captures:

```rust
// Private retained state is represented here conceptually.
struct NumericScrubStart<T> {
    identity: StableWidgetIdentity,
    start_typed_value: T,
    start_canonical_draft: Text,
    start_caret: ScalarIndex,
    start_selection: ScalarRange,
    press_position: LogicalPoint,
    scrub_bounds: LogicalRect,
    press_modifiers: PointerModifiers,
    press_timestamp: Option<InputTimestamp>,
    pointer_capture: PointerCaptureToken,
}

struct NumericScrubAnchor<T> {
    position: LogicalPoint,
    value: T,
}
```

The start snapshot is retained exactly for rollback. A press does not emit
`Begin`; the first effective candidate does. The latched activation chord is
not rechecked at release, and changing Alt/Option cannot turn capture into
ordinary text input or change the transaction boundary.

Before any move, admission initializes `NumericScrubAnchor` explicitly:

```rust
let anchor = NumericScrubAnchor {
    position: press_position,
    value: start_typed_value,
};
```

The first move therefore normalizes from the captured press position and
starting typed value; later moves use the current anchor.

#### Geometry, anchors, and step selection

The latched logical bounds must have finite coordinates and a finite strictly
positive width. Pointer positions used for normalization must be finite and
within those declared bounds. The normalized horizontal delta is the signed
displacement from the current anchor divided by that width: positive increases
and negative decreases. A valid sample with zero horizontal displacement from
the current anchor is a handler-level no-op before invoking
`NumericAdjustment::scrub`: it creates no candidate, edit transaction, update,
or value change and retains the current anchor. Vertical-only motion is such a
sample. Invalid, nonfinite, or out-of-bounds geometry or position is unknown
evidence; the target does not clamp it, invent a coordinate, or publish a
guessed candidate. The invalid sample leaves the anchor and any existing
transaction unchanged.

Every valid move with nonzero horizontal displacement invokes exactly the target
adjustment boundary with its current anchor:

```rust
let candidate = adjustment.scrub(anchor_value, normalized_delta, selected_step)?;
```

The target selects a step per sample after removing the latched activation
chord. The default selectors are Base when no selector is present, Fine for
Shift, and Coarse for the platform command (Command on macOS, Control on
Windows/Linux); Fine wins when both are present. A change in the selected step
reanchors at the current pointer position and current value before applying
new displacement, preventing a modifier-change jump. A successful changed
candidate advances the anchor to the current move position and candidate.
An unchanged candidate publishes nothing and retains the previous anchor, so
sub-quantum motion accumulates.

#### Ordering, formatting, and boundaries

The first candidate that is adjusted successfully, formatted successfully by
`NumericCodec::format_editable`, and changed emits `Begin(start)` with the
exact press `InteractionProvenance::Pointer`, followed by `Update(candidate)`
with the exact effective move provenance. There is no transaction or edit
event for a pending or unchanged candidate. Each later accepted move emits at
most one `Update`; `Begin`, `Commit`, and `Cancel` remain ordered terminal
boundaries in bounded incremental delivery rather than an unbounded batch.

The active draft is the codec-formatted canonical text for the current
candidate, with a collapsed caret at its end. Commit retains that canonical
draft. A matching captured primary release emits `Commit(current)` only when
an effective update opened a transaction. A pending or no-op capture clears
without `Begin`, `Commit`, or value/draft change.

Escape, capture loss, focus loss, identity loss, incompatible reprojection, a
newer external authority, disablement, read-only transition, and explicit
cancellation restore the exact start typed value, canonical draft, caret, and
selection. An active transaction emits exactly one `Cancel(start)` with its
existing transaction identity before cleanup. A pending or no-op capture emits
no edit event. Escape is consumed by the numeric consumer during an active
scrub, including with held modifiers, before host Escape routing.

Reprojection is authority- and identity-fenced. A compatible same-ID
reprojection with an unchanged external value preserves pending or active
scrubbing, including its start snapshot, capture, and anchor. Changed external
value or incompatible identity/capability cancels the old scrub before the new
authority is applied; it never rebases an active scrub onto that authority.

#### Failure and metadata rules

The shipped complete interaction context keeps pointer adjustment and pointer
formatting failures distinct:

```rust
// Public pointer variants on NumericInputInteraction.
enum NumericScrubAttempt {
    Initial,
    Update,
}

enum NumericInputInteraction<T, ScrubError, FormatError> {
    Edit(NumericInputEditBatch<T>),
    ScrubFailed {
        attempt: NumericScrubAttempt,
        normalized_delta: f32,
        step: NumericStep,
        provenance: InteractionProvenance,
        error: ScrubError,
        cancelled: bool,
    },
    PointerFormatFailed {
        attempt: NumericScrubAttempt,
        normalized_delta: f32,
        step: NumericStep,
        provenance: InteractionProvenance,
        error: FormatError,
        cancelled: bool,
    },
}
```

An initial adjustment or format failure returns `ScrubFailed` or
`PointerFormatFailed` with `attempt: Initial` and `cancelled: false`, emits no
transaction, `Begin`, `Update`, or `Cancel`, restores the pre-scrub UI, and
ends the failed capture. After an effective update, a failed adjustment or
format operation suppresses its candidate, restores the transaction start,
and emits exactly one existing-identity `Cancel(start)`. The terminal edit
precedes the typed failure with `attempt: Update` and `cancelled: true`; no
failed `Update` is emitted. Capture ends and a later matching release is
orphaned.

Press, move, release, and cancellation provenance is copied from the relevant
sample without fabrication. `Begin` preserves the exact press modifiers and
timestamp. Every `Update` preserves the effective move's exact modifiers,
timestamp, and supplied `InputSequenceRange`. `Commit` preserves the exact
release modifiers and timestamp. Synthetic pointer inputs use the known
`Pointer` source with absent native timestamp and sequence metadata; no
sequence range is fabricated. When a cancellation boundary supplies pointer
metadata, `Cancel` uses that exact metadata. Otherwise it uses the pointer
source with timestamp, modifiers, and sequence metadata absent. Sequence
ranges are observational only and do not affect ordering, step selection,
accumulation, capture, or value calculation.

Wheel remains existing fallback routing. This pointer-scrub slice
consumes no wheel input and defines no contiguous-wheel burst timeout.

Deterministic pointer-scrub fixtures:

| Fixture | Expected target behavior |
| --- | --- |
| 1. Alt/Option primary versus unmodified primary | An enabled, editable numeric input admits Alt/Option plus primary-button horizontal drag and latches capture. The same press without Alt/Option remains ordinary text caret/selection behavior and does not begin scrub. |
| 2. First effective move and release | With a base-step input, admission initializes the anchor to `{ position: p0, value: start_typed_value }`; the first effective move to `p1` normalizes from that captured press/value and emits `Begin(start)` with press provenance, then `Update(candidate)` with move provenance. Matching captured release emits one `Commit(candidate)` with release provenance. |
| 3. Sub-quantum accumulation | A valid zero-horizontal move, including vertical-only motion, is a handler-level no-op before `scrub`: it creates no candidate, edit transaction, update, or value change and retains the current anchor. A nonzero move whose scrub candidate is unchanged also emits nothing and leaves the anchor at its prior position/value; a later nonzero move accumulates pending displacement, and the first changed candidate opens the transaction. |
| 4. Fine/Coarse selection and reanchor | Removing the latched Alt/Option chord leaves Base unmodified, Fine with Shift, and Coarse with Command on macOS or Control on Windows/Linux; Fine wins when both match. Changing mode reanchors at the current position/value, so the next displacement starts without a jump. |
| 5. Blocked overlap | While text mutation, keyboard adjustment, IME composition, accessibility edit, or another transaction is active, the Alt/Option primary press is blocked: it does not scrub, parse, commit, or cancel the active interaction. |
| 6. Exact cancellation rollback | Starting from a typed value/draft/caret/selection, an effective scrub emits `Begin`, `Update`, then Escape, capture loss, focus loss, explicit cancel, or a disable/read-only boundary emits exactly one `Cancel(start)` before cleanup and restores every starting field. Escape is consumed; a pending capture emits no edit event. |
| 7. Reprojection authority | Same-ID compatible reprojection with unchanged external value preserves pending/active scrub and its anchor. Changed external value or incompatible identity/capability emits cancellation before the new authority is applied; no rebase occurs. |
| 8. Initial and active failures | Initial `scrub` or formatting failure returns typed `ScrubFailed`/`PointerFormatFailed` with `attempt: Initial`, `cancelled: false`, no edit event, and pre-scrub UI restored. After `Begin`/`Update`, a failing adjustment or formatter suppresses its candidate, emits `Cancel(start)` first with the existing identity, then typed `ScrubFailed`/`PointerFormatFailed` with `attempt: Update`, `cancelled: true`; capture ends and release is orphaned. |
| 9. Malformed geometry | Nonfinite coordinates, zero/negative/nonfinite width, or out-of-bounds geometry/position produces unknown evidence with no guessed clamp, candidate, update, or anchor advance. |
| 10. Provenance and synthetic defaults | Press modifiers/timestamp are copied to `Begin`; each move copies its own modifiers/timestamp/sequence range to `Update`; release modifiers/timestamp are copied to `Commit`. No sequence range is fabricated. Synthetic pointer inputs retain `Pointer` source with absent native metadata, and cancellation uses exact boundary metadata when available or pointer source with absent metadata otherwise. |
| 11. Wheel fallthrough | Wheel input over the numeric input is consumed only by an explicitly configured eligible wheel consumer; unconfigured, ineligible, conflicting-owner, or pre-policy unusable samples retain existing widget/scroll-container fallback. Pointer scrub does not consume wheel input. |

### Numeric wheel adjustment and continuity (complete-mode consumer shipped)

Wheel admission uses the shared incumbent-owner gate before unit conversion,
wheel adjustment, or pending-sequence ownership. WheelSequence may start only
when the stable numeric identity has owner None; a different pending or active
owner leaves the sample to the wheel contract's existing unhandled fallback
and never changes the incumbent.

The backend-neutral `WheelDelta`, `WheelPhase`, `WheelSample`, and managed
routing foundation are shipped, and complete-mode NumericInput consumes them
when an explicit `NumericWheelPolicy` is attached. The public policy is a
zero-state opt-in; unit conversion, ownership, and lifecycle state remain
inside the generic widget/runtime seam. Exact samples preserve line/pixel unit,
phase, modifiers, timestamp, and sequence-range evidence through policy output.
Legacy phase-less dispatch remains compatible: hit testing is metadata-neutral,
but selected-widget dispatch preserves supplied metadata. Native adapters that
still collapse line/pixel or phase evidence before this exact seam remain a
separate platform alignment gap; that fallback is not used as evidence for
exact-sample behavior.

Exact unit and phase preservation is limited to qualified widget/policy
routing. The native coalesced scroll-container fallback uses the single-axis
logical-pixel `ScrollUpdate` contract defined in the `Event And Focus` section.

The target-equivalent shapes are:

```rust
// Contract vocabulary; the public implementation uses WheelDelta, WheelPhase,
// and WheelSample while retaining these semantics behind the widget seam.
enum NumericWheelDelta {
    Lines(Vector2),
    Pixels(Vector2),
}

enum NumericWheelPhase {
    Started,
    Changed,
    Ended,
    Cancelled,
    Discrete,
}

struct NumericWheelPolicy {
    // Fixed target equivalence: 40.0 logical pixels per line.
}

struct NumericWheelSample {
    delta: NumericWheelDelta,
    phase: Option<NumericWheelPhase>, // None is the phase-less case.
    modifiers: PointerModifiers,
    timestamp: Option<InputTimestamp>,
    sequence_range: Option<InputSequenceRange>,
}

enum NumericWheelResult<T, AdjustmentError, FormatError> {
    Edit(BoundedEditEvents<T>),
    InitialAdjustmentFailed {
        error: AdjustmentError,
        cancelled: bool,
    },
    UpdateAdjustmentFailed {
        error: AdjustmentError,
        cancelled: bool,
    },
    InitialFormatFailed {
        error: FormatError,
        cancelled: bool,
    },
    UpdateFormatFailed {
        error: FormatError,
        cancelled: bool,
    },
}
```

`NumericWheelPolicy` is responsible for admission, unit conversion, and
continuity only. The explicit builder attachment is shipped:

```rust
numeric_input(value, codec, adjustment)
    .wheel_policy(NumericWheelPolicy::default());
```

#### Numeric wheel unit conversion and direction

The target retains the `Lines(Vector2)` or `Pixels(Vector2)` unit until the
wheel policy invokes `NumericAdjustment::wheel`. The existing Radiant target
line-equivalence is exactly `40.0` logical pixels per line. That conversion
constant must be finite and strictly positive. A line delta uses its finite
vertical component as the signed line-equivalent value; a pixel delta uses its
finite vertical component divided by `40.0`. Precision is preserved until that
policy conversion: the target does not collapse units early, round samples, or
replace a malformed sample with a guessed value. Positive vertical movement
increases the numeric value and negative vertical movement decreases it.

Zero movement, horizontal-only movement, any nonfinite component, malformed
unit/phase/metadata, or an otherwise unusable sample creates no numeric
candidate. Such a sample remains unhandled when it is phase-less or `Discrete`,
so the existing widget/scroll-container fallback remains available. A native
line/pixel conversion that has already collapsed the unit or sanitized a
nonfinite component cannot be treated as proof of target eligibility.

#### Numeric wheel admission and continuity

Admission requires a focused numeric input that is the wheel target under the
pointer, has compatible stable identity and current external authority, is
enabled and non-read-only, and has a shared incumbent owner of None. A
different pending or active owner blocks this admission. An ineligible
sample stays unhandled for the existing widget and scroll-container
fallback. A phase-less or `Discrete` sample rejected before policy invocation
also stays unhandled. An eligible `Started` sample may retain pending ownership
without emitting `Begin`; if its sequence produces no accepted changed
candidate and no policy or formatting failure is reported, `Ended` clears the
pending ownership without an edit event.

#### Numeric wheel ownership matrix

The following routing matrix is normative. A usable sample has passed admission
and validation and is eligible to invoke `NumericAdjustment::wheel`; a changed
candidate differs from the current value.

| Condition | Numeric outcome | Routing and ownership |
| --- | --- | --- |
| Ineligible target or conflicting edit owner | No numeric attempt and no typed failure | Unhandled; existing widget/scroll-container fallback remains available. |
| Sample rejected as unusable before `NumericAdjustment::wheel` invocation (zero, horizontal-only, nonfinite, malformed, or unsupported) | No numeric candidate | Unhandled; existing widget/scroll-container fallback remains available. |
| Adjustment succeeds with a candidate equal to the current value | Formatter is not invoked; no candidate or edit | A phase-less/`Discrete` sample remains unhandled for scroll fallback. An admitted explicit sequence emits no `Update` and retains ownership. |
| Eligible, usable sample whose adjustment returns an error | Numeric-owned handled `InitialAdjustmentFailed { cancelled: false }` | Exact UI is unchanged, no transaction is emitted, and scroll fallback is never available. |
| Eligible, usable sample whose changed candidate formatting returns an error | Numeric-owned handled `InitialFormatFailed { cancelled: false }` | Exact UI is unchanged, no transaction is emitted, and scroll fallback is never available. |
| Successful changed, formatted candidate | Bounded edit lifecycle | A phase-less/`Discrete` sample emits `Begin`, `Update`, `Commit`; the first effective candidate in an admitted explicit sequence emits `Begin`, `Update`, and `Ended` emits `Commit`. |

Policy and formatting failures are handled numeric outcomes under the rows
above. They are not classified as ineffective, unchanged, no-candidate,
unhandled, or fallback.

An explicit `Started` -> `Changed`* -> `Ended` sequence is one transaction.
`Started` captures the exact starting typed value, canonical draft, caret,
selection, stable identity, and routing ownership. The first effective changed
candidate that adjustment and codec formatting accept and that changes the
value emits `Begin(start)` followed by `Update(candidate)`. Each later
effective sample emits at most one `Update`; an unchanged sample emits no
`Update` but retains ownership. `Ended` emits `Commit(current)` when the edit
began. A pending sequence with no accepted changed candidate and no initial
failure emits no `Begin`, `Commit`, or `Cancel` at its end.

The first policy attempt in an admitted explicit sequence follows the initial
failure rows above. An adjustment or changed-candidate formatting failure is a
numeric-owned handled `InitialAdjustmentFailed` or `InitialFormatFailed` result
with `cancelled: false`; it leaves the exact UI unchanged, emits no transaction,
clears pending ownership, and makes later `Changed`/`Ended` samples orphaned.
Those later phases never join guessed history or become scroll fallback.

A phase-less sample and a `Discrete` sample are conservatively one atomic
gesture. One effective sample emits `Begin(start)`, `Update(candidate)`, and
`Commit(candidate)` in that bounded order. A phase-less/`Discrete` sample
rejected before policy invocation, or whose successful adjustment candidate
equals the current value, is unhandled. A phaseful `Changed`, `Ended`, or
`Cancelled` without a matching admitted `Started` sequence is an orphan: it
does not perform an atomic adjustment, never joins guessed history, and remains
available to existing fallback routing.

Step selection is recomputed for each effective sample: Base is unmodified,
Fine is Shift, and Coarse is Command on macOS or Control on Windows/Linux.
Fine wins when both selectors are present. Per-sample modifier changes affect
the next effective sample without starting a second transaction or authorizing
guessed continuity. Coalescing may combine a delta only when unit, phase
ownership, target identity, and routing ownership are compatible. It preserves
phase boundaries and per-sample step decisions; it never uses sequence metadata
as an execution decision.

#### Numeric wheel adjustment, formatting, and failure ordering

The numeric consumer invokes the supplied policy only as a signed wheel
operation, conceptually:

```rust
let candidate = adjustment.wheel(&current_value, signed_vertical_delta, step)?;
```

`NumericAdjustment<T>` exclusively owns domain mapping, clamp, wrap,
quantization, and wheel sensitivity. The consumer adds no second domain policy.
The supplied `NumericCodec<T>::format_editable` accepts each changed candidate
into the canonical draft before the candidate is published. An unchanged
initial `Discrete` candidate publishes nothing and remains available to scroll
fallback. An unchanged sample inside an admitted explicit sequence emits no
`Update` and retains ownership through `Ended`.

The typed result variants distinguish initial and update adjustment/format
failures. An initial adjustment or format failure returns the corresponding
`InitialAdjustmentFailed` or `InitialFormatFailed` result with
`cancelled: false`, without a transaction or edit event. After an effective
update, a failed adjustment or format operation suppresses its candidate,
restores the exact starting typed value, canonical draft, caret, and selection,
emits exactly one same-transaction `Cancel(start)`, and only then returns
`UpdateAdjustmentFailed` or `UpdateFormatFailed` with `cancelled: true`.
Rollback-before-diagnostic is mandatory; no failed `Update` is published.

`Cancelled`, Escape, focus loss, identity loss, incompatible reprojection,
changed external authority, disablement, read-only transition, and explicit
cancel restore the exact starting typed value, canonical draft, caret, and
selection. If the edit began, exactly one same-transaction `Cancel(start)` is
emitted before cleanup. A pending admitted sequence clears without an edit
event. Compatible same-identity reprojection with an unchanged external value
preserves pending or active ownership and its start/continuity state. Changed
authority or identity cancels the old interaction before applying the new
authority; it never rebases an active sequence.

#### Numeric wheel provenance and execution boundaries

Every edit phase uses pointer provenance. Effective samples preserve their exact
modifiers, optional timestamp, and complete supplied sequence range. A terminal
phase with native pointer metadata preserves that exact metadata. A cleanup
boundary that is not an input sample uses pointer provenance with absent
timestamp, modifiers, and sequence range; it never fabricates sample metadata.

There is no idle timeout or timer-based grouping. Timestamps are opaque
provenance, not continuity clocks or deadlines. Sequence ranges are
observational only and never define continuity, sample count, ordering,
density, accumulation, scheduling, cache admission, reuse, renderer resources,
render selection, or execution authority. Observational metadata can never
authorize a schedule, timer, cache, reuse, renderer, resource, or execution
decision.

Deterministic target fixtures:

| Fixture | Expected target behavior |
| --- | --- |
| 1. Focused/hovered admission versus scroll fallback | A focused, enabled, editable numeric input under the pointer and selected as the wheel target may admit an eligible sample. A focused input outside the pointer, an unfocused input, an ineligible target, or a sample over only a scroll container remains unhandled so existing widget/scroll-container fallback can consume it. |
| 2. Unit-preserving line/pixel normalization | `Lines(Vector2::new(0.0, 1.0))` reaches policy conversion as `+1.0`; `Pixels(Vector2::new(0.0, 40.0))` reaches it as `+1.0`; a smaller pixel delta remains precise until conversion. The validated line-equivalence is exactly `40.0` logical pixels per line. |
| 3. Direction and unusable samples | Positive finite vertical movement increases, negative finite vertical movement decreases. Zero, horizontal-only, nonfinite, malformed, and unusable samples create no candidate; phase-less/`Discrete` cases remain unhandled for fallback. |
| 4. Explicit continuity | `Started` -> `Changed`* -> `Ended` is one transaction: the first effective changed candidate emits `Begin(start)` then `Update(candidate)`, later effective samples emit at most one `Update`, and `Ended` commits current. A pending sequence with no changed candidate ends without an edit event. |
| 5. Explicit and non-phase cancellation | `Cancelled`, Escape, focus/identity loss, incompatible reprojection, changed authority, disable/read-only, and explicit cancel restore the exact start snapshot. An active edit emits exactly one same-transaction `Cancel(start)` before cleanup; a pending edit emits none. |
| 6. Atomic discrete and malformed/orphan phases | One effective phase-less/`Discrete` sample emits `Begin`, `Update`, `Commit` in order. An ineligible/conflicting-owner or pre-policy unusable sample (zero, horizontal-only, nonfinite, malformed, or unsupported) remains unhandled with scroll fallback; an eligible usable policy/format failure is numeric-owned handled as a typed initial failure with `cancelled: false` and never falls back. A phaseful `Changed`, `Ended`, or `Cancelled` without a matching admitted start is an orphan and remains unhandled; it never performs an atomic adjustment or joins guessed history. A first policy failure in an admitted explicit sequence clears pending ownership and orphans later phases. |
| 7. Base/Fine/Coarse and modifier changes | Unmodified selects Base, Shift selects Fine, Command on macOS or Control on Windows/Linux selects Coarse, and Fine wins when both match. A per-sample modifier change selects the new step for the next effective sample without a jump, second transaction, or guessed continuity break. |
| 8. Unchanged candidates | When adjustment succeeds with a candidate equal to the current value, the formatter is not invoked and no candidate or edit exists. An atomic phase-less/`Discrete` sample remains unhandled for scroll fallback; an unchanged sample in an admitted explicit sequence emits no `Update` and retains ownership through its terminal phase. This is distinct from a policy/formatting failure, which is handled numerically and never falls back. |
| 9. Typed failures and rollback ordering | For an eligible usable sample, an adjustment/format failure is numeric-owned handled as the typed initial failure with `cancelled: false`; exact UI is unchanged, no transaction is emitted, and scroll fallback is unavailable. On the first policy attempt of an admitted explicit sequence, that failure clears pending ownership and orphans later phases. After an effective update, a failed adjustment/format candidate is suppressed, the exact start is restored, one `Cancel(start)` is emitted first, and the typed update failure with `cancelled: true` follows; no failed `Update` is published. |
| 10. Reprojection and authority | Same-identity compatible reprojection with unchanged external value preserves pending/active state and continuity. Changed authority or identity cancels before the new authority applies and never rebases the old sequence. |
| 11. Exact metadata and cleanup | Every edit phase uses pointer provenance. Effective samples copy exact modifiers, timestamp, and complete sequence range; a native-metadata terminal copies its metadata. Escape/focus/identity/authority/disable/read-only cleanup with no input sample uses absent metadata and fabricates nothing. |
| 12. Observational metadata proof | Delayed samples still follow explicit ownership without an idle timeout; timestamps and sequence ranges do not define continuity, deadlines, scheduling, accumulation, cache admission, reuse, renderer resources, render selection, or execution. Different observational metadata cannot authorize any of those decisions. |

### Target numeric accessibility action lifecycle (runtime dispatch shipped; native adapters remain separate)

The target runtime lifecycle requires the shared incumbent-owner gate at both
its pre-focus and post-focus checks. The shipped runtime consumer performs
those checks around ordinary focus admission: AccessibilityEdit may start only
when the stable numeric identity and the current focused interaction scope have
no incumbent owner; a different pending or active owner is returned as
`Blocked { owner }` without cancelling or mutating the incumbent. The runtime
also revalidates current identity, path, role, materialization, capability, and
focus authority before invoking the widget-local policy.

The widget-local, backend-neutral action vocabulary and typed policy consumer
are now shipped as a crate-private `NumericInputWidget` seam. The public
`NumericAccessibilityAction`, `NumericAccessibilityRejectedReason`,
`NumericAccessibilityBlockOwner`, and
`NumericAccessibilityOutcome<T, AdjustmentError, FormatError>` types describe
the local accepted, unchanged, rejected, blocked, adjustment-failure, and
format-failure results. The widget and its local consumer entry point remain
crate-private; `SurfaceRuntime::dispatch_numeric_accessibility_action` is the
generic public runtime boundary. The current automation snapshot and action
advertisement remain read-only evidence, while the runtime dispatch method
executes only an explicitly supplied neutral request. Native adapter
translation remains separate. Action names are neutral target vocabulary only,
not native action names, handles, APIs, or payload formats.

The illustrative request and result vocabulary is:

```rust
// Illustrative full-dispatch shapes; the local action/outcome types are shipped.
enum NumericAccessibilityAction {
    Increment,
    Decrement,
    SetValueText(String),
}

struct NumericAccessibilityRequest {
    target: AutomationTarget,
    action: NumericAccessibilityAction,
}

enum NumericAccessibilityUnavailableReason {
    UnknownTarget,
    StaleTarget,
    RemovedTarget,
    UnmaterializedTarget,
}

enum NumericAccessibilityRejectedReason {
    UnsupportedAction,
    Disabled,
    ReadOnly,
    FocusDenied,
    NotFocusable,
    Incomplete,
    Invalid,
    OutOfRange,
}

enum NumericAccessibilityDispatchResult {
    Unavailable {
        reason: NumericAccessibilityUnavailableReason,
    },
    Rejected {
        reason: NumericAccessibilityRejectedReason,
    },
    Blocked {
        owner: NumericAccessibilityBlockOwner,
    },
    Accepted {
        widget_id: WidgetId,
        output: WidgetOutput,
    },
}
```

`NumericAccessibilityAction` is the shipped complete neutral vocabulary:
`Increment`, `Decrement`, and `SetValueText(String)`. A future native or
platform adapter may translate a platform request into one of these values,
but this contract selects no native action name, handle, API, or payload
format. Each request is discrete; platform timing does not imply repetition
or continuity. `Accepted` is runtime admission and handler invocation; its
`WidgetOutput` can be downcast by the numeric host to the typed
`NumericAccessibilityOutcome<T, AdjustmentError, FormatError>` local result.

#### Target ownership and conservative authority rules

The application owns the durable `T` value and supplies `NumericCodec<T>` and
`NumericAdjustment<T>`. The numeric input owns draft text, caret, selection,
focus-local edit state, and the edit lifecycle. Runtime ownership includes
the current stable numeric widget identity, authority/revision, focus
transition, materialization status, and active edit-owner admission. A future
adapter owns only translation into the neutral action vocabulary. The
automation snapshot and flattened target projection are read-only evidence;
they do not own dispatch or mutation.

Before each request, current authority is revalidated. The target must still
map to the same stable numeric widget identity, the action must still be
supported, and the widget must be enabled and non-read-only. Admission is
exhaustive and deterministic:

| Current authority/admission state | Exactly one outcome and reason |
| --- | --- |
| Target cannot be resolved, including a missing or unknown target | `Unavailable { reason: UnknownTarget }` |
| Target identity or captured authority is no longer current | `Unavailable { reason: StaleTarget }` |
| Target was removed from the current projection | `Unavailable { reason: RemovedTarget }` |
| Target is virtual and not currently materialized | `Unavailable { reason: UnmaterializedTarget }` |
| Requested action is not currently supported | `Rejected { reason: UnsupportedAction }` |
| Widget is disabled | `Rejected { reason: Disabled }` |
| Widget is read-only | `Rejected { reason: ReadOnly }` |
| Ordinary focus transfer is vetoed | `Rejected { reason: FocusDenied }` |
| Target cannot receive focus | `Rejected { reason: NotFocusable }` |
| Complete text parses as `Incomplete`, `Invalid`, or `OutOfRange` | `Rejected { reason: Incomplete }`, `Rejected { reason: Invalid }`, or `Rejected { reason: OutOfRange }` respectively |
| An active edit owner is present before or after focus transfer | `Blocked { owner }` |
| Current identity/capabilities are valid, the widget is enabled and editable, focus transfer succeeds, the action is supported, and no owner is active | `Accepted` |

Every authority/admission state maps to exactly one listed outcome and reason;
the dispatch contract never returns an `Unavailable`-or-`Rejected` choice.
`Accepted` is the admission classification for the policy path, not an
additional final result variant; that path returns `Edit`, `NoChange`,
`AdjustmentFailed`, or `FormatFailed`. These are post-`Accepted` results, not
admission alternatives. Snapshot revision, advertised actions,
bounds, labels, and values are veto/evidence only; they never independently
authorize execution. An identity/reprojection replacement is stale even if an
older snapshot still contains the target.

The first applicable boundary owns the classification: resolve current
identity and materialization (`Unavailable`); perform the non-mutating
pre-focus owner check (`Blocked`); check action support and enabled/editable
capability (`Rejected`); perform ordinary focus transfer (`Rejected` on veto
or inability); revalidate identity, capability, and owner after transfer
(`Unavailable`, `Rejected`, or `Blocked`); then parse complete set text
(`Rejected`) and invoke the accepted policy/formatting path. This ordered
classification makes combined states deterministic; an adapter cannot choose
another category.

Offscreen or unmaterialized virtual targets are unavailable. This action
contract cannot authorize materialization, scrolling, scheduling, cache
admission, reuse, renderer work, renderer resources, or other work required to
make an unavailable target executable. Those decisions belong to separate
runtime, virtualization, scheduler, cache, and renderer contracts.

#### Target focus and edit-owner rules

Before any focus transfer, the runtime performs a non-mutating check for any
active text edit, keyboard adjustment, pointer scrub, wheel sequence, IME
composition, accessibility edit, or a different shared owner in the current
interaction scope. If one is present, the request returns `Blocked { owner }`
without changing focus. This check does not call focus-loss handling, commit,
cancel, parse, or mutate the existing owner. In particular, a request for
target B cannot cause focused target A's valid active draft to commit during
this check.

If no owner is active, the request performs the ordinary focus transition to
the target. A focus-transfer veto returns `Rejected { reason: FocusDenied }`,
and an inability to focus a non-focusable target returns `Rejected { reason:
NotFocusable }`; neither changes either control. After transfer, the runtime
revalidates the target identity, authority, capabilities, materialization,
supported action, enabled/read-only state, and active-owner state before any
numeric mutation. If authority changed, the deterministic unavailable or
rejected reason above is returned; if an owner appeared, the result is
`Blocked { owner }`. Neither outcome mutates the target.

An action is `Blocked` at either owner check when it would interrupt an active
interaction. At the pre-focus check, `Blocked` leaves focus and the existing
owner's UI and authority untouched. If an owner appears after an otherwise
allowed transfer, the post-transfer check returns `Blocked` before target
mutation and performs no further focus or interaction mutation. A blocked
request does not commit, cancel, parse, or format the existing interaction.

#### Target action semantics and atomic edit result

`Increment` and `Decrement` each invoke exactly one
`NumericAdjustment::step` with the corresponding direction and
`NumericStep::Base`. Accessibility supplies no Fine or Coarse modifiers, and
platform repeat timing cannot turn one request into multiple steps. An
adjustment error returns typed `AdjustmentFailed` with no edit lifecycle and
the exact UI unchanged.

`SetValueText(String)` sends the complete text once through
`NumericCodec::parse`. Only `Valid(T)` proceeds. `Incomplete`, `Invalid`, and
`OutOfRange` each return the corresponding typed `Rejected` reason with no
formatting, mutation, commit, cancel, or edit. The payload is not appended to
a draft and is never silently clamped.

When the policy result is known equal to the current value, the result is
`NoChange`: there is no edit event and `NumericCodec::format_editable` is not
invoked. A changed candidate must first pass
`NumericCodec::format_editable`; a formatting failure returns typed
`FormatFailed`, preserves the exact value/draft/caret/selection/focus, and
emits no `Begin`, `Update`, `Commit`, or `Cancel`.

Each accepted changed request publishes one bounded atomic `EditTransaction` with
one transaction identity and exactly `Begin(start)`, `Update(candidate)`, and
`Commit(candidate)`. All phases use
`InteractionProvenance::Accessibility`. The request does not join an older
keyboard, pointer, wheel, IME, or accessibility sequence, and another request
must independently revalidate current authority.

#### Target provenance and execution boundaries

Accessibility phases carry no fabricated timestamp, modifiers, or sequence
range. Missing native metadata remains absent; platform timing cannot infer
repetition or continuity. Snapshot/action metadata is observational only and
cannot authorize execution, scheduling, cache admission, reuse, renderer
resources, materialization, scrolling, or render work.

Deterministic target fixtures:

| Fixture | Expected target behavior |
| --- | --- |
| 1. Increment and Decrement | For current value `7`, each action invokes exactly one Base `NumericAdjustment::step` in its direction; no Fine/Coarse modifier or repeat is inferred. |
| 2. Valid SetValueText | A complete text whose parse result is `Valid(T)` is formatted once through `NumericCodec::format_editable`, and canonical editable text is published only with the accepted changed transaction. |
| 3. Atomic lifecycle and provenance | A changed action emits one transaction identity with `Begin(start)`, `Update(candidate)`, `Commit(candidate)` in order; every phase is `InteractionProvenance::Accessibility`. |
| 4. Unchanged boundary | An adjustment boundary no-op, or valid text equal to the current value, returns `NoChange`, emits no edit event, and does not invoke the formatter when equality is known. |
| 5. Typed failures without partial lifecycle | Adjustment failure, `Incomplete`, `Invalid`, `OutOfRange`, and formatting failure each produce their typed failure/rejected reason with exact UI unchanged and no `Begin`, `Update`, `Commit`, or `Cancel`. |
| 6. Exhaustive admission mapping | Disabled -> `Rejected { reason: Disabled }`; read-only -> `Rejected { reason: ReadOnly }`; unsupported -> `Rejected { reason: UnsupportedAction }`; missing/unknown -> `Unavailable { reason: UnknownTarget }`; stale -> `Unavailable { reason: StaleTarget }`; removed -> `Unavailable { reason: RemovedTarget }`; unmaterialized -> `Unavailable { reason: UnmaterializedTarget }`; focus denial -> `Rejected { reason: FocusDenied }`; non-focusable -> `Rejected { reason: NotFocusable }`; an active owner -> `Blocked { owner }`. Every state has exactly one outcome/reason, no Unavailable-or-Rejected choice, and every unavailable/rejected case performs no edit. |
| 7. Pre-focus owner check and focus-loss veto | With focused target A holding a valid active draft and a request for target B, the non-mutating pre-focus owner check returns `Blocked { TextEdit }` before ordinary focus transfer; A stays focused with its draft/session unchanged, A's focus-loss path never runs, and no commit/cancel/parse occurs. With no owner, ordinary focus transfer runs; a veto maps to `Rejected { FocusDenied }`, a non-focusable target to `Rejected { NotFocusable }`, and post-transfer authority/owner changes reject or block without target mutation. |
| 8. Every active edit owner blocks | Active text edit, keyboard adjustment, pointer scrub, wheel sequence, IME composition, or accessibility edit returns `Blocked` before focus transfer with focus and interaction state unchanged; if an owner appears after an otherwise allowed transfer, the post-transfer check returns `Blocked` before target mutation and performs no further focus or interaction mutation. |
| 9. Identity/reprojection replacement | A request captured before replacement or incompatible reprojection is stale/unavailable at dispatch and is never rebased; no policy, codec, formatter, or edit lifecycle runs. |
| 10. Missing native metadata | Accepted phases use Accessibility provenance with timestamp, modifiers, and sequence range absent; no native metadata or timing is fabricated. |
| 11. Unmaterialized virtual target | An offscreen/unmaterialized virtual target is unavailable even when advertised by a semantic snapshot; the action cannot authorize materialization or scrolling. |
| 12. Snapshot/action metadata proof | Snapshot revision, action advertisement, geometry, timing, and other observational metadata cannot authorize dispatch, execution, scheduling, cache admission, reuse, renderer resources, renderer work, or materialization without independent current authority. |

### Numeric adjustment contract

Radiant also ships the qualified generic `NumericAdjustment<T>` policy boundary
under `radiant::widgets::interaction`; it is re-exported from
`radiant::widgets` and intentionally excluded from the common prelude. The
policy supplies the checked normalized mapping and inverse, explicit
`NumericStep::Base`, `Fine`, and `Coarse` behavior, plus pure bounded scrubbing
and wheel changes. `NumericStepDirection` describes discrete increase and
decrease operations.

```rust
use radiant::widgets::interaction::{
    NumericAdjustment, NumericStep, NumericStepDirection,
};

struct DomainValue;
struct AdjustmentError;
struct DomainAdjustment;

impl NumericAdjustment<DomainValue> for DomainAdjustment {
    type Error = AdjustmentError;

    fn normalized_to_value(&self, _: f32) -> Result<DomainValue, Self::Error> {
        # Ok(DomainValue)
    }

    fn value_to_normalized(&self, _: &DomainValue) -> Result<f32, Self::Error> {
        # Ok(0.0)
    }

    fn step(
        &self,
        _: &DomainValue,
        _: NumericStepDirection,
        _: NumericStep,
    ) -> Result<DomainValue, Self::Error> {
        # Ok(DomainValue)
    }

    fn scrub(
        &self,
        _: &DomainValue,
        _: f32,
        _: NumericStep,
    ) -> Result<DomainValue, Self::Error> {
        # Ok(DomainValue)
    }

    fn wheel(
        &self,
        _: &DomainValue,
        _: f32,
        _: NumericStep,
    ) -> Result<DomainValue, Self::Error> {
        # Ok(DomainValue)
    }
}
```

Adjustment policies own their finite domain, total monotonic mapping, checked
inverse, explicit steps, and bounded pure sensitivities. Finite adjustment
inputs clamp only at declared boundaries; nonfinite inputs and policy failures
are returned through the associated error. The public `numeric_input` builder
requires an adjustment and validates its checked inverse during construction,
but the compatibility text-only consumer does not route step, scrub, or wheel
methods. Complete mode routes those methods only through their explicit policy
attachments, including the shipped `NumericWheelPolicy`; Radiant does not
expose a concrete public `f32` adjustment in this boundary.

### Value mappings

`ValueMapping` is the qualified domain-mapping foundation for numeric controls.
`ValueMapping::linear(...)` and `ValueMapping::logarithmic(...)` accept finite,
strictly increasing `f32` ranges and return a typed `ValueMappingError` when
validation fails; logarithmic ranges must also be positive.

```rust
use radiant::widgets::ValueMapping;

let cutoff = ValueMapping::logarithmic(20.0..=20_000.0).expect("valid cutoff range");
assert_eq!(cutoff.normalized_to_value(0.0), Some(20.0));
assert_eq!(cutoff.value_to_normalized(20_000.0), Some(1.0));
```

Both conversion methods reject nonfinite input and clamp finite input to the
normalized or domain range, respectively. They use `f64` intermediates and
return `Option<f32>` so invalid input or an unexpected nonfinite result cannot
enter a control state. This foundation currently covers only linear and
logarithmic mappings; `ValueFormat` is a separate shipped policy API with
display-only Slider/Knob builder attachment, while domain mapping and broader
widget/input integration remain separate APIs.

### Value formatting

`ValueFormat` is the qualified, backend-neutral policy foundation for displaying
common numeric values. `decimal(...)`, `percent(...)`, and `frequency()` select
the decimal, percent, and frequency forms; `frequency()` uses two fractional
digits by default, and `frequency_with_digits(...)` selects another precision.

```rust
use radiant::widgets::{DecimalSeparator, ValueFormat};

let format = ValueFormat::frequency()
    .with_decimal_separator(DecimalSeparator::Comma);
let mut display = String::new();
format.write_into(440.0, &mut display).expect("finite value");
assert_eq!(display, "440,00 Hz");
```

`write_into(...)` writes into caller-owned `fmt::Write` storage without an
internal `String` allocation. Decimal output is fixed to the selected number
of fractional digits; percent output scales by 100 and appends `%`, while
frequency output appends ` Hz`. The default separator is `Period`, and
`with_decimal_separator(DecimalSeparator::Comma)` changes only the emitted
decimal separator. The policy never inspects ambient operating-system locale.
Requests above `ValueFormat::MAX_FRACTION_DIGITS` (nine) are clamped to that
named maximum.
Nonfinite values are rejected before writing, and caller writer failures return
the typed `ValueFormatError::WriteFailed` variant.

This slice ships the policy foundation and the decimal, percent, and frequency
forms. The official application Slider and Knob builders consume the policy
only for display/automation value text; the numeric text consumer uses its
application `NumericCodec` for editable text and does not use display-only
`ValueFormat`. Direct low-level/public primitive attachment, domain mapping,
and broader input/runtime behavior remain separate. Grouping, decibel, tempo,
and arbitrary custom formatting are also future. These types are qualified
exports from `radiant::widgets::interaction` and `radiant::widgets`; they are
intentionally not exported through the common prelude.

`ActivationInputResult::Activated { provenance }` preserves the accepted input
source and native evidence while `.activated()` remains the compatibility
boolean projection. Accepted pointer releases carry exact release modifiers and
optional timestamps with `sequence_range: None`; accepted focused Enter/Space
key presses carry their optional key-press timestamp. Synthetic pointer and
keyboard constructors therefore still report `Pointer` and `Keyboard` sources
with absent native evidence.
`InteractiveRowMessage::Activate { provenance }` and
`ActivateWithModifiers { provenance }` preserve the accepted single-activation
provenance, while `InteractiveRowMessage::DoubleActivate { provenance }` preserves the exact modifiers
and optional timestamp from the accepted second native double-click sample as
`InteractionProvenance::Pointer { .. }`; its sequence range is always `None`.
`ButtonMessage::Activate { provenance }` and
`ActivateWithModifiers { provenance }` use the same shared provenance shape.
`ButtonWidget` keeps plain `Activate` for pointer and keyboard activation:
accepted primary pointer releases and focused Enter/Space key presses.
`IconButtonWidget` keeps `ActivateWithModifiers` for pointer activation and
plain `Activate` for focused Enter/Space key presses. Pointer activation owns the
accepted release sample, including its exact release modifiers and optional
timestamp with `sequence_range: None`; keyboard activation owns the accepted
key-press timestamp. Press evidence is not reused, and synthetic pointer and
keyboard inputs retain explicit `Pointer` and `Keyboard` sources with absent
evidence. Direct `Accessibility` and `Programmatic` constructions round-trip
through `ButtonMessage::activation_provenance()` and project default modifiers
through `activation_modifiers()`; `ActivateWithModifiers` projects exact
modifiers only for `Pointer` provenance. Secondary and drag messages return
no activation provenance. The plain `Activate` and modifier-aware variants
retain `Clone + Copy + Debug + PartialEq` without adding `Eq` or `Hash`.
The concise `ButtonBuilder::message`, free `button_message`, `SurfaceNode::button`,
`IconButtonBuilder::message`, constant button/icon-button mappers, and
`ButtonBuilder::click_or_drag` helpers intentionally discard provenance while
preserving their existing host messages and drag routing. Typed
`ButtonBuilder::mapped`/`mapped_with`/`filter_mapped`, free
`button_mapped`/`button_mapped_with`, `SurfaceNode::button_mapped`,
`IconButtonBuilder::mapped`, and typed `WidgetMessageMapper` paths forward the
complete `ButtonMessage`. Adding provenance fields is an intentional
source-level migration for direct enum construction and exhaustive matching;
it does not add provenance to retained widget state, revisions, routing, or
paint state.
`ToggleMessage::ValueChanged { checked, provenance }` carries the new checked
value together with the accepted interaction provenance. Accepted primary
pointer releases copy their exact release modifiers and optional timestamp with
`sequence_range: None`; accepted focused Enter/Space key presses copy their
optional key-press timestamp. Synthetic pointer and keyboard constructors keep
the `Pointer` and `Keyboard` source categories with absent evidence, and missing
timestamps never imply `Programmatic`. Provenance is observational only: it does
not change acceptance, flipping, retained interaction state, or host-message
behavior. `ToggleBuilder::message`, application `toggle_mapped`,
`SurfaceNode::toggle`, and `SurfaceNode::toggle_with_checked` intentionally
project only `checked`; `ToggleBuilder::message_with`/`mapped_with`,
`SurfaceNode::toggle_mapped`/`toggle_mapped_with_checked`, and
`WidgetMessageMapper::toggle`/`toggle_mapped` forward the complete typed
`ToggleMessage`. `Accessibility` and `Programmatic` remain explicit direct
provenance values; no input routes are added for them.
`WidgetInput::primary_double_click(...)` is still synthetic pointer input, so it
emits `Pointer` provenance with default modifiers and no timestamp or sequence
range. `activation_modifiers()` remains a compatibility projection: plain and
double activation use default modifiers, modifier-aware pointer activation
projects exact pointer modifiers, and keyboard, accessibility, and programmatic
provenance projects default modifiers. The concise shared and UI-local
`InteractiveRowActions` helpers intentionally discard both single- and
double-activation provenance; modifier-aware callbacks retain only the
compatibility modifier projection, and host message values remain unchanged.
The provenance-bearing single-activation fields are an intentional source-level
API change for direct enum construction and matching; action callback signatures
and their projected host messages remain unchanged.
Move-derived `InteractiveRowMessage::Hover`, `HoverDropTarget`, and
`ClearDropTarget` carry `InteractiveRowMetadata`. Its `modifiers`, optional
`timestamp`, and optional `sequence_range` preserve the normalized native
`PointerMove` sample; use `InteractiveRowMessage::input_metadata()` to read
that contract. Synthetic pointer moves and non-move messages, including
nested `Drag` messages, return default metadata. Drag provenance remains on
`DragHandleMetadata`; `InteractiveRowMetadata` remains move-only and is not
merged with activation provenance. Secondary-click and drop messages do not
gain row metadata.
`InteractiveRowActions` is a widget-layer router; use `row_actions()` to build
the router from the application facade and
`InteractiveRowActions::route(...)` when custom row wrappers need the same
activation, modifier-aware activation, secondary-click, drag, drop, and
hover-drop routing table that `interactive_row().actions(...)` and
`interactive_row_underlay(...).actions(...)` use. Prefer
`interactive_row_underlay(...).dense_chrome().actions(...)` plus the
underlay builder's host-owned visual-state methods when custom visible row
content only needs standard dense chrome; keep `EmbeddedInteractiveRowWidget`
for unusual widgets that add custom paint beyond Radiant's dense-row fill,
marker, and outline model. Use the keyed variants
(`primary_key(...)`, `primary_with_modifiers_key(...)`, `double_key(...)`,
`secondary_key(...)`, `drag_key(...)`, `drop_key(...)`, and
`hover_drop_key(...)`) when row interactions should route through the same
host-owned item key without duplicating capture closures at each row, chip, or
tree item. Use `drop_target_key(...)` when drop and hover-drop both route
through the same host-owned target key but still produce separate host message
shapes. Use `tracked_drop_candidate_key(...)` when drop, valid-target hover,
and tracked-target clear should route through one host-owned target key. Use
`primary_secondary_key(...)` when primary activation and secondary
context-menu activation share the same host-owned key but emit separate host
message shapes. Use
`primary(...)`/`primary_key(...)` plus `double(...)`/`double_key(...)` when
primary release and double-click should route to the same host action. Use
`primary_with_modifiers(...)` or `primary_with_modifiers_key(...)` when primary
release should preserve modifier state; add a separate `double(...)` or
`double_key(...)` slot when double-click should map to the same action with
default modifiers. Compose primary, double, secondary, drag, drop, and
hover-drop slots directly for row shapes such as tokens, selectable drag rows,
tree rows, outline rows, layers, folders, collections, or lanes.
Use `tree_row(label)` when a compact tree or outline row only needs a label,
depth, disclosure slot, selected state, standard dense-row chrome, and common
`InteractiveRowActions` routing. Use `.stable_row_identity(scope, row_key)` when
one durable row key should identify both the composed row subtree and retained
hit target. Keep `.row_key(...)`, `.input_id(...)`, `.stable_input_id(...)`,
`.stable_u64_input_id(...)`, or `.hit_key(...)` for deliberately split identity
contracts or externally reserved widget IDs. Call `.style(WidgetStyle::...)`
when the row's palette and drop-target outline should resolve from the active
`ThemeTokens` at paint time; keep `.palette(...)` and
`.drop_target_outline(...)` for fixed-color overrides. Configure
`TreeRowDragDropState` for host-owned drag/drop validation
and pair the rows with
`virtual_tree_list_window(...)` when the surrounding list needs virtualization
and descendant guide overlays. Keep `EmbeddedInteractiveRowWidget` for unusual
custom-painted rows that need visuals beyond `tree_row(...)`.
Use the single-activation helpers when double-click has a separate host action
such as rename, drill-in, or open-in-place behavior. Drag-capable controls can use
`DragHandleMessage::phase()`, `position()`, `started_origin()`, `started_position()`,
`moved_position()`, `ended_position()`, `finished_position()`, `is_started()`,
`is_moved()`, `is_ended()`, `is_finished()`, and `is_cancelled()` when reducers
need generic drag lifecycle information or cancellation cleanup without duplicating the
`Started` / `Moved` / `Ended` / `Cancelled` variant shape. Use
`DragHandleMessage::started(...)`, `started_from(...)`, `moved(...)`, `ended(...)`,
`double_activate(...)`, and `cancelled(...)` when tests, reducers, or custom
widgets need to construct drag lifecycle messages directly. Threshold-based
controls use `started_from(...)` so reducers can preserve displacement from the
primary-press origin while painting immediate feedback at the current pointer.
Each pointer-bearing drag variant also carries a `DragHandleMetadata` value.
Use `DragHandleMessage::input_metadata()` to preserve the current normalized
`PointerModifiers`, optional `InputTimestamp`, and optional opaque
`InputSequenceRange`. `Started`, `Ended`, and `DoubleActivate` preserve the
current modifiers and timestamp without a sequence range; `Moved` preserves
the current modifiers, timestamp, and sequence range. Focus-loss
`Cancelled` messages return `DragHandleMetadata::empty()`. The public drag
constructors intentionally leave metadata absent, so synthetic and legacy
messages remain observationally equivalent. `DragHandleMetadata` is exported
from `radiant::widgets` and the prelude for hosts that need to inspect or
construct explicit enum values.
Use
`DragHandlePhase::as_str()` for stable lowercase diagnostic labels. Reducers that
resolve or cancel a drag gesture with both an in-window preview and an armed
native external-drag payload can call `UiUpdateContext::end_drag_session()` instead
of ending those runtime surfaces separately. Use
`UiUpdateContext::begin_drag_session(...)` when one gesture may have an in-window
preview, a native external-drag payload, both, or neither. Use
`UiUpdateContext::begin_drag_with_external(...)` when both requests are already
known to exist and should be started together. Explicit runtime bridges can use
the corresponding `Command` constructors, but normal application handlers should
stay on the typed `UiUpdateContext` surface.
External file drags use `ExternalDragRequest::files(...)` from
`radiant::runtime`; the generic native Vello backend supports Windows OLE and
macOS AppKit file receivers, while unsupported targets deliver an explicit
unsupported error through the same completion callback. The completion mapper
is UI-owned and one-shot. On Windows the native drag call supplies its terminal
effect before Radiant defers the mapper to the next controller drain. On macOS
the native launch only admits an `NSDraggingSession`; AppKit later calls the
dragging-source terminal callback when the target copies or rejects the files,
and Radiant then posts the result back to the originating window before the
next UI drain invokes the mapper. `ExternalDragOutcome::accepted()` is true
for any non-`None` terminal effect. Late, duplicate, replaced, and
post-shutdown results are ignored.
Dense custom row painters can use `push_dense_row_chrome(...)` with
`DenseRowChromeParts`, `DenseRowMarkerStyle`, and `DenseRowOutlineStyle` when
one row needs standard fill, leading/trailing markers, and optional outline
composition from one app-neutral paint descriptor. Use `push_dense_row_fill`,
`push_dense_row_label`, `push_dense_row_vertical_marker`, and
`push_dense_row_inset_stroke` when a row needs individual state-prioritized
fills, centered labels, edge markers, or outlines from Radiant's generic
dense-row geometry helpers without repeating paint-plan guard code. Use
`dense_row_palette_from_style(...)`,
`dense_row_drop_outline_from_style(...)`, and
`dense_row_tree_guide_color(...)` when custom dense rows, tree rows, or outline
rows need standard hover, pressed, selected, drop-target, outline, and guide
colors resolved from `ThemeTokens` plus `WidgetStyle` without host-local color
tokens. The standard palette includes a distinct selected-hover fill so
selected rows can brighten on pointer hover without app-local state-priority
code. Use `DenseRowPalette::interaction_fills(...)`,
`interaction_fills_if(...)`, and `without_interaction_fills(...)` when hovered
and pressed fills should be supplied or suppressed together, especially when
interaction paint follows `InteractiveRowWidget::paints_interaction_fill()`
while selected and committed target state should remain visible.
Use `DenseRowLabelParts` when custom dense rows need row-height-aware label
sizing, text insets, alignment, and wrapping without constructing
`PaintTextRun` manually. Use `DenseRowMarkerParts::leading(width)` and
`trailing(width)` for common selection, status, and activity edge markers
instead of repeating raw marker geometry fields. Use
`DenseRowChromeParts::leading_marker_if(...)`, `trailing_marker_if(...)`, and
`outline_if(...)` when custom rows should add optional markers or outlines from
host-owned state without app-local mutation branches.
Tree and outline rows that need continuous descendant guide lines can use
`tree_row(...)`, `TreeRowDragDropState`, `TreeGuideRow`, `TreeGuideMetrics`,
`TreeGuideStyle`, `StyledTreeGuideStyle`, `TreeGuideOverlayStyle`,
`tree_guide_segments(...)`, `tree_guide_overlay(...)`,
`tree_guide_indent(...)`, and `virtual_tree_list_window(...)`. Use fixed
`TreeGuideStyle` for caller-resolved colors, or pass `StyledTreeGuideStyle` /
`TreeGuideMetrics::new(...).with_widget_style(...)` when guide colors should
resolve from the active theme and a semantic `WidgetStyle`. Applications should
map their domain rows into label/depth/disclosure state plus
`starts_descendant_group` metadata while Radiant owns row chrome, shared
interaction routing, segment projection, paint clipping for materialized
virtual-list windows, passive indent sizing, and the standard fixed-row virtual
tree body composition.
Rows that need active drag-source motion after a retained refresh can opt into
`with_drag_source_motion(...)`; rows that should accept drops without producing
drop-hover messages can use `with_drop_only(...)`. Application-builder rows
can use `drop_target_mode(drag_active, hover_messages)` when the current row
should become either a normal drop target or a drop-only target from
host-owned drag state without app-local `droppable` / `drop_only` branches.
Use `tracked_drop_target(drag_active, active_target)` when the host tracks the
current hover/drop target: candidate rows emit hover-drop messages, while the
already-active target keeps accepting the eventual drop without repeatedly
requesting the same hover-target update.
Use `tracked_drop_candidate(drag_active, current_target, candidate,
active_target)` when host-owned validation decides whether this row is a valid
drop target and non-candidate rows must still report hover once to clear a
previously active target. Those non-candidate hover reports emit
`InteractiveRowMessage::ClearDropTarget` instead of `HoverDropTarget`, allowing
the action router to keep target-enter and target-clear host messages distinct.
Use `InteractiveRowBuilder::filter_mapped(...)` when only selected row events
should emit host messages, such as activation and drop while drag-hover or
secondary-click events are ignored. This avoids routing ignored row interactions
through app-level no-op messages.
Rows emit `DoubleActivate` for primary-button double-click flows such as
opening an item, entering rename mode, or drilling into a details row.
Large list and tree-style surfaces can use `VirtualListController` when they
need durable item-index viewport state outside the declarative scroll container.
It wraps the existing virtual-window, row-scroll, focus guard-band, and
scrollbar projection helpers so applications do not need to keep viewport-start
bookkeeping beside each large list.
Controllers can be configured per projection pass with `configure(...)`, follow
optional app-owned focus using `focus_optional(...)`, and consume native
pixel-scroll offsets with `set_scroll_offset(...)` while preserving the same
clamping and virtual-window contract.
Use `VirtualListProjection::new(total_items, viewport_len, overscan,
guard_band)` when list geometry should be passed as one named projection value
instead of repeated positional arguments. Add `with_context_row()` or
`with_context_rows(...)` for browser, outline, table, or picker lists that
should preserve adjacent context around focused items before guard-band
scrolling moves the viewport.
Use `set_scroll_offset_for_items(...)` when a native scroll update arrives
through a list whose item count may also have changed because of filtering,
search, or app-owned selection.
Use `apply_window_change(...)` when
`virtual_list_windowed(...).on_window_changed(...)` reports a
runtime-originated window change and the app stores durable list state in a
`VirtualListController`. Use
`runtime_viewport_len_or(fallback)` to carry the runtime-reported viewport
length into later projection passes, and use
`runtime_viewport_contains_index(...)` when already-visible focus logic should
only trust a viewport reported by the scroll container. Use
`viewport_contains_index(...)` before reconfiguring after filters, sorts, or
selection changes when an already-visible focused item should not force a scroll
jump.
Use `configure_and_focus_optional(...)` when a projection pass should update
item count, viewport policy, and optional host selection in one controller call.
  Use `configure_projection_and_focus_optional(...)` or
  `configure_projection_and_focus_changed_optional(...)` when the same projection
  inputs are reused across a virtualized pane or should stay readable beside
  host-owned focus-key logic.
  Use `configure_and_focus_optional_with_context_row(...)` for browser, outline,
  table, or picker lists that should keep one adjacent context row around the
  focused item before guard-band scrolling moves the viewport.
  Use `VirtualListFollowState` and `VirtualListFocusTarget` with
  `configure_and_focus_changed_optional(...)` or
  `configure_and_focus_changed_optional_with_context_row(...)` when a list should
  scroll newly selected items into view without overriding manual scroll while
  the same app-owned item key remains selected. Use
  `configure_projection_and_focus_changed_unless_visible_optional(...)` when a
  pointer or host selection can move to another item that is already visible and
  direct runtime scroll position should stay authoritative. Use
  `VirtualListSliceFocus::from_slice_by(...)` with
  `configure_slice_focus_changed_optional(...)` when the host owns a filtered or
  sorted item slice and stable focus key while Radiant should derive the item
  count, resolve the selected key in that slice, and update changed-key follow
  state in one pass without keeping the item slice borrowed during the mutable
  controller call. Use
  `VirtualListFocusTarget::from_slice_by(...)` when the focused item key must be
  resolved against the current filtered or sorted item projection before
  following selection.
  Overlay and retained-geometry code that needs to mirror compact stack spacing
  can use `StackedLayoutCursor` to accumulate item extents and gaps without
  app-local offset arithmetic. Use the chainable `advanced(...)` and
  `advanced_many(...)` forms when repeated rows precede an overlay target, and
  `advanced_if(...)` when optional rows should affect overlay anchors without
  introducing mutable cursor plumbing at the call site. Use `StackedLayoutItem`
  with `StackedLayoutCursor::from_items(...)` when the stack prefix is easier to
  describe as data, such as mixed fixed rows, optional rows, and repeated
  labeled-control rows before an overlay target. Use `offset_within_item(...)`
  when an overlay or retained marker should anchor to a nested control inside
  the current stacked item rather than the item's start edge.
Use `local_drop_marker(...)` for non-interactive insertion markers that should
be positioned in a local stack or row layer, such as details-header reorder
targets or list drop indicators, without rebuilding spacer and feedback-overlay
composition in application code. The marker paints from its assigned bounds and
clamps to the visible local range, so constrained or clipped headers keep a
visible insertion affordance instead of dropping the marker when the target lies
near the trailing edge.
Timeline and waveform-style surfaces can use `IndexViewport` for generic
integer range navigation. It owns clamping, visible fraction, scrollbar offset,
anchor-preserving zoom, visible-span pan, `pan_by_visible_ratio_drag(...)` for
drag gestures expressed as local ratios, and visible-to-absolute ratio
projection, plus absolute-to-visible point and clipped range projection, so
apps do not need to keep small-but-risky viewport math beside every custom
canvas. Use `IndexViewportScope` when one surface repeatedly applies those
operations against the same total item count and minimum visible span. Use
`visible_normalized_range(...)` when a clipped absolute `NormalizedRange`
should stay typed for downstream canvas or timeline paint helpers instead of
being unpacked into local start/end floats in application code.
`NormalizedRange::from_fractions(...)`,
`NormalizedRange::from_edge_fraction(...)`,
`NormalizedRange::with_edge_fraction(...)`,
`NormalizedRange::shifted_by_fraction(...)`, `NormalizedRangeDrag`,
`NormalizedRangeEdge`, `normalized_fraction_to_milli(...)`,
`normalized_fraction_to_micros(...)`, and `normalized_fraction_to_nanos(...)`
convert floating point interaction ratios into the stable normalized units used
by timeline, canvas, and retained visualization APIs while keeping common
range creation, fixed-edge resizing, edge dragging, and clamped movement
behavior out of host code.
Application scrollbars can use `ScrollbarBuilder::message(...)` when reducers
only need the normalized offset, or `mapped(...)` when they need the raw
`ScrollbarMessage`.
Custom canvas widgets can use `CanvasGestureState` to turn raw `WidgetInput`
pointer events into local and normalized hover, press, drag, release,
double-click, drop, wheel, and focus-change events. This keeps waveform,
timeline, node-editor, and other direct-manipulation widgets on a shared
backend-neutral interaction contract while the application still owns domain
actions such as range selection or marker editing.
Use `CanvasPointer::is_inside(...)`, `normalized_x()`, and `normalized_y()` to
classify projected pointer events and read normalized axes without repeating
host-coordinate bounds checks or raw vector-field access in app widgets.
Each pointer-like `CanvasGestureEvent` also carries a `CanvasGestureMetadata`
value. Use `input_metadata()` to preserve the current normalized input's
`PointerModifiers`, optional `InputTimestamp`, and optional opaque
`InputSequenceRange` through canvas gesture delivery. Move and wheel events
carry the current modifiers, timestamp, and sequence range; press, release,
double-click, and drop events carry the current modifiers and timestamp without
a sequence range. `Drag::modifiers` remains the original press modifiers for
backward-compatible gesture semantics, while `input_metadata().modifiers`
reports the current move modifiers. Public `WidgetInput` constructors leave
these metadata fields absent.
Use `CanvasGestureEvent::pointer()`, `origin()`, `button()`, `modifiers()`,
`delta()`, and `pointer_is_inside(...)` when a custom canvas needs shared
gesture metadata without matching every hover, press, drag, release,
double-click, wheel, and drop variant separately. Use `hover_pointer()`,
`press_pointer(...)`, `release_pointer(...)`, `double_click_pointer(...)`, and
`wheel_pointer_delta()` when common routed event shapes should stay declarative
while app code owns the resulting domain messages. Use
`press_pointer_inside(...)`, `release_pointer_inside(...)`,
`double_click_pointer_inside(...)`, and `wheel_pointer_delta_inside(...)` when
the routed event should also be filtered to a widget or sub-surface bounds.
Custom widgets that handle `WidgetInput` directly can use
`pointer_position()`, `pointer_start_position()`, `pointer_start_inside(...)`,
and `pointer_start_outside(...)` to share Radiant's backend-neutral pointer
classification without repeating local press/double-click/wheel bounds checks.
Custom clickable widgets that need their own paint code can use
`handle_activation_input` with `ActivationInputPolicy::pointer_only()` or
`ActivationInputPolicy::focusable()` to share Radiant's hover, pressed, focus,
pointer activation, and keyboard activation transitions without reimplementing
that state machine. Focused widget tests, automation, previews, and embedded
hosts can use `Widget::paint_primitives(...)` or
`paint_primitives_with_defaults(...)` when they need one widget's paint output
as a vector without repeating primitive-buffer, layout, and default-theme setup.
Text-like widgets support semantic foreground roles such as
`TextColorRole::Muted`, so applications can express low-emphasis labels without
app-local paint-only text widgets or hard-coded theme colors.
Passive cell, legend, and swatch indicators can use `ColorMarkerWidget` to draw
small aligned color markers without application-owned paint-only widgets.
`marker_run(color, count)` covers repeated same-color compact indicators, while
`marker_run_colors(colors)` paints one compact marker per supplied color.
Transparent overlay layers that need to consume or observe pointer traffic
without painting can use `PointerShieldWidget`. It emits generic
`PointerShieldMessage` values for configured pointer moves, presses, releases,
drop, and wheel input, so applications can block interaction during
modal/loading states or clear stale drag-hover state without app-local invisible
hit-test widgets. `PointerShieldProps::wheel` and
`PointerShieldBuilder::wheel(...)` control wheel interception; existing
move-only and drop-only convenience constructors leave wheel disabled.
Each emitted message preserves the optional `InputTimestamp` carried by normalized
`WidgetInput`; high-rate move and wheel messages also preserve its optional opaque
`InputSequenceRange`. Press, release, and drop messages intentionally carry no
sequence range. Public and synthetic `WidgetInput` constructors leave these
metadata fields absent. The metadata is observational only: it does not change
shield bounds, acceptance, routing, focus, capture, refresh, scheduling, paint,
or application mapping behavior.
Convenience constructors such as `.pointer_move_only(...)` and
`.pointer_drop_only(...)` cover common transparent overlay policies.
Container-owned pointer targets can use
`ViewNode::pointer_target(...)`, `pointer_target_if(...)`,
`pointer_move_target(...)`, and `pointer_drop_target(...)` for bounded drag,
drop, cancellation, and hover-clear behavior without hand-building overlay
stacks. When multiple pointer targets are stacked on the same owner, Radiant
routes each pointer input to the topmost target that accepts that event kind.
For example, a move-only target above a release/drop target observes motion
without shadowing the lower target's release or drop handling.
Popover and menu stacks can use `dismiss_layer(message)` as a transparent
full-surface activation layer behind foreground content, avoiding app-local
empty input-only buttons for outside-click dismissal.
When the caller has separate base content and foreground overlay content,
`dismissible_overlay(base, overlay, message)` composes the standard
base/dismiss/foreground stack so apps do not repeat the ordering required for
outside-click dismissal.
Use `dismissible_overlay_with_interactive_base(base, overlay, message)` when
the base surface contains controls that should remain clickable while the
foreground overlay is open; Radiant routes non-interactive base space to the
dismiss layer and keeps foreground overlay content on top.
Base content with optional transient UI should normally use `scene(base)`.
`Scene` is Radiant's declarative root surface model: applications decide which
typed layers are active from state each frame, while Radiant owns generic scene
projection and layer z-order. The preferred pattern is to declare overlays
beside the component that owns them with
`ViewNode::overlays(ui::overlays().floating_opt(...).blocking_modal_opt(...))`.
`Overlays` provides typed helpers for `floating(...)`, `popover(...)`,
`modal(...)`, `blocking_modal(...)`, `context_menu(...)`,
`dismissible_context_menu(...)`, `tooltip(...)`, and `drag_preview(...)`, plus
matching `*_opt(...)` helpers for optional surfaces. Keep `Overlays::layer(...)`
and `layer_opt(...)` for unusual custom `Layer` policy or advanced/manual
composition.
The root `scene(base)` collects descendant declarations during normal lowering,
so the root view does not need a registry of every popup, modal, menu, tooltip,
or drag preview the app might show.
Use `radiant::Layer::floating(...)`, `radiant::Layer::popover(...)`,
`radiant::Layer::modal(...)`, `radiant::Layer::context_menu(...)`,
`radiant::Layer::tooltip(...)`, and `radiant::Layer::drag_preview(...)` only
when a host needs explicit advanced layer policy. Attach those custom layers locally through
`ViewNode::overlays(ui::overlays().layer(...))`, or attach them explicitly at
the root with `Scene::layer(...)`, `Scene::layer_opt(...)`, or
`Scene::layers(...)` when a host deliberately owns a root-level transient that
does not belong to one component.
Layer input policy is explicit and Radiant-owned. `Layer::pass_through()` is
the default and adds no synthesized input surface. `Layer::block_input()` adds a
transparent full-scene input surface below that layer's foreground content,
consuming pointer and wheel input behind modals or other blocking surfaces.
`Layer::dismiss_on_outside_click(message)` emits the supplied message for
outside pointer press/drop and blocks wheel input behind the layer, while
foreground content still routes above the dismiss surface.
`Layer::input_policy()` returns the declared `LayerInputPolicy`.
View-local collection is a lowering-time move through the declarative view tree,
not a persistent overlay registry or imperative runtime service. A scene with no
view-local or explicit layers follows the same base layout, traversal, input,
focus, native drop target lookup, and widget state synchronization path as the
base view. When both view-local and explicit root layers exist, Radiant collects
descendant layers first and then appends explicitly supplied scene layers before
applying fixed kind z-order.
Scenes can also carry presentation declarations that belong to the root
surface instead of launch wiring. Use `Scene::frame_clock(...)` or
`Scene::frame_clock_opt(...)` for host-state frame messages, and
repeat `Scene::overlay(...)` or use `Scene::overlay_opt(...)` for ordered, keyed
paint-only transient overlays over the cached scene. Overlay declaration order
is their paint z-order; each descriptor's `when(...)` predicate gates its own
painter, while the runtime combines the active overlays' paint-only demand and
cadence. A later declaration with an existing key replaces that binding in its
original slot, so duplicate keys update the last value without changing
z-order; distinct keys append in declaration order. Activity is sampled once
per host animation poll and consumed by the corresponding paint. Direct paint
or a newly projected scene evaluates an unsampled predicate once as a fallback.
Presentation declarations do not become layout or input children, so they do
not change base hit testing, layer ordering, or widget state synchronization.
Root-scoped shortcuts should also be declared on the scene with
`Scene::shortcuts(...)` and `ShortcutCatalog`. A catalog contains ordered
`ShortcutLayer` values plus an optional fallback resolver for dynamic keys such
as navigation. Scene shortcuts resolve before focused-widget key routing and
fall back to app-builder `.shortcuts(...)` only when unhandled.
`Scene::into_view()` projects a runtime scene that paints layers in this fixed
order: base layout, generic floating layers, popovers, modals, context menus,
tooltips, and drag previews. Lower-level callers can still use
`overlay_stack(base)` for bounded local overlays such as loading feedback,
paint-only markers, or advanced transparent input shields that share one content
region's bounds. Prefer attaching ordinary bounded pointer/drop routing to the
owning view with `.pointer_target(...)`, `.pointer_target_opt(...)`, or the lazy
conditional `.pointer_target_if(...)` and a `pointer_target(...)`,
`pointer_drop_target(...)`, or `pointer_move_target(...)` builder. Add optional
overlay-stack children with
`OverlayStack::overlay_opt(...)` and `OverlayStack::input_opt(...)`, then call
`OverlayStack::into_view()`.
It delegates projection to `stack_layers(...)`, so a base-only stack returns the
base view unchanged while multiple children become a normal `stack(...)`.
Use `stack_layers(...)` directly only when the caller already owns an untyped
layer list; it returns `empty()` for zero layers, returns the only layer
unchanged for one layer, and builds a normal `stack(...)` for multiple layers.
Dropdown menus rendered as stack-level overlays can use
`dropdown_menu_overlay_below_trigger(...)` when the menu is anchored below
Radiant's standard dropdown trigger, avoiding app-local calls to
`dropdown_height(...)` just to recover the trigger height.
Composite controls can use `input_overlay(content, input)` when visible content
and a transparent input surface should share bounds without repeating a local
two-child stack. Use `input_underlay(content, input)` when the input surface
should stay below visible content so it can paint hover, selection, drag, or
drop-target feedback behind custom row contents.
Clickable swatches, status filters, and other compact selectable options should
use `selectable(...).color_marker(...)` with `.color_marker_side(...)`,
`.color_marker_inset(...)`, or `.color_marker_align(...)` instead of composing a
passive `color_marker(...)` below a selectable input surface.
Passive visual feedback layers can use `FeedbackOverlayWidget` for background
tints, determinate progress fills, and edge-band accents without app-local
paint-only custom widgets.
Status surfaces and background-job indicators can use `ProgressBarWidget` for
theme-backed determinate or indeterminate horizontal progress, with optional
pointer activation when the bar should open details. Use `.passive()` for
display-only progress bars that should paint without host output mappings. Use
`ProgressBarBuilder::message(...)` for simple activation actions, or
`mapped(...)` when reducers need to inspect `ProgressBarMessage` directly.
Applications that already track work with `ProgressSnapshot` can use
`progress_bar_for_snapshot(...)` to choose determinate or indeterminate
progress without app-local branching.
Long-running work that reports fractional progress from tight worker loops can
use `ProgressUpdateGate` to coalesce updates by time and delta before sending
messages back into the UI, while still accepting terminal updates immediately.
Use `ThrottledProgressReporter` when the worker should run accepted fractions
through a callback instead of manually checking the gate before every send.
Use `ProgressPhase` when a multi-stage worker needs to map completed/total
step counters into one normalized progress subrange such as `0.25..0.75`.
Retained custom surfaces can use `RetainedSegmentPlan` with
`RetainedSegmentRevisions` to name static and overlay paint segments, derive
stable invalidation masks, and bump only the revisions affected by a change.
This keeps segment ownership explicit for dense retained surfaces without each
application inventing a separate bit layout and diagnostic vocabulary.
`NativeRunOptions` keeps platform/window integration policy behind Radiant's
native runtime boundary. Common launch code can stay platform-neutral while
still configuring `window.title`, `window.geometry`, `window.behavior`,
`window.icon`, `frame.target_fps`, `frame.devtools`, and whether native file
drag-and-drop is requested on platforms that support it. Native animation frame
rates are normalized through `NativeRunOptions::normalized_target_fps()` and
the exported `MIN_NATIVE_TARGET_FPS` / `MAX_NATIVE_TARGET_FPS` bounds before
timed redraws or present-mode selection use them. Focused text-input caret
animation uses a lower native cadence when it is the only timed animation
demand, while explicit application or overlay animation frame-rate caps remain
authoritative. Set `window.behavior.reveal_after_surface_setup` to `false` only
when a host-managed or profiling flow must create and present the native surface
without making the window visible after setup. Window launch and manifest
builders provide integer `.size(...)` convenience methods
plus `.logical_size(...)`, `.min_logical_size(...)`, and `.position(...)` for
fractional dimensions and initial ordinary-window placement. Popup builders
retain `.popup_position(...)` for popup-native placement.
Ordinary window launches should use the typed `FrameRate` choices
`FrameRate::Hz30`, `FrameRate::Hz60`, or `FrameRate::Hz120` with
`radiant::window(...).frame_rate(...)`. `WindowSpec::frame_rate(...)` provides
the same typed policy for host-managed manifests; the existing raw
`WindowSpec::target_fps(...)` builder remains available as an advanced escape
hatch for custom cadences.
On macOS, hosts that need direct development builds to appear as normal
LaunchServices applications can use `scripts/dev_app_bundle.sh` after building
their binary. The helper stages a minimal `.app` wrapper, writes `Info.plist`,
copies the executable into `Contents/MacOS`, ad-hoc signs when possible, and
launches with `open`, so app-level automation tools can attach by application
name or bundle id. Hosts provide generic environment inputs such as
`RADIANT_DEV_APP_NAME`, `RADIANT_DEV_APP_BINARY`, `RADIANT_DEV_APP_BUNDLE_ID`,
`RADIANT_DEV_APP_VERSION`, and optional `RADIANT_DEV_APP_ICON` `.icns` assets;
Radiant owns the bundle mechanics while the host keeps build flags, product
naming, logging arguments, and app-specific launch policy.
Native dev and automation sidecars can set `RADIANT_AUTOMATION_TARGET_EXPORT`
to a JSON path. The native Vello runtime exports
`GuiAutomationTargetSnapshot` after surface refreshes with atomic file
replacement and unchanged-payload suppression; set
`RADIANT_AUTOMATION_TARGET_EXPORT_PRETTY=1` for readable JSON during manual
automation work.
For host-visible platform services, ordinary reducers should use
`UiUpdateContext::pick_folder(...)`, `pick_file(...)`, `save_file(...)`, `open_path(...)`,
`reveal_path(...)`, `open_url(...)`, `copy_text(...)`,
`copy_file_paths(...)`, `read_text(...)`, `read_file_paths(...)`, or
`confirm(...)`; `notify(...)`, `write_clipboard(...)`, and `read_clipboard(...)`
cover neutral notifications and the app-instance-owned typed clipboard.
Hosts that need an owner-qualified, replaceable platform operation can use
`UiUpdateContext::platform_effect(&mut latest, owner, request, map)` with
`EffectOwner::Application` or an exact `EffectOwner::Declarative(...)`.
Hosts that genuinely need the unqualified raw protocol can explicitly import
`PlatformRequest` and call `UiUpdateContext::platform_request(...)`.
Custom bridges handle those requests via
`RuntimeBridge::request_platform_service(...)`; bridges that do not provide a
platform service return an explicit unsupported error through the normal
completion callback instead of blocking the UI thread or forcing app code to
depend on a native dialog or clipboard crate.
Platform and external-drag completion callbacks are UI-owned and may capture
`Rc`, `RefCell`, or other local state. The app runtime assigns each platform
request an opaque identity, sends only the request and `PlatformResult` across
the worker boundary, and invokes the one-shot completion mapper while draining
the UI queue. Late, duplicate, or post-shutdown completions are rejected before
message reduction.
`NativeGpuOptions` and `NativeGpuBackend` keep WGPU backend selection explicit
without exposing normal app code to raw WGPU setup; the default remains WGPU's
environment-aware adapter selection, while diagnostics or platform work can
request a specific backend such as DX12, Vulkan, Metal, GL, or browser WebGPU.
`NativeTextOptions` lets hosts provide embedded font bytes or preferred font
files before Radiant falls back to environment or system fonts, keeping
text/font policy explicit without moving application asset loading into the
renderer. Use `EmbeddedFont::from_static(include_bytes!("fonts/App.ttf"))` with
`.embedded_font(...)` on `radiant::window(...)`, `radiant::app(...)`, or
`WindowSpec` when an application should ship as a portable package without
depending on installed font files.
`ImageRgba::try_new(...)` validates row-major RGBA8 image payloads with a typed
`ImageRgbaError`; `ImageRgba::new(...)` remains the `Option`-returning
convenience wrapper for compact tests and examples.
`ListSelectionController` provides reusable index-based focus, anchor, toggle,
range, additive range, select-all, and revision tracking for dense virtual
lists. Use `ListSelectionIntent::from_extend_toggle(...)` with
`select_with_intent(...)` when pointer or keyboard modifiers should map to
replace, range extend, toggle, or additive range selection without application
adapters. Use `ListSelectionModifiers::from_extend_toggle(...)` for older
callers that only need replace, range extend, or toggle. `KeyedListSelection<K>`
provides the same focus, anchor, range, toggle, additive range, navigation,
additive navigation, and select-all behavior over stable row keys while the
application passes the current ordered visible keys into operations that depend
on list order. Use it for lists whose durable selection identity is a path,
database id, document id, or other stable app key rather than a transient
visible index. Use `list_index_after_delta(...)` for clamped keyboard
navigation and `cyclic_list_index_after_delta(...)` for wrapped menu,
autocomplete, command-palette, and dropdown-style option navigation. Use
`unit_interval_index(...)` when a normalized hit, scrub, random, or continuous
input coordinate should resolve to one bounded row index without application
edge-case math. When that
wrapped option navigation is bound to a transient query or prefix, use
`CyclicListSelectionCycle` to keep the selected index for the current query,
reset display selection for new queries, and clear state when the visible
option list is empty. Use `active_selected_index(...)` when a fresh query
should show options without selecting one yet, and
`move_selection_from_edge(...)` when first ArrowDown/ArrowUp-style movement
should select the first or last option before subsequent movement wraps.
`CancellationToken` and `context.business().background(...).cancellable()`
provide a small cooperative-cancellation contract for long host-owned jobs.
Radiant still does not force-stop work; applications keep a token clone and
workers check `radiant::runtime::BusinessWorkContext::is_cancelled()` at natural
boundaries before returning early.
`WindowSpec` describes one host-managed window without opening the platform
runtime. `WindowManifest` stores ordered specs and rejects duplicate stable
keys, non-positive or non-finite logical sizes, and non-finite ordinary or popup
positions, returning typed `WindowManifestError` / `WindowSpecError` diagnostics so
multi-window or embedded hosts can validate a window set and attach a separate
bridge or view to each spec. `radiant::window(...).spec("main")`
converts the no-state launch builder into the same manifest shape.
Floating popups use the same surface and runtime model as normal windows while
requesting popup-native window policy. Use `WindowSpec::popup(...)`,
`NativeRunOptions::popup(...)`, or `.floating_popup()` on launch builders for
borderless transient windows such as drag previews, context menus, tooltips, and
small floating panels that need to render outside the main application window.
Use `WindowSpec::prewarmed_popup(...)`,
`NativeRunOptions::prewarmed_popup(...)`, or `.prewarmed_popup(...)` on launch
builders when the host wants one already-presented popup ready for instant
native reveal.
Native popup windows are revealed as soon as the window surface and initial
Radiant scene are prepared, then the first redraw is requested immediately, so
apps can treat one popup as an instant transient UI surface rather than a
deferred background launch.
`NativePopupOptions` controls the optional initial screen position,
transparency, topmost behavior, focus-on-open behavior, resizability, taskbar
presence, first-present hiding for prewarmed surfaces, and an optional top-edge
native drag region where the platform supports those hints. Hosts that need a
guaranteed instant first popup interaction can prewarm one offscreen visible
popup surface with `NativePopupOptions::prewarmed_at(...)`, wait until the
runtime hides it after its first presented frame, prime the non-focusing
show/hide path, and then park the prepared surface visible at the offscreen
prewarm position before user input reaches the popup trigger. They can then move
and reveal the prepared native window on demand without rebuilding the GPU
surface, renderer, first scene, first present, first post-hide native reveal,
first visible placement, or first native show during the click. If the popup
also needs focus, request foreground activation after the already-rendered
surface is visible so first activation cannot delay the visual reveal. Direct
`NativeRunOptions` launch paths can call
`.validate()` before startup, and the native runtime returns
`NativeGenericRunError::InvalidWindowOptions` instead of passing non-finite or
non-positive geometry into the platform window layer.
Use `NativeRunOptions::default().devtools_overlay_enabled(true)` or
`.devtools_overlay(DevtoolsOverlayOptions::enabled())` to opt into Radiant's
runtime-local devtools overlay for native inspector builds. The primary-window
`WindowBuilder` and `StatefulAppBuilder` expose the same option through their
`.devtools_overlay(...)` methods. The overlay is disabled by default and paints
as runtime overlay content, so ordinary apps do not pay for inspector
presentation unless they enable it.

Serious apps use the same builder API. `radiant::app(...)` supports
`.subscriptions(...)` for interval and worker-message sources, `.on_startup(...)`,
`.on_shutdown(...)`, `.on_close_requested(...)`, `.run_with_artifacts()`, and
retained-surface painters registered through `.retained_painter(...)`.
Use `.on_scroll(...)` only as an advanced lifecycle hook for custom scroll
observation; declarative fixed-row virtual lists should prefer
`virtual_list_windowed(...).on_window_changed(...)` so scroll-window state flows
through ordinary app messages.
Root-scoped frame clocks and paint-only transient overlays should normally be
declared on `ui::scene(...)`:

```rust
ui::scene(layout::shell(state))
    .frame_clock(
        ui::FrameClock::message(GuiMessage::Frame)
            .fps(60)
            .repaint_scope(
                |state| state.frame_repaint_scope_before_update(),
                |state, scope| state.frame_can_use_paint_only(scope),
            ),
    )
    .overlay(
        ui::TransientOverlay::new(1_u64)
            .paint_only()
            .when(|state| state.waveform_is_playing())
            .fps(60)
            .paint(|state, context, primitives| {
                state.paint_playback_overlay(context, primitives);
            }),
    )
    .into_view();
```

Radiant passes the common-prelude callback payload `TransientOverlayContext`
with the latest `SurfacePaintPlan`, viewport, and animation time. This keeps
structural state, layout, and Vello scene refreshes out of animation paths for
visuals such as playheads, drag previews, tooltip affordances, cursor markers,
and lightweight spectrogram overlays.

For app-builder code that needs the same descriptors outside a root scene, use
`.presentation(...)`:

```rust
radiant::app(state)
    .view(view)
    .presentation(
        ui::presentation()
            .frame_clock(
                ui::FrameClock::message(GuiMessage::Frame)
                    .fps(60)
                    .repaint_scope(
                        |state| state.frame_repaint_scope_before_update(),
                        |state, scope| state.frame_can_use_paint_only(scope),
                    ),
            )
            .transient_overlay(
                ui::TransientOverlay::new(1_u64)
                    .paint_only()
                    .when(|state| state.waveform_is_playing())
                    .fps(60)
                    .paint(|state, context, primitives| {
                        state.paint_playback_overlay(context, primitives);
                    }),
            ),
    )
    .handle_message(update)
    .run();
```

`FrameClock` is for host-state frame messages. `TransientOverlay` is for
paint-only presentation work over the cached surface, and a `Presentation` can
declare multiple keyed overlays in order with repeated
`.transient_overlay(...)` or `.transient_overlays(...)`. These descriptors lower
to the same runtime animation and transient-overlay hooks whether they are
attached to `Scene` or to the app builder. Reducer messages request a surface repaint by
default, while frame-clock messages with `repaint_scope(...)` can resolve to
paint-only repaint when the frame update did not require a structural surface
refresh. Duplicate presentation keys use the same in-place replacement rule;
activity samples are not reused after a scene presentation is cleared.

For realtime-feeling desktop surfaces, prefer a 60Hz frame clock with a strict
`repaint_scope(...)` policy over a conditional frame clock that starts and stops
around foreground activity. The steady clock gives the runtime a predictable
cadence to measure and diagnose; the repaint scope, retained surface revisions,
layout/text caches, and transient overlays keep stable frames from doing full
surface work when nothing relevant changed.

Compatibility policy: root-scoped app presentation should use
`Scene::frame_clock(...)` and `Scene::overlay(...)`. App-builder
`.presentation(...)` is the compatibility path for hosts that need descriptor
based presentation without a root `Scene`. The older launch-level
`.animation(...)`, `.on_frame(...)`, `.transient_overlay(...)`,
`.transient_overlay_animation(...)`, `.animated_transient_overlay(...)`,
`.transient_overlay_animation_at(...)`, and
`.animated_transient_overlay_at(...)` hooks remain public, supported,
lower-level lifecycle APIs for direct runtime control, custom hosts, examples
that intentionally demonstrate the runtime lifecycle, and migration of existing
callers. They are not deprecated in this phase because they still map to real
runtime capabilities and are exercised by public API tests, but new root-scoped
application presentation should prefer the `Scene` descriptors unless direct
lifecycle wiring is specifically required. The built-in app bridge keeps those
launch-level hooks in an isolated adapter so ordinary frame-clock demand remains
the canonical presentation path. Custom runtime bridges can report the same
split explicitly with `RuntimeAnimationActivity` and
`RuntimeAnimationDemand`, distinguishing frame-message animation from paint-only
presentation work and optionally carrying a per-activity target FPS.
When a paint-only transient overlay is present, the native Vello runtime also
caches the composed Vello scene plus retained GPU surfaces as a base frame, so
later overlay-only frames can present that stable composition and draw the
moving overlay without re-encoding retained GPU surfaces until the scene, paint
plan, or runtime GPU-surface overlays change. This supports visuals such as a
playback playhead without refreshing the declarative surface, rebuilding the
cached Vello scene, or recompositing. Scene overlays drive this path directly
instead of queueing app frame messages.
Overlay painters that attach to existing content should use `context.plan` as
the authoritative cached geometry source. For common widget anchors, use
`SurfacePaintPlan::first_widget_rect(widget_id)` instead of matching primitive
variants in every animation frame.
Retained canvas views reserve stable cached surfaces with
`retained_canvas(key).revision(...).dirty_mask(...).volatile(...).on_input(...)`, while the
app painter owns the corresponding backend-neutral `PaintFrame`.
GPU-heavy retained views can be placed directly with
`render_canvas(key, revision, RenderCanvasContent::...)`. This lowers through the
same generated-ID, layout, focus, hit-test, and paint-plan path as standard
widgets, then emits `PaintPrimitive::GpuSurface` for native GPU backends.
Applications that need custom capability flags, runtime pointer-line policies,
or runtime-owned overlay behavior can use
`render_canvas_with_capabilities(key, revision, content, capabilities)`.
Use `RenderCanvasConfiguredParts` with
`render_canvas_configured_from_parts(...)`
for advanced named-field construction that also needs lightweight
backend-composited overlays.
Render canvases that need host-visible input can use
`render_canvas_input(key, revision, content, |input| Message::CanvasInput(input))`;
plain `render_canvas(...)` remains passive so pointer motion over retained visual
surfaces does not force unnecessary message dispatch or relayout.
The former `gpu_surface*` builders and `GpuSurface*` construction names remain
available only through explicit application or runtime module imports as
transitional compatibility APIs; they are not part of the common prelude.

Current supported 0.1.x boundary: the `RenderCanvas` vocabulary is a
compatibility alias over `GpuSurface` vocabulary and its paint output is
`PaintPrimitive::GpuSurface`. The existing keyed/revision builders,
capability/configured-parts/input helpers, `GpuSurface*` names,
`RenderCanvas*` aliases, `PaintRenderCanvas`, and ungated
`RenderCanvasContent::CustomShader` remain supported throughout 0.1.x.

The target-only registered renderer-neutral API introduces `CanvasProgram`,
`CanvasGraph`, `CanvasContractVersion`, `CanvasPayloadVersion`, typed
`CanvasUniforms`, bounded `CanvasGraphLimits`, typed capabilities, and a
required primitive fallback. Its provisional builder is
`render_canvas_program(canvas)`. The canonical one-argument
`render_canvas(canvas)` and `PaintPrimitive::RenderCanvas` are not adopted in
0.1.x; they become valid only at an explicit 0.2 breaking boundary after
recorded migration evidence. See the [normative OPT-1407 render-canvas contract](DESIGN_DIRECTION.md#render-canvas-compatibility-contract-opt-1407)
for the closed graph vocabulary, validation/fallback diagnostics, identity
fences, WGSL feature gate, migration examples, and unresolved renderer risks.

## Soft-Deprecated First-Use Boilerplate

The old explicit first-use path is soft-deprecated:

- constructing `NativeRunOptions` directly for a hello-world app
- hand-writing a closure bridge before the app has meaningful state
- wrapping one label in `Arc<UiSurface<_>>`
- manually composing `SurfaceNode`, `SurfaceChild`, explicit numeric IDs, and
  `WidgetSizing` just to render a starter view

New docs and examples should use `radiant::prelude`, `radiant::window`,
`radiant::app`, and the application view builders instead. This is a
documentation and guardrail deprecation, not a Rust `#[deprecated]` attribute on
the explicit control objects. The `radiant::runtime` module, `RuntimeBridge`,
`UiSurface`, `SurfaceNode`, `SurfaceChild`, `NativeRunOptions`, `WidgetSizing`,
and native runtime entry points remain supported as low-level adapter
infrastructure for unusual embedding and runtime tests, not as the ordinary
path for feature-complete applications. They remain supported and non-deprecated for hosts that need precise runtime, layout, or bridge control.

## App

An application is host-owned state plus a projection function and message handler.
Radiant does not define the domain model. The public `App<Message>` contract is
implemented by every `RuntimeBridge<Message>`: hosts can provide a custom bridge
or use `declarative_runtime_bridge(state, project, reduce)` to project an
immutable `UiSurface<Message>` from state and reduce messages back into state.
Apps that need runtime-visible follow-up work should use
`radiant::app(...).handle_message(...)` with `UiUpdateContext`. Ordinary app
messages automatically request surface repaint unless the handler requests an
explicit surface or paint-only repaint. `RepaintPolicy` lets app-builder code
override that ordinary-message default outside the handler, while frame-clock
messages use
`FrameClock::repaint_scope(...)` for paint-only frame optimization. The older
command-returning and alternate-name update hooks are intentionally removed from
the normal app builder path. The app builder lowers into
Radiant's bridge internally while keeping side effects and domain state
host-owned. Low-level hosts can still provide a custom bridge or use
`declarative_command_runtime_bridge(state, project, update)` when embedding
Radiant outside the application builder.

`radiant::app(state).view_with_context(...)` is an additive opt-in projection
for the main window. Its closure receives `(&State, &WindowEnvironment)` while
the existing `.view(|state| ...)` closure remains unchanged. The snapshot is
backend-neutral and immutable: it contains the effective `DpiScale`, an
optional `WindowColorScheme`, contrast preference, and reduced-motion
preference. Unknown platform values use `None` or `false`; the default scale is
`DpiScale::ONE`. `UiUpdateContext::window_environment()` and
`RuntimeContext::window_environment()` expose the same per-window snapshot.
Auxiliary windows update their runtime snapshot but do not run a separate
high-level application projection.

Application presentation inputs are carried by the additive
`ApplicationEnvironment` snapshot. Construct a validated `LocaleId` and
`TextScale`, provide an explicit ordered fallback chain and immutable
`TextCatalog`, then attach the snapshot with
`ViewProjection::with_application_environment(...)` or
`UiSurface::with_application_environment(...)`. `TextKey` resolution returns
`LocalizedText`, whose exact, explicit-fallback, source-fallback, and missing
outcomes are deterministic; `LocalizedText::to_content()` lets visible and
accessibility values share the same `TextContent` bytes. Shortcut matching
continues to use `ShortcutGesture`; `ShortcutDisplaySpec` requires a caller
supplied semantic character or named-key label and `ShortcutPresenter` emits
compact and spoken forms without deriving a legend from a physical key code.
Stateful apps that keep this snapshot in state can add
`.application_environment(|state| snapshot)`. The runtime samples that cheap
source before selecting a repaint scope, so an unchanged paint-only request
does not reproject the app while a changed locale or catalog promotes the
request to surface work. Custom `RuntimeBridge` hosts may implement the same
optional snapshot hook directly.

Widget paint receives the same snapshot through the cloneable combined
`ResolvedEnvironment` projection. `WidgetPaintContext` exposes the assigned
logical bounds, layout, theme, and environment to additive
`Widget::append_paint_with_context(...)` and
`Widget::append_runtime_overlay_paint_with_context(...)` hooks. Their defaults
delegate once to the existing required paint hooks, so legacy widgets and
object-safe trait callers keep the same behavior. The projection is lossless:
it carries window display scale, optional color scheme, contrast, reduced-motion
preference, and the immutable application snapshot without choosing theme or
animation policy. Widget paint borrows the projection, while durable runtime
witnesses clone it.

Current shipped boundary: the native environment exposes display scale, color
scheme, contrast, and reduced-motion preference, and Unicode-scalar editing is
shipped. The additive `ApplicationEnvironment` snapshot carries explicit
locale fallback, direction, text scale, catalog generation, and shortcut
presentation generation. Phase-1 logical RTL container geometry is shipped;
TextWidget, TextInputWidget, ButtonWidget, BadgeWidget, ToggleWidget,
SelectableWidget, ListItemWidget, and NumericInput resolve intrinsic metrics and
paint font from text scale. Dense-row helpers expose additive environment-aware
label/chrome entry points while existing bounds-derived helpers remain
legacy-compatible. Embedded interactive rows and TreeRow resolve one immutable
declared text-metrics witness; row, guide, expander, icon, hit, and capture
geometry remains physical, and TreeRow semantics preserve host/runtime
ownership. Native paragraph shaping receives the requested locale and explicit
writing direction from the published application snapshot. Both values qualify
shape/view cache reuse; a change retires native input geometry before reseeding
the current plan. Shell geometry remains staged work under OPT-1386.
Retained bidi and complex shaping are implemented under OPT-1402.

Framework-owned menus resolve title and command-row intrinsic heights from the
application text scale. Each command owns its visible label, shortcut hint,
accessible name, and input state under one widget identity; RTL mirrors the
label and shortcut columns. Automatic context-menu width scales its character
estimate within the declared physical limits, and automatic height scales text
rows while retaining physical padding and gaps. Explicit `.width(...)` and
`.size(...)` constraints remain physical. The `localization_foundation` example
cycles English, French, and Arabic with a larger text scale and an RTL menu.

Appearance selection is a separate, backend-neutral policy. `AppearancePolicy::FollowEnvironment`
resolves light, dark, and high-contrast tokens from the current window snapshot;
an unknown color scheme conservatively selects the dark palette while the
lossless environment remains `None`. `AppearancePolicy::Fixed(theme)` preserves
explicit `ThemeTokens` byte-for-byte and ignores system appearance, scale, and
motion preferences. `ResolvedAppearance` is immutable and `Copy`; its
`tokens()` accessor is available through `WidgetPaintContext::appearance()`.
Native system-follow rendering resolves one snapshot per paint pass so clear,
base, clipped, and runtime-overlay primitives cannot diverge. Reduced motion
remains an independent environment policy and does not select a palette.

`RuntimeBridge` is the minimal projection and update contract for custom hosts.
Optional host behavior is declared through `RuntimeHostCapabilities` and focused
traits for input policy, task scheduling, platform services, runtime queues,
animation, windows, retained surfaces, transient overlays, diagnostics, and
lifecycle. `RuntimeHostCapabilities` is cached once when `SurfaceRuntime` is
created, so availability remains stable and absent capabilities add no dynamic
lookup or allocation to frame paths. Most applications should still reach these
responsibilities through `radiant::app(...)`; the capability traits extend the
same runtime model instead of creating a second custom-host framework.

A minimal custom host only projects a surface and handles messages:

```rust
use radiant::runtime::{Command, RuntimeBridge, UiSurface};
use std::sync::Arc;

struct MinimalHost {
    surface: Arc<UiSurface<()>>,
}

impl RuntimeBridge<()> for MinimalHost {
    fn project_surface(&mut self) -> Arc<UiSurface<()>> {
        Arc::clone(&self.surface)
    }

    fn update(&mut self, _message: ()) -> Command<()> {
        Command::none()
    }
}
```

An advanced capability host implements and registers only the hooks it owns:

```rust
use radiant::runtime::{
    NativeFrameDiagnostics, PaintPrimitive, RuntimeBridge,
    RuntimeFrameDiagnosticsHost, RuntimeHostCapabilities,
    RuntimeTransientOverlayHost, TransientOverlayContext,
};

impl RuntimeTransientOverlayHost for AdvancedHost {
    fn paint_transient_overlay(
        &mut self,
        context: TransientOverlayContext<'_>,
        primitives: &mut Vec<PaintPrimitive>,
    ) {
        self.paint_overlay(context, primitives);
    }
}

impl RuntimeFrameDiagnosticsHost for AdvancedHost {
    fn observe_frame_diagnostics(&mut self, diagnostics: NativeFrameDiagnostics) {
        self.record_frame(diagnostics);
    }
}

// Inside `impl RuntimeBridge<Message> for AdvancedHost`:
// fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, Message> {
//     RuntimeHostCapabilities::new()
//         .with_transient_overlays()
//         .with_frame_diagnostics()
// }
```
Stateful embedding tests and custom hosts that do need a `SurfaceRuntime` can
skip the intermediate bridge variable with `SurfaceRuntime::new_declarative(...)`
or `SurfaceRuntime::new_declarative_owned(...)`, depending on whether the view
projector returns a shared `Arc<UiSurface<_>>` or a fresh owned `UiSurface<_>`.
Use `DeclarativeSurfaceRuntime<State, Message, Project, Reduce>` or
`DeclarativeOwnedSurfaceRuntime<State, Message, Project, Reduce>` when a test
fixture, helper, or host adapter needs to name one of those common runtime
controller shapes without spelling the full bridge stack.

## View, Element, And Widget

`View<Message>` is the root declarative view snapshot and is a public alias for
`UiSurface<Message>`. `Element<Message>` is the generic element tree and is a
public alias for `SurfaceNode<Message>`: container nodes hold `SurfaceChild`
entries and widget nodes hold object-safe `Widget` leaves. Built-in primitives
and user-authored leaves implement the same `Widget` trait, so the runtime does
not maintain a closed widget catalog. Widget primitives such as `ButtonWidget`,
`BadgeWidget`, `TextWidget`, `TextInputWidget`, `ToggleWidget`,
`ScrollbarWidget`, `SelectableWidget`, `CardWidget`, `ImageWidget`,
`CanvasWidget`, and `ListItemWidget` describe reusable UI behavior without
host-domain semantics.

Explicit widget construction should prefer named parts when multiple public
fields define identity, content, state, or sizing. Each built-in primitive
exports a `*WidgetParts` type, such as `ButtonWidgetParts`, `TextWidgetParts`,
`SliderWidgetParts`, `CanvasWidgetParts`, and `ImageWidgetParts`, plus
`WidgetSizingParts` for explicit sizing. The positional `new(...)` constructors
remain as compact compatibility helpers, but `from_parts(...)` is the clearest
shape for examples, tests, and host code where several semantic values appear
next to each other. This keeps the explicit widget API aligned with the
declarative builder model: stable IDs, content, state, and layout contracts are
named at the construction boundary instead of being inferred from argument
order.
Named-parts types and `_from_parts` entry points are imported from their owning
module; they remain public advanced construction contracts without expanding
the common wildcard surface.
Named `Parts` types are not required for every small value object. Keep them
public when they prevent long positional argument lists, carry optional or
defaultable configuration, encode semantic distinctions that raw booleans or
numbers would obscure, or reserve a forward-compatible construction contract.
Prefer direct constructors for compact metric/value types whose fields are few
and already have clear domain names, such as `FlowLayoutMetrics::new(...)`.
Application-level compound controls follow the same rule: dropdown option parts
use `DropdownOptionSelection` instead of a raw selected flag, so public examples
can distinguish current-value state from the host message routed on activation.
Use `DropdownOption::from_selection(...)` when constructing options from
computed state, and `DropdownOption::selected(...)` or
`DropdownOption::unselected(...)` when the option state is static at the call
site. Use `DropdownOption::for_value(...)` or
`DropdownOption::for_optional_value(...)` when a dynamic option list is built
from concrete values and the host message should carry the selected value; the
helpers compare against the current selection before mapping the value into a
message. The older `DropdownOption::new(label, selected, message)` constructor
remains available for compatibility, but new code should prefer the named
selection or value constructors when readability would otherwise depend on a
positional boolean.

Single-line text editing is split between reusable state and widget routing:
`TextInputState` owns the portable value, caret, and selection model, while
`TextInputWidget` adapts that model to `WidgetInput` and emits
`TextInputMessage`. Custom retained surfaces that draw their own field chrome can
use `TextInputState::apply_edit_command`, `apply_key`, `insert_text`, and
`set_caret` directly instead of reimplementing paste sanitization, selection
replacement, Unicode-scalar caret movement, word-boundary navigation, and
character-limit behavior. Native text inputs route Ctrl/Cmd+Left and
Ctrl/Cmd+Right through the same backend-neutral `TextEditCommand` path, with
Shift extending the current selection by word. Ctrl/Cmd+Backspace and
Ctrl/Cmd+Delete also use backend-neutral word-delete commands, deleting the
active selection first when one exists. For
host-rendered editors, `has_selection`, `clear_selection`, `select_word_at`,
`replace_selection`, `delete_selection`, and the borrowed `selected_text_slice`
expose the same reusable single-line replacement semantics without requiring a full
`TextInputWidget` or allocating just to inspect the active UTF-8 selection.

Text-aware `TextWidget`, `TextInputWidget`, `ButtonWidget`, `BadgeWidget`,
`ToggleWidget`, `SelectableWidget`, and `ListItemWidget` declarations resolve their
intrinsic sizing, font size, baseline, and insets once against the attached
`ApplicationEnvironment::text_scale()`. Custom widgets preserve the legacy
unscaled contract by default and may opt into the same resolver through
`TextScaleParticipation` and `WidgetPaintContext::resolved_environment()`.
`TextAlign::Start` and `TextAlign::End` are logical declarations; `Left`,
`Center`, and `Right` remain physical renderer alignment values. Participating
text intrinsic metrics resolve declared values once with `text_scale`; explicit
parent `.width(...)` and `.height(...)` slots remain physical constraints, and
DPI conversion occurs once when the native scene is projected.
Widgets that participate in focused text editing can also expose borrowed
selection text through `Widget::selected_text_slice`, and
`SurfaceRuntime::focused_text_selection_slice` keeps runtime-level focus
inspection on the same allocation-free path. The owned
`focused_text_selection` helper remains available for callers that need to keep
the selection after releasing the runtime borrow.

Advanced text input capabilities are intentionally staged behind this
single-line contract. Multiline editing should not be added by teaching
`TextInputWidget` ad hoc newline behavior; it should be a generic text-area
capability with layout-aware vertical navigation, line metrics, wrapping policy,
and cursor-stop mapping shared with renderer text layout. Undo and redo should
be widget-local edit history for text mutations and selection groups, separate
from application undo stacks; hosts may mirror submitted values into their own
history, but Radiant text editing should not assume a host undo model. Password
or secret entry should be a first-class masked text-input mode, not only a paint
hack: display and automation value text should be masked, copying selected text
should be disabled by default unless the mode explicitly allows it, and tests
should prove selection/caret behavior still operates on the underlying logical
value. Native IME composition belongs at the platform adapter boundary, which
should translate platform preedit/commit/cancel events into backend-neutral
composition state and final text commits; the widget model should own the
logical composition range once that generic event exists. Unicode-scalar editing
is shipped. The additive `ApplicationEnvironment` snapshot provides explicit
locale fallback, direction, text scale, catalog generation, and shortcut
presentation generation. Phase-1 logical RTL container geometry is shipped;
built-in dense/interactive row and TreeRow text-scale propagation is shipped,
and native shaping/cache identity consumes the application locale and direction.
Shell geometry remains staged under OPT-1386. Bidirectional text and complex shaping belong to renderer text
layout and cursor-stop mapping; their retained implementation is under
OPT-1402, while `TextInputState` continues to store logical Unicode-scalar
positions instead of renderer glyph positions. The selected architecture is
recorded in [`TEXT_SHAPING_ARCHITECTURE.md`](TEXT_SHAPING_ARCHITECTURE.md).

Implement `Widget` directly when a downstream application needs a new focusable
leaf with its own input handling, host-routable output payload, or
backend-neutral paint contribution. Compose existing primitives when the desired
control is only a row, column, stack, styling change, message mapper, or
combination of built-in widgets. `SurfaceNode::widget` and
`SurfaceNode::static_widget` accept any owned widget implementation, including
Radiant's built-in primitives, without requiring an enum wrapper.

Widget values and their optional `WidgetSemantics` are owned by the window/UI
runtime. They may contain `Rc`, `RefCell`, or other UI-local state and do not
need to be `Send` or `Sync`; consequently `Box<dyn Widget>` does not imply
thread-safe transfer. `EmbeddedInteractiveRowWidget` follows the same local
ownership rule for the custom row value and its associated `Message`. A
`WidgetOutput` is a synchronous UI-local erased envelope: its typed payload may
contain `Rc`/`RefCell` and is routed, cloned, and downcast only on the owning UI
runtime. Keep worker requests, results, and platform/subscription transfer
payloads explicitly `Send` (and `Sync` where required); their completion/output
mappers run later on the owning UI thread and may produce ordinary UI-local
messages.

The declarative owner records are UI-affine too: `ViewNode`,
`WidgetViewContext`, `UiSurface`, `SurfaceNode`, installed `SurfaceWidget`
values, and `SurfaceRuntime` carry private zero-sized affinity evidence and
cannot be moved into `std::thread::spawn`. This does not add fields to public
literal-constructed records such as `WidgetCommon`, change mapper storage, or
require `Message` or mapper types to implement `Send`; worker/effect boundaries
retain their explicit transfer bounds.

The application builder uses the same ownership model through `WidgetView`.
Any `Widget + Clone + 'static` is a non-emitting `WidgetView`, so it can be
placed directly with prelude `widget(my_widget)`. Interactive application
widgets can use `custom_widget_direct(widget)` when the widget's typed output is
already the host message, `custom_widget_mapped(widget, |payload| message)` for
typed custom outputs that need conversion, or
`MappedWidget::new(widget, WidgetMessageMapper::...)` when they need an
explicit mapper object. For fully dynamic custom output,
`DynamicWidget` and the compatibility `custom_widget(...)` helper wrap a boxed
`Widget` plus a `WidgetOutput` mapper. This keeps widget variation in widget
implementations and their mapper adapters instead of a central application enum
or built-in widget list.

Common declarative composition should use the generic `SurfaceNode::widget`,
`SurfaceNode::static_widget`, `SurfaceNode::row`, `SurfaceNode::column`,
`SurfaceNode::grid`, and `SurfaceChild::fill` path when a host only needs
ordered structure, fill slots, and widget leaves. Built-in primitive modules may
provide convenience constructors on `SurfaceNode`, but those helpers are owned by
the primitive modules rather than the runtime surface core. Adding a widget
should mean adding that widget module and optional helpers, not editing a central
runtime widget catalog.
The low-level named parts used to assemble `SurfaceChild` and surface
containers stay internal to `radiant::runtime`; host code should use
`SurfaceChild::new`, `SurfaceChild::fill`, and `SurfaceNode::container` for
explicit runtime surface composition.
Built-in widgets should keep widget-specific model, input, and paint behavior in
or directly under their owning primitive module. Shared support modules are for
reusable contracts such as common widget state, activation helpers, shared
chrome, and theme token resolution, not for hiding a widget's primary behavior
away from the widget implementation.
`SurfaceNode::custom_widget` and the prelude `custom_widget(...)` builder accept
owned `Widget` implementations. The application builder assigns generated,
keyed, or explicit IDs by updating the widget's `WidgetCommon` before lowering,
so custom widgets participate in the same focus, hit-test, sizing, and paint
paths as built-ins.
`SurfaceNode::stack` overlays children in slot order so hosts can compose a card
background with nested rows, columns, labels, and controls. Lower-level
`SurfaceNode::container` plus `ContainerPolicy` and `SlotParams` remains
available for explicit runtime composition, while `SurfaceNode::layout` offers
the same bounded custom measure/place policy boundary at that layer.
`SurfaceNode::scroll_area` wraps one content child in a generic scroll viewport;
`SurfaceNode::virtual_scroll_area` adds a `VirtualizationPolicy` for large
linear lists without tying the framework API to any host content-list model.

Widget identity is explicit through stable `WidgetId` values. Stable identity is
required for focus, input capture, message routing, and efficient updates.
The application view builders generate IDs as a convenience, then lower into these
same `SurfaceNode`, `SurfaceChild`, and `WidgetSizing` contracts.
When a host update reprojects the surface, the runtime matches widgets by stable
ID and calls `Widget::synchronize_from_previous(...)`. Built-in widgets use that
hook for transient interaction state such as text-input caret/selection and
scrollbar drag grip state, and custom widgets can use the same hook for their
own retained state without adding runtime downcasts or central widget cases.
Before that synchronization can transfer retained state, the runtime gives each
installed stateful widget one additive `Widget::prepare_replacement(...)` seam.
It passes an exact proposed compatible successor when the identity, path,
revision, and compatibility evidence is unambiguous; removal, identity loss,
incompatible replacement, or ambiguous evidence passes `None`. The retiring
widget owns its local teardown and may return a UI-local `WidgetOutput`. Radiant
maps that output through the retiring `SurfaceWidget` mapper, collects outputs in
the previous widget order before discarding the old surface, installs the new
surface, and only then reduces the bounded batch through the existing deferred
command path. The successor is borrowed only for the call and is never retained.
The default hook is a no-op, so existing custom `Widget` implementations remain
source-compatible; compatible unchanged successors continue through ordinary
`synchronize_from_previous(...)` with no terminal output.
Retained synchronization is compatibility-aware: the additive default
`Widget::compatibility_kind()` descriptor is derived from the concrete custom
widget type, so an ID reused by a different widget kind is treated as a safe
replacement rather than receiving cross-type state. Replacement refreshes clear
controller-owned focus, pointer capture, and hover ownership before restoration
and expose bounded `SurfaceIdentityReplacement` entries through
`SurfaceRefreshDiagnostics`. Existing custom widgets remain source-compatible;
same-kind keyed reorders keep the normal retained-state behavior.
An exact interaction update may also admit a changed stateful leaf when the
request is a projection-only update, the old and new leaves both advertise
`supports_prepared_state_synchronization()`, and the complete identity,
revision, capability, membership, path, source, and interaction-ownership
witnesses remain exact. The successor synchronizes its retained state in the
detached candidate before publication; selected old leaves then receive the
same replacement hook and mapper ordering as a full refresh. Any unsupported,
ambiguous, conservative, or broader-scope evidence takes the single-pull full
refresh path, while a candidate callback panic or later authority loss drops
the inert candidate without replaying projection or synchronization.
Custom widgets also expose an additive `Widget::revision()` hook. Its
`WidgetRevision::conservative()` default is correct for existing custom widgets
and for any widget that cannot prove exact immutable-input changes. A widget
that can prove those changes may call
`WidgetRevision::exact(structure, geometry, paint, interaction)` with four
independently typed `Eq + 'static` values. Values are compared by typed
equality; they are not hashes or caller-provided integer fingerprints, and a
component type mismatch widens to that component's safe effect. Exact revisions are UI-local
clonable values rather than `Copy` values because they retain arbitrary
component ownership. This foundation hook still does not enable refresh or
repaint optimization, and widgets should exclude stable identity and mutable
runtime state from their immutable evidence. Advanced hosts should keep using
the conservative default when any affected input is unavailable or ambiguous.
The source-compatible v1 `WidgetCapabilities` descriptor is deliberately
semantics-only: its public shape remains the two fields `contract_version` and
`semantics`, and `WIDGET_CAPABILITIES_CONTRACT_VERSION` remains `1`. Historical
two-field struct literals therefore continue to compile. The additive
`WidgetCapabilitiesV2` descriptor has private fields and contract version `2`;
its borrowed, `Copy`, object-safe builders and accessors cover `WidgetSemantics`,
`WidgetHitTest`, and `WidgetPointerMotion`. `Widget::capabilities()` remains the
v1 seam, while `Widget::capabilities_v2()` is the defaulted v2 seam.

The optional `WidgetSemantics` capability has the same conservative posture:
its default `WidgetSemantics::revision()` returns
`WidgetSemanticsRevision::conservative()`. A custom capability may return
`WidgetSemanticsRevision::exact(value)` with one UI-local `Eq + 'static` value;
`WidgetCapabilities::semantics_revision()` exposes that evidence without
evaluating role, label, value, or metadata output methods. Supported v2
semantics and its advertised automation actions take precedence over valid v1
semantics; absent or unsupported v2 falls back to v1 and then neutral defaults.
Unknown v1 contract versions do not enable semantics or actions. Capability
presence changes, unsupported descriptor contract versions, and conservative
evidence take the structural/full-scene fallback. An optional v2 capability
absent on both sides is unchanged, preserving legacy fallback without
spurious structural invalidation. Changed or type-mismatched exact semantic
evidence is an interaction change, while equal exact evidence is unchanged.
Existing custom `WidgetSemantics` and `Widget` implementations remain
source-compatible through the default hooks.
Hosts that want deterministic test failures can configure
`SurfaceRuntime::set_identity_audit(IdentityAudit::strict())`. The default
`IdentityAudit` policy is observational: every replacement completes cleanup and
commits `last_refresh_diagnostics()` plus the pending frame aggregate before
returning. Strict mode performs the same work, then uses a deliberate
`panic_any` failure for a non-paint refresh with one or more replacements. The
failure message reports the total replacement count, the bounded records in
paint order, omitted records, and truncated paths without formatting concrete
widget type names. `IdentityAudit` is available from `radiant::runtime`, not the
common prelude.
Pointer-driven custom widgets should keep transient hover and cursor state local
when the state is only paint chrome. Export a `WidgetPointerMotion` capability
for widgets such as timelines, canvases, and editors that need stable pointer
moves after hover has already entered the widget. Its
`WidgetPointerMotion::accepts_pointer_move()` decision controls whether those
stable `PointerMove` events are admitted. Admitted events request repaint even when `handle_input` returns `None`, so a snapped cursor, clip hover, or
resize-handle preview can refresh smoothly without emitting host messages or
forcing the app reducer to run for every mouse move. Captured drag motion
follows the same contract: if the active widget only changes local preview
chrome, it can repaint locally without a host message. Emit a `WidgetOutput`
only when the host-owned model changes, such as seek, create, move, resize, or
delete.
The supported v2 descriptor takes precedence over the restored legacy
`Widget::accepts_pointer_move()` hook; an absent or unsupported v2 descriptor
uses that legacy hook and its historical default. The descriptor remains a
read-only, UI-local observation called after culling: it cannot own capture,
focus, scheduling, renderer, or application authority.
Use `WidgetPointerMotion::pointer_capture_policy()` for widgets that need to control pointer
motion while they own capture. `PointerCapturePolicy::Exclusive` is for
splitters, resize handles, and similar controls that should not activate hover
or pointer-motion behavior on unrelated widgets before release. During retained
surface refreshes, exclusive capture also clears copied hover state from
non-captured widgets while preserving durable widget state. The default
`PointerCapturePolicy::PassThrough` keeps drag-source behavior where widgets
under the pointer can still receive live feedback while the source remains
captured.
Native focus loss and external drag handoff cancel pointer capture without
routing a synthetic release to the host. Radiant clears the captured widget's
transient retained state through the widget input path and requests repaint
only when that local state changed. Slider and the official retained Knob
adapter opt into the capture-cancellation hook to deliver their typed `Cancel`
batch before cleanup. The bare public `KnobWidget` and legacy Knob mappings
retain their compatibility behavior: focus loss emits the legacy terminal
message, while pointer-capture cancellation remains suppressed. Hosts should model
durable drag/drop results as messages, but they should not duplicate generic
pressed, capture, or focus-loss cleanup in application reducers.
Custom widgets must still be pointer hit-test eligible before pointer hooks can
run. Export `WidgetHitTest` when a widget needs event-aware opaque or
pass-through classification or a cursor choice; `WidgetHitTest::hit_test(...)`
is called after bounds and clip culling, and it cannot reorder traversal or own
capture. `WidgetHitTest::Opaque` selects the front-most admitted target, while
`WidgetHitTest::PassThrough` continues front-to-back traversal. A cursor applies
only to an admitted opaque target or capture owner. An absent or unsupported v2
hit-test descriptor falls back to the legacy `Widget::accepts_pointer_input()`
hook and the historical opaque rectangular default. Use
`WidgetCommon::with_pointer_focus()` for hover, drag, tooltip,
cursor, or paint-only overlay widgets that should skip keyboard traversal, or
`WidgetCommon::with_keyboard_focus()` when the same custom surface also handles
keyboard input.
High-frequency editor widgets can go further with
`WidgetPointerMotion::prefers_pointer_move_paint_only()` and
`Widget::append_runtime_overlay_paint(...)`. Pair that preference with
`WidgetPointerMotion::pointer_move_overlay_is_valid()` only when the overlay
callback fully represents the transient state. Put pointer-following visuals such
as timeline cursor lines, hover outlines, captured drag previews, and resize
handles in the runtime overlay hook, then keep the stable base widget paint free
of those transient states. The native Vello runtime can then present those
overlay rectangles over the cached scene on stable pointer motion and captured
paint-only drag motion instead of rebuilding the Vello scene for every
mouse-move event. Widgets that paint pointer-motion state in `append_paint(...)`
or cannot prove a valid overlay should not opt into the paint-only pointer path.
The v2 paint-only path requires both the preference and valid overlay evidence;
the restored `Widget::prefers_pointer_move_paint_only()` hook is used only when
v2 pointer-motion evidence is absent or unsupported. The runtime falls back to
full-scene repaint when the descriptor is absent,
unsupported, or ambiguous. Semantics-only v1 descriptors and unknown capability
versions advertise no optional behavior.

## Message And Runtime Follow-Up

Radiant routes widget outputs into host-defined `Message` values through
`WidgetMessageMapper`. `SurfaceRuntime` dispatches input, emits mapped messages,
calls the host update hook, executes returned commands, and requests a fresh
surface snapshot. `RuntimeBridge::reduce_message` remains the simplest reducer
hook for hosts that only mutate state; `RuntimeBridge::update` can return
`Command<Message>` for hosts that need runtime-visible follow-up work.
`SurfaceRuntime::dispatch_message` and `SurfaceRuntime::execute_command` both
return `CommandOutcome` with dispatched-message and repaint-request summaries.
`Command<Message>` is the runtime-visible follow-up value used by Radiant
internals, explicit runtime bridges, tests, and advanced embedders. Normal
applications should not use it as a general side-effect or worker escape hatch;
they should use `UiUpdateContext` capabilities, typed platform services, and
`context.business()` from `.handle_message(...)`. Hosts that inspect only the
immediate messages in a command can use
`Command::into_messages_into(...)` to reuse caller-owned storage, while
`Command::into_messages()` remains the allocating convenience wrapper.
The qualified `radiant::runtime::Effect<Message>` facade from OPT-1387 is the additive
construction surface for `Effect::after(...)`, `Effect::worker(...)`,
`Effect::ordered_stream(...)`, `Effect::latest_stream(...)`, and
`Effect::platform(...)`. Each constructor
requires `&mut LatestTask` and an explicit `EffectOwner::Application` or
`EffectOwner::Declarative(...)`, reserves a `TaskTicket`, and exposes that ticket
plus a cloned per-effect `CancellationToken`. Declarative owner selection resolves
only against the accepted projection; absent or ambiguous owners reject atomically,
restore the predecessor latest ticket, and never fall back to `Application`.
Worker code transports only owned `Send` output or event values; `Message` and all
typed `TaskCompletion` mappers remain UI-local and need not be `Send` or `Sync`.
Convert an effect with `Command::effect(effect)` or `From<Effect<Message>>`; both
bridges preserve the existing separate timer and worker lanes and the controller's
common private lifecycle policy. Platform effects use the existing platform
registry/controller ingress and later-turn `PlatformResult` delivery; they do
not add a scheduler or queue. The facade does not migrate `ResourceTasks`,
subscriptions, scheduler/queue/thread ownership, or product state, and it does
not replace the legacy business/latest/keyed-latest APIs. External drag remains
on its separate compatibility lane.
Tests and diagnostics can use `Command::business_task_priority(...)` to verify
that a named one-shot or streaming worker effect was queued on the expected
runtime worker lane without pattern-matching hidden command internals. Worker
closures transport only owned `Send` payloads; their application-message
mappers remain registered on the UI owner.
`RepaintScope` is the typed repaint specificity contract: `Surface` requests a
surface refresh plus repaint, while `PaintOnly` repaints the current paint plan
for overlay-only motion. Reducers can queue `Command::repaint(scope)` or
`UiUpdateContext::repaint(scope)`, and diagnostics can inspect
`Command::repaint_scope()` to see the merged effective scope for nested command
batches. Mixed batches promote to `Surface` so a paint-only overlay request
cannot accidentally suppress a needed surface refresh.
`ResourceSlot<T>`, `ResourceRequest`, `ResourceLoad<T>`, `ResourceCompletion<T>`, and
`ResourceLoadState` provide a small runtime-level state contract for host-owned
background resource work. Radiant does not own the filesystem or asset decoder,
but examples and apps can use the same key/state/result shape for loading
images, previews, manifests, fonts, or other resources through
`context.business().background(...).resource(&mut slot).run(...)`. Use
`ResourceSlot::begin_load()` and `ResourceSlot::apply_for(...)` when repeated
loads for the same key can overlap; stale worker completions are ignored instead
of replacing the current result. `ResourceRequest::ready(...)` and
`ResourceRequest::failed(...)` construct keyed results from the request token so
worker code does not need to clone or duplicate resource-key text manually.
The business resource builder performs that request/result wiring for fallible
resource loads and returns a `ResourceCompletion<T>` through the normal message
path.
Use `ResourceSlot::cancel_load()` to invalidate in-flight work while preserving
the last ready value; use `ResourceSlot::clear()` when the value and error
should be dropped.

Any widget can emit its own output type with `WidgetOutput::typed(...)` and
route it with `WidgetMessageMapper::typed(...)`. Built-in primitive modules may
provide typed convenience mappers such as `WidgetMessageMapper::button`, but
those mappers are also owned by the primitive module rather than the runtime
surface core.
Constant-message controls, menus, overlays, lists, and tree composition keep
their application messages on the UI owner and therefore require only
`Message: Clone + 'static`; those messages may contain `Rc`, `RefCell`, or other
UI-local state. Typed widget output payloads follow the same UI-local ownership
model: `WidgetOutput` is a synchronous routing envelope, not a transferable
worker boundary. Composite tree rows emit `InteractiveRowMessage` through that
same local envelope and apply their application-action mapper afterward on the
UI owner.
`WidgetOutput::custom(...)` remains an alias for user-defined widget payloads,
and `WidgetOutput::typed_cloned::<T>()`, `typed_copied::<T>()`,
`custom_cloned::<T>()`, and `custom_copied::<T>()` provide owned payload
extraction for tests, automation, and custom-widget adapters without repeating
manual downcast chains. `WidgetMessageMapper::dynamic(...)` is available when a
host needs manual downcast or filtering behavior. Adding a widget should not
require adding a central output enum variant.

Advanced UI-local event paths may opt into `runtime::EventMapper::with_revision`
when an `Eq + 'static` value exactly describes a mapper's captured behavior.
`EventMapper::new` remains the conservative default. Exact mappers can be passed
to `WidgetMessageMapper::dynamic_mapped`, or the corresponding
`SurfaceNode`/`SurfaceContainer` scroll entry points. The atomic
`SurfaceNode::with_native_file_drop_mapped` entry point combines exact mapper
evidence with native-drop target acceptance; existing closure aliases and
builders retain conservative behavior. For the smallest interactive controls,
`WidgetMessageMapper::button_mapped`/`toggle_mapped` and the builder
`button(...).mapped_with(...)`/`toggle(...).message_with(...)` adapters preserve
the typed evidence while adapting the callback to `WidgetOutput`. Reconciliation
compares only the explicit `Eq` evidence; it never inspects or invokes the
callback, so callers must assert that the evidence covers every captured
behavior that can affect the mapped message.

Asynchronous business work remains host-owned, but normal apps use Radiant's
app runtime to wire it into the UI. `UiUpdateContext::business()`,
`UiUpdateContext::after(...)`, typed platform-service helpers, and
`Subscription` provide message delivery and repaint wakeups; the app still owns
the work and resulting domain messages.

### Declarative effect-owner boundary

Bounded timer, one-shot business-worker, ordinary ordered and coalesced owner-scoped stream consumers, cancellable ordinary ordered and coalesced owner-scoped stream consumers, ordered/coalesced latest-task owner streams, cancellable latest-task one-shot and ordered/coalesced owner streams, and the capability-qualified application-owned `KeyedLatestTasks` one-shot, ordered-stream, and coalesced-stream routes are now public. They expose a qualified opaque
`DeclarativeEffectOwner`, explicit `ViewNode::effect_owner` and
`Layer::effect_owner` markers, and
`UiUpdateContext::after_for_owner(...)` /
`UiUpdateContext::after_latest_for_owner(...)`, plus
`BusinessRequest::run_for_owner_with_receipt(...)` and
`CancellableBusinessRequest::run_for_owner_with_receipt(...)` /
`BusinessLatestRequest::run_for_owner_with_receipt(...)` /
`BusinessRequest::stream_for_owner_with_receipt(...)` /
`BusinessRequest::stream_latest_for_owner_with_receipt(...)` /
`CancellableBusinessRequest::stream_for_owner_with_receipt(...)` /
`CancellableBusinessRequest::stream_latest_for_owner_with_receipt(...)` /
`CancellableBusinessLatestRequest::run_for_owner_with_receipt(...)` /
`CancellableBusinessLatestRequest::stream_for_owner_with_receipt(...)` /
`CancellableBusinessLatestRequest::stream_latest_for_owner_with_receipt(...)` /
`BusinessLatestRequest::stream_for_owner_with_receipt(...)` /
`BusinessLatestRequest::stream_latest_for_owner_with_receipt(...)` /
`BusinessRequest::latest_for(...).run_for_owner_with_receipt(...)` /
`BusinessRequest::latest_for(...).stream_for_owner_with_receipt(...)` /
`BusinessRequest::latest_for(...).stream_latest_for_owner_with_receipt(...)`. Markers are eligible
only for durable keyed nodes or overlays; ownership is never inferred from
traversal or visibility. Ordinary timers and business requests remain
application-owned. Owner admission refreshes the accepted surface and rejects
absent, ambiguous, ineligible, stale, retired, or incompatible handles with no
fallback. Late owner wakes and worker completions are fenced before mapping.
No general effect ownership, semantic demand/refresh/provider budget, scheduler,
custom-coordinate, platform, or product wiring API is promised.

Current shipped ownership is narrower than the target model: the private
`EffectOrigin` boundary supplies application, auxiliary, and selected
declarative provenance, while `ResourceTasks` remains application-owned.
The qualified `runtime::Effect<Message>` facade is now shipped by OPT-1370 for explicit
`EffectOwner::Application` or `EffectOwner::Declarative(...)` selection across
after/worker/ordered-stream/latest-stream construction. Its constructors require
`&mut LatestTask`, reserve a `TaskTicket`, expose a cloned `CancellationToken`,
and lower into the existing command lanes; the private registry and owner ledger
remain hidden. Invalid or ambiguous declarative selection rejects atomically
without application fallback and restores the predecessor ticket. The existing
owner-scoped business, latest-task, keyed-latest, and timer routes keep their
public handles, cancellation, admission, rollback, and stale/late protections.
`runtime/effects` is not complete. The remaining effect-ownership boundaries
are future work tracked by OPT-1390 and OPT-1421; subscriptions, ResourceTasks
ownership, scheduler policy, and product wiring remain outside this slice.

The broader target contract is described in [the normative declarative effect-ownership design](DESIGN_DIRECTION.md#declarative-effect-ownership-and-cancellation). These shipped owner-scoped consumers select only one exact keyed/overlay candidate by explicit handle; candidates have no implicit precedence, and an invalid selection is rejected without fallback. Legacy ordinary timers and business work remain application-owned unless a facade or existing owner-scoped API explicitly selects a declarative owner. Ordinary ordered and coalesced owner-scoped streaming, the cancellable ordinary ordered owner stream, ordered and coalesced latest-task owner streaming, and the application-owned `KeyedLatestTasks` one-shot, ordered-stream, and coalesced-stream routes are shipped. The coalesced keyed-latest route retains the exact host key, keyed ticket, replacement transaction, owner generation, and receipt; keeps only the newest pending intermediate payload before UI drain; delivers the uncoalesced final exactly once after the retained event; and passes exact `KeyedTaskCompletion<Key, _>` values to UI-local/non-`Send` mappers. Keyed supersession and owner retirement independently fence worker, mapping, and reduction. Invalid, removed, ambiguous, unkeyed, incompatible, stale, host, capacity, closing, and same-update admissions fail closed without `Application` fallback and restore only the affected key's predecessor; sibling keys remain unchanged. The cancellable ordinary ordered owner-stream route reuses the same accepted surface, owner-generation ledger, worker registry, bounded FIFO ingress, and controller-composed cancellation probe. Callers clone `request.token()` before consuming the request. Token cancellation and declarative owner retirement are independent OR-composed fences for cooperative work, events, final delivery, mapping, and reduction, including later entries already queued for one UI drain; the admission receipt does not change after it resolves. Invalid, removed, ambiguous, unkeyed, incompatible, stale, same-update, host, capacity, and closing admissions reject atomically without spawn, mapping, retry, or `Application` fallback, and event/final mappers stay UI-local/non-`Send`. `ResourceTasks` ownership, platform ownership, and shared-resource semantics remain outside this public slice.

The cancellable latest-task one-shot and ordered/coalesced owner streams are
also shipped; they use the token plus declarative owner-generation fences and
existing latest ticket/transaction rollback described above.

The cancellable ordinary coalesced owner-stream route uses the same accepted
surface, owner-generation ledger, worker registry, bounded latest-wins ingress,
and controller-composed cancellation probe as the ordered route. It retains one
newest pending intermediate payload and one queued marker before UI drain,
records the existing coalescing diagnostic when a pending event is replaced,
and delivers the uncoalesced final exactly once after the retained event.
Events separated by a UI drain map separately; token cancellation and owner
retirement fence queued event/final mapping and reduction, and the
admission-only receipt remains unchanged after acceptance.

The cancellable ordinary owner one-shot reuses the accepted surface,
owner-generation ledger, worker registry, and admission receipt. Its explicit
token and declarative owner probes are OR-composed fences for cooperative work,
deferred mapping, and reduction; only this token-cancellable owner one-shot
defers mapping, while application-owned and non-cancellable owner one-shots
remain eager. Its receipt does not change after admission resolves and its
UI-local mapper need not be `Send` or `Sync`. Invalid, removed, ambiguous,
unkeyed, incompatible, stale, same-update, host, capacity, and closing
admissions fail closed without spawn, mapping, retry, or `Application` fallback.

That target contract also requires stable owner identity and exact generations
across reprojection and keyed reorder, retirement on removal or incompatible
replacement, fresh generations on reinsertion, sibling isolation, and rejection
of same-update owner-scoped work before registration. Retired or mismatched
worker completions, timer wakes, platform results, and chained commands must be
rejected before mapper invocation and before reduction. Recovery and cached hide
do not implicitly retire a retained owner. Shared `ResourceTasks` remain
application-owned; a disappearing overlay or keyed consumer releases interest
without implicitly cancelling the shared task or discarding cached ready state.
Dynamic unkeyed nodes cannot provide the durable identity required for
owner-scoped cancellation, so they remain on the application-owned path unless
a later contract supplies an explicit stable identity.

This is a bounded public timer, one-shot business-worker, qualified platform,
cancellable ordinary owner one-shot and ordered owner-stream, ordinary ordered
and coalesced owner-scoped stream-consumer, ordered/coalesced latest-task
owner-stream, cancellable latest-task one-shot and ordered/coalesced owner-stream,
and application-owned `KeyedLatestTasks` one-shot/ordered/coalesced-stream
slice, not the complete target effect model. The public surface is limited to
the qualified owner/effect methods and typed platform service values described
above; Command internals, `EffectOrigin`, the ledger, and effect registration
remain crate-private. It makes no claim about demand/refresh/provider budgets,
scheduler budgets/fairness/queue capacity/wake ordering, `ResourceTasks`
ownership, OS notification UI, custom-coordinate transforms, renderer, or
product wiring.

Owner identity, admission, and retirement defer queue capacity, budgets, fairness,
priority, wake ordering, and stage ordering to the separately normative [`Next
scheduler policy contract`](DESIGN_DIRECTION.md#next-scheduler-policy-contract);
overlay/keyed-node cancellation is implementation sequencing, not authority to
define scheduler policy.

## UI-First Runtime Threading

Radiant treats the native UI/event/render owner as the priority path. The
window event loop, input routing, repaint requests, surface refresh, and native
Vello presentation must stay responsive and should not wait on application
business work.

Application reducers run synchronously because they decide the next UI state, so
they must stay short. Slow IO, filesystem metadata checks, database access,
decoding, indexing, analysis, loading, cache hydration, blocking waits or joins,
thread creation, process/network work, and other business work must use
`UiUpdateContext::business()` with the appropriate interactive, background, or
idle lane. Delayed messages must use `UiUpdateContext::after(...)`, and
long-lived recurring sources should use `Subscription`. The application runtime
offloads business work to
runtime-managed business threads and returns results through the normal message
queue. Finite business jobs run on a bounded business worker lane so bursts of
host work do not create unbounded OS threads beside the UI path. If that lane
cannot be started or a job cannot be queued, Radiant reports the offload failure
instead of running the work synchronously on the UI/event/render owner. If an app
explicitly needs immediate synchronous behavior, it can dispatch a normal
message and do that short UI-state work in the reducer, but the default
architecture is UI-first and non-blocking.
Delayed messages use a runtime-owned timer lane rather than one sleeping OS
thread per delay, so timer bursts do not monopolize the UI path or create
unbounded background threads. The lane transports opaque timer wakes only; the
UI runtime maps a wake to its registered message and reduces that message on
the UI owner. Interval subscriptions use the same wake lane for recurring
ticks. Receiver-backed worker subscriptions use
`Subscription::worker_payload(...)`: the dedicated thread transports only the
owned `Send` payload while its application-message mapper stays on the UI
owner.
Internally, `AppBridge` owns the generic application-message and frame queues
directly. A separate non-generic shared ingress owns worker payload deliveries,
opaque timer wakes, lifecycle admission, repaint signaling, diagnostics, and
the bounded business pool. The ingress has a private monotonic
`Accepting -> Closing -> Stopped` lifecycle and closes before application
teardown; it publishes `Stopped` only after queued UI/shared work,
reservations, timers, and registries have been cleared.
`RuntimeLifecycleHost::on_runtime_closing` is an additive, non-vetoing callback
with a default implementation, cached and invoked at most once on the UI owner
immediately after the controller enters `Closing` and before controller
teardown. Worker and timer sources receive only that shared ingress, so
adding UI-local state to `Message` does not make those ownership paths generic.
Worker payloads, platform results, and timer wakes receive one admission
sequence there and are mapped or reduced in that order on the UI owner.

The current native runtime keeps Vello/window rendering on the event-loop path
because those backend/platform constraints require it. Future render-worker or
scene-preparation split points should preserve the same rule: UI wakeups and
input responsiveness take precedence, while app-owned business work stays off
the UI path.

Background commands and messages are drained in bounded slices. If startup
hooks, timers, workers, or subscriptions produce more work than one UI pass
should reduce, Radiant keeps the remaining commands/messages ordered, requests
another wakeup, and lets the backend return to input/render work before
continuing the queue.

`SurfaceRuntime::runtime_diagnostics()` returns a generic
`RuntimeDiagnostics` snapshot for tests and future debug panels. The
`business` section reports accepted, started, completed, cancelled, rejected,
failed, and currently running business work, plus bounded recent lifecycle
events with task name, priority, queue delay, run duration, checkpoint gap, and
stream-event gap where applicable. It also reports per-priority maximum queue
delay and run duration, cooperative checkpoint counts and maximum checkpoint
gap, streaming event counts plus maximum gap between stream events, and warning
counts/recent events for tasks that exceed the configured checkpoint or stream
event warning threshold without reporting progress.
The `ui` section reports update-handler counts, the longest observed update
duration, and the latest handler that crossed the configured slow-handler
threshold, including handler type, message type, elapsed time, threshold, and
guidance to move business work to `context.business()` or typed platform
services. Use
`SurfaceRuntime::set_update_handler_diagnostics_policy(...)` with
`UiUpdateHandlerDiagnosticsPolicy::warn_at(threshold)` for controlled warning
thresholds, `panic_at(threshold)` for test/development fail-fast harnesses, or
`disabled()` only when an otherwise verified release path needs to remove even
the timing read. The default policy warns in debug/test builds and is disabled
in release builds. These values are diagnostics, not portable pass/fail
performance budgets outside a controlled harness; use them to find blocking
reducers, missing `UiUpdateContext::business()` handoffs, worker saturation, and
stale cancellation paths without coupling Radiant to an application's domain
data.

The `lifecycle` section of this snapshot is controller-owned generic runtime
evidence. It is available after `SurfaceRuntime` construction and exposes the
typed `RuntimeLifecyclePhase` values `Starting`, `Running`, `Recovering`,
`Closing`, and `Stopped`; `Unknown` is the unavailable default for a standalone
`RuntimeLifecycleDiagnostics` value. The controller records the construction
transition from `Starting` to `Running`, then records only accepted lifecycle
transitions through its private authority. `transition_count` saturates rather
than wrapping, and `history` retains at most eight transitions in
oldest-to-newest order. Repeated, invalid, backward, and post-`Stopped`
transitions are vetoed and do not change the phase, count, or history.

`RuntimeLifecycleDiagnostics`, `RuntimeLifecyclePhase`, and
`RuntimeLifecycleTransition` are qualified exports under `radiant::runtime`.
They are intentionally not common-prelude exports. This generic evidence does
not add native recovery behavior or scheduler policy.

### Qualified deterministic runtime testing host

`radiant::runtime::testing::DeterministicHost` is the qualified test-facing
headless host for one production `SurfaceRuntime`. Its
`DeterministicHostConfig` fixes the logical viewport and the currently shipped
`WindowEnvironment`; the host enables only the task, queue, and result-only
platform capabilities needed by deterministic tests. Input, messages, commands,
focus, overlay projection, layout, automation, paint-plan generation, and
invalidation continue through the production runtime controller. Configuration
validation also requires the timer-registration bound to fit within the queue
bound, and pending timers reserve those queue slots, so a fully due timer batch
cannot permanently wedge the host.

`advance_time(...)` moves only the host's virtual `Duration` clock and releases
opaque timer wakes; the same virtual instant drives production tooltip and
delayed-widget repaint deadlines. Wall-clock update-handler timing diagnostics
are disabled for this host. `complete_worker(...)` is the explicit action that runs one
stored worker closure, and `complete_platform_request(...)` sends one neutral
platform result to its runtime-owned sink. Neither action invokes an
application mapper or reducer; the result is admitted only by a later
`DeterministicHost::turn()`. `run_until_idle()` is bounded by the configured
step budget and never runs an unrequested worker or platform completion.

Each turn builds a complete `NormalizedSnapshot` candidate and publishes it
only after normalization succeeds. The versioned snapshot includes normalized
layout, automation, focus, paint summary, invalidation/identity and layout-state
diagnostics, refresh-stage counters, command outcomes, non-timing runtime
diagnostics, explicit pending work, repaint state, and an optional caller-owned
JSON application observation. `DeterministicHost::paint_plan()` additionally
exposes the current production raw `SurfacePaintPlan` for focused structural
assertions without introducing a renderer.
`NormalizedSnapshot::to_json_bytes()` omits `Instant`, elapsed-duration, native,
GPU-resource, and backend presentation data; its compact field-ordered JSON is
the byte-comparison artifact for repeated deterministic runs. The qualified
`DeterministicTraceCapture` and `DeterministicTrace` API records exact host
configuration, caller-supplied state/view identity, caller-decoded normalized
inputs, virtual-time advances, completion ids/results, and explicit publication
snapshots. Use `capture_publication(...)` at an actual host publication
boundary; Radiant does not automatically capture typed application events.
Decode preflights format, version, canonical encoding, geometry, ordering,
snapshot, identity/value, and all configured budgets before replay invokes its
factory; replay uses the existing host completion APIs and reports the first
bounded JSON-path divergence. Native windows, GPU rendering, IME,
accessibility consumers, presentation, and production scheduler policy remain
outside this boundary.

`SurfaceRuntime::devtools_snapshot()` returns a backend-neutral
`DevtoolsSnapshot` for in-app inspectors, debug overlays, tests, and embedded
host diagnostics. The snapshot includes the current viewport, a stable
surface-node tree with node kinds, resolved bounds, widget focusability and
interaction state, backend-neutral widget automation semantics, layout
diagnostics grouped by node, a selected-node
candidate derived from pointer capture/focus/hover state, aggregate
`SurfacePaintStats`, and the same generic `RuntimeDiagnostics` described
above. Use `devtools_snapshot_with_theme(...)` when paint statistics should be
computed with a non-default theme. The snapshot is deliberately generic:
applications may add host labels or presentation around it, but Radiant does
not expose raw backend handles or application-domain state through this API.
Call `DevtoolsSnapshot::inspector_projection()` when a debug view needs the
flattened tree rows plus selected-node and runtime detail lines used by
Radiant's built-in overlay.
Native inspector builds can enable a lightweight runtime overlay with
`NativeRunOptions::default().devtools_overlay_enabled(true)` or configure it
directly with `DevtoolsOverlayOptions`; this reuses the same snapshot data and
stays disabled by default. The inspector is observational: it uses the
existing normal hit-testing and focus state without taking focus or intercepting
input. The overlay paints a compact surface tree, selected-node detail panel,
and runtime summary from backend-neutral paint primitives.

## Layout

`radiant::layout` provides slot-based measurement and placement. Containers use
`ContainerPolicy`, `ContainerKind`, `SlotParams`, and `LayoutNode` to describe
rows, columns, overlays, fixed sizing, fill behavior, spacing, padding, and
stable output rectangles. Layout is deterministic and independent from any
renderer backend.
`LayoutOutput::rect_for` and `LayoutOutput::rect_for_clamped` provide the
shared measured-rectangle lookup contract for adapters that need stable
fallback bounds after a layout pass.

The first declarative custom-container extension is the qualified
`radiant::layout::{LayoutPolicy, SizeHint, MeasureChildren, PlaceChildren}`
boundary. The application `layout(policy, children)` builder accepts an
immutable UI-local policy and ordinary `ViewNode` children without application
supplied runtime IDs. Measurement receives only normalized, child-bounded
constraints; placement receives the resolved container bounds and must place or
explicitly omit every declared child exactly once. Non-finite or contradictory
size hints, invalid child requests, invalid rectangles, duplicate dispositions,
and unresolved children are diagnosed; unresolved children are conservatively
absent from `LayoutOutput`.

This slice is deliberately limited to measure and place. It does not add custom
chrome, environment or appearance contexts, interaction or semantics
capabilities, alternate reading order, animation, virtualization attachment,
exact custom-policy revisions, or custom cache reuse. Built-in
`ContainerPolicy`/`ContainerKind` behavior remains unchanged. OPT-1272 is Done;
this boundary does not reopen that issue.

Large item-indexed lists can use `VirtualListWindowRequest` and
`VirtualListWindow` from `radiant::gui::list` before projecting widgets. This
keeps host-side list projection bounded while `layout::VirtualizationPolicy`
continues to handle pixel-based scroll-container virtualization.
Editable list/tree projections use named construction parts such as
`EditableTreeRowParts` and `EditableTreeDraftInputParts` so selection,
hierarchy, draft text, validation, and focus policy remain explicit at call
sites instead of being encoded as positional boolean lists.
Application-builder code that owns a resolved logical window can use
`virtual_list_window(...)` for fixed-height rows; it preserves full scroll
extent with spacer rows while only projecting the materialized item range.
Prefer `virtual_list_windowed(...)` when runtime scrolling should update the
host-owned logical window through normal messages. Runtime pixel scrolling that
does not change the resolved logical window stays runtime-local, so sub-row
wheel/touchpad motion does not force host reprojection:

```rust
ui::virtual_list_windowed(|index| row(index))
    .row_height(22.0)
    .window(current_window)
    .overscan_px(88.0)
    .on_window_changed(Message::ListWindowChanged)
    .view()
```

When host state has already fetched or projected the current materialized
window, use `virtual_list_materialized_windowed(window, rows, |index, row| ...)`
instead of adapting the global row index back into the local slice at every call
site:

```rust
ui::virtual_list_materialized_windowed(current_window, rows, |index, row| {
    row_view(index, row)
})
.row_height(22.0)
.overscan_px(88.0)
.on_window_changed(Message::ListWindowChanged)
.view()
```

Use `virtual_tree_list_windowed(...)` for fixed-height tree or outline rows when
runtime scrolling should update the host-owned logical window through normal
messages and the same materialized range should include a standard tree-guide
overlay. Use the direct `virtual_tree_list_window(...)` helper when the host
already handles scroll-window updates separately. Pass `StyledTreeGuideStyle`
when guide color should follow the frame theme instead of a fixed
`TreeGuideStyle` color.
Use `virtual_list_window_body(...)` when the materialized range needs to be
composed as one body, such as row groups, table overlays, guide overlays, or
other decoration spanning several fixed-height rows, while Radiant still owns
the full-scroll spacer geometry.
Apps that need a one-off declarative scroll mapping can attach
`ViewNode::on_scroll_update(...)`; use `ViewNode::on_scroll_update_opt(...)`
when a high-frequency scroll surface should suppress host messages for
unchanged logical state. Lower-level hosts can still observe runtime-owned
scroll containers with app-builder `.on_scroll(...)` or, for custom bridges,
`RuntimeBridge::scroll_updated(ScrollUpdate)`. `ScrollUpdate` stays in the
common prelude because each of these normal callback and helper signatures
shares that payload.
`virtual_list_view_start_after_scroll_delta` applies signed logical-row scroll
deltas to virtual-list viewport starts with the same allocation-free clamping
contract, leaving hit testing and platform input normalization to the host or
runtime adapter.
`virtual_list_scroll_delta_from_units` converts already-normalized scroll units
into bounded row deltas for wheel, touchpad, keyboard, or host-defined scroll
inputs.
Transient fixed-row list surfaces such as autocomplete popups, command
palettes, compact inspectors, and resizable panels can use
`bounded_list_visible_rows`, `fixed_row_stack_height`,
`bounded_list_height`, and `bounded_list_height_with_gap` to share the generic
"hide when empty, account for inter-row gaps, cap visible rows, then scroll
overflow" sizing contract without baking product-specific popup rules into app
code. Application-builder surfaces can use
`BoundedScrollColumnParts`, `bounded_scroll_column(...)`, and
`bounded_scroll_column_from_parts(...)` when the host owns row projection but
Radiant should own the capped scroll viewport, empty-list behavior, chrome
padding, and viewport styling. Use `CompactOptionListItem`,
`CompactOptionListParts`, and `compact_option_list(...)` for selected
primary/secondary option rows in autocomplete popups, command palettes,
compact pickers, and similar transient result lists while the host keeps
ownership of option values and messages. The returned builder composes
`.on_activate(...)`, `.on_hover(...)`, `.filter_map_activate(...)`, and
`.filter_map_hover(...)` without multiplying constructor names. Use
`.floating_above(CompactOptionListFloatingAbove::new(...))` when such a result
list should be anchored above an editor or trigger inside the same stack layer
without app-local height and offset arithmetic. Use
`.anchored(CompactOptionListAnchor::new(...))` when the same list should be
projected in a parent-anchored overlay layer, such as a full-surface
autocomplete layer above a bottom panel. Finish every configuration with
`.view()`.
Compact toolbars and action strips can use
`layout::fixed_width_row_rects_start`, `layout::fixed_width_row_rects_end`, and
`layout::visible_suffix_widths` to place fixed-width controls through the
generic layout engine while preserving stable widget IDs.
Hot paths can use the matching `*_into` variants to reuse caller-owned buffers
instead of allocating geometry vectors on every layout or paint pass.
`layout::grouped_fixed_width_row_width` computes grouped control-cluster widths
for compact toolbars without baking product-specific toolbar concepts into the
layout adapter. `layout::fixed_width_item_extent_for_available_width` resolves
the largest fixed item extent that fits a compact row after caller-reserved gaps.
Compact control strips can use `ToolbarParts`, `ToolbarAlignment`,
`toolbar(...)`, and `toolbar_from_parts(...)` when the app owns the actual
controls but Radiant should own common strip height, padding, spacing,
start/center/end alignment, and trailing-control group placement.
Declarative views can use `SurfaceNode::scroll_area` and
`SurfaceNode::virtual_scroll_area` for the scroll viewport itself, then project
generic rows, cards, images, badges, selectables, or host-defined canvas cells as
children.
Dense card or tile grids can use `VirtualGridWindowRequest` and
`VirtualGridWindow` from the same module to resolve an allocation-free
row-major item window before projecting visible grid cells into
`SurfaceNode::grid` or a virtual scroll area.
Timeline and signal visualizations can use `ColorRamp` and `ColorRampStop` for
reusable normalized heatmap/intensity palettes, `DenseGridLayout` and
`DenseGridCell` for reusable dense-grid projection and hit testing,
`DenseGridLabelLayout` for row and column label gutters around dense grids,
`DenseGridRasterLayout` for seam-aware top-down or bottom-up raster cell
projection,
`SignalChromeState` for reusable
status/reference/channel chrome, `SignalToolFlags` and `SignalToolState` for
generic enabled/visible tool flags, `SignalRasterPreview` for retained raster
image payloads and loading state, `horizontal_progress_fill_rect` for resolving normalized
progress-track fill geometry, `push_horizontal_progress_fill` for guarded
progress-fill paint emission, `horizontal_progress_activity_rect` for
indeterminate progress segments, `horizontal_progress_track_rect` for switching
between determinate and indeterminate progress tracks, `horizontal_meter_fill_rect` and
`horizontal_discrete_meter_fill_rect` for reusable meter geometry,
`horizontal_value_range_rect`, `horizontal_value_range_edge_rects`, and
`horizontal_wrapped_value_range_rects` for normalized horizontal track ticks,
top/bottom range rails, and wrapped phase/activity segments,
`horizontal_value_cursor_rect`, `push_horizontal_value_cursor_fill`, and
`push_horizontal_value_cursor_fills` for pixel-stable full-height cursors on
timeline, waveform, scrubber, and progress-like tracks,
`vertical_bipolar_value_at_point` and `vertical_bipolar_fill_rect` for centered
signed vertical controls, `vertical_value_at_point`,
`vertical_center_track_rect`, `vertical_value_knob_rect`,
`vertical_meter_lane_fill_rect`, and `vertical_value_line_rect` for normalized
vertical faders and meters, and
`inline_indicator_layout` for compact text-relative status indicator clusters,
`TimelineAxis` for reusable beat/time/sample-to-pixel, point-to-value, and range-rectangle projection,
`TimelinePanelLayout` for reusable header, ruler, and lane panel splits,
`TimelineItemLayout` for reusable lane-centered item rectangles with optional
horizontal and vertical insets,
`TimelinePitchLayout` for reusable top-down pitch-row projection and hit testing,
`TimelinePitchItemLayout` for reusable note-like item rectangles on pitch rows,
`TimelineValueMarkerLayout` for reusable velocity and automation marker geometry,
`HorizontalValueAxis` and `HorizontalValueAxisParts` for reusable linear
value-to-x and x-to-value projection,
`VerticalValueAxis` and `VerticalValueAxisParts` for reusable bottom-up
value-to-y and y-to-value projection,
`HorizontalLogValueAxis` and `HorizontalLogValueAxisParts` for reusable
positive logarithmic value-to-x and x-to-value projection,
`TimelineLaneLayout` for reusable track, lane, and aligned label-gutter rectangles,
`HorizontalStripLayout` and `HorizontalStripLayoutParts` for gapped dense
channel/tool-strip projection, hit testing, and insertion markers,
`VerticalStripStackLayout`, `VerticalStripStackLayoutParts`, and
`VerticalStripStackOrigin` for repeated top- or bottom-anchored control slots
inside dense strips,
`vertical_value_marker` and `VerticalValueMarker` for bottom-anchored value stems
and interactive handles,
`CanvasLayer`, `DragHandle`, `canvas_selection_rect`,
`CanvasSelectionAffordanceHitTestParts`, `CanvasSelectionAffordanceStyle`,
`CanvasSelectionBodyHandleHitTestParts`,
`CanvasSelectionBodyHandleParts`, `CanvasSelectionBodyHandlePaintParts`,
`CanvasSelectionBodyHandleStyle`, `CanvasSelectionEdgeHitTestParts`,
`CanvasSelectionEdgeVisualPaintParts`, `CanvasSelectionEdgeVisualStyle`,
`CanvasSelectionPaintStyle`,
`CanvasSelectionTrailingControlHitTestParts`, `CanvasSelectionTrailingControlPaintParts`,
`CanvasSelectionTrailingControlStyle`,
`canvas_selection_body_handle_rect`,
`canvas_selection_trailing_control_rect`, `canvas_selection_edge_handles`,
`canvas_selection_edge_visual_rect`, and `horizontal_resize_edge_bracket_rects`
for generic retained-canvas layering, selection, control, resize handle geometry,
selection affordance hit testing, guarded selection-affordance paint emission,
and standard selection chrome color derivation from a host-supplied base color,
`TimelineViewport` for normalized viewport bounds, including construction from
integer `IndexViewport` ranges,
`TimelineTransportState` for cursor/playhead/selection positions,
`TimelineEditPreview` and `TimelineEditPreviewParts` for editable range and
fade/curve handles, `TimelineEditRamp` plus
`TimelineEditPreview::from_normalized_ramps(...)` for projecting host-neutral
leading/trailing ramp lengths, outer extensions, and curve controls into a
standard edit preview, plus `TimelineEditHandle` and
`TimelineEditHandleGeometry` for standard edit-handle projection and
visible-selection geometry construction,
`TimelineEditHandle::standard_order()` for default edit-handle priority, and
`TimelineEditPreview::standard_handle_at(...)` for standard edit-handle hit
testing, and `TimelineEditRegion` plus `TimelineEditRegionGeometry` for
leading/trailing edit-region projection. Use `standard_handle_rects(...)` and
`standard_region_rects(...)` when custom widgets need to paint or inspect all
standard edit affordances while keeping host-specific colors and commands, or
`push_standard_handle_fills(...)` and `push_standard_region_fills(...)` when the
standard affordances should be emitted as guarded filled rectangles with
host-supplied colors. Use `TimelineEditPaintStyle`,
`push_standard_styled_region_fills(...)`,
`push_standard_styled_handle_fills(...)`, and
`TimelineEditPaintStyle::curve_stroke_parts(...)` when Radiant should also own
the standard inner/outer region alpha split plus handle and curve color
derivation from a host-supplied base color. Use `TimelineEditCurveStrokeParts`,
`TimelineEditRampSide`, and
`TimelineEditPreview::push_standard_ramp_curve_strokes(...)` when leading and
trailing edit ramps need sampled curve strokes with Radiant-owned projection,
visibility guards, sample-density policy, and paint emission while the host
owns the domain value curve,
`TimelineFeedbackEvents` for transient operation feedback tokens,
`TimelinePresentationState` for guide spacing, repeat state, and compact labels,
`TimelineMarkerPreview` for retained marker overlays, and
`TimelineMotionState` for motion-frame overlays that group a retained timeline
surface with generic signal chrome and tool state.

## Style And Theme

`radiant::theme::ThemeTokens` and widget visual-token resolution provide
domain-neutral colors, spacing, borders, typography scale, and interaction
states. `ViewportScaleTier`, `clamp_ui_scale`, and `effective_ui_scale` provide
generic density policy for hosts that choose layout scale from available
viewport width or user preferences. Product visual identity should be supplied
by the host or translated through generic tokens instead of baked into Radiant
primitives. Use `WidgetStyle::subtle(...)`, `WidgetStyle::normal(...)`, and
`WidgetStyle::strong(...)` for common tone-plus-prominence combinations without
repeating the explicit `WidgetProminence` at call sites.

## Renderer

Radiant's generic runtime produces a backend-neutral `SurfacePaintPlan` made of
`PaintPrimitive` values. The public `Renderer` trait is the minimal replay
boundary for backend adapters that consume those paint plans. Native Vello
support is an adapter that consumes this paint plan through
`run_native_vello_runtime`. Renderers should consume paint plans and report
frame results without owning host state.

`SurfaceFrame` packages one host-controlled rendering frame as a viewport,
resolved layout, and backend-neutral paint plan. `UiSurface::frame(...)` is the
direct embedded-host path when the application or plugin framework owns the
window, native surface, or render pass; `UiSurface::frame_with_layout_options(...)`
keeps layout state, debug primitives, and diagnostics available for hosts that
need scroll offsets, virtualization state, or layout debugging.
`UiSurface::layout_at_size(...)`, `frame_at_size(...)`,
`frame_with_default_theme(...)`, and `frame_at_size_with_default_theme(...)`
cover common smoke-test, automation, plugin preview, and embedded-host cases
where the viewport starts at the origin or custom theme tokens are not part of
the behavior under test.
`SurfaceRuntime::borrowed_frame(...)` is the preferred immediate-render path for
custom host loops because it borrows the runtime's current layout instead of
cloning the resolved layout maps every frame. Hosts that render synchronously
and keep a frame scratch buffer can call `SurfaceRuntime::borrowed_frame_into(...)`
to reuse `SurfacePaintPlan` primitive storage as well. `SurfaceRuntime::frame(...)`
packages the same event-driven runtime state into an owned `SurfaceFrame` for
hosts that need to retain the frame after borrowing the runtime.
`SurfaceRuntime::frame_with_default_theme(...)` covers smoke-test, automation,
example, and embedded-preview cases where custom theme tokens are not part of
the behavior under test.
`SurfacePaintPlan::stats()` returns `SurfacePaintStats` primitive counts for
diagnostics, benchmarks, and host renderers that need to inspect Vello-friendly,
custom retained, and GPU-surface frame shape without duplicating primitive
matching logic.

Paint primitive generation is owned by the projected surface types that carry
the visual contract: widgets implement widget paint through the `Widget` trait,
and containers/overlays append their own chrome, clipping, scroll affordances,
and overlay primitives during surface traversal. The surface runtime
orchestrates layout-aware traversal and collection; backend adapters consume the
resulting paint plan. Runtime paint plans pre-size their primitive storage from
resolved layout shape before traversal, so large declarative surfaces avoid
starting every frame from an allocation-free but undersized command buffer.
Widgets default to `PaintBounds::ClipToRect`, and the runtime wraps normal
widget paint plus runtime overlay paint in matching `PaintPrimitive::ClipStart`
and `PaintPrimitive::ClipEnd` entries for the assigned widget rectangle. Custom
editor-style widgets can also emit nested clip primitives for internal
viewports, timelines, canvases, and lanes without relying on per-shape geometry
clamping.

Standard widgets emit Vello-friendly paint primitives such as fills, batched
same-color rectangle fills, strokes, text, images, clips, and overlays.
Specialized realtime visuals can instead emit `PaintPrimitive::GpuSurface`
through the application builders `render_canvas(...)`,
`render_canvas_with_capabilities(...)`,
`render_canvas_configured_from_parts(...)`, or `render_canvas_input(...)`,
or through `RenderCanvasWidget` in lower-level host
code. GPU surfaces are still normal Radiant widgets: they own stable identity,
receive layout bounds, can route widget input, and paint through the same
`SurfacePaintPlan` as Vello-backed widgets.

Use retained GPU surfaces for dense visuals where the payload is naturally
texture, signal, or shader data: waveform bodies, meters, scopes, large preview
atlases, and other surfaces that benefit from backend-owned GPU caches. Keep
normal panels, controls, labels, selection chrome, and editor overlays in
standard Radiant widgets unless they need custom GPU resources. The public
contract is `key` plus `revision` plus validated `RenderCanvasContent`; bump the
revision only when the retained GPU payload changes, and keep transient cursor
or drag previews in overlays or paint-only repaint paths. This preserves one
Radiant widget model instead of creating separate Vello and WGPU application
models.

That keyed/revision contract is the current supported 0.1.x contract. The
target-only migration path is registered `CanvasProgram` plus immutable,
typed, bounded `CanvasGraph`, reached provisionally with
`render_canvas_program(canvas)`. It does not add a second current renderer API
or reinterpret the existing `GpuSurface` path. The target graph permits only
typed immutable inputs, typed graph-lifetime transient resources, ordered
compute/fullscreen-render passes, and closed typed operations. It excludes
shader source, loops, pointers, native handles, and mutable application
payloads. Structural validation must finish before adapter handoff; an invalid
graph, unsupported contract version/capability, compilation failure, or
recovery mismatch selects a mandatory primitive fallback and emits a typed
diagnostic, never silent omission. The complete target identity includes
program/contract/payload versions, retained allocation identity, uniforms,
bounds, and adapter/target generations; hashes are lookup aids only.

`PaintGpuSurface` supports the built-in v1 content payloads
`RenderCanvasContent::RgbaAtlas`, `SignalBands`, and `SignalSummaryBands`, plus
`RenderCanvasContent::CustomShader` for advanced surfaces that need to carry
backend-neutral shader identity, optional WGSL source, explicit vertex and
fragment entry-point names, and opaque uniform/storage bytes through the normal
widget, layout, input, and paint-plan path. `entry_point` names the vertex
stage for compatibility with the original descriptor, while
`fragment_entry_point(...)` names the color-producing fragment stage a native
WGPU renderer needs for direct execution. If a descriptor provides WGSL source,
validation requires a fragment entry point so the backend handoff is complete
before a native pipeline implementation consumes it. The native WGPU path can
execute WGSL-backed descriptors that use Radiant's built-in surface uniform
ABI at `@group(0) @binding(0)`, optional app uniform payload bytes at
`@group(0) @binding(1)`, and optional read-only storage payload bytes at
`@group(0) @binding(2)`. Descriptors may also carry optional volatile
presentation-uniform bytes at `@group(0) @binding(3)`. `storage_identity` and
`storage_revision` form the immutable payload fence, while
`presentation_uniform_revision` on the descriptor and `presentation_revision`
on volatile updates form the latest-only volatile fence per target plus storage
fence.
Presentation-uniform payloads may be empty, but every non-empty descriptor or
update payload must have a byte length divisible by four for WGPU uniform
writes. `GpuShaderPresentationUniformUpdate::try_new` reports an alignment
error for invalid updates, while `RenderCanvasContent::validate()` reports a
typed descriptor validation error.
`UiUpdateContext::update_gpu_shader_presentation_uniform` and
`Command::update_gpu_shader_presentation_uniform` are paint-only updates: they
do not enter application messages or force projection. The presentation
payload is bounded and latest-only per target plus storage fence;
stale-generation updates are ignored unless
their storage fence matches the currently presented immutable payload. Native
frame diagnostics expose direct custom-shader work, including custom shader
pipeline rebuilds, under
`NativeGpuSurfaceDiagnostics::custom_shader`: `surfaces_rendered`,
`pipeline_rebuilds`, `binding_rebuilds`, `binding_cache_hits`,
`static_writes`, `static_write_bytes`, `presentation_writes`, and
`presentation_write_bytes`, so rendered surfaces, shader pipeline/bind-group
cache activity, and custom-shader buffer upload activity stay distinct from
descriptors that cannot be handed to the direct WGPU path. Native WGPU
validation failures are counted separately through
`custom_shader.failures.surfaces_failed`,
`custom_shader.failures.shader_module_failures`,
`custom_shader.failures.pipeline_failures`, and
`custom_shader.failures.binding_failures`; the native renderer also logs the
backend validation error through tracing. Descriptors that do not provide source
or stage entry points report skipped surfaces through
`custom_shader.unsupported.surfaces`, `custom_shader.unsupported.vertices`,
`custom_shader.unsupported.source_bytes`,
`custom_shader.unsupported.uniform_bytes`, and
`custom_shader.unsupported.storage_bytes` instead of silently treating them as
built-in atlas or signal content.
`RenderCanvasContent::validate()` returns a typed `RenderCanvasContentError` for
invalid atlas rectangles, signal ranges, empty payloads, and summary-shape
mismatches. `is_renderable()` and `signal_render_shape()` remain convenience
checks over the same shared payload contract used by widget projection and
native renderers, so invalid signal shapes or empty texture sources do not leak
into backend work.
Runtime behavior is declared explicitly through `RenderCanvasCapabilities`:
`fast_pointer_move` allows pointer-motion overlay updates without reprojecting
the app surface, `coalesce_vertical_wheel` allows vertical wheel deltas to be
batched until redraw, and `runtime_overlays.pointer_vertical_line` lets the
native runtime compose a lightweight pointer-following vertical line. These
capabilities are part of the GPU-surface
contract, not side effects inferred from overlays. Custom shader program support
should extend this current descriptor and diagnostics contract rather than
adding backend-specific runtime special cases. In the target contract,
arbitrary WGSL is reserved for the separately named `WgslCanvasProgram` behind
the `expert-wgsl` feature gate. The current ungated
`RenderCanvasContent::CustomShader` compatibility path remains supported until
migration evidence authorizes a later boundary; it is not silently converted to
`CanvasGraph`.

The target-only `CanvasDiagnostic` vocabulary includes `InvalidGraph`,
`UnsupportedContractVersion`, `MissingCapability`, `CompilationFailed`, and
`RecoveryIdentityMismatch`, each paired with a primitive fallback decision.
The one-argument `render_canvas(canvas)` and
`PaintPrimitive::RenderCanvas` remain target-only and may be adopted only in
0.2 after maintained examples/fixtures, downstream migrations, deterministic
fallback and identity evidence, and applicable adapter/platform evidence are
recorded. The current API and native renderer behavior described above are not
changed by this contract record.

Native runtime entry points return `RuntimeRunReport<Artifacts, Error>` when
artifact capture is requested. The report envelope is generic: Radiant owns the
result transport while each runtime path chooses its artifact payload and typed
error boundary. The generic Vello runtime reports `NativeGenericRunError`
variants for event-loop build and run failures, native initialization failures,
frame-render failures, terminal native surface failures, and unexpected render-device loss.
`NativeInitialization { stage, message }`
uses the backend-neutral `NativeInitializationStage` for native window creation,
WGPU surface creation, compatible-device acquisition, render-surface creation,
and renderer creation; backend-specific error details remain owned text at the
native adapter boundary. `SurfaceAcquireOutOfMemory` is returned when native
surface texture acquisition runs out of memory; the runner records that terminal
cause before requesting event-loop exit, so it takes precedence over an otherwise
successful run or a secondary event-loop error. Startup and shutdown artifacts
remain in the report, including when initialization fails after startup timing
begins. Simple `.run()` helpers continue returning the compatibility
`radiant::Result` string form, including the stable string for typed failures.
`FrameRender(message)` reports a Vello scene render failure using owned backend
text, including an unwinding panic contained immediately around the renderer
call; static-string and owned-`String` payloads are normalized to owned text,
while opaque payloads use a deterministic fallback. The configured panic hook
still runs. This boundary does not convert ordinary application or widget
panics outside the renderer call, and `panic=abort` or foreign aborts remain
outside its scope. The failed frame is not presented, counted, committed as
scene-texture-clean, or composed through later direct-WGPU work. When the
failure is from that narrow boundary, the runner may internally reconstruct
that window's complete native resource bundle once for the exact current
adapter generation, quarantine the old bundle behind its completion witness,
invalidate dependent frame state, rebuild the scene, and request a fresh redraw;
successful reconstruction does not become a public run error. A missing or
stale window, lifecycle/generation/capacity veto, repeated same-generation
failure, or candidate construction failure enters bounded Closing while
preserving the original `FrameRender` first cause.
`RenderDeviceLost(message)` reports an unexpected WGPU device loss using owned
backend text; an empty backend message uses a deterministic fallback, while
normal device destruction is ignored. An accepted loss from the exact current
adapter generation and callback witness enters a private bounded `Recovering`
phase. One fresh adapter/device candidate is prepared asynchronously from an
empty WGPU render context, then the existing primary `WindowId` and
`Arc<Window>` are reused for a complete generation-bound resource publication;
application effects, geometry, and runtime-local UI state are preserved. Visible
auxiliary windows rebuild independently at most one per event-loop opportunity;
cached hidden auxiliaries rebuild before they are shown, and retiring
auxiliaries never revive. Stale, duplicate, unknown, mismatched, and late loss
events are ignored. Candidate, reconstruction, publication, or bounded
quarantine-capacity failures enter the existing bounded Closing phase while
preserving the original `RenderDeviceLost` cause. Successful recovery remains
internal and does not change the public runtime API. `RenderDeviceError { kind, message }` reports an uncaptured WGPU
device error through the backend-neutral `NativeRenderDeviceErrorKind` values
`OutOfMemory`, `Validation`, and `Internal`; empty backend descriptions use a
deterministic non-empty fallback. Validation errors captured by the scoped custom
shader diagnostics path remain scoped diagnostics rather than becoming terminal
run errors. Auxiliary-window frame, device-loss, and uncaptured device-error
failures use the same parent report boundary.
This keeps compatibility diagnostics and generic runtime diagnostics on the same
mechanism without coupling the public runtime API to a host application model.
`radiant::gui::paint` also exposes lower-level backend-neutral paint payloads
such as `PaintFrame`, `Primitive`, `TextRun`, `FillRect`, `FillCircle`,
`FillLinearGradient`, `DrawImage`, `horizontal_line_rect`, and
`vertical_line_rect` for retained renderer adapters that need frame-oriented
scene data rather than a full declarative `SurfacePaintPlan`.

## Context

Runtime context is split deliberately:

- Host context lives in the host application state and reducer.
- Layout context is the viewport and resolved `LayoutOutput`.
- Style context is the active `ThemeTokens`.
- Runtime context is exposed as `RuntimeContext`, a borrowed view over
  `SurfaceRuntime` containing the current viewport, surface, and resolved
  layout. `RuntimeContext::resolved_environment()` exposes the same cloneable
  widget-facing environment projection used by paint traversal.
  `SurfaceRuntime` owns focus target, widget hit testing, and message dispatch.

Paint traversal derives one `ResolvedEnvironment` from the current
`UiSurface` snapshot per plan and carries it through clipped base traversal and
runtime overlays. `WidgetPaintContext` borrows layout/theme data and borrows
the combined environment, keeping environment-aware widget paint allocation-free
per widget while preserving the legacy `Widget` hooks through default
delegation.

## Event And Focus

Backend input is normalized into Radiant input primitives such as
`Event`, `WidgetInput`, `PointerButton`, and `WidgetKey`. The runtime performs
hit testing, pointer capture and cancellation, focus changes, pointer
press/release routing,
keyboard routing to the focused widget, and message mapping. `Event` is the
backend-neutral runtime event surface for resize, pointer, keyboard, focus
traversal, focus-clear, and pointer-capture-cancellation operations;
`SurfaceRuntime::dispatch_event` is the
primary event-routing entry point for backend adapters. Focus behavior is
declared by widget contracts rather than by host-domain code.
Scroll input uses one backend-neutral offset-direction contract across
`Event::Scroll`, `WidgetInput::Wheel`, `CanvasGestureEvent::Wheel`,
`ScrollUpdate::delta`, `WheelDelta`, and the wheel/scroll routing APIs: positive
`x`/`y` increases the logical horizontal/vertical scroll offset, reveals content
right/down, and causes layout to render that content left/up. The controller
applies `current + delta` and clamps the resulting offset; layout places scroll
content at `origin - offset`. Native adapters are the single sign/unit boundary:
AppKit/winit content-direction deltas are negated once, line deltas retain the
40 logical-pixels-per-line rule, pixel deltas receive the existing DPI
conversion, and generic routing performs no coordinate-origin flip or second
sign conversion. Exact unit and phase preservation is limited to qualified
widget/policy routing. Ordinary native scroll-container coalescing projects
each sample to logical pixels, selects horizontal only when `|x| > |y|` and
vertical otherwise (including ties), drops the orthogonal component, and emits
a phase-less `ScrollUpdate`. It retains the newest modifiers and timestamp;
when available, the sequence range spans the first through newest contributing
sample, and an axis change flushes the prior pending sample before queueing the
new axis.
`radiant::gui::input::logical_point_to_u16_coords` provides the shared
clamp/round contract for adapters that must project logical pointer positions
into compact integer coordinates.
`radiant::gui::text_layout::snap_text_baseline_to_pixel` provides shared
baseline snapping for retained text rows. `TextLineLayoutCache` lets renderer
adapters own text-line placement caches explicitly instead of sharing a
process-global lock.
`Rect::inset` provides product-neutral four-sided inset geometry for plotting
areas, panels, and control tracks. `Rect::inset_horizontal` provides
horizontal-only text and control inset geometry.
`Rect::horizontal_ratio_span` provides full-height horizontal sub-rect
projection for dense strip and control layouts.
`Rect::center` provides shared midpoint geometry for routing, hit testing, and
retained rendering adapters.
`Rect::empty_at_min` and `Rect::empty_at_max` provide explicit zero-size
fallback geometry at either resolved corner.
`Rect::inset_vertical` provides product-neutral vertical inset geometry for
rows, panels, and scroll regions.
`Rect::split_at_y` provides reusable vertical partitioning for split panes,
bands, and sectioned panels.
`Rect::inset_horizontal_saturating` provides symmetric horizontal insets capped
at half width for centered zero-width collapse.
`Rect::inset_uniform_saturating` provides symmetric two-axis insets capped at
half width and height for centered zero-size collapse.
`Rect::centered_pixel_square` and `Rect::centered_odd_pixel_square` provide
pixel-snapped icon-box geometry for reusable controls.
The prelude `SvgIcon::from_svg(...)` parses embedded SVG source into a retained
SVG document that emits a backend-neutral `PaintSvg` primitive.
`SvgIcon::try_from_svg(...)` and `PaintSvgDocument::try_from_svg(...)` return a
typed `SvgParseError` when hosts need parser diagnostics. Single-color static
icons whose tint follows theme or interaction state can use
`SvgIcon::from_svg_with_current_color(...)`,
`SvgIcon::try_from_svg_with_current_color(...)`, or a static
`SvgIconTintCache` so repeated projections clone retained tinted documents
instead of reparsing formatted SVG strings. Use `SvgIconTintPalette` with
`SvgIconTintCache::icon_for_state(...)` when enabled, active, and disabled icon
states should resolve through one app-owned palette instead of repeated
state-color branches. `svg_with_current_color(...)`
provides the same root-attribute injection for one-off asset preparation. The
native Vello backend appends retained SVG documents through `vello_svg` during
scene encoding. `SvgIcon::empty()` creates a no-paint icon for defensive
fallbacks or temporarily unavailable vector assets.
`Rect::stroke_aligned_rect` provides stroke-grid snapping for retained border
geometry.
`Rect::top_left_square`, `Rect::top_right_square`,
`Rect::bottom_left_square`, and `Rect::bottom_right_square` provide anchored
overlay geometry for controls, badges, range handles, and secondary glyphs.
`Rect::square_around(...)` provides compact point-marker geometry for retained
canvas and chart overlays, with callers free to clamp the result to their
surface bounds.
`Rect::top_edge_strip`, `Rect::bottom_edge_strip`, `Rect::left_edge_strip`,
`Rect::right_edge_strip`, `Rect::horizontal_center_strip`, and
`Rect::vertical_center_strip` provide edge and centered strip geometry for
reusable retained paint paths and editor handles. Coordinate-centered variants
`Rect::horizontal_strip_around_y` and `Rect::vertical_strip_around_x` shift
inside bounds for edge-adjacent handles that should keep their requested size
where possible.
`Rect::intersection` provides explicit shared-area geometry for combining
independent layout bands, hit regions, and retained paint overlays without
hand-assembling min/max corners in application code.
`Rect::union` provides shared bounding-box aggregation for retained rendering,
hit testing, and automation paths.
`RevisionCounter` provides a tiny GUI-state revision nonce for invalidating
retained widget identity, cached projections, or host-owned retained resources
after application-owned interaction state changes.
`StatusSegments::new(...)`, `StatusSegments::primary(...)`,
`StatusSegments::left_center(...)`, and the `with_left(...)` /
`with_center(...)` / `with_right(...)` builders provide a structured
left/center/right status-bar model for application chrome. Use
`StatusSegments::left_center(...)` when the status bar has left and center text
plus trailing live content but no right text segment.
Application-builder views can use `StatusBarParts`, `status_bar(...)`, and
`status_bar_from_parts(...)` to render those segments with standard compact
status-bar sizing, padding, spacing, truncation, and optional trailing content
such as a progress bar.
`SurfaceRuntime::focus_widget`, `SurfaceRuntime::clear_focus`,
`SurfaceRuntime::focused_widget`, `SurfaceRuntime::traverse_focus`, and
`FocusTraversal` expose deterministic backend-neutral focus ownership and
sequential traversal. `focused_widget()` and the public result of
`traverse_focus(...)` remain widget-only: when traversal selects a private
runtime-owned split separator stop, it installs the existing private separator
focus owner and returns `None`. Separators do not enter the public widget order,
public key-routing target, or public focus API. The crate-private
`traverse_focus_with_disposition(...)` distinguishes `NoDestination`,
`AdmittedWidget`, `AdmittedPrivateSplitPaneSeparator`, `Vetoed`, and
`Invalidated`; only `NoDestination` is eligible for a future key-routing
fallback, while veto and invalidation are terminal. `Event::TraverseFocus`
remains unchanged.
`UiSurface::keyboard_focus_order_into(...)` writes the same deterministic order
into caller-owned storage for diagnostics or host integrations that inspect
focus order repeatedly without reallocating.
Pointer dispatch through `dispatch_input_at` can assign focus from hit testing;
keyboard dispatch through `dispatch_focused_input` routes input to the focused
widget by stable `WidgetId`. When a pointer press or double-click would move
focus and the current widget vetoes focus loss, Radiant suppresses that
initiating input and rolls back provisional pointer capture. A pointer press or
double-click on a non-focusable hit target preserves the pre-existing behavior
of clearing current controller focus before routing target input. If the current
owner vetoes that clear, Radiant suppresses target input and unwinds provisional
pointer capture.
The generic managed pointer path is controller-owned and pins the exact admitting
widget and initiating button. It is installed only after target selection,
preflight, focus validation, and press dispatch admission. Matching move,
modifier, and release samples remain with that exact owner; a nonmatching button
cannot terminate or rebase it. Authority loss, focus loss, removal, disabled or
read-only state, incompatible replacement, and explicit cancellation clear the
managed record conservatively, with bounded button-specific orphan suppression for
a delayed matching release. Scrollbar and layout hit precedence remains ahead of
widget preflight, and `Blocked` never reaches widget dispatch, focus transfer,
capture, mapping, or host output. The NumericInput PointerScrub and wheel
consumers are shipped separately; their policy, output, failure, geometry, and
continuity contracts remain generic and backend-neutral. Native unit/phase
translation remains a separate platform boundary.
`Event::pointer_capture_cancelled()` uses the same runtime cancellation path for
host or lifecycle boundaries: it clears managed, widget, layout, and scrollbar
drag capture, delivers at most one cancellation to the captured owner, leaves
keyboard focus unchanged, and does not synthesize a release or commit. A later
matching release is ignored through the existing button-specific orphan-release
tombstone.
Tests, automation, and embedded hosts that need ordinary pointer activation can
use `SurfaceRuntime::dispatch_pointer_click(...)` or
`dispatch_primary_click(...)` / `dispatch_secondary_click(...)`; the returned
`PointerClickOutcome` reports the press target, release target, and completed
widget while still routing through the same backend-neutral press/release event
path as native adapters.
Runtime event tests, automation, and embedded hosts can use `Event::resize(...)`,
`pointer_move(...)`, `pointer_press(...)`, `primary_press(...)`,
`secondary_press(...)`, `pointer_double_click(...)`, `primary_double_click(...)`,
`pointer_release(...)`, `primary_release(...)`, `secondary_release(...)`,
`pointer_capture_cancelled()`,
`key_press(...)`, `character(...)`, `traverse_focus(...)`, `clear_focus(...)`,
and `scroll(...)` instead of repeating backend-neutral event struct literals.
`Event::PointerMove` and `WidgetInput::PointerMove` also carry observational
`PointerModifiers`, optional `InputTimestamp`, and optional opaque
`InputSequenceRange` sample metadata. A direct accepted native sample receives
a singleton range; an existing coalescing owner extends the first endpoint to
the newest contributing sample. Sequence values are allocated independently per
native runner/window and provide no sample count, density, arithmetic, or
cross-window ordering promise. The
public position-only `Event::pointer_move(...)` and `WidgetInput::pointer_move(...)`
constructors remain source-compatible and use default modifiers with no
timestamp; native adapters preserve captured sample metadata while synthetic
and backend-neutral paths omit it.
`Event::Scroll` and `WidgetInput::Wheel` carry the same observational modifier,
optional timestamp, and optional sequence-range metadata. The public `Event::scroll(...)`, `WidgetInput::wheel(...)`,
and `WidgetInput::plain_wheel(...)` constructors remain source-compatible:
`scroll(...)` and `plain_wheel(...)` use default modifiers, `wheel(...)` preserves
its supplied modifiers, and all three omit the timestamp. Native wheel adapters
capture one sample timestamp and preserve it, together with effective modifiers,
through direct routing and coalesced delivery. Scroll-container fallback and
scrollbar-drag delivery expose that provenance in the `ScrollUpdate::metadata`
value: modifiers and timestamp come from the newest contributing sample, while
the opaque sequence range spans the first through newest sample. Axis changes
flush the prior coalescing owner, and focus loss discards pending input. Synthetic,
programmatic, command, and backend-neutral scroll paths use
`ScrollUpdateMetadata::default()`.
Accepted wheel edits from `KnobWidget` emit one `KnobMessage::WheelGesture` with
the existing three ordered `KnobAutomationEvent` values and a copyable
`KnobWheelMetadata` payload. Its modifiers, optional timestamp, and optional
sequence range are copied unchanged from the accepted `WidgetInput::Wheel`.
Typed pointer ingress is an additive API in `gui::pointer_ingress`. Hosts may
construct checked `PointerIngress` and `GestureIngress` values with device,
contact, phase, logical-coordinate, button, modifier, pressure, tilt, timestamp,
and sample-range evidence. Surface runtimes route these through a fixed
sixteen-record device/contact table. A started or hover sample has no sequence
token; only the runtime can issue a nonzero opaque token, and a continuation is
admitted only when its token, device, contact, and runtime identity match.
`dispatch_pointer_ingress_with_admission` returns that opaque token for a
started sequence, including layout, scrollbar, and explicitly unsupported
admissions that have no widget callback; hosts can pass it to the checked
continuation constructor without minting or inspecting the token.
`Widget::handle_pointer_event` is the opt-in extension used by
`RetainedCanvasBuilder::on_pointer`, `gpu_surface_pointer`, and
`render_canvas_pointer`; existing `Event`, `WidgetInput`, and canvas gesture
contracts remain source-compatible. Valid pan, pinch, and rotate ingress is
reported as an explicitly admitted unsupported consumer until the later gesture
arena phase. Typed drag payloads, cross-window payloads, and external offers
remain outside this phase.

Run `cargo run --example typed_pointer` for an ordinary application update
handler receiving an admitted mouse sequence from `render_canvas_pointer`.
The example also checks public token admission and replayed-terminal rejection
through `SurfaceRuntime`. Malformed native touch terminals cancel only their
exact retained contact at its last valid logical position, preserving the
issued token and freeing both native and runtime sequence slots.

`KnobWheelGesture::new(...)` remains the compatibility constructor and uses
`KnobWheelMetadata::default()`; use `KnobWheelGesture::new_with_metadata(...)`
or `input_metadata()` for explicit provenance. The added public `metadata`
field means external destructuring of `KnobWheelGesture` must account for that
field. This metadata is observational only and does not affect Shift fine-step
selection, direction, clamping, routing, acceptance, or repaint behavior.
Accepted primary pointer edits from `KnobWidget` emit the existing incremental
`KnobMessage::GestureStarted`, `ValueChanged`, and `GestureEnded` lifecycle with
a copyable `KnobPointerMetadata` field. The public reset variant is
`KnobMessage::Reset { value, metadata }`; update public constructions and
destructuring patterns to account for that field. For an accepted primary
double-click reset, `metadata` copies the modifiers and optional timestamp from
the second accepted double-click sample and always has an absent sequence range.
`WidgetInput::primary_double_click(...)` is synthetic, so it produces default
metadata. Reset emits exactly one message even when the value already equals the
configured default. Use `KnobMessage::pointer_gesture_metadata()` to read pointer
provenance; it returns `Some(metadata)` for reset and the incremental pointer
lifecycle, and `None` for keyboard and wheel messages. Press and terminal
release/drop messages preserve their current `PointerModifiers` and optional
`InputTimestamp` without a sequence range. Each accepted value-changing captured
move preserves the move's current modifiers, timestamp, and complete opaque
sequence range. Moves that leave the clamped value unchanged emit no message.
Focus-loss cancellation emits exactly one ended message with
`KnobPointerMetadata::empty()` and a later release emits no message. Legacy
pointer-capture cancellation emits no message, while the official retained
typed path emits a `Cancel` batch for both focus loss and capture loss. Pointer
provenance is observational only: it is copy-only and is not retained in widget
state, and does not alter capture, fine adjustment, reprojection,
disabled-terminal, reset, or value behavior.
Accepted keyboard edits from `KnobWidget` emit one
`KnobMessage::KeyboardGesture` with the existing three ordered
`KnobAutomationEvent` values and a copyable `KnobKeyboardMetadata` payload.
Its optional timestamp is copied unchanged from an accepted focused, enabled,
value-changing `WidgetInput::KeyPress`; unsupported, unfocused, disabled, and
no-op key inputs emit no keyboard gesture. `KnobKeyboardGesture::new(...)`
remains the compatibility constructor and uses
`KnobKeyboardMetadata::default()`; use
`KnobKeyboardGesture::new_with_metadata(...)` or `input_metadata()` for
explicit provenance. Synthetic `WidgetInput::key_press(...)` inputs and
inputs with no timestamp retain absent timestamp metadata. The added public
`metadata` field means external destructuring of `KnobKeyboardGesture` must
account for that field. This metadata is observational only and does not
affect key mapping, focus, clamping, routing, acceptance, or repaint behavior.
`Event::KeyPress` and the corresponding `WidgetInput::KeyPress` form carry the
normalized `KeyboardModifiers` state, the native `repeat` flag, and an optional
input timestamp. `KeyboardModifiers` keeps command, control, shift, and alt
distinct. The public `Event::key_press(...)` and `WidgetInput::key_press(...)`
constructors use no modifiers, `repeat: false`, and no timestamp. Native
adapters keep two projections of each physical sample: host `KeyPress` retains
platform shortcut semantics, including Control folded into `command` on
Linux/Windows, while focused widgets receive lossless physical modifiers with
Super/Meta as `command` and Control as `control`. Press, repeat, and release
use the same current native projection; host resolution remains first and a
handled shortcut does not reach the widget. Generated logical shortcut fallback
has no physical modifier sample and therefore retains only its timestamp.
Public and synthetic dispatch still derives widget modifiers field-for-field
from the supplied `KeyPress`. This metadata remains observational for ordinary
widget key mapping, shortcut precedence, and edit provenance; complete-mode
explicit-policy `KeyboardAdjustment` semantic numeric stepping is shipped.
Backend adapters that need redraw policy can route pointer motion through
`SurfaceRuntime::dispatch_pointer_move_with_outcome(...)`. Its
`PointerMoveOutcome` reports the target widget, hover changes, pointer capture,
scene-rebuild repaint requests, paint-only overlay requests, and exit requests
in one controller-owned result. Native and embedded renderers should use that
outcome when deciding between rebuilding the cached scene and presenting a
runtime overlay over the existing scene.
Native renderers that receive very high frequency pointer updates can use
`SurfaceRuntime::dispatch_pointer_move_deferred_refresh_with_outcome(...)` to
reduce emitted widget messages immediately while deferring surface projection,
layout, and scene rebuild until the next redraw. This keeps drag reducers
current without forcing one declarative refresh per OS cursor event.
Custom widget tests, automation, and embedded hosts can use
`WidgetInput::pointer_move(...)`, `pointer_press(...)`, `primary_press(...)`,
`pointer_double_click(...)`, `primary_double_click(...)`,
`pointer_release(...)`, `primary_release(...)`, `pointer_drop(...)`,
`primary_drop(...)`, `wheel(...)`, and `plain_wheel(...)` to build
backend-neutral widget inputs without repeating pointer-event struct literals.
Root application shortcuts should normally be declared with
`Scene::shortcuts(ShortcutCatalog::new()...)`. `ShortcutCatalog` maps
normalized `KeyPress` values through ordered `ShortcutLayer` values, supports
modal layers that consume unmatched keys, and can attach a fallback resolver for
dynamic keys such as shifted navigation. Returning
`ShortcutResolution::action(message)` dispatches a normal app message before
focused-widget key routing, while `ShortcutResolution::handled()` suppresses
the fallback without coupling Radiant to an application command model. Use
`ShortcutLayer::bind_all(...)` when several equivalent gestures should dispatch
the same host action, and `ShortcutLayer::modal_escape(...)` for modal surfaces
whose Escape key dismisses the surface while other keys remain shielded.
Application-builder `.shortcuts(...)` remains available as an advanced
compatibility hook when a host needs pending-chord or `FocusSurface` access.

## Performance Harness

The [Platform Acceptance and Evidence Policy](PLATFORM_ACCEPTANCE.md) classifies
benchmark configuration and output parsing as lane C, deterministic workload
execution as lane A, and native frame-profile or fairness claims only through
the applicable native lanes. Existing performance contracts and thresholds
remain authoritative; this policy adds no timing threshold.

Radiant includes a standalone performance harness for trend and profiling
evidence. Run it with:

```powershell
cargo bench --bench perf_harness
```

The harness prints parseable `radiant_perf` metric lines for layout, runtime
surface, application projection, and GPU-surface data preparation scenarios.
Use `--jsonl` when collecting trend artifacts for scripts or CI storage:

```powershell
cargo bench --bench perf_harness runtime_virtualized_list_hover -- --jsonl
```

Each JSON line includes `type`, `scenario`, `category`, `group`, `iterations`,
`total_us`, `avg_us`, and finite nearest-rank `p50_us`, `p95_us`, and `p99_us`,
plus any scenario-owned counters such as
`scene_rebuild_count`, `static_rebuild_count`, `paint_only_count`,
`surface_refresh_count`, `relayout_count`, `dirty_mark_count`,
`overlay_paint_count`, `overlay_rebuild_count`, `text_cache_hit_count`,
`retained_surface_cache_hit_count`, `gpu_surface_count`,
`frame_cadence_due_count`, `frame_cadence_wait_count`,
`widget_callback_allocation_count`, `text_storage_allocation_count`, and
`allocation_sensitive_work_count`, `encoded_paint_primitive_count`, and
`scene_append_count`. This keeps
performance history parseable without scraping prose or losing which target
area and review-risk group the scenario validates.
The `p50_us`, `p95_us`, and `p99_us` are finite nearest-rank percentiles.

The maintained `examples/arrangement_shell` implementation is used directly by
the standalone GUI consumer contract; the harness does not copy or simplify
that workload. Run the focused lanes with:

```powershell
cargo bench --bench perf_harness runtime_arrangement_shell -- --jsonl
```

The `standalone_gui` lanes are:

- `runtime_arrangement_shell_frame_refresh`: continuous frame update followed
  by the current combined refresh and paint-plan materialization;
- `runtime_arrangement_shell_structural_toggle`: browser/inspector structural
  toggle followed by full refresh and relayout; and
- `runtime_arrangement_shell_hover_paint_only`: existing hover movement followed
  by paint-only output with zero application projection, runtime projection,
  widget-state synchronization, and layout.

The lanes preserve exact counter deltas and assert repeated identical runs have
identical counters. Sampling uses bounded batches; bounded batches avoid a
clock read around every tiny iteration. Percentiles are finite and assert
`p50_us <= p95_us <= p99_us`; average-based baseline comparison is unchanged,
and legacy baseline JSONL remains readable. These are measured consumer-contract
lanes only: they establish no production staged Projection/Reconciliation/
Layout/Paint execution and receive no design-only credit.
Capture a machine-local baseline artifact directly with
`--write-baseline-jsonl`:

```powershell
cargo bench --bench perf_harness runtime_virtualized_list_hover -- --jsonl --write-baseline-jsonl .\perf-baseline.jsonl
```

Compare a focused run against a previously captured JSONL artifact with
`--baseline-jsonl`:

```powershell
cargo bench --bench perf_harness runtime_virtualized_list_hover -- --jsonl --baseline-jsonl .\perf-baseline.jsonl
```

When a baseline file is supplied, every emitted metric includes
`baseline_status`. A matching baseline scenario adds `baseline_avg_us`,
`baseline_ratio`, and `baseline_status` to the output; the status is `faster`,
`similar`, or `slower` using a small tolerance. A missing baseline scenario reports
`baseline_status=missing` so incomplete trend artifacts are visible during
review. After the matched scenarios finish, the harness also emits a
`radiant_perf_summary` line with `baseline_matched`, `baseline_missing`,
`baseline_faster`, `baseline_similar`, and `baseline_slower` counts so CI
artifacts and code reviews can quickly see whether the baseline file covered the
run. It also emits one `radiant_perf_category_summary` line per target-area
category in the run, carrying the same baseline counts for just that category,
so reviewers can spot whether a regression or missing baseline belongs to text,
layout, runtime, resource, or GPU-facing work. These statuses are trend context
for review and investigation, not a portable pass/fail gate by default. CI or
release jobs that intentionally pin a machine-specific baseline can opt into a
gate with
`--fail-on-baseline-regression`; the harness then exits with status `1` when
any matched scenario reports `baseline_status=slower`, while still printing the
normal metric and summary lines:

```powershell
cargo bench --bench perf_harness runtime_virtualized_list_hover -- --jsonl --baseline-jsonl .\perf-baseline.jsonl --fail-on-baseline-regression
```

Use `--fail-on-missing-baseline` when a target-area run should also fail if the
baseline file does not contain every matched scenario. This is useful for
release or CI jobs that require complete baseline coverage before interpreting
category summaries:

```powershell
cargo bench --bench perf_harness -- --category runtime_virtualized --jsonl --baseline-jsonl .\perf-baseline.jsonl --fail-on-missing-baseline
```

List the available scenarios without running them with:

```powershell
cargo bench --bench perf_harness -- --list
```

Metric lines and list output both include each scenario's target-area category,
blessed review group, default iteration count, and advertised counters, so
reviewers can quickly spot whether a run covered layout, runtime, text,
resource, or GPU-facing work.
Run a whole target-area category without spelling every scenario with
`--category`:

```powershell
cargo bench --bench perf_harness -- --category runtime_virtualized --jsonl
```

Run a blessed high-risk scenario group with `--group`:

```powershell
cargo bench --bench perf_harness -- --group pointer_motion --jsonl
```

When running from the Wavecrate repository root instead of `vendor/radiant`,
use the same harness through the Radiant manifest:

```powershell
cargo bench --manifest-path vendor/radiant/Cargo.toml --bench perf_harness -- --group pointer_motion --jsonl
```

Blessed high-risk groups:

| Group | Run before PRs touching | Focused command |
| --- | --- | --- |
| `pointer_motion` | hover routing, pointer-move repaint policy, paint-only overlays, menu/popover anchors, GPU-surface cursor overlays | `cargo bench --bench perf_harness -- --group pointer_motion --jsonl` |
| `virtual_lists` | virtualized list layout, fixed-row scrolling, row-window projection, nested scroll regions, dense row builders | `cargo bench --bench perf_harness -- --group virtual_lists --jsonl` |
| `scene_cache` | scene rebuild policy, dirty layout caches, retained invalidation masks, refresh/reprojection paths | `cargo bench --bench perf_harness -- --group scene_cache --jsonl` |
| `text_layout` | text paint plans, text-line layout cache use, word selection/deletion, wrapping or clipped text rows | `cargo bench --bench perf_harness -- --group text_layout --jsonl` |
| `retained_gpu_surfaces` | GPU surface projection and compositor occlusion planning, retained atlas/custom shader paths, signal-summary data preparation, renderer cache diagnostics | `cargo bench --bench perf_harness -- --group retained_gpu_surfaces --jsonl` |
| `frame_cadence` | resize cadence, animation activity, paint-only frame policy, route-time frame-drain behavior | `cargo bench --bench perf_harness -- --group frame_cadence --jsonl` |

Keep baselines machine-local unless a stable CI baseline is intentionally
introduced. Capture and compare each blessed group with these copy/paste
commands:

```powershell
cargo bench --bench perf_harness -- --group pointer_motion --jsonl --write-baseline-jsonl .\target\radiant-pointer-motion-baseline.jsonl
cargo bench --bench perf_harness -- --group pointer_motion --jsonl --baseline-jsonl .\target\radiant-pointer-motion-baseline.jsonl
cargo bench --bench perf_harness -- --group virtual_lists --jsonl --write-baseline-jsonl .\target\radiant-virtual-lists-baseline.jsonl
cargo bench --bench perf_harness -- --group virtual_lists --jsonl --baseline-jsonl .\target\radiant-virtual-lists-baseline.jsonl
cargo bench --bench perf_harness -- --group scene_cache --jsonl --write-baseline-jsonl .\target\radiant-scene-cache-baseline.jsonl
cargo bench --bench perf_harness -- --group scene_cache --jsonl --baseline-jsonl .\target\radiant-scene-cache-baseline.jsonl
cargo bench --bench perf_harness -- --group text_layout --jsonl --write-baseline-jsonl .\target\radiant-text-layout-baseline.jsonl
cargo bench --bench perf_harness -- --group text_layout --jsonl --baseline-jsonl .\target\radiant-text-layout-baseline.jsonl
cargo bench --bench perf_harness -- --group retained_gpu_surfaces --jsonl --write-baseline-jsonl .\target\radiant-retained-gpu-surfaces-baseline.jsonl
cargo bench --bench perf_harness -- --group retained_gpu_surfaces --jsonl --baseline-jsonl .\target\radiant-retained-gpu-surfaces-baseline.jsonl
cargo bench --bench perf_harness -- --group frame_cadence --jsonl --write-baseline-jsonl .\target\radiant-frame-cadence-baseline.jsonl
cargo bench --bench perf_harness -- --group frame_cadence --jsonl --baseline-jsonl .\target\radiant-frame-cadence-baseline.jsonl
```

It currently covers:

- Layout scenarios: `layout_deep_nesting`, `layout_wrap_1k`, `layout_virtualized_10k`,
  `layout_virtualized_fixed_10k`, `layout_virtualized_fixed_scroll_10k`,
  `layout_mark_dirty_subtree_10k`, and `layout_dirty_virtual_cache_10k`
- Application projection scenarios: `app_virtual_list_projection_10k`,
  `app_virtual_list_projection_generated_child_ids_10k`,
  `app_virtual_selectable_list_projection_10k`, and
  `app_virtual_list_window_projection_10k`, plus
  `app_constant_message_controls_projection_1k` for allocation-sensitive
  constant-message binding coverage and `app_static_text_controls_projection_1k`
  for zero-allocation static label projection
- Runtime surface scenarios: `runtime_surface_large_tree`, `runtime_text_paint_plan_1k`,
  `runtime_horizontal_scroll_paint_1k`, `runtime_virtualized_list_wheel_10k`,
  `runtime_virtualized_list_hover_10k`,
  `runtime_virtualized_list_stable_hover_10k`,
  `runtime_virtualized_list_hover_paint_10k`,
  `runtime_pointer_overlay_paint_10k`,
  `runtime_retained_segment_invalidation_1k`,
  `runtime_virtualized_nested_scroll_hover_10k`,
  `runtime_refresh_large_tree`, `runtime_resize_large_tree`,
  `runtime_animation_frame_cadence_1k`, `runtime_command_flattening_512`,
  `runtime_command_drain_1k`, and `runtime_nested_command_drain_1k`. The paired
  Vello artifact strategy probes are
  `vello_artifact_strategy_4x256_full_reencode` and
  `vello_artifact_strategy_4x256_append_one_changed`; they use four
  resource-free direct-Vello rectangle segments with 256 fills each. The first
  encodes all 1,024 fills, while the second encodes one changed 256-fill segment,
  resets its destination, and appends four segment scenes. Both assert equivalent
  final Vello encoding counts and report the strategy counters; they make no
  runtime-type, text, image, clip, resource, or timing-threshold claim.
- Resource lifecycle scenarios: `resource_slot_stale_completions_1k`
- Text scenarios: `text_line_cache_1k`, `text_word_selection_1k`, and
  `text_word_deletion_1k`
- GPU data and surface scenarios: `gpu_signal_summary`, `gpu_surface_projection`,
  `gpu_surface_stack_projection_128`, and `gpu_custom_shader_projection`
- Vello artifact strategy scenarios:
  `vello_artifact_strategy_4x256_full_reencode` and
  `vello_artifact_strategy_4x256_append_one_changed`

Pass a scenario substring to run one focused slice, for example:

```powershell
cargo bench --bench perf_harness runtime_virtualized_list_hover
```

The harness performs sanity assertions, but it does not enforce machine-dependent
pass/fail timing thresholds; use the output for local comparisons, profiling
runs, trend capture, and regression investigation.
Run `cargo run --example rendering_benchmark` for a checked public-API sandbox
that builds a large declarative surface, runs layout plus paint-plan generation,
and prints parseable primitive-count diagnostics.
Run `cargo run --example host_surface_frame` for a checked embedded-host
sandbox that drives `SurfaceRuntime` with backend-neutral events, requests a
`SurfaceFrame`, and reports `SurfacePaintStats` without opening the native
window runtime.

For interactive native runs, set `RADIANT_NATIVE_RENDER_PROFILE=1` before
launch to emit a per-frame `radiant native render profile` tracing line. The
same counters are also exposed to custom hosts through the explicitly registered
`RuntimeFrameDiagnosticsHost` capability as `NativeFrameDiagnostics`, so apps
can collect frame diagnostics without parsing logs. The observer is called once
for each successfully presented frame from the primary or an auxiliary window;
auxiliary delivery is forwarded through the parent runtime event boundary before
that event's messages are dispatched. The scene diagnostics
are grouped into `traversal`, `text`, `media`, and `surfaces` buckets so hosts
can inspect paint-plan traversal, text encoding, image/SVG encoding, and
GPU/custom-surface handoff without treating the payload as one flat counter bag.
The profile separates retained-surface bridge/cache/miss counts,
custom-surface fallback counts, GPU-surface render/cache counts,
transient-overlay primitive counts, and timing for surface
refresh, paint-plan generation, Vello render-to-texture, composed-base refresh
or cache hits for transient overlays, transient-overlay paint callbacks,
GPU-surface composition, and presentation.
`NativeFrameDiagnostics::cpu_fairness` is an opt-in, bounded one-window summary
of the existing native CPU scheduler's observed turns. When `available` is
false, the window has no retained scheduler-turn state and the summary remains
at its default no-state values. When available, `latest_disposition` reports
`NativeCpuFrameFairnessDisposition::Unknown`, `NotDue`, `Selected`, or
`DueButDeferred`; the summary also separates requested and effective target FPS
and exposes saturating turn and cursor-admission totals. The summary is
observational only: it does not select work, change admission, impose quotas or
budgets, defer stages, alter deadlines, or affect rendering. The
`NativeCpuFrameFairnessDiagnostics::latest_due_lateness_us` field is optional,
saturating missed-presentation-deadline evidence in microseconds measured at
the latest turn's original cadence `due_at` boundary; it is `None` for waiting,
idle/not-applicable, unknown, absent, or evicted state. This field is
observational and never changes scheduling policy. It is populated
only when the explicitly registered `RuntimeFrameDiagnosticsHost` capability is
enabled and is attached after the existing frame-observation and schedule-
admission publication gates.
`NativeFrameDiagnostics::cpu_observation` is an opt-in, bounded observational
summary of the existing parent-owned CPU frame observation ledger for one
window. It exposes only whether a completed sample is available, the latest
`NativeCpuFrameCompletionOutcome`, whether that latest frame had exact routed
interaction evidence in `latest_exact_interaction`, and the ledger's
saturating admitted, successful, skipped-or-vetoed, incomplete, failed, and
recovery-triggered totals (`admitted_redraws`, `successful_presentations`,
`skipped_or_vetoed_redraws`, `incomplete_frames`, `failed_frames`, and
`recovery_triggered_frames`). When
`available` is false, `latest_outcome` is `Unknown` and all boolean/counter
fields remain at their zero/default values; this means the evidence is
unavailable, including when frame diagnostics are disabled or the bounded
window key was not retained. The summary is observational only: it does not
select work, change admission, route input, render, or alter publication
ordering. Primary evidence is attached at the existing
`publish_staged_frame_diagnostics` boundary, while auxiliary evidence uses the
existing parent-owned ledger/key at `forward_auxiliary_frame_diagnostics`.
`NativeFrameDiagnostics::frame_sequence` is an `Option<u64>` monotonic sequence
scoped to one native window. It starts at `1` and is allocated only after a
successful presentation, so it remains monotonic across recovery. It is `None`
before the first presentation or after the counter is exhausted; it never wraps
or reuses a value. The native render-profile and slow-profile tracing lines
include the same `frame_sequence` field when a sequence is available.
`NativeFrameDiagnostics::window_identity` is an opaque, read-only identity for
the native runner that presented the frame. The pair
`(window_identity, frame_sequence)` uniquely correlates a presented frame across
the primary and auxiliary windows. Identities are allocated from `1` within one
native runtime run: the primary runner receives `1`, and each newly admitted
auxiliary runner receives a fresh checked value. An identity remains fixed for
that runner across frames, hide/show, cache-on-close, surface or renderer
reconstruction, target-generation changes, and device recovery. Destroying a
runner does not make its identity reusable; a recreated runner gets a fresh
identity. When the parent allocator is exhausted, new auxiliary admission
receives `None` without wrapping or reusing an identity and without changing
scheduling or presentation. This diagnostic identity is distinct from the
public logical `WindowKey` and from the auxiliary projection key. The native
render-profile and slow-profile tracing lines expose the same numeric
`window_identity` alongside `frame_sequence`.
`NativeFrameDiagnostics::input_to_present_latency_us` is an opt-in,
saturating-microsecond measurement from Radiant's native event-loop arrival of
the latest tracked interactive event to the next successful presentation. It
uses a bounded latest-wins slot: a newer tracked arrival replaces an older
unpresented arrival, failed or absent presentations retain the arrival, and a
successful presentation consumes it once. The measurement starts at
event-loop arrival and does not include platform queue time before event-loop
arrival; it is not a native host input timestamp. Radiant tracks the already
routed `Focused`, `CursorEntered`, `CursorMoved`, `CursorLeft`, `MouseInput`,
`MouseWheel`, `KeyboardInput`, and `ModifiersChanged` window events only.
Auxiliary windows own their value and forward it through the existing parent
diagnostics boundary without changing publication order.
`NativeFrameTimingDiagnostics::gpu_timing_status` continues to report
`NativeGpuTimingStatus::CpuEnvelopeOnly`, which keeps the existing diagnostics
fields as CPU-side encode/submit/present envelopes. The primary native GPU
timestamp producer uses the separate correlated callback below and does not
change the meaning of these existing fields; they remain not backend GPU
timestamp query durations.
Frame timings are grouped into `frame_work`, `composited_base`, and
`transient_overlay` buckets so hosts can inspect related work without treating
the diagnostics payload as one flat timing bag.
Use `NativeFrameTimingDiagnostics::cpu_envelope_total()` for a single tracked
CPU-side frame-work total; it excludes `since_last_present`, which is frame
cadence rather than work performed for the current frame. The native render
profile log emits the same tracked total as `frame_cpu_envelope_total_us` for
profiling.
`NativeRunOptions::frame.retained_surface_cache` accepts
`RetainedSurfaceCachePolicy` for apps that need to tune or disable retained
custom-surface frame reuse during profiling.
`NativeFrameDiagnostics::text` groups native text diagnostics into
`cache.layout`, `cache.atom`, and `quality` counters. The cache groups expose
layout-cache and text atom-cache hits, misses, and evictions; the quality group
exposes shaping-sensitive run/scalar counts and fallback/missing glyph counts
so hosts can detect repeated text measurement, cache churn, basic-layout
Unicode limits, or font coverage gaps without parsing renderer logs.
`NativeTextDiagnostics::has_shaping_limits()`,
`has_font_coverage_gaps()`, and `has_text_quality_warnings()` provide the
stable summary predicates applications can use for debug overlays, telemetry, or
local quality gates without duplicating raw counter policy.
`NativeTextDiagnostics::quality_status()` returns a `NativeTextQualityStatus`
classification, and the native render profile emits the same policy as
`text_quality_status`, so hosts can distinguish clean frames, shaping-limited
frames, font-coverage-limited frames, and frames with both issues.

### Fixed-cost native frame profiling

`ProfilingOptions` configures the first bounded public profiling path through
`ProfilingMode::Off` (the default) or `ProfilingMode::Frame`:

```rust
app.profiling(ProfilingOptions::frame())
    .on_frame_profile(|state, profile| {
        // Inspect or retain the backend-neutral profile in application policy.
        let _ = (state, profile);
    });
```

`WindowBuilder::profiling(...)` and `StatefulAppBuilder::profiling(...)` carry
the same option into `NativeFrameOptions`. Stateful applications receive
profiles through `StatefulAppWithView::on_frame_profile(...)`; lower-level
runtime hosts opt in by implementing `RuntimeFrameProfileHost` and registering
`RuntimeHostCapabilities::with_frame_profile()`.

`FrameProfile` is a copyable, backend-neutral projection of the existing native
frame diagnostics. It contains the optional native window identity and
successful-presentation sequence, input-to-present latency, stable work and
invalidation labels, fixed CPU timing groups, and bounded scene/text/surface,
recovery, fairness, and observation counters. `FrameProfile::from_native_frame_diagnostics(...)`
is available when a host needs to project an existing diagnostics value itself.

Profile publication occurs only after successful presentation and uses the same
bounded primary publication boundary and auxiliary parent handoff as native
frame diagnostics. Auxiliary windows evaluate their own profiling option, and
delivery remains ordered with existing diagnostics before application messages.
An exhausted frame sequence is represented as `None` rather than suppressing a
profile. `FrameProfileGpuTimingStatus::Unavailable` is explicit: current native
timing fields are CPU-side envelopes and are never relabeled as GPU timestamps.

`FrameGpuTimingSample` is the separate correlated GPU-timing callback contract:

```rust
app.profiling(ProfilingOptions::frame())
    .on_frame_profile(|state, profile| {
        let _ = (state, profile);
    })
    .on_frame_gpu_timing(|state, sample| {
        let _ = (state, sample);
    });
```

Stateful applications register this callback with
`StatefulAppWithView::on_frame_gpu_timing(...)`; lower-level hosts implement
`RuntimeFrameGpuTimingHost` and opt in with
`RuntimeHostCapabilities::with_frame_gpu_timing()`.

The sample's aggregate interval starts at the first frame-owned GPU command and
ends after final composition; CPU present and display/scanout are excluded.
`FrameGpuTimingOutcome` is either an available duration or an explicit
`Unavailable` reason (`NoWork`, `Unsupported`, `CapacityRefused`,
`MappingFailed`, or `ConversionFailed`). The generic native primary and
auxiliary runners emit the supported, unsupported, capacity-refused,
mapping-failed, and conversion-failed terminal outcomes through this callback
when that window's frame profiling and the observer are both enabled. Auxiliary
samples retain their exact window identity and successful-present frame sequence
when forwarded through the existing parent handoff and ordering boundary.
Delivery is independent of the single successful-present `FrameProfile` callback,
whose existing profiling option and delivery semantics remain unchanged. Stale
or lifecycle-invalid completions publish nothing; device-loss/recovery and
shutdown retain bounded conservative cancellation rather than comprehensive
timing draining.

This surface intentionally does not provide `Detailed(ProfileSelection)`, runtime
mode switching, a debug inspector, or backend GPU timestamp queries. Renderer-
owned resource lifetime/budgeting and live native-window acceptance remain
backend capabilities. macOS live acceptance runs on the M5 Pro development
host; current Linux and Windows CI is limited to portable/build/compile/check
evidence. The target GitHub Actions lanes must eventually add integration and
headless Wayland/native-host smoke coverage where runners permit; until then,
no Linux/Windows host, IME, accessibility, presentation, latency, GPU, or
performance acceptance is established.

### macOS live frame-profile acceptance

Policy classification: `N/NOT_RUN` for the live desktop procedure. The
[Platform Acceptance and Evidence Policy](PLATFORM_ACCEPTANCE.md) requires a
qualifying manifest before this procedure can support a native acceptance
claim.

`macos_frame_profile_acceptance` is the checked public-API harness for native
Off/Frame acceptance. It is explicitly macOS-only. The selected modes are read
once at startup; restart the harness to change them. The callback retains only
fixed counters and last-value fields, so it performs no file I/O, unbounded
logging, or unbounded collection on the presentation path.

Build the checked example and stage it as a normal macOS application:

```bash
cargo build --example macos_frame_profile_acceptance
export RADIANT_DEV_APP_NAME=RadiantFrameProfileAcceptance
export RADIANT_DEV_APP_BINARY="$PWD/target/debug/examples/macos_frame_profile_acceptance"
scripts/dev_app_bundle.sh --main=off --aux=off
```

For a direct checked-example smoke run without the `.app` wrapper, use
`cargo run --example macos_frame_profile_acceptance -- --main=off --aux=off`.

Repeat the final command with each startup configuration below. Close the
application between runs so each recorder starts from zero:

```bash
scripts/dev_app_bundle.sh --main=frame --aux=off
scripts/dev_app_bundle.sh --main=off --aux=frame
scripts/dev_app_bundle.sh --main=frame --aux=frame
```

Use the visible primary `Record click` and auxiliary `Record auxiliary click`
controls as needed, close the auxiliary window to expose the primary when
needed, then resize the primary window during each run. Inspect the bounded
recorder text in both windows. Expected evidence is:

- `--main=off --aux=off`: zero primary and auxiliary profile callbacks.
- `--main=frame --aux=off`: at least two primary successful-present profiles,
  one stable primary identity, strictly increasing available sequences, and
  `FrameProfileGpuTimingStatus::Unavailable` for every recorded profile; the
  auxiliary callback count remains zero.
- `--main=off --aux=frame`: zero primary callbacks and at least two auxiliary
  successful-present profiles with stable identity and increasing sequences.
- `--main=frame --aux=frame`: both windows expose at least two profiles; the
  auxiliary identity differs from the primary identity and the recorder shows
  the auxiliary primary handoff callbacks.

This is live native macOS presentation evidence only. It does not claim
Linux/Windows presentation, runtime profiling-mode switching, or GPU timestamp
queries. Current Linux/Windows CI is limited to portable/build/compile/check
evidence; the target headless Wayland/native-host smoke lanes are future
evidence, not current acceptance.

### macOS live devtools acceptance

Policy classification: `N/NOT_RUN` for the live desktop procedure. The
[Platform Acceptance and Evidence Policy](PLATFORM_ACCEPTANCE.md) requires a
qualifying manifest before this procedure can support a native acceptance
claim.

`macos_devtools_acceptance` is the checked public-API harness for the existing
runtime-local devtools overlay. It enables the overlay through
`radiant::app(...).devtools_overlay(DevtoolsOverlayOptions::enabled())`, uses
ordinary buttons, a toggle, and one text input, and is explicitly macOS-only.
The non-macOS checked target returns an explicit unsupported error; it does not
claim native overlay acceptance on other platforms.

The inspector is observational and uses the app's normal hit testing and focus
paths; it does not take focus or block interaction.

The harness keeps only fixed-size application state: a bounded action counter,
one input string capped at 64 Unicode characters, one toggle value, and one
last-action string. It performs no file I/O or unbounded logging. Build and
stage it as a normal macOS app:

```bash
cargo build --example macos_devtools_acceptance
export RADIANT_DEV_APP_NAME=RadiantDevtoolsAcceptance
export RADIANT_DEV_APP_BINARY="$PWD/target/debug/examples/macos_devtools_acceptance"
scripts/dev_app_bundle.sh
```

For a direct checked-example smoke run without the `.app` wrapper, use
`cargo run --example macos_devtools_acceptance`.

With the overlay visible at startup, move the pointer across both buttons, the
toggle, and the text input. The selected tree row, hover state, selected-node
metadata, and highlighted bounds should follow normal hit testing. Click and
edit the text input, then use Tab/Shift-Tab to traverse focusable controls;
focus state should change without the overlay taking focus or blocking
interaction. Resize the primary window with a control selected and confirm the
selected bounds and tree geometry update while the controls remain usable.
This is live native macOS presentation evidence only. Current Linux/Windows CI
is limited to portable/build/compile/check evidence; the prescribed headless
Wayland/native-host smoke lanes are a future target where runners permit, and
no Linux/Windows host, IME, accessibility, presentation, latency, GPU, or
performance acceptance is established.

### macOS live external-drag acceptance

Policy classification: `M/NOT_RUN` for the manual native Finder procedure. The
[Platform Acceptance and Evidence Policy](PLATFORM_ACCEPTANCE.md) keeps a
documented procedure separate from a completed hardware/user result.

`macos_external_drag_acceptance` is the checked public-API harness for outgoing
file drags. It uses `drag_handle()`,
`UiUpdateContext::begin_drag_with_external(...)`, and
`ExternalDragRequest::files(...)` so the in-window preview and native file
payload are armed as one gesture. The live macOS path creates one disposable
source file in the system temporary directory and removes it when the harness
exits; the non-macOS fallback and tests use a synthetic relative path and do
not create a temporary source.

The harness reports a bounded callback count, terminal `ExternalDragEffect`,
whether the terminal outcome was accepted, and whether the completion mapper
has received its terminal result. Build and run it directly on macOS with:

```bash
cargo run --example macos_external_drag_acceptance
```

Drag the handle out of the window into Finder or another file receiver, then
wait for the native session to finish. A successful terminal copy should show
`Terminal effect: copy`, `Accepted: true`, and `Callback terminal: true`; a
cancelled or rejected session should report `none` and `false` instead. This
section documents the manual acceptance procedure and does not claim that a
live Finder run has been performed.

### macOS numeric accessibility acceptance

Policy classification: current policy-compliant evidence is `N/NOT_RUN` because
no complete policy manifest is recorded. A historical bounded AppKit/Computer
Use result is retained below as bounded N evidence; it is not VoiceOver or
release evidence. VoiceOver-specific acceptance remains `M/NOT_RUN`. The
[Platform Acceptance and Evidence Policy](PLATFORM_ACCEPTANCE.md) keeps those
claims scoped to the exercised adapter and screen-reader capability.

`macos_numeric_accessibility_acceptance` is the checked public-API harness for
ordinary materialized `NumericInput` accessibility increment and decrement
actions. It builds the control through the application builder and reports
updates through the normal runtime dispatcher. The non-macOS checked target
returns an explicit unsupported error; it does not claim native accessibility
acceptance on other platforms.

Build and run the checked example directly with:

```bash
cargo build --example macos_numeric_accessibility_acceptance
cargo run --example macos_numeric_accessibility_acceptance
```

On macOS, use VoiceOver or an AX client to invoke the control's increment and
decrement actions and inspect the application-owned value and status. The
automated example tests inspect the production runtime projection and reducer
shape only. Exact fresh-bundle activated Computer Use/AppKit evidence verifies
discoverability and numeric action, bounded set-value, and restart acceptance
for this bounded primary-window consumer: the stepper moved from `42.00` to
`43.00` and back to `42.00`, bounded `SetValueText` produced `55.50` and
`57.25` with fresh reads showing normal app-owned Begin/Update/Commit events,
and a fresh restarted instance exposed the same tree. VoiceOver-specific
acceptance remains unperformed; repeated negative-geometry AppKit runtime
diagnostics remain a separate unverified follow-up if reproducible.
Current Linux/Windows validation remains limited to portable build, compile,
and test evidence.

### macOS live Japanese IME acceptance

Policy classification: `M/NOT_RUN` for the live Japanese IME procedure. The
[Platform Acceptance and Evidence Policy](PLATFORM_ACCEPTANCE.md) does not
allow deterministic projection checks or a documented command to substitute
for native IME evidence.

`macos_text_input_ime_acceptance` is the checked public-API harness for the
already-shipped single-line `TextInput` path in the primary window. The
non-macOS checked target returns an explicit unsupported error; it does not
claim native IME acceptance on other platforms.

Build and run the checked example directly on macOS with:

```bash
cargo build --example macos_text_input_ime_acceptance
cargo run --example macos_text_input_ime_acceptance
```

Keep the primary window focused, select the Japanese Hiragana IME, type romaji,
and observe the underlined preedit, candidate panel, and caret. Choose a
candidate or press Return to commit and confirm one committed application-state
change. Start another composition and press Escape to cancel; then repeat a
composition and switch focus to another application or window to confirm the
committed value is restored without a change. The status in the harness is
application-owned and counts changed messages. The automated tests inspect only
the production runtime projection, including the focused caret-area source used
by the existing native Winit publication path; they do not claim live AppKit or
candidate-panel evidence. Native Japanese IME acceptance remains unperformed
until these live steps are actually run.

## Examples And Sandboxes

Radiant examples are maintained API and sandbox contracts. They should compile
as checked example targets and participate in normal example validation:

```powershell
cargo test --examples
```

Use the example set as a target-area map when choosing the smallest sandbox for
manual validation:

| Target area | Focused examples |
| --- | --- |
| First-use application API | `hello_world`, `generic_native`, `counter` |
| State, commands, and background work | `todo_list`, `message_routing`, `background_loading`, `status_bar`, `list_actions`, `animation_showcase` |
| Localization and shortcut presentation | `localization_foundation` |
| Typed pointer admission and capture continuity | `typed_pointer` |
| Layout, scrolling, and virtualization | `layout_rows_columns`, `custom_layout`, `split_pane_static`, `split_pane_runtime`, `grid_gallery`, `scroll`, `controlled_scroll`, `sizing`, `list`, `virtualized_list` |
| Logical semantic provider attachment | `logical_provider_attachment` |
| Styling, theming, and reusable widgets | `styling`, `theme_playground`, `widget_gallery`, `toolbar_icons`, `svg`, `form`, `volume_slider`, `passive_widgets` |
| Input, focus, menus, and editor interactions | `focus_controls`, `keys`, `scene`, `context_menu`, `floating_overlay`, `tree_and_details`, `folder_browser`, `paint_helpers` |
| Custom widgets and retained GPU surfaces | `custom_widget`, `curve_area_fill`, `render_canvas`, `custom_shader_surface`, `render_canvas_stack_overlay`, `waveform_view`, `spectrogram` |
| Advanced creative-tool surfaces | `node_editor`, `timeline_editor`, `plugin_panel`, `eq_editor`, `spectrogram`, `mixer_console`, `piano_roll`, `modulation_matrix`, `arrangement_shell`, `inspector_panel`, `split_workspace` |
| Text, diagnostics, and performance inspection | `typography`, `layout_diagnostics`, `rendering_benchmark`, `host_surface_frame`, `macos_frame_profile_acceptance`, `macos_devtools_acceptance`, `macos_text_input_ime_acceptance` |
| Window and host integration | `multi_window_manifest`, `popup_window`, `host_surface_frame`, `dpi_scaling`, `macos_frame_profile_acceptance`, `macos_devtools_acceptance`, `macos_external_drag_acceptance`, `macos_numeric_accessibility_acceptance`, `macos_text_input_ime_acceptance` |

Run `cargo run --example logical_provider_attachment` to inspect the portable
declarative provider attachment shape; the qualified custom-coordinate resolver
is an additional application-owned option on the same parts declaration.

Run `cargo run --example controlled_scroll` to inspect generation-fenced
controlled offsets, one-shot reveal requests, and settled offset messages.

For multi-region application shells, use `workspace_shell(main_workspace)` when
the readable app shape is a top bar, central workspace row, optional leading or
trailing sidebars/panels, and optional status bar. The builder composes ordinary
Radiant views through `top_bar(...)`, `leading_sidebar(...)`,
`trailing_sidebar(...)`, `status_bar(...)`, and view-local `overlays(...)`;
applications still own panel state, product copy, and region contents. Keep
`row(...)` and `column(...)` for small custom layouts, and use
`workspace_shell(...)` when the shell structure is itself the public contract a
reader or test should recognize. The
`arrangement_shell` example demonstrates this contract without making DAW,
transport, clip, or mixer semantics part of Radiant.

Some maintained examples are intentionally advanced synthetic domain
simulations rather than canonical API-contract starters. They validate dense
control panels, retained custom-widget painting, runtime-local hover/drag
previews, high-frequency frame updates, and message routing under realistic
interaction pressure, but they do not define Radiant-owned product semantics:

| Advanced simulation | Radiant behavior it validates | Non-authoritative domain behavior |
| --- | --- | --- |
| `plugin_panel` | compact control-panel layout, toggles, and explicit messages | plugin preset or host lifecycle policy |
| `eq_editor` | custom response-curve widget paint and handle routing | DSP, analyzer, or audio-processing behavior |
| `mixer_console` | dense rows, meters, faders, drag previews, and multi-selection | mixer, channel, send, solo, mute, or DSP semantics |
| `piano_roll` | retained canvas editing, gesture previews, selection overlays, and frame overlays | MIDI note editing, quantization, piano-key semantics, velocity editing, or DAW workflow policy |
| `modulation_matrix` | dense matrix interaction, hover overlays, and value editing | synthesizer modulation-routing semantics |
| `arrangement_shell` | multi-pane workspace composition, timeline paint, and paint-only hover/playhead overlays | DAW arrangement, clips, tracks, transport, mixer, or audio behavior |

Run an example interactively with `cargo run --example <name>`. Showcase
examples use portable defaults. `folder_browser` accepts an optional root for
real local data while keeping mutations inside the example sandbox:

```powershell
cargo run --example folder_browser -- C:\path\to\root
$env:RADIANT_FOLDER_BROWSER_ROOT = "C:\path\to\root"
```

If no folder root is supplied, `folder_browser` uses an in-memory resource
sandbox. Supplying a root path loads a read-only tree/details snapshot for UI
exploration; create, rename, delete, and drag-move interactions still mutate
only the example's in-memory resource graph. Host applications own real file
management policy. `waveform_view` uses a generated synthetic signal by default
and accepts `RADIANT_WAVEFORM_PATH` for optional host-side input exploration.
Run `cargo run --example waveform_view` to inspect the default synthetic signal
path.
`waveform_view` uses deterministic synthetic signal data to exercise waveform
summaries, viewport interaction, overlay painting, and GPU-surface projection
without teaching file decoding or audio preprocessing as Radiant API guidance.
The waveform view keeps the dense signal body in a
retained `RenderCanvasContent::SignalSummaryBands` surface. It still
demonstrates the advanced launch-level `.animated_transient_overlay_at(...)`
hook for a playback playhead anchored through
`SurfacePaintPlan::first_widget_rect`; new root app composition should prefer
`Scene::overlay(...)` for paint-only transient presentation unless direct
lifecycle wiring is specifically needed.
Run `cargo run --example generic_native` for the compact native-runtime starter
that demonstrates the current application-builder first-use path.
Run `cargo run --example hello_world` for the smallest windowed app skeleton.
Run `cargo run --example counter` for a minimal state-update and button message
flow.
Run `cargo run --example localization_foundation` for an explicit application
environment source that switches a visible localized label and its compact and
spoken shortcut help text while preserving the existing shortcut matcher.
Run `cargo run --example todo_list` for text input, submit binding, row
selection, drag handles, drop markers, and scroll composition in one small app.
Run `cargo run --example form` for text binding and boolean controls.
Run `cargo run --example render_canvas` for a small retained-canvas sandbox
that uses the prelude `render_canvas(...)` application builder with generated
demo atlas data.
Run `cargo run --example custom_shader_surface` for a checked custom shader
surface sandbox that builds `RenderCanvasContent::CustomShader` with a
backend-neutral `RenderCanvasShaderSurfaceDescriptor` carrying executable WGSL source
for the native surface-uniform ABI. Native runs expose custom shader
render/cache/failure diagnostics; shader module, pipeline, or bind-group
validation failures are counted separately from missing handoff data. Backends
without a matching shader handoff still report the surface through
`NativeGpuSurfaceDiagnostics::custom_shader.unsupported.surfaces` and the
related skipped vertex/source/uniform/storage counters rather than creating a
separate WGPU-facing application API.
Run `cargo run --example multi_window_manifest` for a checked manifest sandbox
that uses `WindowManifest` to describe multiple windows and separate views
without expanding the native runtime event loop.
Run `cargo run --example popup_window` for a launcher-and-popup sandbox: the
normal main window lets you choose a popup mode, starts a real borderless
Radiant popup window in a child process, and the popup can be dragged by its
title area or closed through the normal runtime exit command. This demonstrates
the current host-owned
multi-window adapter boundary while keeping transient UI surfaces on the same
Radiant app and widget model.
Run `cargo run --example layout_diagnostics` for a layout diagnostics sandbox
that collects `LayoutDiagnostic` entries and debug primitives from
`LayoutDebugOptions::all_enabled()`.
Run `cargo run --example virtualized_list` for a large application-builder list
sandbox that keeps 10k selectable rows responsive through
`virtual_list_windowed(...)`. Use the windowed helper for large fixed-height
lists so projection stays bounded to a `VirtualListWindow`; use
`virtual_list_materialized_windowed(...)` when app state already owns the
materialized rows for the current window; use `virtual_tree_list_window(...)`
when a fixed-height tree or outline should compose materialized rows with
standard guide overlays, including style-resolved `StyledTreeGuideStyle`
overlays when guide color should follow the active theme; use
`virtual_tree_list_windowed(...)` for the same tree-guide composition when
runtime scroll-window changes should be emitted as ordinary app messages; use
`virtual_list_window_body(...)` when the materialized window needs a shared
overlay or grouped row body outside the standard tree-guide case. Smaller
eagerly projected lists should use `list(...)`, `scroll_column(...)`, or
`bounded_scroll_column(...)` so large-list virtualization is reserved for
window-owned projection.
Run `cargo run --example inspector_panel` for a compact inspector/property
panel sandbox that uses `PropertyRow`, `property_rows(...)`,
`property_panel(...)`, and `message_selectable_property_panel(...)` on the same
application-builder path as other stateful examples. `property_rows(...)`
builds read-only property rows without adding a titled panel shell, so host
applications can embed standard inspector rows inside app-owned panel sections.
`property_panel(...)` is read-only and can be used with any host message type;
use `message_selectable_property_panel(...)` when property rows should emit
host messages handled by the app reducer. Compact titled panels with optional
header actions can use
`PanelSectionParts`, `panel_section(...)`, `panel_section_from_parts(...)`, and
`closeable_panel_section_from_parts(...)` instead of rebuilding title rows,
close buttons, padding, spacing, and neutral panel chrome in application code.
Use `PanelSectionHeaderParts` with `panel_section_from_header_parts(...)` when
the app owns a custom header view, such as a resize strip, segmented toolbar, or
compact tab row, but Radiant should still own the standard panel container
chrome.
Use `PanelSectionHeaderParts::resize_header(...)` when that custom header is
Radiant's standard full-width hover-only resize strip; add `.header_id(...)`
when tests, automation, or host integrations need a stable id for the header
separately from the section container.
Use `PanelSectionGeometry` when app-owned resize constraints or fixed-content
panels need the same panel padding, title-height, and spacing calculations
without constructing view parts.
Use `PanelSectionParts::trailing_resize_handle(...)` when a resizable titled
panel should use Radiant's standard compact drag handle while the host reducer
keeps owning durable size, constraints, and resize messages.
Use `panel_section_resize_header(...)` when a collapsible panel needs the whole
header strip to act as a subtle hover-only resize hit target while the host
still owns durable size and collapse policy.
Compact control panels can use `LabeledControlParts`,
`labeled_control(...)`, `labeled_control_from_parts(...)`, and
`labeled_control_control_offset(...)` for label-over-control groups and overlay
anchors without repeating label text styling and stacked spacing.
Use `form_row(...)` when a panel needs compact horizontal label/control rows
with Radiant-owned label width, row padding, spacing, and hover behavior. Use
`dense_form_row(id, label, control, label_width)` when a sidebar filter,
popover editor, or compact inspector row needs the same label/control geometry
with a caller-chosen label width, but without row padding or hover chrome
because the surrounding panel already owns that feedback. Use `FormRowParts`
and `form_row_from_parts(...)` only when the row needs custom metrics, style, or
selected-state policy beyond those normal forms.
Use `button_row(...)` or `button_row_from_parts(...)` when dialogs, popovers,
inspectors, or utility panels need a compact horizontal group of app-owned
buttons with Radiant-owned spacing and row height.
Use `ToolbarParts`, `ToolbarAlignment`, `toolbar(...)`, or
`toolbar_from_parts(...)` when top bars, transport strips, inspector toolbars,
or similar app-owned control strips need Radiant-owned height, padding, spacing,
alignment, and optional trailing controls.
`toolbar(...)` honors its controls' intrinsic heights, including application
text scale, with a 34-pixel minimum strip height and physical vertical padding.
For example, a default 36-pixel button produces a 42-pixel strip at text scale
1 and a 78-pixel strip at scale 2. `toolbar_from_parts(...)` retains the fixed
physical height declared by `ToolbarParts`; `.height(...)` also remains an
explicit physical constraint. Both paths share the controls' localized visible
and accessible labels and logical RTL ordering.
Centered fixed-size foreground surfaces can use `CenteredLayerParts`,
`centered_layer(...)`, and `centered_layer_from_parts(...)` instead of
rebuilding spacer rows and columns in application code.
Fixed-size foreground surfaces that need edge or center placement can use
`AnchoredLayerParts`, `LayerHorizontalAnchor`, `LayerVerticalAnchor`,
`anchored_layer(...)`, and `anchored_layer_from_parts(...)` for generic
top/center/bottom and left/center/right placement with edge insets.
Arbitrary floating content that should sit above or below a trigger rectangle
inside a caller-owned stack layer can use `FloatingLayerAnchorParts`,
`FloatingLayerPlacement`, `floating_layer_above(...)`,
`floating_layer_below(...)`, and `floating_layer_around_from_parts(...)`
instead of hand-computing popup, autocomplete, tooltip, or compact editor
offsets in application code.
Use `AnchoredPopoverParts`, `AnchoredPopoverAnchor`,
`anchored_popover_from_parts(...)`, and
`dismissible_anchored_popover_from_parts(...)` for the preferred anchored
popover path when content needs trigger-relative or pointer-relative placement,
horizontal viewport clamping, bottom-edge flipping, interactive hit testing,
and optional outside-click dismissal as one primitive. Dropdowns, context
menus, and custom app popovers should wrap this path rather than rebuilding
spacer rows or separate overlay geometry.
Fixed-row transient lists can use `BoundedScrollColumnParts`,
`bounded_scroll_column(...)`, and `bounded_scroll_column_from_parts(...)` so
application code projects domain-specific rows while Radiant owns capped
height, empty-list elision, scroll wrapping, padding, and viewport styling.
Dropdown overlays anchored to a trigger can use
`DropdownMenuOverlayBelowParts`, `dropdown_menu_overlay_below(...)`, and
`dropdown_menu_overlay_below_from_parts(...)` so application code supplies the
trigger rectangle and gap rather than hand-adding trigger height to menu
coordinates. Use `dropdown_menu_overlay_below_labeled_control(...)` when a
standard dropdown trigger is nested inside Radiant's compact `labeled_control`
row and the overlay should be anchored from the row top. Use
`dropdown_trigger(...)` when the toggle should stay in normal layout while the
menu is projected as a separate stack-level overlay.
Transient dropdowns, menus, and popovers can use `dismissible_overlay(...)`
when foreground overlay content should sit above a transparent outside-click
dismiss layer while preserving the base content underneath. Use
`dismissible_overlay_with_interactive_base(...)` for dropdown groups where
clicking another trigger should switch menus instead of only closing the
current one. Fixed-size titled popovers, dialogs, and inspector panels that use
Radiant's standard dialog chrome can use `DialogLayerParts`,
`dialog_layer_from_parts(...)`, or `closeable_dialog_layer_from_parts(...)` to
keep title, content, tone, size, full-surface anchored placement, and optional
close routing in one generic contract. Use `dialog_layer(...)` or
`closeable_dialog_layer(...)` for the common centered fixed-size dialog case.
Use `PanelSectionLayerParts`,
`panel_section_layer_from_parts(...)`, or
`closeable_panel_section_layer_from_parts(...)` when a fixed-size anchored
surface needs custom panel-section parts or non-dialog chrome. Use
`PanelSectionParts::dialog(...)` when a modal, popover, or floating utility
panel should use Radiant's standard strong dialog chrome inside another
panel-section composition.
Use `dropdown_menu_overlay_below_stacked_labeled_control(...)` when a dropdown
trigger lives inside a compact stacked labeled-control panel and the menu should
anchor below the current `StackedLayoutCursor` item without repeating label
offset arithmetic in the host.
Context menus use one fluent `context_menu(title, commands)` entry point.
Supply the required surface point with `.anchor(...)`; the builder then uses
Radiant's compact automatic-width policy and foreground-only placement by
default. Use `.width_policy(MessageMenuWidthPolicy)` for custom min/max bounds,
`.width(...)` for a deliberately fixed width with standard compact height, or
`.size(...)` for an exact logical size. Add `.dismiss_on(message)` when the
menu should own a full-surface outside-click backing; omit it when a `Scene`
layer owns dismissal through `Layer::dismiss_on_outside_click(...)`. Finish the
configuration with `.view()`. This keeps sizing, anchoring, styling, and
dismissal discoverable on one builder and avoids app-local
`message_menu_height(...)` arithmetic for the common automatic-width case.
Run `cargo run --example scene` for the preferred root-scene sandbox. It
keeps the root `Scene` focused on base layout, shortcuts, frame clocks, and
paint-only overlays while status-bar, browser, and workspace components declare
their own popovers, context menus, modals, tooltips, and drag previews as
view-local transient layers.
Run `cargo run --example native_file_drop` for a view-local native OS file-drop
target that maps `NativeFileDrop` events into normal app messages.
Run `cargo run --example context_menu` for a generic menu/context-menu sandbox
that composes `MenuCommand`, `message_menu(...)`, and `context_menu(...)` with
normal app messages.
Run `cargo run --example floating_overlay` for a floating-layer sandbox that
positions an overlay menu without changing the underlying page layout.
Run `cargo run --example split_workspace` for an editor-style split workspace
that uses `SplitPaneSidebarState`, `SplitPaneSlot`, and generic Radiant views
without adding docking-specific runtime concepts.
Run `cargo run --example node_editor` for a node-editor-style workspace that
composes retained canvas metadata, connection markers, draggable card stacks,
selectables, and port rewiring through public application builders.
Run `cargo run --example timeline_editor` for a timeline-editor-style sandbox
that projects `TimelineSurfaceState`, `TimelineMotionState`, retained canvas
metadata, marker selection, and transport controls through normal app views.
Run `cargo run --example animation_showcase` for an advanced frame-driven UI
sandbox that uses the lower-level `.animation(...)` and `.on_frame(...)`
stateful application hooks. Prefer `Scene::frame_clock(...)` for new root
surface frame-message animation.
Run `cargo run --example render_canvas_stack_overlay` for a retained GPU surface
with normal widget overlays plus a transient animated blob that repaints every
frame through the advanced launch-level `.animated_transient_overlay_at(...)`
hook without refreshing the declarative surface, rebuilding the cached Vello
scene, or recompositing the stable retained GPU surface on every overlay-only
frame. The overlay caps its paint-only cadence to 60 FPS and anchors to the
cached GPU-surface rectangle through `SurfacePaintPlan::first_widget_rect`.
Prefer `Scene::overlay(...)` for normal root-scoped paint-only presentation.
Run `cargo run --example background_loading` for a background-work sandbox that
uses `ResourceSlot`, `ResourceCompletion`, and
`UiUpdateContext::business().background(...).resource(...)` to route worker
resource results back into the normal state update path.
Run `cargo run --example typography` for a focused text sandbox that exercises
wrapping, truncation, fixed text heights, fill sizing, and explicit baselines
through the application-builder API.
Run `cargo run --example widget_gallery` for a reusable-widget gallery that
shows `badge(...)`, `selectable(...)`, and passive `card()` composition through
the prelude builders. Use `badge(...).passive()` when a styled or active badge
should paint without emitting host messages. Use `interactive_badge(...)` when
a badge or pill should keep standard badge chrome while emitting generic
dense-row interactions such as primary activation, secondary activation, drag,
drop, or drop-hover. This is useful for labels, chips, tags, tokens, and
compact filter pills that need richer interaction than a simple badge click
without hand-building transparent input overlays in application code. Use
`InteractiveBadgeBuilder::tracked_drag_source(drag_active, drag_source)` when
host-owned badge drag state should configure draggable, drag-active,
drag-source, and pointer-motion policy together; use
`tracked_drag_source_with_motion(...)` when the retained active badge source
should keep emitting pointer movement after projection. Use
`tracked_drop_candidate(...)` when badge or pill drop candidacy is
host-validated but Radiant should own target-enter and stale-target clear
routing.
Run `cargo run --example custom_widget` for a custom widget authoring sandbox
that implements paint and input dispatch through the public widget trait.
Run `cargo run --example volume_slider` for a focused parameter-control sandbox
that uses the prelude `slider(...)` builder, horizontal value changes, and a
checkbox-backed mute state through explicit value messages.
Run `cargo run --example list_actions` for a compact stateful list sandbox
with selectable rows, stable row IDs, insertion, removal, and small `+` / `-`
row actions.
Run `cargo run --example toolbar_icons` for a horizontal SVG-icon toolbar
sandbox that uses custom toggle buttons, state-driven active highlights, and
muted inactive vector icons. Compact action strips should use direct
`row(...)`, `spacer()`, padding, spacing, and sizing composition so the
application owns product-specific toolbar structure.
Run `cargo run --example svg` for a focused SVG icon sandbox that parses
inline vector assets through `SvgIcon::from_svg(...)` and paints them through
the standard `icon_button(...)` builder. For common compact controls, use
`close_button()` and `disclosure_button(expanded)` so apps do not repeat literal
text labels or parse their own standard close/disclosure icons. Icon-button
builders support both message-style `.message(...)` routing and direct
callback routing for compatibility. Use `.message(...)` for normal
application interactions. Use `.label(...)` on an icon-button builder when an
icon-only action needs an exact automation name; it exposes a button role and
the supplied label independently of any visual `.tooltip(...)`. Use
`icon_button(...).passive()`,
`close_button().passive()`, or `disclosure_button(expanded).passive()` when a
standard icon should paint as decorative chrome while another parent surface
owns interaction routing. Use `.tooltip_opt(...)` when tooltip text is
controlled by optional app state, or `.tooltip_if(...)` when a boolean condition
controls one tooltip, so projection code does not repeat
`if enabled { view.tooltip(...) } else { view }` wrappers. Button reducers can use
`ButtonMessage::is_activate()`, `secondary_position()`, and `drag_message()` to
route primary activation, context-menu clicks, or drag lifecycle events without
repeating the raw button enum shape. Button-backed drags emit `Cancelled` when
focus loss aborts an active drag before release.
Use `button_row(...)` for compact horizontal dialog, popover, inspector, and
utility-panel button groups where the app owns button text, tone, messages, and
widths while Radiant owns the group spacing and row height.
Use `text_input(value).clear_button(message)` for compact search, filter,
rename, or command fields where the app owns value/messages but Radiant should
own the input row, fixed clear-button slot, spacing, and hidden-button
behavior. Use `.clear_button_mapped(...)` when the clear action needs to build
the host message lazily instead of cloning one message value. The clear-button
slot uses compact defaults; `.id(...)` and `.key(...)` identify the text input,
and Radiant derives the child clear-button identity. Use
`text_input_clear_button_id(input_id)` only in tests, automation, or host
integration code that needs to address that generated child.
Use `drag_handle().hover_chrome_only()` for subtle splitters or reorder handles
that need a persistent hit target but should hide idle chrome until hover,
press, or focus.
Run `cargo run --example status_bar` for a bottom status-bar sandbox that shows
button actions, toggle state, animation updates, and background worker progress
flowing into a one-line log and retained-canvas progress strip. Compact status
strips can use `StatusBarParts`, `status_bar(...)`, and
`status_bar_from_parts(...)` with generic `StatusSegments` when the app owns the
labels and optional trailing progress/action content but should not rebuild the
status-row chrome locally.
Run `cargo run --example layout_rows_columns` for a compact row/column layout
sandbox with padding and fill sizing.
Run `cargo run --example custom_layout` for a small external-style measure/place
policy with ordinary declarative children.
Run `cargo run --example split_pane_static` for a product-neutral static
two-pane geometry sandbox that inspects the public `split_pane(...)` builder.
Run `cargo run --example split_pane_runtime` for a deterministic runtime-owned
divider sandbox that reports the projected target, captured live resize, and
commit cleanup without application projection.
Run `cargo run --example grid_gallery` for a fixed-column gallery sandbox that
uses `grid_with_gaps(...)` with normal nested views and styling.
Run `cargo run --example tree_and_details` for tree-list and sortable details
list composition with drag-aware row controls. Use
`interactive_row_underlay(content)` when arbitrary visible row content should
stay above a generic interactive row that owns activation, secondary
activation, drag, drop, focus, and row feedback paint while preserving a stable
input widget id or key. Use `.input_key(...)`, `.stable_input_id(...)`, or
`.stable_u64_input_id(...)` on dynamic underlay rows when only the input layer
needs explicit identity; use `.stable_row_identity(...)` when the same durable
row key should also key the composed row subtree. Use `.custom_paint_hit_target()`,
`.activation_modifiers()`, `.tracked_drag_source(...)`, or
`.tracked_drag_source_with_motion(...)` on underlay rows when app-owned visible
content still needs standard Radiant row input presets without dropping to
`.row(|row| ...)`. Use `.tracked_drop_target(...)` or
`.tracked_drop_candidate(...)` when underlay rows need Radiant-owned drop-target
lifecycle routing around host-owned domain state. Use `.dense_chrome()`,
`.selected(...)`, `.candidate(...)`, or `.visual_state(...)` on underlay rows
whose visible content is app-owned but whose dense row feedback should remain
Radiant-owned.
Use `.dense_chrome_palette(...)`, `.leading_marker(...)`,
`.trailing_marker(...)`, and `.outline(...)` when that generic underlay needs
app-specific dense row fills or edge/status markers.
Run `cargo run --example theme_playground` for a theme-token sandbox that
compares density scale, tone, prominence, and interactive state through normal
application views. It is intended to make theme policy visually inspectable, not
only to prove that token colors resolve.
Run `cargo run --example dpi_scaling` for a native DPI sandbox that forces the
active runtime DPI scale from the example UI, then shows logical-point sizing,
physical framebuffer conversion, and pointer remapping through `DpiScale`.
Run `cargo run --example paint_helpers` for direct paint helper output around
borders and text-field chrome.
Run `cargo run --example passive_widgets` for passive button, toggle, text
input, canvas, and spacer surfaces that do not emit normal interaction messages.
Run `cargo run --example list` for a basic list-row composition sandbox.
Run `cargo run --example styling` for tone, prominence, danger, subtle, and
hoverable styling examples.
Run `cargo run --example scroll` for simple scroll-column composition.
Run `cargo run --example sizing` for explicit, minimum, preferred, and fill
sizing behavior.
Run `cargo run --example message_routing` for `UiUpdateContext` follow-up
messages and repaint requests.
Run `cargo run --example keys` for stable keys and reversed list identity.
Run `cargo run --example focus_controls` for an input/focus sandbox that uses
`UiUpdateContext::focus(...)` and shortcuts to move keyboard
focus from normal app messages.
Run `cargo run --example plugin_panel` for an advanced synthetic control-panel
simulation that stays on generic Radiant layout, style, focus, and
message-first update APIs; plugin SDK integration and preset policy remain
outside Radiant.
Run `cargo run --example eq_editor` for an advanced synthetic curve-editor
simulation that paints a visual response curve, analyzer-style overlay,
editable handles, and parameter-routing messages without modeling DSP or audio
processing.
Run `cargo run --example spectrogram` for a retained heatmap visualization that
scrolls deterministic synthetic spectrum data through frame-driven messages,
hover readout, and transport controls without modeling DSP or audio processing.
Run `cargo run --example mixer_console` for an advanced synthetic dense-panel
simulation with deterministic meter levels, faders, grouped drag previews,
strip reordering, and paint-only hover overlays. It validates Radiant
interaction and paint contracts; channel, send, mute, solo, and DSP semantics
are not Radiant API guidance.
Run `cargo run --example piano_roll` for an advanced synthetic retained-editor
simulation with a keyboard-like lane, grid, synthetic note blocks, drag-create
and move/resize previews, marquee selection, velocity-like handles, and
paint-only hover, drag, and playhead overlays. It validates Radiant retained
canvas and gesture contracts; MIDI note editing, quantization, piano-key
semantics, velocity editing, and DAW workflow policy are non-authoritative.
Run `cargo run --example modulation_matrix` for an advanced synthetic matrix
simulation with source and destination labels, bipolar amount editing,
clear/delete behavior, synthetic activity markers, and paint-only hover
overlays. Synth routing semantics are non-authoritative.
Run `cargo run --example arrangement_shell` for an advanced synthetic
multi-pane workspace simulation that uses `workspace_shell(...)` for readable
top/sidebar/workspace/status composition around transport-like controls, a
browser pane, timeline overview, inspector, compact status strip, synthetic
clips/meters, and paint-only hover/playhead overlays. Arrangement, track,
transport, mixer, audio, DSP, and plugin behavior remain host-owned.

## Quality Gate

The [Platform Acceptance and Evidence Policy](PLATFORM_ACCEPTANCE.md) classifies
this repository quality procedure as lane C, with lane A applying when a
deterministic runtime behavior is exercised. These commands do not establish
headless native-host, logged-in desktop, or manual/hardware acceptance.

The local validation lane runs formatting, Clippy with warnings denied, library
and integration tests, checked examples, a rustdoc build, Rust source-level
doctests, no-default-features library checks for the documented Linux and macOS
targets, and a perf-harness smoke pass that lists scenarios and proves baseline
capture/comparison with `--fail-on-missing-baseline`.

Radiant's normal local quality lane is:

```powershell
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --lib --tests
cargo test --examples
cargo doc --no-deps
cargo test --doc
rustup target add x86_64-unknown-linux-gnu x86_64-apple-darwin
cargo check --lib --no-default-features --target x86_64-unknown-linux-gnu
cargo check --lib --no-default-features --target x86_64-apple-darwin
cargo bench --bench perf_harness -- --list
cargo bench --bench perf_harness runtime_virtualized_list_hover -- --jsonl --write-baseline-jsonl .\target\perf-baseline.jsonl
cargo bench --bench perf_harness runtime_virtualized_list_hover -- --jsonl --baseline-jsonl .\target\perf-baseline.jsonl --fail-on-missing-baseline
```

`cargo doc --no-deps` validates generated rustdoc and source-level intra-doc
references. The current setup does not establish `rustdoc with broken intra-doc
links denied`, and this command does not validate Markdown API-reference
snippets in `docs/API.md`. `cargo test --doc` runs doctests for public
documentation examples extracted from Rust doc comments; it does not execute
Markdown API-reference snippets.

The perf-harness listing is a smoke check for scenario registration. The
focused baseline round trip proves the JSONL capture/comparison path and missing
baseline failure mode without treating timing as a portable pass/fail gate.
Additional focused benchmark comparisons should be run when the change touches a
hot path. Keep new lint exceptions local and specific instead of adding broad
crate-level Clippy allows.

## Automation

`radiant::gui::automation` owns the serializable automation snapshot contract:
`AutomationNodeId`, `AutomationRole`, `AutomationBounds`,
`AutomationPoint`, `AutomationFocusHints`, `AutomationLiveRegion`,
`AutomationNodeSemantics`, `AutomationNodeSnapshot`, `AutomationTarget`, and
`GuiAutomationSnapshot` / `GuiAutomationTargetSnapshot`. `SurfaceRuntime` exposes
`automation_snapshot()` and `automation_target_snapshot()` to derive this tree
and its flattened target projection from the current projected surface, layout
bounds, and widget contracts. Backends and test tools can consume this semantic
tree without depending on a host application's state types or reducer.
Common widgets populate generic role, label, value text, checked/selected,
disabled/read-only, focusable/focused, live-region, and metadata fields when the
data already exists. `AutomationNodeSnapshot` keeps compatibility aliases such
as `role`, `label`, `value`, `enabled`, `selected`, and `metadata`, while the
richer `semantics` payload is the preferred source for new tests, devtools, and
future adapters. Snapshot nodes also derive conservative default action names
such as `focus`, `press`, `toggle`, `select`, `set_text`, and `set_value` from
their role and state. `GuiAutomationSnapshot::target_snapshot()` flattens the
tree into coordinate-bearing automation targets with tree order, depth,
root-to-node path, bounds, center point, role, label/value text, current state,
actions, and metadata; this is the supported bridge shape for tests, devtools,
Computer Use sidecars, and native adapters that need stable GUI targets
without coupling to host state. Runtime `SurfaceRuntime::automation_snapshot()`
uses schema version 3 for this staged-and-published tree.
`SurfaceRuntime::automation_target_snapshot()`
adds runtime-owned `AutomationTargetAuthority` evidence and schema version 3;
the pure `GuiAutomationSnapshot::target_snapshot()` helper is a read-only
schema-version-3 flattening helper because its targets can carry
`AutomationRole::Separator`. Runtime-owned split-pane automation reads
consume only the committed crate-private divider projection: each valid
projection publishes one `AutomationRole::Separator` directly between its
split container's two content children. Its stable ID is
`radiant:layout-target:<container-id>:<region-id-as-16-lowercase-hex>`, its
value is the shortest round-tripping normalized `f32` ratio, and its metadata
includes `orientation=horizontal` or `orientation=vertical`. The separator is
enabled and materialized, but has no actions and no focus, selection, checked,
read-only, or traversal behavior; its flattened `interaction_target` is false.
The complete insertion set is preflighted, so stale, missing, malformed,
ambiguous, colliding, or otherwise invalid evidence returns the unchanged
ordinary snapshot without partial separator nodes. Directional focus hints and
live-region values are backend-neutral hints only. The private primary-window
macOS/AppKit consumer is shipped for this passive separator projection: one
unique current target with materialized authority must match the exact ID,
semantic path, role, normalized ratio value, bounds, orientation, and
actionless/non-focusable state before it publishes one `AXSplitter` between the
two pane children. It retains native token/object identity across an admitted
ratio update and falls back to the ordinary native tree without provider calls,
runtime refresh, partial topology, or interaction authority when evidence is
stale, unmaterialized, duplicate/mismatched, malformed, focused, or actionful.
Manual macOS/VoiceOver acceptance remains unverified. The crate-private generic
native plain `Tab`/`Shift-Tab` consumer is shipped using committed sequential
disposition: focused-key/text input gets first refusal; modified `Tab` is
unchanged; repeats/releases do not retraverse; focus loss/regain clears its
per-window latch; and only `NoDestination` feeds the existing host-first/widget
fallback. Other platform adapters, public/native focus, spatial traversal,
keyboard/arrow-key resizing, semantic actions, pointer/collapse mapping, and
paint/cursor/renderer behavior remain future work;
ordinary application APIs do not expose AccessKit, screen-reader, or OS tree
handles.
The macOS development app-bundle helper improves process/window discovery for
app-level automation tools. `RADIANT_AUTOMATION_TARGET_EXPORT` pairs with that
launch path by exposing the current flattened target snapshot to external
sidecars, but it does not replace the semantic automation snapshot and does not
by itself expose per-widget native accessibility nodes.
The current snapshot `actions` field and action-name export are advertisement
and inspection only: they do not dispatch an action, transfer focus,
materialize a virtual target, or authorize execution. The generic runtime
boundary `SurfaceRuntime::dispatch_numeric_accessibility_action` accepts an
explicit request only after current authority, focus, capability, and owner
revalidation, and no current automation export or native adapter executes the
neutral `Increment`, `Decrement`, or `SetValueText(String)` vocabulary.

`radiant::gui::snapshot` owns deterministic rendered-frame snapshot primitives:
`VisualSnapshot`, `SnapshotPrimitive`, `SnapshotTextRun`, `SnapshotRect`,
`SnapshotPoint`, `SnapshotColor`, and `SnapshotTextAlign`.
`visual_snapshot_from_paint_frame` converts generic `PaintFrame` payloads into
this serializable schema. These APIs are for fixture generation, renderer
verification, and visual regression tooling. Host or compatibility adapters may
build these snapshots from their own frame models, but the serializable
snapshot schema is generic Radiant API.

## Generic Panels And Forms

`radiant::gui::chrome` contains generic chrome/status models such as
`StatusSegments` and the grouped `ContentViewChrome` tab, search, activity,
sort, and footer copy models. `radiant::gui::feedback` contains compact
feedback models such as `StatusLineLog` and `StatusLineEntry` for bounded
one-line status messages from buttons, background workers, animations, and
other app-owned systems. Host applications map product-specific copy into these
slots; Radiant defaults stay product-neutral.

`radiant::gui::panel` contains generic split-pane and sidebar models such as
`SplitPaneAxis`, `SplitPaneCollapsePolicy`, `SplitPaneLayout`,
`SplitPaneLayoutParts`, `SplitPaneSlot`,
`SplitPaneAssignmentState`, `SplitPaneAssignedRow`, `SplitPaneTreePanel`, and
`SplitPaneSidebarState`, plus `anchored_panel_rect`
for clamped popup/panel placement and `PanelResizeState` with
`PanelResizeConstraints` or `CollapsiblePanelResizeConstraints` for
splitter-driven pane resizing. Use `PanelResizeConstraints::left(...)`,
`right(...)`, `top(...)`, or `bottom(...)`, and the matching
`CollapsiblePanelResizeConstraints` constructors, for common edge-specific
resize handles. Use `PanelResizeState::resize_collapsible(...)` when a resize
handle should collapse the panel to a host-chosen size on double activation,
then restore the last expanded size on the next double activation.
For typed shared-edit delivery, use the qualified
`PanelResizeState::resize_edit(...)` or
`PanelResizeState::resize_collapsible_edit(...)` methods; each accepted drag
boundary returns one `EditEvent<f32>`, and cancellation rolls the size back to
the transaction start. The existing concise methods remain available for
hosts that only need size projection. Use `SplitPaneLayout::from_parts(...)`
to resolve backend-neutral first, divider, and second rectangles along a
horizontal or vertical axis; non-finite ratios use `0.5`, non-finite divider
or minimum extents use `0.0`, and the result reports whether both pane minima
were satisfied. The static application builder
`radiant::application::split_pane(first, second)` and its common-prelude
export accept `.axis(...)`, `.initial_ratio(...)`, `.min_first(...)`,
`.min_second(...)`, and `.divider_extent(...)`, plus the additive
`.runtime_owned_ratio()` and
`.controlled_ratio(radiant::layout::Controlled::new(value, generation))`
opt-ins, plus the additive `.on_ratio_settled(|ratio| Message::Settled(ratio))`
output mapper and `.collapse_policy(SplitPaneCollapsePolicy::FirstPane)` or
`.collapse_policy(SplitPaneCollapsePolicy::SecondPane)`. The collapse option is
inert unless runtime-owned ratio mode is selected. It lowers exactly two
ordered children through a dedicated
`ContainerKind::SplitPane` policy and keeps the existing `SplitPanePolicy`
fields and defaults source-compatible. The static form owns no runtime ratio,
pointer, focus, hit-region, capability, or semantic state. Runtime-owned mode
seeds a mounted ratio once from the sanitized `initial_ratio`; controlled mode
accepts its mount value and only strictly newer generations. A
runtime-owned/controlled transition is an explicit state reset, while a
compatible same-identity projection preserves the mounted slot. Missing,
incompatible, unavailable, capacity-exhausted, or retired state falls back to
the declarative ratio. Accepted state affects only top-down placement;
measurement and measurement-cache identity remain independent of the runtime
ratio.
The qualified `radiant::layout::{LayoutCapabilities,
LayoutInteraction, LayoutInteractionRevision}` contract provides
backend-neutral UI-local capability registration, exact/conservative revision
evidence, validated normalized hit-region declaration/projection, and (for
contract versions 3 and 4) typed pointer admission with runtime-owned capture
for generic surface containers. Version 4 additionally provides the optional
typed `ContainerStateDeclaration` / `LayoutContainerStateContext` seam with
bounded UI-local state; version 2 remains projection/query-only.
`SurfaceRuntime::layout_hit_target_at(...)` is a read-only query over those
projected targets. Runtime-owned splits with a positive resolved divider expose
one clipped built-in divider target matching the quantized child geometry;
primary pointer capture drives the mounted ratio through the shared
`PanelResizeState` lifecycle, while static and controlled-ratio splits remain
inert. A collapse policy makes an admitted primary divider double activation
resolve the selected pane to its authoritative declared minimum through the
same current viewport, divider extent, opposite minimum, and quantization
rules as `SplitPaneLayout`; capacity-limited or undersized minima
are rejected rather than using the ordinary layout fallback. The next accepted
activation restores the last finite normalized expanded ratio, including the
latest committed drag ratio. Active drags, invalid or stale evidence, no-ops,
missing or unavailable capacity, incompatible state, unmount, static mode, and
controlled mode are inert. Meaningful collapse and restore mutate mounted
state, request the existing runtime/layout work, and then map exactly one
settled ratio after cleanup. A settled mapper is runtime-owned output only, not
persistence: it emits once for a meaningful successful drag commit or discrete
collapse/restore of the final finite normalized ratio, and remains silent for
intermediate, no-op, cancelled, lost, incompatible, unmounted, static, and
controlled interactions. Passive separator semantics are published by the pure
automation read above; the private primary-window macOS/AppKit consumer now
publishes each qualified separator as one `AXSplitter` between its two pane
children. They remain non-focusable, actionless, and non-interactive; invalid
or ambiguous evidence falls back to the ordinary native tree. The explicit
backend-neutral sequential traversal consumer reads the private committed
mixed-order sidecar and treats each exact current runtime-owned separator as
one private stop between its pane widget subtrees, including nested separators.
Invalid or unavailable evidence uses the complete widget-only order. The
crate-private traversal disposition distinguishes `NoDestination`,
`AdmittedWidget`, `AdmittedPrivateSplitPaneSeparator`, `Vetoed`, and
`Invalidated`. The generic native runtime consumes an unclaimed plain
`Tab`/`Shift-Tab` using that committed sequential disposition:
`AdmittedWidget` and `AdmittedPrivateSplitPaneSeparator` are consumed
destinations; `Vetoed` and `Invalidated` are terminal with no fallback/retry;
only `NoDestination` reaches the existing host-first/widget fallback exactly
once. Focused-key/text input ownership and command/control/alt-modified `Tab`
retain precedence. Repeat/release do not traverse again, and the per-window
sequence latch is cleared on native focus loss/regain. An invalidated separator
does not retry or choose an alternate destination. This consumer is distinct
from private pointer ownership for divider acquisition and collapse/restore.
The passive `AXSplitter` projection remains non-focusable, actionless, and
non-interactive; it does not itself authorize key routing. Public/native
separator focus, spatial traversal, keyboard/arrow-key resizing, semantic
actions, paint/cursor/renderer behavior, and `VirtualLayoutPolicy` remain future
work.
Internally, the
controller may retain
a bounded crate-private `SplitPaneSeparatorProjection` collection after the
mounted-state commit. It is read-only evidence joining the exact
`MountedContainerStateId` generation, existing divider `LayoutTargetIdentity`,
axis, final clipped bounds, and finite normalized live ratio. The pure runtime
automation compositor may consume that evidence for the passive separator
publication above; it is not a public API or an authority for focus, key
handling, actions, paint, relayout, provider/native calls, or application
projection. The controller may also retain a private source-candidate sequence
alongside the widget keyboard order and a committed mixed-order evidence
sequence after exact projection/lifecycle reconciliation. This evidence is
non-authorizing for key routing, pointer behavior, native publication,
rendering, or public APIs; the explicit backend-neutral sequential traversal
consumer is its only focus consumer.
Use the lower-level `PanelResizeDrag`,
`update_panel_resize_drag`, and `update_collapsible_panel_resize_drag` helpers
only when the host deliberately stores durable size separately from transient
drag state. Host applications map product-specific navigation, workspace,
project, or asset concepts onto these reusable panel structures.

`radiant::gui::badge` contains compact label and pill primitives such as
`SelectablePill`, `PillEditorPanel`, `InlineBadgeMetrics`,
`inline_badge_width_in_range`, `inline_badge_rects_for_labels`, and
`inline_badge_text_origin`. Repeated layout or paint paths can use
`inline_badge_labels_owned_into`,
`inline_badge_rects_for_labels_into`, and `inline_badge_rects_into` to reuse
caller-owned buffers. Hosts can use these to render dense badge clusters for
metadata, filters, status chips, or other product-specific labels without
embedding domain terms in Radiant.
Wrapped chip, token, recipient, or pill editors can use
`FlowLayoutMetrics::new(...)` for the compact item-gap, row-gap, and item-height
policy, plus
`FlowTrailingItemParts` and `pack_flow_rows_with_trailing_item` when a trailing
input/control should stay on the current row only if enough editing room
remains. Use
`pack_flow_rows_with_trailing_item_and_following_item(...)` and
`FlowTrailingItemParts::reserve_following_item(...)` when that trailing editor
must reserve room for a compact following action such as a picker, library
toggle, or add-menu button; use `flow_width_with_following_item_reserved(...)`
for the same reservation policy when building an atomic trailing group
manually. Use `push_flow_row_group` when several flow items, such as a prefix
token plus its editor, should wrap atomically instead of splitting across rows.
Use `pack_flow_rows_with_trailing_group` when callers need the common form of
packing existing items and appending one such atomic trailing group.
Use `pack_flow_rows_with_flexible_trailing_group(...)` when that atomic trailing
group contains a flexible editor/control and an optional following action that
must reserve width while the whole group wraps together.
Use `FlowRowPacker` when rows are built incrementally and repeated appends
should retain the current row width instead of rescanning the trailing row.
Use `capped_flow_rows_height(...)` when the editor should grow to a maximum
visible row count before switching to a scrollable content area.
Use `FlowFieldMetrics` and `FlowFieldLayout` when a bounded inline editor needs
shared content-width, visible-height, and scroll-threshold calculations around
the packed rows while the host still owns domain-specific labels, ordering,
messages, and styling. Call `layout(...)` when the container width is available,
or `layout_for_content_width(...)` after using the resolved content width to
pack rows.

`radiant::gui::form` contains reusable form and picker models such as
`DecimalTextInputPolicy`, `SummaryField`, `OptionItem`, `OptionSelectionState`,
`PairedPickerTarget`, `PairedPickerValue`, `PairedStatusPanel`,
`PreferencePanelVisibility`, and `PreferencePanelState`.
`PairedStatusPanel` models a two-sided status/picker surface with summary rows,
active picker identity, and option lists while leaving the meaning of those
options to the host. `PreferencePanelState` models generic settings-panel
visibility through a named state, a primary text value, fixed-size toggle
state, and an auxiliary label without owning product-specific preference names.
Titled panel code that needs to anchor popovers, completion lists, or other
foreground chrome to the panel content area can use
`PanelSectionGeometry::header_only_height()`,
`PanelSectionParts::content_top_offset()`,
`content_top_inset_from_bottom(...)`, `content_bottom_inset()`,
`section_height_for_content_height(...)`, and
`content_height_for_section_height(...)` so the host does not duplicate
Radiant's panel padding, title-height, and spacing geometry.

`radiant::gui::text_layout` contains retained text-line placement helpers such
as `TextLineInsets`, `centered_text_line`, `top_text_line`,
`centered_text_baseline`, and `TextLineLayoutCache`. The common placement and
baseline helpers are also available through `radiant::prelude` for custom
widget painters. The module also exposes deterministic approximate width helpers
such as `TextWidthEstimate`, `estimated_text_width_in_range`, and
`estimated_text_width_for_char_count_in_range` for layout decisions that must be
made before renderer shaping metrics are available. Use
`estimated_text_width_for_segments` or
`estimated_text_width_for_segments_in_range` when the displayed label is
assembled from stable pieces such as an inline completion suffix, prefix, or
adornment and the host should not allocate a temporary joined string just to size
the control. Inline token, recipient, and chip editors can use
`TextInputWidthPolicy` to share draft-value, completion-suffix, placeholder,
minimum-visible-character, and min/max width sizing without local helper logic.
The plain placement and width helpers are deterministic and
side-effect free; renderer adapters that need retention can pass an owned cache
and font-family cache key to `centered_text_line_with_cache` or
`top_text_line_with_cache`. That keeps hot-path text geometry reuse explicit,
backend-owned, and free of hidden global synchronization while avoiding
host-domain text semantics.

`radiant::gui::visualization` contains generic visualization models such as
`TimelineAxis`, `TimelineLaneLayout`, `TimelineViewport`,
`TimelineTransportState`, `TimelineEditPreview`, `TimelineFeedbackEvents`,
`TimelinePresentationState`, `SignalRasterPreview`, `TimelineSurfaceParts`,
`TimelineSurfaceState`, `TimelineMotionState`, `CanvasSelectionGeometry`, and
`normalized_milli_point_in_rect`. Hosts can map product-specific media,
timeline values, lanes, normalized selections, or spatial surfaces into these
reusable visualization slots while keeping domain workflow state outside
Radiant. Use `CanvasSelectionGeometry` when one projected normalized selection
needs several generic affordances such as a body move handle, resize edge
visuals, or a trailing control; its paint helpers append guarded fill
primitives for those affordances while hosts keep product-specific colors and
messages. Use `CanvasSelectionGeometry::from_viewport_range(...)` when a
canvas-like surface needs to clip an absolute normalized range through an
`IndexViewportScope` before projecting the visible selection geometry. Use
`CanvasSelectionAffordanceStyle::push_fills(...)` with
`CanvasSelectionAffordancePaintParts` when one selection should paint a grouped
set of optional body, edge, and trailing-control affordances from one dimension
style. Its `affordance_at_point(...)` helper can resolve optional body, edge,
and trailing-control hit targets while host applications keep their own command
mapping and domain-specific priority. Use
`CanvasSelectionAffordanceStyle` when the same selection should expose a reusable
set of optional body, edge, and trailing-control affordances from one grouped
style instead of rebuilding low-level hit-test parts in app code. Use
`CanvasSelectionBodyHandleStyle`, `CanvasSelectionEdgeVisualStyle`, and
`CanvasSelectionTrailingControlStyle` when hit testing and painting the same
canvas affordance should share one reusable dimension policy without duplicating
constants across input and paint code. Use `CanvasSelectionPaintStyle` when a
canvas widget should derive selection fill, boundary cursor, body-handle,
resize-edge, and trailing-control colors from a host-supplied base color while
still allowing state-specific alpha overrides. Use `TimelineEditPreview` with
`TimelineEditHandleGeometry` and `TimelineEditRegionGeometry` when timeline
editors need standard handle hit rectangles and leading/trailing region paint
rectangles without duplicating viewport projection math. Use `TimelineEditRamp`
and `TimelineEditPreview::from_normalized_ramps(...)` when a host already has a
normalized selected range plus leading/trailing ramp lengths, outer extensions,
and optional curve values. Use
`TimelineEditPreview::push_standard_region_fills(...)` and
`push_standard_handle_fills(...)` when Radiant should own standard edit-preview
paint emission while the host owns colors and domain commands. Use
`TimelineEditPaintStyle` plus
`push_standard_styled_region_fills(...)`,
`push_standard_styled_handle_fills(...)`, and
`TimelineEditPaintStyle::curve_stroke_parts(...)` when the host only needs to
provide a base color and Radiant should derive standard region, handle, and
curve colors. Use
`TimelineEditPreview::push_standard_ramp_curve_strokes(...)` when Radiant
should also own standard leading/trailing ramp curve projection and guarded
stroke emission while the host owns the ramp value function.

## Invalidation And Lifecycle

Hosts project immutable surface snapshots. Radiant compares widget identity,
layout inputs, style tokens, and paint data to keep redraw work focused. Generic
invalidation primitives such as `InvalidationMask`, `RetainedSegmentMask`,
`RetainedSegmentRevisions`, `RevisionCounter`, `StableFingerprint`, repaint
signals, and frame feedback exist so backend runtimes can avoid unnecessary
full-tree rebuilds and full redraws while still falling back conservatively
when a host cannot provide fine-grained hints.

`SurfaceRuntime` retains a `LayoutEngine` across refreshes and viewport changes
instead of using the stateless one-shot layout helpers internally. That lets the
runtime preserve layout measurement and virtualization caches while pruning
stale measurement entries that were not touched by the latest layout pass.
It can still accept fresh immutable `UiSurface` snapshots from the host. Direct
`UiSurface::frame(...)` calls remain one-shot by design for embedded hosts that
want a single packaged frame without owning runtime state.

The declarative lifecycle contract is snapshot based, not object-instance
based. Application builders may create a fresh `View<Message>` or
`UiSurface<Message>` on every refresh; continuity comes from stable widget
identity, host-owned state, retained resource identity, and runtime caches.
Use `.key(...)`, explicit widget IDs, or resource IDs for dynamic rows,
editor handles, retained GPU surfaces, text inputs, and focusable controls that
must survive insertion, removal, reordering, or scroll-window changes. Generated
IDs are suitable for static local structure, but dynamic collections should not
depend on positional identity when user focus, selection, drag state, or cache
reuse matters.
Use either `.id(...)` or `.key(...)` on one view node; if both are chained, the
last identity modifier wins. Prefer explicit IDs for external automation,
focus, and tests that need stable numeric handles, and prefer scoped keys for
ordinary repeated view structure.

Reducers own all durable application state. Widget input emits host messages,
and runtime-local state is limited to GUI concerns such as focus, hover,
pointer capture, scroll offsets, layout caches, repaint flags, and retained
surface caches. A reducer that changes durable state should request a normal
surface repaint so Radiant can project a new immutable snapshot. Use
paint-only repaint scopes only for overlay motion, cursor previews, handles, or
other transient visuals that can reuse the current declarative surface and
paint plan without hiding a real state change.

The lifecycle is:

1. Host state is projected into `UiSurface<Message>`.
2. Radiant measures layout and builds a paint plan.
3. Backend input is routed to widgets.
4. Widget outputs are mapped to host messages.
5. The host reducer mutates host state and may request repaint.
6. Radiant refreshes the surface and rebuilds only the necessary runtime data.

### Prepared surface refresh evidence (private production Projection consumer)

There is exactly one externally visible complete
`CommittedFrameState`/last-complete frame. A staged consumer may prepare an
invisible private `PreparedSurfaceRefresh` containing candidate
surface/traversal, source projection, layout root, view-delta decision,
candidate layout, candidate paint plan, damage, and timing evidence.
Preparation may mutate candidate-owned storage only; it never mutates active
focus/capture/composition/wheel ownership, the declarative owner,
accessibility/automation projection, active layout, retiring-widget ownership,
or the last-complete frame.

Immediately before irreversible replacement cleanup, revalidate runtime
identity, lifecycle-transition generation, active-surface generation,
layout-state generation, viewport, window environment, requested refresh
revision, and existing native window/adapter/target/stage/owner/revision
fences. A mismatch, stale generation, lifecycle transition, resize/recovery,
newer visual work, unsupported/ambiguous/incomplete evidence, or pre-commit
failure drops the candidate with no active mutation, callback, terminal message,
or presentation and retains the combined correctness-first fallback. After
validation, perform irreversible replacement cleanup once, atomically publish
complete candidate state, then dispatch terminal messages. No scheduler yield
follows cleanup start; a panic then is terminal recovery/shutdown, not rollback.

The prepared refresh path is the first private production Projection-stage
consumer of the safe-boundary owner. After candidate-local preparation passes
currentness, it completes the Projection ticket, admits/checks one exact
private Layout ticket, and completes Layout before admitting/checking one exact
private PaintPlan ticket immediately before irreversible runtime publication;
the PaintPlan ticket completes after the candidate publication call returns.
This remains one synchronous candidate transaction with no scheduler yield: it
does not claim independently scheduled Reconciliation/Layout/Paint, a public
API, a second event loop, or cross-window policy. Projection remains the
no-replay boundary; a later Layout or PaintPlan veto drops the inert candidate
without re-entering the combined path. The existing combined path remains
authoritative for virtual materialization and unsupported paths.
`WindowStageOwner` now admits private Deadline work plus this synchronous
Projection-to-Layout-to-PaintPlan handoff. Diagnostics and timing remain
observational and non-authoritative.

Current shipped boundary: prepared refresh constructs the Projection, layout,
and paint-plan candidate synchronously, then uses a later no-yield publication
gate. Independently schedulable Reconciliation, Layout, and Paint stages remain
future work under OPT-1389.

The current interaction-only reconciliation slice is narrower than the full
prepared-refresh contract. A `RuntimeBridge` may opt in by supplying a
provider authority and returning `SurfaceUpdate::ExactChangedRoots` with
request-fenced, disjoint widget-leaf paths. Before exact admission, the sampled
application environment is a fence: `None` retains candidate ownership, while
`Some(sampled)` is admitted only when it equals the installed surface
application environment. A mismatch uses the complete refresh path so the
sampled environment is applied before publication. Radiant then admits a path
only when cached exact structure, geometry, paint, source, mapper, hit-test, pointer,
file-drop, and state-membership evidence agrees and the leaf changes only
interaction or semantics revisions. It swaps those leaves atomically while
retaining installed layout, traversal, source, and base paint state. Stateful
leaves, structural or geometry/paint changes, virtual content, custom layout,
opaque mapper evidence, stale fences, and ambiguous paths use the complete
refresh candidate and its normal fallback behavior. The bridge remains the
authority for the exact changed-root list; recursive classifiers and
diagnostics do not authorize admission.

### Native visual request packet handoff (private native-window contract)

The native event loop has one crate-private `NativeVisualRequestPacket` handoff
per window. The packet is deliberately non-`Clone` and carries only the exact
Winit `WindowId`, a checked non-wrapping `NativeVisualOwnerGeneration`, a
checked non-wrapping `NativeVisualRevision`, and a private observational origin:
`ScheduledOrRuntime` (which records `FrameWork`) or
`NativeInvalidationFallback`. It is deliberately target-unbound and
adapter-unbound: current adapter and target generations are eligibility
evidence at `RedrawRequested`, not packet identity. `FrameWork` remains
diagnostic evidence; it does not authorize rendering, scene reuse, partial
work, or presentation.

Each window owns a fixed mailbox with `requested`, `consuming`, and `pending`
state, with at most two retained packet owners. `requested` and `consuming` are
mutually exclusive. The requested state is the packet for the outstanding
Winit redraw; an offer while `requested` is occupied replaces it with the
newest revision and emits no additional Winit wakeup. `RedrawRequested` moves
that packet to the consuming owner; only an offer while `consuming` is occupied
uses or replaces the one newest `pending` successor. The private
`NativeVisualRequestAdapter` is the only caller of raw
`Window::request_redraw`. All other runtime paths record `FrameWork` and enqueue
or reissue a packet. A reversible `suspended` state is distinct from retire: it
rejects enqueue and unsolicited fallback, clears packet ownership and wake
timing, advances the owner generation, and survives invalidation or native
resource recovery until an explicit resume. Auxiliary hide/cache dormancy
suspends the mailbox; inactive recovery rebuilds state without enqueue, an
inactive `RedrawRequested` reasserts dormancy, and show resumes the mailbox and
issues exactly one fresh latest-state packet.

Only a `WindowEvent::RedrawRequested` boundary may begin the existing redraw
kernel and reach presentation. Eligibility is a crate-private logical
presentation-capability predicate: an initialized primary is eligible even
while its host visibility is initially hidden or unknown, while an auxiliary
also requires `active` and `admitted`. Neither scheduling nor admission may
call or infer authority from Winit `is_visible()`. Every path still requires
the exact live `WindowId`/owner, a complete current adapter resource bundle,
running lifecycle, no recovery/closing/stopped state, and an ordinary known,
unfenced target. Runtime and scene dirty state remains recorded even when no
packet is admissible. Ordinary offers and scheduler demand use only that local
predicate; exact current adapter-resource validation remains at the redraw
callback. A requested packet whose begin eligibility fails returns the private
`RequestedVetoed` outcome, clears the requested packet and associated pending
packet, advances the owner generation, clears wake timing/stale state and the
recovery exception, and performs no redraw, completion, or fallback. A
`WrongWindow` result preserves the current packet; with no requested packet the
result is `Ineligible`. Missing adapter generation follows `RequestedVetoed`
when a requested packet exists rather than returning early. Requested packets
may additionally cross a temporary target fence only for a validated nonzero
pending resize that can restore the target or the existing one-shot `Other`
recovery permit; an unsolicited fallback is restricted to ordinary eligibility.
Scheduler demand exists only for ordinary eligibility or an outstanding
requested packet with a valid recovery exception. Exhausted fenced state has no
cadence/repaint demand, retry deadline, or `frame_wait` reinsertion until an
explicit rearm. For the primary scheduler, retry deadlines, and `frame_wait`,
the stored adapter owner must exist and expose the exact generation in the
active resource bundle; missing or mismatched evidence is quiescent and does
not reinsert work. Packet offering remains adapter-unbound so initialization
and recovery may enqueue while holding the adapter externally; auxiliary
scheduler admission remains parent-generation-authoritative. A primary
`RedrawRequested` callback with no stored adapter owner is a hard
`RequestedVetoed` transition: it clears requested and pending ownership,
stale wake timing, and the recovery exception, advances the owner when packet
work existed, and performs no redraw, finish, fallback, or diagnostics. With
no packet it only clears inconsistent wake/recovery state. After
resize/acquisition and before scene work, the current target must again be
known and unfenced. Hide/cache dormancy, close/retire, owner replacement,
recovery/loss, and resource isolation retain their explicit invalidation
fences.

Radiant stores desired visibility in crate-private
`logical_window_visible`, separate from physical Winit application. An explicit
policy update changes that desired state and applies it only while the local
lifecycle is running. Recovery, loss, and closing physically conceal the
window without changing desired state; successful renderer/device publication
reapplies the latest desired state, while failed recovery remains physically
hidden. Initial hidden and explicitly hidden windows remain hidden. Visibility
intent received during recovery updates desired state and is applied only after
successful publication. No path reads Winit visibility back.

A stale owner generation or revision drops the packet and does not fall back
into rendering. The redraw kernel returns one private typed disposition:
`Presented`, `RetrySamePacket`, or `DropPacket` (with the existing renderer
failure result kept separate). `Timeout` may return `RetrySamePacket` only
within its bounded nonzero-size permit; exhausted/zero-size timeout drops.
Nonzero `Lost`/`Outdated` invalidates and reconfigures, then may enqueue a fresh
policy-authorized requested packet; zero-size variants drop/defer. `Other` may
use at most one fresh requested recovery packet, never an unsolicited
fallback; out-of-memory, renderer failure, pre/post-acquire veto, missing
device, and no-submission paths drop. A completion promotes the newest pending
successor; when a retry sees a pending successor, that newer packet wins, and
only an empty pending slot retries the exact consuming identity. Offers allocate
their newer revision before any stale-wake reissue, and route-end flushing
rechecks current mailbox state and timestamp. Primary and auxiliary windows use
the same typed begin/redraw/finish kernel and parent-owned observational frame
evidence.

Hide/cache, close/retire, `WindowId` replacement, adapter/device recovery or
loss, and native resource isolation/replacement clear every mailbox state and
advance the owner generation. Ordinary deferred resize and its
target-generation advancement do not clear the mailbox: the packet claimed
for that redraw survives its own target transition. Owner and request
revisions are checked, start at one, and fail closed at exhaustion; neither
wraps or is reused. Primary and auxiliary windows use the same begin/finish
kernel and the same parent-owned observational frame evidence.

### Native encode/present snapshot ticket (private native-window contract)

After deferred Deadline drain, route/input coalescing, deferred resize,
scene admission/reuse, prepared-refresh terminal dispatch, auxiliary sync, and
transient overlay work complete, the native kernel stages volatile GPU
presentation updates into the existing fixed 32-slot mailbox. Staging is
two-phase: an acquisition or lifecycle/renderer veto aborts the snapshot and
retains every selected update; successful presentation commits only the exact
selected presentation revisions, so a newer update admitted while the snapshot
is in flight remains pending for the next frame. The steady-state path uses
the existing preallocated storage and never makes the update mailbox a dirty
bit or render-policy authority.

Immediately before `get_current_texture`, the window's `WindowStageOwner`
admits one crate-private non-`Clone` `NativeEncodePresentTicket`. It binds the
exact stage-owner identity and revision, consuming `NativeVisualRequestPacket`
identity, current adapter/resource generation, post-resize target generation,
running lifecycle, finalized direct-resize or composited path, and a checked
non-wrapping `NativeFrameSnapshotRevision`. No pending visual successor vetoes
this complete snapshot unless lifecycle, resource, or target evidence changes.
Every `get_current_texture`, surface-view creation, scene render, GPU encode,
submit, and present operation is behind that exact current ticket. Each
post-admission veto or failure consumes the exact ticket once without replay;
a wrong ticket preserves the real owner. Direct-resize and composited paths
share the same kernel and success-only completion gate.

Only exact successful completion advances frame sequence and publishes frame
diagnostics/profile, input-to-present latency, last-present timing, and first
reveal. Surface loss, timeout, out-of-memory, renderer failure, lifecycle
transition, resource loss, and target transition preserve the existing
recovery matrix and abort the volatile snapshot. A same-packet retry prepares a
fresh snapshot and fresh ticket. This remains crate-private and does not add a
public API/schema, dirty-bit suppression, renderer retention, render thread,
budget, timestamp, or platform claim.

### Native submission maintenance ticket (private native-window contract)

For normal `Running` native submission maintenance, after the parent scheduler
selects one exact stable window key, that window's `WindowStageOwner` admits
one crate-private non-`Clone` `MaintenanceStageTicket`. The ticket binds the
exact stable window key, the current window-owner adapter and target
generations, the exact active or quarantine resource slot, and the exact
`NativeSubmissionCompletionIdentity` witness. The owner adapter/target
generations fence the `FrameStageIdentity`; the bound resource generation is
independent and may be older for a quarantine slot. Admission requires a
known/valid owner fence, a known resource generation, and a completion witness
with that same resource generation; it does not require resource generation to
equal the current adapter generation. An inactive cached auxiliary may maintain
exact resources but stays visually dormant and cannot present. For one key with
jointly due visual and maintenance work, the first opportunity selects visual;
only after a visual `Deadline` bundle completes successfully while maintenance
remains due does the next same-key fairness-eligible opportunity select bounded
`Maintenance`, even if visual work remains due. An owed same-key maintenance
opportunity remains below another key's `Deadline`.

The ticket executes one bounded nonblocking unit with at most one device
`PollType::Poll`, one completion callback observation/rearm, and one exact
quarantined-resource removal. `NeverSubmitted` quarantine retires only its
exact slot; submitted retirement requires the exact callback; coalesced
completion requires the rearm and second callback; indeterminate completion
remains fenced until exact callback evidence. If work remains, scheduling
records only a bounded future maintenance deadline. Missing, unknown, wrong,
stale, exhausted, fenced, non-running, conflicting, or ambiguous key,
generation, target, slot, or witness evidence is inert: it performs no poll,
rearm, removal, redraw, present, or broad fallback scan, and preserves the
resources and real owner while demand is recomputed. Completion user events
are wake-only and do not perform maintenance or request unrelated `FrameWork`.
This private ticket path is only for normal `Running` submission maintenance;
startup/preflight, recovery, `Closing`, and already-retiring auxiliary cleanup
retain lifecycle-authoritative bounded paths and reuse the same witness
transition kernel.

### Native lifecycle admission ticket (private native-window contract)

An accepted current-generation device-loss callback is a per-window
`SchedulerStage::Lifecycle` operation before it is a recovery-resource
operation. Before any native or controller lifecycle phase changes from
`Running` to `Recovering`, the primary window and every admitted auxiliary
window stage one exact non-`Clone` `LifecycleStageTicket`. The shared
`WindowStageOwner` precomputes checked owner-generation and revision evidence,
atomically retires all lower-stage pending, in-flight, and completion state,
then installs one exact `Lifecycle` identity and owner token. Lower stages are
blocked until that exact ticket completes or is vetoed, and all older
lower-stage tickets are stale.

The private native ticket binds `BeginDeviceRecovery`, source phase `Running`,
the exact stable key and Winit `WindowId` presence, the accepted shared adapter
generation, the exact active-resource generation or absence, the exact
`NativeTargetGeneration` state (including unknown), the target-fenced state,
and the underlying lifecycle ticket. Unknown or absent evidence is exact
evidence; admission does not use ordinary `prepare_fence` or require a usable
post-transition adapter or target. A wrong completion or veto preserves the
real owner; an exact veto preserves the advanced fence.

The existing current registration/generation and publication-capacity
preflight remains authoritative. The callback registration and generation are
rechecked immediately before staging, then the complete primary-plus-
auxiliary ticket set is revalidated synchronously without a yield. Only after
that validation do the existing native/controller recovery hooks apply their
presentation, mailbox, target, diagnostic, and fairness fences; every exact
ticket completes before the existing one-candidate recovery worker starts.
Any staging, currentness, transition, or completion failure vetoes the staged
set, starts no recovery candidate, and converges once through the existing
bounded `Closing` path with the original loss cause. A stale registration or
generation remains inert, visibility and dormancy intent are unchanged, and
recovery/closing maintenance retains its existing bounded authority. This is
crate-private and does not add a public API, schema, queue, worker, or event
loop.

The terminal-only `BeginClosing` admission uses the same private lifecycle
stage owner for whole-run shutdown. The primary and every resident auxiliary
child whose native phase is `Running` or `Recovering` are staged in stable
primary-then-resident-vector order, including active, cached, unmaterialized,
and recovery-pending children whose wrapper is `Admitted` or `Retiring`.
Children already in native `Closing` or `Stopped` are skipped, and a repeated
primary `Closing` or `Stopped` request is inert. Each exact non-`Clone` ticket
binds the stable key, source phase, optional Winit identity, optional adapter
generation, optional active-resource generation, exact target generation
(including unknown), and target-fenced state. Absent adapter evidence is
represented by `None`; `Some(unknown)` is invalid. The terminal owner admission
encodes absent adapter evidence as an unknown lifecycle identity while
retiring all lower-stage state and advancing the owner once; existing recovery
admissions still require a known adapter, and lower-stage admissions retain
their known adapter and target requirements.

The complete ticket set is revalidated without a yield before any native,
controller, presentation, wrapper, cause, recovery, mailbox, or resource
mutation. After validation, existing per-window Closing preparation applies:
recovery-cause precedence and the first terminal cause are preserved,
recovery is cancelled, native and controller phases enter `Closing`, native
accessibility closes, presentation and mailboxes are fenced, fairness,
reopen, and wake state is cleared, and auxiliary wrappers retire without
dispatching close messages. Each exact ticket completes only after its
window's Closing fences. Only then does unchanged bounded resource retirement,
scheduling, and event-loop exit run. Any staging/currentness fault vetoes its
staged tickets once; before terminal intent, failure to admit the primary or an
eligible auxiliary, complete-set currentness failure, or primary Closing
preparation rejection returns inert without cause, recovery, controller,
native, resource, wrapper, visibility, budget, or event-loop mutation, without
ticket-free convergence or automatic retry. A later independent event may
make a fresh admission. Primary native Closing admission occurs before
cause/recovery mutation, preserving recovering-cause precedence even when
preparation is rejected. After primary Closing preparation succeeds, any
auxiliary-preparation, stage-owner, or completion fault converges one-way
through bounded Closing without a cause argument; it requires the primary
already Closing and preserves the first/original cause, without ticket
retry/replay, redraw, visibility restore, or lower-stage execution.
That fallback first invalidates each resident stage owner once, clearing any
surviving lower or lifecycle work and making its tickets stale before bounded
Closing continues.

An independently requested destructive close of a non-cached auxiliary is a
separate child-local `BeginClosing` transaction. Cached `CloseRequested` remains
a hide/reuse path and consumes no lifecycle ticket. For a non-cached admitted
child, the event boundary stages exactly one non-`Clone` ticket carrying the
child key, source phase (`Running` or `Recovering`), optional Winit identity,
optional adapter and active-resource generations, exact target generation
(including unknown), and target-fenced state. The ticket and exact live
`AuxiliaryWindowOwner` are carried to the parent lifecycle route; the parent
revalidates both without a yield before any child, owner, message, wrapper,
recovery, presentation, mailbox, lower-stage, or resource mutation.

Only after that preflight does the child cancel its own recovery while
preserving the original child recovery cause, enter native and controller
`Closing`, fence accessibility, presentation, mailboxes, fairness, wake, and
lower-stage work, and transition its wrapper to `Retiring`. The parent retires
that exact owner generation before dispatching the app-owned close message,
then completes the exact child ticket after all local fences. A missing close
message follows the same accepted path and still receives bounded resource
retirement. A pre-terminal admission, evidence, owner, currentness, or
preparation failure vetoes the staged ticket once, retains the message, and is
inert; a post-terminal owner or completion fault invalidates only the child
stage owner, converges locally, and emits the already-committed close message
exactly once without replay, parent or sibling mutation, or whole-run
shutdown. Retiring removal suppresses same-key recreation through that sync
turn; a later independent sync may recreate the projection. This private
transaction does not widen the whole-run, `DiscreteInput`, or
`ImmediateTransient` contracts and adds no public API.

The parent owns a distinct retiring-auxiliary maintenance deadline on the
canonical 16 ms native-resource cadence. An accepted destructive close arms
that deadline due immediately, even when there is no close message or event
proxy. In `Running` `AboutToWait`, one shared `NativeResourceMaintenanceTurn`
may advance retiring auxiliary children only, preserving the existing one-drop
budget; it rearms to `now + 16 ms` while any child remains and clears the
deadline when none remain. The exact retirement deadline is composed into the
parent `FrameScheduleDeadlines` and `WaitUntil` plan even with zero window
demand. When that retirement opportunity is due, a normal maintenance-stage
ticket is skipped for that opportunity but remains due for the next one.
`NativeResourceMaintenanceRequested` remains wake-only: it may accelerate an
existing retiring deadline to now, but performs no poll, sync, removal, or
message dispatch. `Recovering` and `Closing` retain their existing lifecycle
authorities; a return to `Running` arms due-now cleanup when a retiring child
remains, while `Stopped` clears the deadline. A removal marks deferred sync,
and same-key recreation waits for a later independent sync boundary.

### Native DiscreteInput stage contract

The native event-loop owner admits exactly one non-`Clone` `DiscreteInput`
ticket for each `MouseInput`, `KeyboardInput`, `ModifiersChanged`, or `Ime`
event. Primary and auxiliary windows use the same private stage owner and the
same route boundary. The input timestamp is captured at event arrival before
any forced exact deferred-`Deadline` terminal drainage. If a deferred
`Deadline` ticket is present, that exact owner is drained and completed before
input admission; input is never bypassed, coalesced, replayed, or queued as an
alternative.

Admission binds the stable window key, live native `WindowId`, active native
resource generation matching the adapter, known unfenced target, running
nonterminal native-window eligibility, event kind, and captured timestamp.
Auxiliary admission also binds caller-supplied active, admitted, materialized
wrapper eligibility; inactive, cached, retiring, or unmaterialized children
remain inert. Exact owner and native evidence currentness are revalidated
immediately before routing; a pre-route veto is inert and does not route or
apply lower-stage work. The ticket remains live through synchronous native
routing and message reduction, and completes before lower-stage route-outcome
work such as timed drainage, refresh, redraw, auxiliary synchronization, or
wakeup. A post-route completion mismatch never reroutes, replays, or applies a
lower-stage fallback. A Lifecycle admission may stale the input ticket; the
stale route remains terminal and is not applied.

This slice covers only `MouseInput`, `KeyboardInput`, `ModifiersChanged`, and
`Ime`. Cursor, wheel, focus and cursor-boundary, file/platform/lifecycle,
resize, redraw, and `ImmediateTransient` behavior remain outside this
contract.

### Native ImmediateTransient stage contract

The native event-loop owner admits exactly one non-`Clone` `ImmediateTransient`
ticket for each `Focused(false)`, `Focused(true)`, `CursorEntered`,
`CursorMoved`, `CursorLeft`, or `MouseWheel` event. Primary and auxiliary
windows use the same private stage owner and route boundary; a wheel ticket
retains the exact native `TouchPhase`. The event timestamp is captured at
native arrival before any forced exact deferred-`Deadline` terminal drainage.
If a deferred `Deadline` ticket is present, that exact owner is drained and
completed before transient admission, never bypassed or replayed.

Admission binds the stable window key, live native `WindowId`, active native
resource generation matching the adapter, known unfenced target, running
nonterminal native-window eligibility, exact event kind/phase, and captured
timestamp. Auxiliary admission also binds caller-supplied active, admitted,
materialized wrapper eligibility; inactive, cached, retiring, or unmaterialized
children remain inert. Exact owner and native evidence currentness are
revalidated immediately before routing. An existing `DiscreteInput` owner is
never drained or replaced: transient admission fails inertly and leaves that
owner current. A pre-route veto is fully inert with no retry, replay, fallback,
or lower-stage mutation.

The ticket remains live through synchronous runtime-local mutation, native
routing, message reduction, and the current semantic flush. Exact completion
occurs before timed-frame merge or `Deadline`, refresh/projection, redraw
publication, auxiliary synchronization, wakeup, or presentation. A post-route
completion mismatch never reroutes, replays, or applies a lower-stage outcome;
a wrong ticket never clears the real owner. Focus and cursor boundaries, plus
wheel `Started`, `Ended`, and `Cancelled`, remain non-coalesced. `CursorMoved`
and wheel `Moved` preserve the existing direct/coalesced policy, accumulated
delta, newest metadata, sequence range, axis-change flush, focus-loss clearing,
and admitted queued-sample completion behavior. This private slice adds no
pointer-motion coalescer, fairness consumer, or public API.

`DiscreteInput` and `ImmediateTransient` are authoritative native
`input_transient` soft-budget consumers. Every admitted `DiscreteInput` and
`ImmediateTransient` kind binds the current effective-FPS `input_transient`
budget independently of diagnostics and frame observation. This includes every
`ImmediateTransient` kind: `Focused`, `CursorEntered`, `CursorMoved`,
`CursorLeft`, and `MouseWheel`, including every wheel phase. The typed exact
completion result is either `Completed` with `NotBudgeted`, `Within`, or
`Exceeded`, or `Mismatch`. `Completed(NotBudgeted)` and `Completed(Within)` map
to `ContinueNow`; `Completed(Exceeded)` maps to `DeferLowerPriority`.
`Mismatch` authorizes no policy, fallback, publication, or replay. Stale,
wrong, repeated, vetoed, or lifecycle-invalidated tickets publish nothing and
never clear the real owner.

`Exceeded` completes runtime-local state, semantic routing, coalescer updates,
and message reduction exactly once, then defers only due `Deadline` and
lower-priority visual/publication work through the existing bounded state.
`CursorMoved` and wheel `Moved` remain latest/coalesced; focus and cursor
boundaries, plus wheel `Started`, `Ended`, and `Cancelled`, remain
non-coalesced. Primary and auxiliary routes share the same disposition;
`Exceeded` on an auxiliary route defers sibling synchronization. External-drag
launch occurs only after exact successful completion: `Exceeded` still launches
immediately, and only its visual follow-up defers; `Mismatch` never launches.
`Exit`, lifecycle/terminal intent, discrete input, semantic effects, external
launch, and the event itself are never deferred, replayed, or rolled back.

Every successful native `Recovering`-to-`Running` transition consumes an exact
`FinishDeviceRecovery` Lifecycle ticket. After the fresh primary bundle and
target transition are published, the primary and every admitted auxiliary that
has no rebuild pending (including an unmaterialized auxiliary) are staged as
one no-yield set and synchronously revalidated before any finish phase mutates.
The finish ticket binds source phase `Recovering`, the exact Winit identity and
stable key, the accepted new shared adapter generation, and the exact active
resource generation or absence, target generation, and target-fenced state.
Materialized evidence requires a present Winit identity, an active resource
generation equal to the accepted adapter generation, and a known unfenced
target; unmaterialized evidence is admitted only for an auxiliary key with
absent Winit/resource state, a retained target generation (possibly unknown),
and a fenced target.
Materialized auxiliaries retain the existing lazy one-window rebuild order;
after fresh bundle publication and target transition they stage and revalidate
the exact ticket, apply the local native/controller finish hook, then complete
that exact ticket before rebuild, visibility, or redraw. Retiring auxiliaries
are excluded. Any finish staging, currentness, transition, or
completion failure consumes or vetoes only its exact ticket, performs no retry,
replay, redraw, or visibility restoration, and converges through bounded
`Closing` with the original `RenderDeviceLost` cause.
