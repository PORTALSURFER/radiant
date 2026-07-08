# Radiant Core API

Radiant is a reusable declarative GUI library. Host applications own domain
state and business logic; Radiant owns view-tree identity, layout, input
routing, focus, style resolution, invalidation, renderer-facing paint plans,
typed platform services, and business-work scheduling.
For a contributor-facing map of subsystem ownership, rendering/text/platform
boundaries, and validation lanes, see `docs/ARCHITECTURE.md`. For the preferred
shape of application-facing APIs, examples, and cleanup tickets, see
`docs/API_STYLE.md`.

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

Small stateful apps should use the same message-first model as larger apps.
Widgets emit explicit messages, and the update handler owns durable state
changes:

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
        .view(|state| {
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
event phase. The app-builder `.on_native_file_drop(...)` hook remains available
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

The prelude boundary is intentionally conservative. It should contain common
app-facing types that ordinary declarative UI code reaches for repeatedly:
builders, messages, widget contracts, geometry used in signatures, theme
tokens, backend-neutral paint primitives, and typed runtime commands. Advanced
host-control APIs, renderer or windowing implementation details, backend crates
such as Vello/WGPU/winit, and platform-specific adapters stay on explicit
modules. Examples may import `radiant::runtime`, `radiant::widgets`,
`radiant::layout`, `radiant::theme`, or `radiant::gui` beside the prelude when
they are demonstrating custom widgets, tests, retained surfaces, diagnostics,
or other advanced control; that explicit import is the signal that the example
has moved beyond the common app import set.

| Area | Common prelude entries |
| --- | --- |
| Application setup | `window`, `app`, `IntoView`, `View`, `UiUpdateContext`, `EmbeddedFont` |
| Basic views | `text`, `button`, `button_row`, `toolbar`, `row`, `column`, `scroll`, `scroll_column`, `list`, `list_row`, `empty`, `spacer`, `toggle`, `text_input`, `dropdown_trigger`, `custom_widget` |
| Widget authoring | `Widget`, `WidgetCommon`, `WidgetSizing`, `WidgetInput`, `WidgetOutput`, `PointerButton`, `FocusBehavior`, `ActivationInputPolicy`, `handle_activation_input` |
| Geometry and theme | `Rect`, `Point`, `Vector2`, `LayoutOutput`, `ImageRgba`, `ImageRgbaError`, `Rgba8`, `ThemeTokens` |
| Generic chrome and feedback | `StatusSegments`, `StatusLineLog`, `StatusLineEntry`, `ContentViewChrome` |
| Assets and paint helpers | `SvgIcon`, `SvgIconTintCache`, `SvgIconTintPalette`, `horizontal_progress_fill_rect`, `horizontal_line_rect`, `vertical_line_rect` |
| Paint primitives | `PaintPrimitive`, `PaintClipStart`, `PaintClipEnd`, `PaintFillRect`, `PaintFillRectBatch`, `PaintFillPath`, `PaintPathCommand`, `PaintTransform`, `PaintTextRun` |

Custom widgets can use `Rgba8::new`, `Rgba8::with_alpha`,
`Rgba8::with_alpha_if`, `Rgba8::blend_toward`, and
`Rgba8::blend_opaque_toward` for common color manipulation. Use
`Rect::from_size(width, height)` for origin-based widget, viewport, and test
bounds, or `Rect::from_xy_size(x, y, width, height)` for positioned widget
bounds, instead of repeating `Point` plus `Vector2` construction. Dense
visualizations can use `ColorRamp` and `ColorRampStop` for normalized heatmap
and intensity palettes without local interpolation helpers.

Custom canvas, image, GPU surface, and overlay widgets can use
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
projection. Custom matrix or heatmap widgets can use `DenseGridLayout` and
`DenseGridCell` for reusable row/column cell projection and hit testing.

For paint-plan emission, `WidgetPaint`, `push_fill_rect`,
`push_fill_rect_batch`, `push_stroke_rect`, `push_stroke_rect_batch`,
`push_fill_polygon`, `push_stroke_polyline`, `push_text`,
`PaintTextMetrics`, and `push_text_run_with_metrics` provide the reusable
primitive construction path used by complex examples and custom widgets. Dense
custom widgets can use `push_visible_fill_rect` when derived or clipped
geometry should only enter the paint plan if it has finite positive area. Use
`WidgetPaint::new(...)` when several primitives are emitted for the same custom
widget and local code would otherwise thread the same primitive buffer and
widget id through every helper call.

Timeline, waveform, progress, and scrubber-style custom widgets can use
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
Radiant's generic paint path while the host owns the curve math.

Tests, automation, and embedded hosts that inspect paint plans can use
`SurfacePaintPlan::text_runs()`, `text_labels()`, `text_label_strings()`,
`first_text_run(...)`, `contains_text(...)`, `first_text_run_after_x(...)`,
`contains_text_after_x(...)`, `first_text_rect(...)`,
`first_text_color(...)`, `text_inputs()`, `first_text_input()`,
`contains_text_input()`, `paint_primitives()`,
`contains_paint_primitives()`, `clip_starts()`, `rects()`,
`contains_rect_matching(...)`, `paint_rects()`,
`contains_paint_rect_matching(...)`, `fill_rects()`, `stroke_rects()`,
`fill_polygons()`, `stroke_polylines()`, `svgs()`, and `gpu_surfaces()`.
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
`fill_polygon()`, `stroke_polyline()`, `svg()`, and `gpu_surface()` to query
common paint primitives without app-local exhaustive primitive matches.

## Large Virtual Lists

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
Use `.stream(work, map_event, map_final)` when one worker should report
progressive results, such as progress, preview-ready, and final-ready states,
without exposing UI state to the worker or using an app-local message channel.
Streaming workers receive a `BusinessEventSink<Event>` and emitted events are
mapped back through the normal message queue. `latest(...).stream(...)` tags
both intermediate events and the final output with the same `TaskCompletion`
ticket; keyed/latest resource streams tag events and final output with a
`KeyedTaskCompletion<Key, Output>` so hosts can keep stale-result protection
while adopting staged loading designs.
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
`UiUpdateContext::after_latest(...)` for debounced one-resource UI delays when a
selection, search query, or inspector target should only start after it remains
current for a short delay; the delayed message carries the same ticket type and
should be accepted through the same `LatestTask` methods.
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
`.auxiliary_windows(...)` and `AuxiliaryWindow::new(...)`. Use
`.on_close(message)` to route native close requests back into the host reducer.
Frequently reopened utility windows such as settings panels and inspectors can
also call `.cache_on_close()` so native close hides and retains the prepared
window; a later projection with the same key updates and shows the cached
window instead of recreating the native window and renderer state.
Applications that need lightweight UI-cadence diagnostics can use
`FrameCadenceMonitor` with `FrameCadenceConfig` to classify first-frame,
warning-spike, error-spike, periodic, and normal frame deltas while keeping
application-specific context in the host log payload.
Higher-level application helpers follow the same logical-coordinate sizing
model as view modifiers: fixed details-list columns use `f32` logical widths
through `DetailsColumn::fixed(...)`, matching `.size(...)`, `.fixed(...)`, and
other layout builders instead of introducing a separate integer sizing model.
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
`InteractiveRowMessage::activation_modifiers()`,
`single_activation_modifiers()`, `is_single_activation()`,
`is_double_activation()`,
`secondary_position()`, `drag_message()`, `hover_drop_position()`,
`clear_drop_position()`, and `is_drop()` when custom row widgets need to map
Radiant row interactions into host-specific row messages without repeating
exhaustive event-shape matches.
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
`DragHandleMessage::phase()`, `position()`, `started_position()`,
`moved_position()`, `ended_position()`, `finished_position()`, `is_started()`,
`is_moved()`, `is_ended()`, `is_finished()`, and `is_cancelled()` when reducers
need generic drag lifecycle information or cancellation cleanup without duplicating the
`Started` / `Moved` / `Ended` / `Cancelled` variant shape. Use
`DragHandleMessage::started(...)`, `moved(...)`, `ended(...)`,
`double_activate(...)`, and `cancelled(...)` when tests, reducers, or custom
widgets need to construct drag lifecycle messages directly. Use
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
`Scene::overlay(...)` or `Scene::overlay_opt(...)` for paint-only transient
overlays over the cached scene. Presentation declarations do not become layout
or input children, so they do not change base hit testing, layer ordering, or
widget state synchronization.
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
plus `.logical_size(...)` and `.min_logical_size(...)` when hosts need
fractional logical dimensions.
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
For host-visible platform services, reducers can queue typed
`PlatformRequest` commands through `UiUpdateContext::platform_request(...)`,
`pick_folder(...)`, `pick_file(...)`, `save_file(...)`, `open_path(...)`,
`reveal_path(...)`, `open_url(...)`, `copy_text(...)`,
`copy_file_paths(...)`, `read_text(...)`, `read_file_paths(...)`, or
`confirm(...)`. Custom bridges handle those requests via
`RuntimeBridge::request_platform_service(...)`; bridges that do not provide a
platform service return an explicit unsupported error through the normal
completion callback instead of blocking the UI thread or forcing app code to
depend on a native dialog or clipboard crate.
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
keys, non-positive or non-finite logical sizes, and non-finite popup positions,
returning typed `WindowManifestError` / `WindowSpecError` diagnostics so
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
runtime-local devtools overlay for native inspector builds. The overlay is
disabled by default and paints as runtime overlay content, so ordinary apps do
not pay for inspector presentation unless they enable it.

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

Radiant passes a `TransientOverlayContext` with the latest `SurfacePaintPlan`,
viewport, and animation time. This keeps structural state, layout, and Vello
scene refreshes out of animation paths for visuals such as playheads, drag
previews, tooltip affordances, cursor markers, and lightweight spectrogram
overlays.

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
paint-only presentation work over the cached surface. These descriptors lower to
the same runtime animation and transient-overlay hooks whether they are attached
to `Scene` or to the app builder. Reducer messages request a surface repaint by
default, while frame-clock messages with `repaint_scope(...)` can resolve to
paint-only repaint when the frame update did not require a structural surface
refresh.

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
`gpu_surface(key, revision, GpuSurfaceContent::...)`. This lowers through the
same generated-ID, layout, focus, hit-test, and paint-plan path as standard
widgets, then emits `PaintPrimitive::GpuSurface` for native GPU backends.
Applications that need custom capability flags, runtime pointer-line policies,
or runtime-owned overlay behavior can use
`gpu_surface_with_capabilities(key, revision, content, capabilities)`.
Use `GpuSurfaceConfiguredParts` with `gpu_surface_configured_from_parts(...)`
for advanced named-field construction that also needs lightweight
backend-composited overlays.
GPU surfaces that need host-visible input can use
`gpu_surface_input(key, revision, content, |input| Message::GpuInput(input))`;
plain `gpu_surface(...)` remains passive so pointer motion over retained visual
surfaces does not force unnecessary message dispatch or relayout.

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

`RuntimeBridge` remains the single explicit adapter trait for custom hosts, but
its hooks are organized by responsibility rather than by backend: surface
projection, state updates and input policy, runtime scheduling, platform
services, runtime-owned queues, animation policy, retained/transient rendering,
diagnostics, and lifecycle. Most applications should reach those responsibilities
through `radiant::app(...)`; custom bridges should override only the groups they
own instead of using the trait as a second application framework.
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
logical composition range once that generic event exists. Bidirectional text and
complex shaping belong to renderer text layout and cursor-stop mapping, while
`TextInputState` continues to store logical Unicode-scalar positions instead of
renderer glyph positions.

Implement `Widget` directly when a downstream application needs a new focusable
leaf with its own input handling, host-routable output payload, or
backend-neutral paint contribution. Compose existing primitives when the desired
control is only a row, column, stack, styling change, message mapper, or
combination of built-in widgets. `SurfaceNode::widget` and
`SurfaceNode::static_widget` accept any owned widget implementation, including
Radiant's built-in primitives, without requiring an enum wrapper.

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
available for custom layout policy.
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
Pointer-driven custom widgets should keep transient hover and cursor state local
when the state is only paint chrome. Leave `Widget::accepts_pointer_move()`
enabled for widgets such as timelines, canvases, and editors that need stable
pointer moves after hover has already entered the widget. Those stable
`PointerMove` events request repaint even when `handle_input` returns `None`,
so a snapped cursor, clip hover, or resize-handle preview can refresh smoothly
without emitting host messages or forcing the app reducer to run for every mouse
move. Captured drag motion follows the same contract: if the active widget only
changes local preview chrome, it can repaint locally without a host message.
Emit a `WidgetOutput` only when the host-owned model changes, such as seek,
create, move, resize, or delete.
Use `Widget::pointer_capture_policy()` for widgets that need to control pointer
motion while they own capture. `PointerCapturePolicy::Exclusive` is for
splitters, resize handles, and similar controls that should not activate hover
or pointer-motion behavior on unrelated widgets before release. During retained
surface refreshes, exclusive capture also clears copied hover state from
non-captured widgets while preserving durable widget state. The default
`PointerCapturePolicy::PassThrough` keeps drag-source behavior where widgets
under the pointer can still receive live feedback while the source remains
captured. Older custom widgets that only override
`Widget::allows_captured_pointer_pass_through()` keep the same behavior through
the default policy implementation.
Native focus loss and external drag handoff cancel pointer capture without
routing a synthetic release to the host. Radiant clears the captured widget's
transient retained state through the widget input path and requests repaint
only when that local state changed. Hosts should model durable drag/drop
results as messages, but they should not duplicate generic pressed, capture, or
focus-loss cleanup in application reducers.
Custom widgets must still be pointer hit-test eligible before pointer hooks can
run. Use `WidgetCommon::with_pointer_focus()` for hover, drag, tooltip, cursor,
or paint-only overlay widgets that should skip keyboard traversal, or
`WidgetCommon::with_keyboard_focus()` when the same custom surface also handles
keyboard input.
High-frequency editor widgets can go further with
`Widget::prefers_pointer_move_paint_only()` and
`Widget::append_runtime_overlay_paint(...)`. Put pointer-following visuals such
as timeline cursor lines, hover outlines, captured drag previews, and resize
handles in the runtime overlay hook, then keep the stable base widget paint free
of those transient states. The native Vello runtime can then present those
overlay rectangles over the cached scene on stable pointer motion and captured
paint-only drag motion instead of rebuilding the Vello scene for every
mouse-move event. Widgets that paint pointer-motion state in `append_paint(...)`
should not opt into the paint-only pointer path.

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
Host-side unit tests that need to execute queued business work without a runtime
adapter can use `Command::run_inline_for_tests(...)`. It runs `Message`, `Batch`,
`Perform`, and `PerformStream` commands synchronously and preserves streamed
message order, while intentionally ignoring repaint, timer, focus, drag,
platform, window, and exit commands that require an installed runtime adapter.
Tests and diagnostics can use `Command::business_task_priority(...)` to verify
that a named one-shot or streaming business command was queued on the expected
runtime worker lane without pattern-matching hidden command internals.
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
`WidgetOutput::custom(...)` remains an alias for user-defined widget payloads,
and `WidgetOutput::typed_cloned::<T>()`, `typed_copied::<T>()`,
`custom_cloned::<T>()`, and `custom_copied::<T>()` provide owned payload
extraction for tests, automation, and custom-widget adapters without repeating
manual downcast chains. `WidgetMessageMapper::dynamic(...)` is available when a
host needs manual downcast or filtering behavior. Adding a widget should not
require adding a central output enum variant.

Asynchronous business work remains host-owned, but normal apps use Radiant's
app runtime to wire it into the UI. `UiUpdateContext::business()`,
`UiUpdateContext::after(...)`, typed platform-service helpers, and
`Subscription` provide message delivery and repaint wakeups; the app still owns
the work and resulting domain messages.

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
unbounded background threads. Interval subscriptions use the same timer lane for
recurring ticks; receiver-backed worker subscriptions keep a dedicated thread
only when they must wait on a host-owned blocking receiver.

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
stays disabled by default. The overlay paints a compact surface tree,
selected-node detail panel, and runtime summary from backend-neutral paint
primitives.

## Layout

`radiant::layout` provides slot-based measurement and placement. Containers use
`ContainerPolicy`, `ContainerKind`, `SlotParams`, and `LayoutNode` to describe
rows, columns, overlays, fixed sizing, fill behavior, spacing, padding, and
stable output rectangles. Layout is deterministic and independent from any
renderer backend.
`LayoutOutput::rect_for` and `LayoutOutput::rect_for_clamped` provide the
shared measured-rectangle lookup contract for adapters that need stable
fallback bounds after a layout pass.

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
`RuntimeBridge::scroll_updated(ScrollUpdate)`.
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
`CompactOptionListParts`, `compact_option_list(...)`, and
`compact_option_list_from_parts(...)` for selected primary/secondary option
rows in autocomplete popups, command palettes, compact pickers, and similar
transient result lists while the host keeps ownership of option values and
messages. Use `compact_option_list_from_parts_with_activation(...)` and
`compact_option_list_anchored_with_activation(...)` when primary option-label
activation should map a clicked row index into a host message without the host
rebuilding the compact-list row chrome. Use
`compact_option_list_from_parts_with_interaction(...)` and
`compact_option_list_anchored_with_interaction(...)` when pointer hover and
activation should both map row indices into host messages while preserving the
same compact-list chrome. Use `CompactOptionListFloatingAboveParts` and
`compact_option_list_floating_above(...)` when such a result list should be
anchored above an editor or trigger inside the same stack layer without
app-local height and floating-offset arithmetic. Use
`CompactOptionListAnchoredParts` and `compact_option_list_anchored(...)` when
the same list should be projected in a parent-anchored overlay layer, such as a
full-surface autocomplete layer above a bottom panel.
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
through the application builders `gpu_surface(...)`,
`gpu_surface_with_capabilities(...)`, `gpu_surface_configured_from_parts(...)`,
or `gpu_surface_input(...)`, or through `GpuSurfaceWidget` in lower-level host
code. GPU surfaces are still normal Radiant widgets: they own stable identity,
receive layout bounds, can route widget input, and paint through the same
`SurfacePaintPlan` as Vello-backed widgets.

Use retained GPU surfaces for dense visuals where the payload is naturally
texture, signal, or shader data: waveform bodies, meters, scopes, large preview
atlases, and other surfaces that benefit from backend-owned GPU caches. Keep
normal panels, controls, labels, selection chrome, and editor overlays in
standard Radiant widgets unless they need custom GPU resources. The public
contract is `key` plus `revision` plus validated `GpuSurfaceContent`; bump the
revision only when the retained GPU payload changes, and keep transient cursor
or drag previews in overlays or paint-only repaint paths. This preserves one
Radiant widget model instead of creating separate Vello and WGPU application
models.

`PaintGpuSurface` supports the built-in v1 content payloads
`GpuSurfaceContent::RgbaAtlas`, `SignalBands`, and `SignalSummaryBands`, plus
`GpuSurfaceContent::CustomShader` for advanced surfaces that need to carry
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
`@group(0) @binding(2)`. Native frame diagnostics expose direct custom-shader
work, including custom shader pipeline rebuilds, under
`NativeGpuSurfaceDiagnostics::custom_shader`: `surfaces_rendered`,
`pipeline_rebuilds`, `binding_rebuilds`, and `binding_cache_hits`, so rendered
surfaces and shader pipeline/bind-group cache activity stay distinct from
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
`GpuSurfaceContent::validate()` returns a typed `GpuSurfaceContentError` for
invalid atlas rectangles, signal ranges, empty payloads, and summary-shape
mismatches. `is_renderable()` and `signal_render_shape()` remain convenience
checks over the same shared payload contract used by widget projection and
native renderers, so invalid signal shapes or empty texture sources do not leak
into backend work.
Runtime behavior is declared explicitly through `GpuSurfaceCapabilities`:
`fast_pointer_move` allows pointer-motion overlay updates without reprojecting
the app surface, `coalesce_vertical_wheel` allows vertical wheel deltas to be
batched until redraw, and `runtime_overlays.pointer_vertical_line` lets the
native runtime compose a lightweight pointer-following vertical line. These
capabilities are part of the GPU-surface
contract, not side effects inferred from overlays. Custom shader program support
should extend this descriptor and diagnostics contract rather than adding
backend-specific runtime special cases.

Native runtime entry points return `RuntimeRunReport<Artifacts, Error>` when
artifact capture is requested. The report envelope is generic: Radiant owns the
result transport while each runtime path chooses its artifact payload and typed
error boundary. The generic Vello runtime reports `NativeGenericRunError`
variants for event-loop build and run failures, while simple `.run()` helpers
continue returning the compatibility `radiant::Result` string form. This keeps
compatibility diagnostics and generic runtime diagnostics on the same mechanism
without coupling the public runtime API to a host application model.
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
  layout. `SurfaceRuntime` owns focus target, widget hit testing, and message
  dispatch.

## Event And Focus

Backend input is normalized into Radiant input primitives such as
`Event`, `WidgetInput`, `PointerButton`, and `WidgetKey`. The runtime performs
hit testing, pointer capture, focus changes, pointer press/release routing,
keyboard routing to the focused widget, and message mapping. `Event` is the
backend-neutral runtime event surface for resize, pointer, keyboard, focus
traversal, and focus-clear operations; `SurfaceRuntime::dispatch_event` is the
primary event-routing entry point for backend adapters. Focus behavior is
declared by widget contracts rather than by host-domain code.
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
`FocusTraversal` expose deterministic keyboard focus ownership and traversal.
`UiSurface::keyboard_focus_order_into(...)` writes the same deterministic order
into caller-owned storage for diagnostics or host integrations that inspect
focus order repeatedly without reallocating.
Pointer dispatch through `dispatch_input_at` can assign focus from hit testing;
keyboard dispatch through `dispatch_focused_input` routes input to the focused
widget by stable `WidgetId`.
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
`key_press(...)`, `character(...)`, `traverse_focus(...)`, `clear_focus(...)`,
and `scroll(...)` instead of repeating backend-neutral event struct literals.
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
`total_us`, and `avg_us`, plus any scenario-owned counters such as
`scene_rebuild_count`, `paint_only_count`, `surface_refresh_count`,
`relayout_count`, `text_cache_hit_count`, `retained_surface_cache_hit_count`,
`gpu_surface_count`, `frame_cadence_due_count`,
`frame_cadence_wait_count`, and `allocation_sensitive_work_count`. This keeps
performance history parseable without scraping prose or losing which target
area and review-risk group the scenario validates.
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
| `retained_gpu_surfaces` | GPU surface projection, retained atlas/custom shader paths, signal-summary data preparation, renderer cache diagnostics | `cargo bench --bench perf_harness -- --group retained_gpu_surfaces --jsonl` |
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
  `app_virtual_list_window_projection_10k`
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
  `runtime_command_drain_1k`, and `runtime_nested_command_drain_1k`
- Resource lifecycle scenarios: `resource_slot_stale_completions_1k`
- Text scenarios: `text_line_cache_1k`, `text_word_selection_1k`, and
  `text_word_deletion_1k`
- GPU data and surface scenarios: `gpu_signal_summary`, `gpu_surface_projection`,
  `gpu_surface_stack_projection_128`, and `gpu_custom_shader_projection`

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
same counters are also exposed to custom hosts through
`RuntimeBridge::observe_frame_diagnostics(...)` as `NativeFrameDiagnostics`, so
apps can collect frame diagnostics without parsing logs. The scene diagnostics
are grouped into `traversal`, `text`, `media`, and `surfaces` buckets so hosts
can inspect paint-plan traversal, text encoding, image/SVG encoding, and
GPU/custom-surface handoff without treating the payload as one flat counter bag.
The profile separates retained-surface bridge/cache/miss counts,
custom-surface fallback counts, GPU-surface render/cache counts,
transient-overlay primitive counts, and timing for surface
refresh, paint-plan generation, Vello render-to-texture, composed-base refresh
or cache hits for transient overlays, transient-overlay paint callbacks,
GPU-surface composition, and presentation.
`NativeFrameTimingDiagnostics::gpu_timing_status` currently reports
`NativeGpuTimingStatus::CpuEnvelopeOnly`, which makes the boundary explicit:
these timing buckets are CPU-side encode/submit/present envelopes, not backend
GPU timestamp query durations. Future timestamp-query support should extend this
status instead of silently changing the meaning of existing timing fields.
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
| Layout, scrolling, and virtualization | `layout_rows_columns`, `grid_gallery`, `scroll`, `sizing`, `list`, `virtualized_list` |
| Styling, theming, and reusable widgets | `styling`, `theme_playground`, `widget_gallery`, `toolbar_icons`, `svg`, `form`, `volume_slider`, `passive_widgets` |
| Input, focus, menus, and editor interactions | `focus_controls`, `keys`, `scene`, `context_menu`, `floating_overlay`, `tree_and_details`, `folder_browser`, `paint_helpers` |
| Custom widgets and retained GPU surfaces | `custom_widget`, `gpu_surface`, `custom_shader_surface`, `gpu_surface_stack_overlay`, `waveform_view`, `spectrogram` |
| Advanced creative-tool surfaces | `node_editor`, `timeline_editor`, `plugin_panel`, `eq_editor`, `spectrogram`, `mixer_console`, `piano_roll`, `modulation_matrix`, `arrangement_shell`, `inspector_panel`, `split_workspace` |
| Text, diagnostics, and performance inspection | `typography`, `layout_diagnostics`, `rendering_benchmark`, `host_surface_frame` |
| Window and host integration | `multi_window_manifest`, `popup_window`, `host_surface_frame`, `dpi_scaling` |

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
retained `GpuSurfaceContent::SignalSummaryBands` surface. It still
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
Run `cargo run --example todo_list` for text input, submit binding, row
selection, drag handles, drop markers, and scroll composition in one small app.
Run `cargo run --example form` for text binding and boolean controls.
Run `cargo run --example gpu_surface` for a small retained-GPU-surface sandbox
that uses the prelude `gpu_surface(...)` application builder with generated
demo atlas data.
Run `cargo run --example custom_shader_surface` for a checked custom shader
surface sandbox that builds `GpuSurfaceContent::CustomShader` with a
backend-neutral `GpuShaderSurfaceDescriptor` carrying executable WGSL source
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
Dismissible context menus can use `dismissible_context_menu_auto_width(...)`
when Radiant should derive compact menu width from the title and command labels
while also using the standard compact menu height. Use
`dismissible_context_menu_with_width_policy(...)` with
`MessageMenuWidthPolicy` when an app needs custom min/max menu width bounds, or
`dismissible_context_menu_with_width(...)` when the width is deliberately fixed.
When a context menu is declared as a `Scene` layer and should use
`Layer::dismiss_on_outside_click(...)`, use
`anchored_message_menu_overlay_auto_width(...)` or its explicit-width variants
for the foreground-only menu content so dismissal stays owned by the scene
layer policy. The older `message_context_menu_overlay_*` helpers remain
compatibility wrappers over the same anchored menu primitive.
These helpers avoid app-local `message_menu_height(...)` sizing and hard-coded
context-menu width constants.
Run `cargo run --example scene` for the preferred root-scene sandbox. It
keeps the root `Scene` focused on base layout, shortcuts, frame clocks, and
paint-only overlays while status-bar, browser, and workspace components declare
their own popovers, context menus, modals, tooltips, and drag previews as
view-local transient layers.
Run `cargo run --example native_file_drop` for a view-local native OS file-drop
target that maps `NativeFileDrop` events into normal app messages.
Run `cargo run --example context_menu` for a generic menu/context-menu sandbox
that composes `MenuCommand`, `message_menu(...)`, and
`anchored_message_menu_overlay(...)` with normal app messages.
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
Run `cargo run --example gpu_surface_stack_overlay` for a retained GPU surface
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
application interactions. Use `icon_button(...).passive()`,
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

The local validation lane runs formatting, Clippy with warnings denied, library
and integration tests, checked examples, rustdoc with broken intra-doc links
denied, doctests for public documentation examples, no-default-features library
checks for the documented Linux and macOS targets, and a perf-harness smoke pass
that lists scenarios and proves baseline capture/comparison with
`--fail-on-missing-baseline`.

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
Computer Use sidecars, and future native adapters that need stable GUI targets
without coupling to host state. Directional focus hints and live-region values
are backend-neutral hints only; Radiant does not implement AccessKit,
screen-reader bridges, web accessibility, or OS accessibility trees in the
current phase.
The macOS development app-bundle helper improves process/window discovery for
app-level automation tools. `RADIANT_AUTOMATION_TARGET_EXPORT` pairs with that
launch path by exposing the current flattened target snapshot to external
sidecars, but it does not replace the semantic automation snapshot and does not
by itself expose per-widget native accessibility nodes.

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
`SplitPaneSlot`, `SplitPaneAssignmentState`, `SplitPaneAssignedRow`,
`SplitPaneTreePanel`, and `SplitPaneSidebarState`, plus `anchored_panel_rect`
for clamped popup/panel placement and `PanelResizeState` with
`PanelResizeConstraints` or `CollapsiblePanelResizeConstraints` for
splitter-driven pane resizing. Use `PanelResizeConstraints::left(...)`,
`right(...)`, `top(...)`, or `bottom(...)`, and the matching
`CollapsiblePanelResizeConstraints` constructors, for common edge-specific
resize handles. Use `PanelResizeState::resize_collapsible(...)` when a resize
handle should collapse the panel to a host-chosen size on double activation,
then restore the last expanded size on the next double activation.
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
