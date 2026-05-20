# Radiant Core API

Radiant is a reusable declarative GUI library. Host applications own domain
state and side effects; Radiant owns view-tree identity, layout, input routing,
focus, style resolution, invalidation, and renderer-facing paint plans.
For a contributor-facing map of subsystem ownership, rendering/text/platform
boundaries, and validation lanes, see `docs/ARCHITECTURE.md`.

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
  stateful apps.
- `radiant::runtime`, `radiant::widgets`, `radiant::layout`, `radiant::theme`,
  and `radiant::gui` expose the same model with more explicit control over
  projection, commands, sizing, layout, styling, input, invalidation, and
  backend integration.

## Application API

Radiant's application API is designed to be easy to read without hiding the
runtime model. `radiant::prelude` re-exports the common symbols: `window`,
`app`, `text`, `button`, `row`, `column`, `scroll`, `scroll_column`, `list`,
`list_row`, `toggle`, `text_input`, `custom_widget`, `IntoView`, `View`,
`StateView`, `Command`, `EmbeddedFont`, `StatusSegments`, `StatusLineLog`,
`StatusLineEntry`, `ContentViewChrome`, common custom-widget authoring contracts such as
`Widget`, `WidgetCommon`, `WidgetSizing`, `WidgetInput`, `WidgetOutput`,
`PointerButton`, `FocusBehavior`, and backend-neutral paint primitives such as
`PaintPrimitive`, `PaintFillRect`, `PaintFillPath`, `PaintPathCommand`,
`PaintTransform`, and `PaintTextRun`. It also includes the geometry, layout, image, color, and theme
types needed in widget method signatures, including `Rect`, `Point`, `Vector2`,
`LayoutOutput`, `ImageRgba`, `ImageRgbaError`, `Rgba8`, and `ThemeTokens`, plus
app-facing asset helpers such as `SvgIcon`, plus the builder types needed by method chains. These
builders lower into the same `UiSurface`, `SurfaceNode`, `SurfaceChild`,
`WidgetSizing`, and `RuntimeBridge` contracts available through the explicit
runtime modules.

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

Small stateful apps can mutate state directly from widget callbacks while still
lowering into the same message/command runtime:

```rust
use radiant::prelude::*;

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
                button("Increment").on_click(|state: &mut State| {
                    state.count += 1;
                }),
            ])
        })
        .run()
}
```

Application builders generate deterministic structural IDs during projection and
provide default widget sizing. Production apps and tests can opt back into
explicit control with `.id(...)`, `.sizing(...)`, `.size(...)`, `.fixed(...)`,
`.min_size(...)`, `.preferred_size(...)`, `.baseline(...)`, and `.spacing(...)`.
Rows, columns, and fixed-column grids use intrinsic main-axis child sizing by
default, so list rows and grid tiles do not stretch just because there are only
a few items. Apps can request
stretch behavior explicitly with `.fill()`, `.fill_width()`, `.fill_height()`,
and `.grow(...)`, add container padding with `.padding(...)`, `.padding_x(...)`,
and `.padding_y(...)`, and use `.primary()`, `.danger()`, `.subtle()`,
`.wrap()`, `.truncate()`, or `.align_text(TextAlign::Center)` for common style
and text policies. Stateful
examples should use stable keys or explicit IDs for controls whose focus or
input state must survive list edits. The launch builders expose `.options(...)`
for callers that need the full `NativeRunOptions` surface. Apps that prefer
explicit message routing can use `.message(...)` on widgets plus `.update(...)`
or `.update_command(...)` on the app when reducers need to return
`Command<Message>` values directly. Reducers that need the full app runtime can
use `.update_with(...)` and an `UpdateContext<Message>` to emit messages,
request repaint, move focus, start background work, schedule delayed messages,
or request runtime exit. Use `LatestTask` with `UpdateContext::spawn_latest(...)`
for one-resource background loads where a newer selection should invalidate an
older completion. The resulting message receives a `TaskCompletion<Output>`;
call `LatestTask::finish(completion.ticket)` before applying the output so stale
work is rejected consistently without host-specific task-id plumbing. Use
`KeyedLatestTasks` with `UpdateContext::spawn_latest_for(...)` when the same
replace-latest behavior is needed independently for many keys, such as row
previews, folder scans, or document-local workers.
Text inputs can use `.message(...)` for value-only routing or
`.message_event(...)` when the host needs to distinguish edits from submissions.
Inline edit flows can seed caret and selection state with `.selection(...)` or
`.select_all()` while staying on the application-builder path.
Higher-level application helpers follow the same logical-coordinate sizing
model as view modifiers: fixed details-list columns use `f32` logical widths
through `DetailsColumn::fixed(...)`, matching `.size(...)`, `.fixed(...)`, and
other layout builders instead of introducing a separate integer sizing model.
Large list and tree-style surfaces can use `VirtualListController` when they
need durable item-index viewport state outside the declarative scroll container.
It wraps the existing virtual-window, row-scroll, focus guard-band, and
scrollbar projection helpers so applications do not need to keep viewport-start
bookkeeping beside each large list.
Timeline and waveform-style surfaces can use `IndexViewport` for generic
integer range navigation. It owns clamping, visible fraction, scrollbar offset,
anchor-preserving zoom, visible-span pan, and visible-to-absolute ratio
projection so apps do not need to keep small-but-risky viewport math beside
every custom canvas.
Custom canvas widgets can use `CanvasGestureState` to turn raw `WidgetInput`
pointer events into local and normalized hover, press, drag, release,
double-click, drop, wheel, and focus-change events. This keeps waveform,
timeline, node-editor, and other direct-manipulation widgets on a shared
backend-neutral interaction contract while the application still owns domain
actions such as range selection or marker editing.
Retained custom surfaces can use `RetainedSegmentPlan` with
`RetainedSegmentRevisions` to name static and overlay paint segments, derive
stable invalidation masks, and bump only the revisions affected by a change.
This keeps segment ownership explicit for dense retained surfaces without each
application inventing a separate bit layout and diagnostic vocabulary.
`NativeRunOptions` keeps platform/window integration policy behind Radiant's
native runtime boundary. Common launch code can stay platform-neutral while
still configuring window title, logical size, minimum size, maximized state,
decorations, icon, target frame rate, and whether native file drag-and-drop is
requested on platforms that support it. Native animation frame rates are
normalized through `NativeRunOptions::normalized_target_fps()` and the exported
`MIN_NATIVE_TARGET_FPS` / `MAX_NATIVE_TARGET_FPS` bounds before timed redraws
or present-mode selection use them. Focused text-input caret animation uses a
lower native cadence when it is the only timed animation demand, while explicit
application or overlay animation frame-rate caps remain authoritative. Window
launch and manifest builders provide integer `.size(...)` convenience methods
plus `.logical_size(...)` and `.min_logical_size(...)` when hosts need
fractional logical dimensions.
For host-visible platform services, reducers can queue typed
`PlatformRequest` commands through `UpdateContext::platform_request(...)`,
`pick_folder(...)`, `pick_file(...)`, `save_file(...)`, `open_path(...)`,
`open_url(...)`, or `confirm(...)`. Custom bridges handle those requests via
`RuntimeBridge::request_platform_service(...)`; bridges that do not provide a
platform service return an explicit unsupported error through the normal
completion callback instead of blocking the UI thread or forcing app code to
depend on a native dialog crate.
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
range, select-all, and revision tracking for dense virtual lists. Applications
keep durable row identity in their own model and map Radiant's selected indices
back to paths, database ids, or other domain keys after filtering and sorting.
`CancellationToken` and `UpdateContext::spawn_cancellable(...)` provide a
small cooperative-cancellation contract for long host-owned jobs. Radiant still
does not force-stop work; applications keep a token clone and workers check it
at natural boundaries before returning early.
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

Serious apps use the same builder API. `radiant::app(...)` supports
`.subscriptions(...)` for interval and worker-message sources, `.animation(...)`
for frame-cadenced redraws, optional `.on_frame(...)` messages for animations
that mutate application state, `.on_scroll(...)` for observing
runtime-owned scroll offsets, `.on_startup(...)`, `.on_shutdown(...)`,
`.on_close_requested(...)`, `.run_with_artifacts()`, and retained-surface
painters registered through `.retained_painter(...)`. Apps with small
frame-rate overlays can register `.transient_overlay(...)` to append
backend-neutral paint primitives over the cached scene each present frame.
Radiant passes a `TransientOverlayContext` with the latest `SurfacePaintPlan`,
viewport, and animation time. This keeps structural state, layout, and Vello
scene refreshes out of animation paths for visuals such as playheads, drag
previews, tooltip affordances, cursor markers, and lightweight spectrogram
overlays. Use `.transient_overlay_animation(...)` or the combined
`.animated_transient_overlay(...)` helper when an overlay can derive motion from
`context.animation_time`; the native runtime then schedules paint-only frames
over the cached surface without queuing application frame messages. Use
`.transient_overlay_animation_at(...)` or
`.animated_transient_overlay_at(...)` when an overlay should run below the
window's maximum native cadence; Radiant clamps the requested rate through the
same `NativeRunOptions` frame-rate bounds and never exceeds the window target.
Custom runtime bridges can report the same split explicitly with
`RuntimeAnimationActivity`, distinguishing frame-message animation from
paint-only presentation work and optionally carrying a per-activity target FPS.
When a paint-only transient overlay is present, the native Vello runtime also
caches the composed Vello scene plus retained GPU surfaces as a base frame, so
later overlay-only frames can present that stable composition and draw the
moving overlay without re-encoding retained GPU surfaces until the scene, paint
plan, or runtime GPU-surface overlays change.
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
Applications that need custom capability flags or overlays can compose the same
path explicitly with `widget(GpuSurfaceWidget::new(...).with_capabilities(...))`.
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

An application is host-owned state plus a projection function and reducer.
Radiant does not define the domain model. The public `App<Message>` contract is
implemented by every `RuntimeBridge<Message>`: hosts can provide a custom bridge
or use `declarative_runtime_bridge(state, project, reduce)` to project an
immutable `UiSurface<Message>` from state and reduce messages back into state.
Apps whose update flow returns runtime-visible follow-up work should use
`radiant::app(...).update_command(...)` or `.update_with(...)`. The app builder
lowers into Radiant's bridge internally while keeping side effects and domain
state host-owned. Low-level hosts can still provide a custom bridge or use
`declarative_command_runtime_bridge(state, project, update)` when embedding
Radiant outside the application builder.

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

Single-line text editing is split between reusable state and widget routing:
`TextInputState` owns the portable value, caret, and selection model, while
`TextInputWidget` adapts that model to `WidgetInput` and emits
`TextInputMessage`. Custom retained surfaces that draw their own field chrome can
use `TextInputState::apply_edit_command`, `apply_key`, `insert_text`, and
`set_caret` directly instead of reimplementing paste sanitization, selection
replacement, Unicode-scalar caret movement, and character-limit behavior. For
host-rendered editors, `has_selection`, `clear_selection`, `replace_selection`,
`delete_selection`, and the borrowed `selected_text_slice` expose the same
reusable single-line replacement semantics without requiring a full
`TextInputWidget` or allocating just to inspect the active UTF-8 selection.
Widgets that participate in focused text editing can also expose borrowed
selection text through `Widget::selected_text_slice`, and
`SurfaceRuntime::focused_text_selection_slice` keeps runtime-level focus
inspection on the same allocation-free path. The owned
`focused_text_selection` helper remains available for callers that need to keep
the selection after releasing the runtime borrow.

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
widgets can use `custom_widget_mapped(widget, |payload| message)` for typed
custom outputs, or `MappedWidget::new(widget, WidgetMessageMapper::...)` when
they need an explicit mapper object. For fully dynamic custom output,
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

## Message And Command

Radiant routes widget outputs into host-defined `Message` values through
`WidgetMessageMapper`. `SurfaceRuntime` dispatches input, emits mapped messages,
calls the host update hook, executes returned commands, and requests a fresh
surface snapshot. `RuntimeBridge::reduce_message` remains the simplest reducer
hook for hosts that only mutate state; `RuntimeBridge::update` can return
`Command<Message>` for hosts that need runtime-visible follow-up work.
`SurfaceRuntime::dispatch_message` and `SurfaceRuntime::execute_command` both
return `CommandOutcome` with dispatched-message and repaint-request summaries.
`Command<Message>` is the generic runtime-visible follow-up value for host
reducers that need to queue messages, batch runtime-visible work, request
repaint, schedule delayed messages, run background work, move focus, or request
runtime exit. Hosts that inspect only the immediate messages in a command can
use `Command::into_messages_into(...)` to reuse caller-owned storage, while
`Command::into_messages()` remains the allocating convenience wrapper.
`RepaintScope` is the typed repaint specificity contract: `Surface` requests a
surface refresh plus repaint, while `PaintOnly` repaints the current paint plan
for overlay-only motion. Reducers can queue `Command::repaint(scope)` or
`UpdateContext::repaint(scope)`, and diagnostics can inspect
`Command::repaint_scope()` to see the merged effective scope for nested command
batches. Mixed batches promote to `Surface` so a paint-only overlay request
cannot accidentally suppress a needed surface refresh.
`ResourceSlot<T>`, `ResourceRequest`, `ResourceLoad<T>`, `ResourceCompletion<T>`, and
`ResourceLoadState` provide a small runtime-level state contract for host-owned
background resource work. Radiant does not own the filesystem or asset decoder,
but examples and apps can use the same key/state/result shape for loading
images, previews, manifests, fonts, or other resources through
`Command::perform(...)`, `UpdateContext::spawn(...)`, or the higher-level
`UpdateContext::spawn_resource(...)`. Use
`ResourceSlot::begin_load()` and `ResourceSlot::apply_for(...)` when repeated
loads for the same key can overlap; stale worker completions are ignored instead
of replacing the current result. `ResourceRequest::ready(...)` and
`ResourceRequest::failed(...)` construct keyed results from the request token so
worker code does not need to clone or duplicate resource-key text manually.
`spawn_resource(...)` performs that request/result wiring for fallible resource
loads and returns a `ResourceCompletion<T>` through the normal message path.
Use `ResourceSlot::cancel_load()` to invalidate in-flight work while preserving
the last ready value; use `ResourceSlot::clear()` when the value and error
should be dropped.

Any widget can emit its own output type with `WidgetOutput::typed(...)` and
route it with `WidgetMessageMapper::typed(...)`. Built-in primitive modules may
provide typed convenience mappers such as `WidgetMessageMapper::button`, but
those mappers are also owned by the primitive module rather than the runtime
surface core.
`WidgetOutput::custom(...)` remains an alias for user-defined widget payloads,
and `WidgetMessageMapper::dynamic(...)` is available when a host needs manual
downcast or filtering behavior. Adding a widget should not require adding a
central output enum variant.

Asynchronous side effects remain host-owned, but normal apps use Radiant's app
runtime to wire them into the UI. `Command::perform(...)`, `Command::after(...)`,
`UpdateContext`, and `Subscription` provide message delivery and repaint
wakeups; the app still owns the work and resulting domain messages.

## UI-First Runtime Threading

Radiant treats the native UI/event/render owner as the priority path. The
window event loop, input routing, repaint requests, surface refresh, and native
Vello presentation must stay responsive and should not wait on application
business work.

Application reducers run synchronously because they decide the next UI state, so
they should stay short. Slow IO, decoding, indexing, analysis, loading, and other
business work should use `Command::perform(...)`, `UpdateContext::spawn(...)`,
`Command::after(...)`, or `Subscription`; the application runtime offloads that
work to runtime-managed business threads and returns results through the normal
message queue. Finite `Command::perform(...)` jobs run on a bounded business
worker lane so bursts of host work do not create unbounded OS threads beside the
UI path. If that lane cannot be started or a job cannot be queued, Radiant
reports the offload failure instead of running the work synchronously on the
UI/event/render owner. If an app explicitly needs immediate synchronous
behavior, it can dispatch a normal message and do that short work in the
reducer, but the default architecture is UI-first and non-blocking.
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
Application-builder code that owns a resolved logical window can use
`virtual_list_window(...)` for fixed-height rows; it preserves full scroll
extent with spacer rows while only projecting the materialized item range.
Apps that drive a host-owned logical window from native scrolling can observe
runtime-owned scroll containers with `.on_scroll(...)` or, for custom bridges,
`RuntimeBridge::scroll_updated(ScrollUpdate)`.
`virtual_list_view_start_after_scroll_delta` applies signed logical-row scroll
deltas to virtual-list viewport starts with the same allocation-free clamping
contract, leaving hit testing and platform input normalization to the host or
runtime adapter.
`virtual_list_scroll_delta_from_units` converts already-normalized scroll units
into bounded row deltas for wheel, touchpad, keyboard, or host-defined scroll
inputs.
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
Declarative views can use `SurfaceNode::scroll_area` and
`SurfaceNode::virtual_scroll_area` for the scroll viewport itself, then project
generic rows, cards, images, badges, selectables, or host-defined canvas cells as
children.
Dense card or tile grids can use `VirtualGridWindowRequest` and
`VirtualGridWindow` from the same module to resolve an allocation-free
row-major item window before projecting visible grid cells into
`SurfaceNode::grid` or a virtual scroll area.
Timeline and signal visualizations can use `SignalChromeState` for reusable
status/reference/channel chrome, `SignalToolFlags` and `SignalToolState` for
generic enabled/visible tool flags, `SignalRasterPreview` for retained raster
image payloads and loading state, `horizontal_progress_fill_rect` for resolving normalized
progress-track fill geometry, `horizontal_progress_activity_rect` for
indeterminate progress segments, `horizontal_progress_track_rect` for switching
between determinate and indeterminate progress tracks, `horizontal_meter_fill_rect` and
`horizontal_discrete_meter_fill_rect` for reusable meter geometry, and
`inline_indicator_layout` for compact text-relative status indicator clusters,
`TimelineViewport` for normalized viewport bounds,
`TimelineTransportState` for cursor/playhead/selection positions,
`TimelineEditPreview` and `TimelineEditPreviewParts` for editable range and
fade/curve handles,
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
primitives.

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
`SurfaceRuntime::borrowed_frame(...)` is the preferred immediate-render path for
custom host loops because it borrows the runtime's current layout instead of
cloning the resolved layout maps every frame. Hosts that render synchronously
and keep a frame scratch buffer can call `SurfaceRuntime::borrowed_frame_into(...)`
to reuse `SurfacePaintPlan` primitive storage as well. `SurfaceRuntime::frame(...)`
packages the same event-driven runtime state into an owned `SurfaceFrame` for
hosts that need to retain the frame after borrowing the runtime.
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

Standard widgets emit Vello-friendly paint primitives such as fills, strokes,
text, images, clips, and overlays. Specialized realtime visuals can instead
emit `PaintPrimitive::GpuSurface` through `GpuSurfaceWidget` or a custom
`Widget` implementation. GPU surfaces are still normal Radiant widgets: they
own stable identity, receive layout bounds, can route widget input, and paint
through the same `SurfacePaintPlan` as Vello-backed widgets.

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
backend-neutral shader identity and opaque uniform/storage bytes through the
normal widget, layout, input, and paint-plan path. Native backends that do not
yet implement a matching shader pipeline report the skipped surfaces through
`NativeGpuSurfaceDiagnostics::unsupported_custom_shader_surfaces`,
`unsupported_custom_shader_vertices`, `unsupported_custom_shader_uniform_bytes`,
and `unsupported_custom_shader_storage_bytes` instead of silently treating them
as built-in atlas or signal content.
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
`FillLinearGradient`, and `DrawImage` for retained renderer adapters that need
frame-oriented scene data rather than a full declarative `SurfacePaintPlan`.

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
`Rect::inset_horizontal` provides product-neutral horizontal text and control
inset geometry.
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
typed `SvgParseError` when hosts need parser diagnostics. The native Vello
backend appends retained SVG documents through `vello_svg` during scene
encoding.
`Rect::stroke_aligned_rect` provides stroke-grid snapping for retained border
geometry.
`Rect::top_right_square` provides anchored overlay geometry for controls that
compose primary and secondary glyphs.
`Rect::top_edge_strip`, `Rect::bottom_edge_strip`, `Rect::left_edge_strip`, and
`Rect::right_edge_strip` provide border-edge geometry for reusable retained
paint paths.
`Rect::union` provides shared bounding-box aggregation for retained rendering,
hit testing, and automation paths.
`StatusSegments::new(...)`, `StatusSegments::primary(...)`, and the
`with_left(...)` / `with_center(...)` / `with_right(...)` builders provide a
structured left/center/right status-bar model for application chrome.
`SurfaceRuntime::focus_widget`, `SurfaceRuntime::clear_focus`,
`SurfaceRuntime::focused_widget`, `SurfaceRuntime::traverse_focus`, and
`FocusTraversal` expose deterministic keyboard focus ownership and traversal.
`UiSurface::keyboard_focus_order_into(...)` writes the same deterministic order
into caller-owned storage for diagnostics or host integrations that inspect
focus order repeatedly without reallocating.
Pointer dispatch through `dispatch_input_at` can assign focus from hit testing;
keyboard dispatch through `dispatch_focused_input` routes input to the focused
widget by stable `WidgetId`.
Backend adapters that need redraw policy can route pointer motion through
`SurfaceRuntime::dispatch_pointer_move_with_outcome(...)`. Its
`PointerMoveOutcome` reports the target widget, hover changes, pointer capture,
scene-rebuild repaint requests, paint-only overlay requests, and exit requests
in one controller-owned result. Native and embedded renderers should use that
outcome when deciding between rebuilding the cached scene and presenting a
runtime overlay over the existing scene.
Application builders can register host-owned shortcut catalogs with
`.shortcuts(...)`. The runtime supplies pending chord state, normalized
`KeyPress`, and `FocusSurface`; returning `ShortcutResolution::action(message)`
dispatches a normal app message before focused-widget key routing, while
`ShortcutResolution::handled()` suppresses the fallback without coupling Radiant
to an application command model. `ShortcutLayer` maps normalized
`ShortcutGesture` values to host actions, supports modal layers that consume
unmatched keys, and offers `resolve_or_else(...)` for dynamic fallbacks such as
shifted navigation. This keeps modal shortcut shielding and simple global
accelerators declarative while still leaving command catalogs and focus policy
in the host application.

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

Each JSON line includes `type`, `scenario`, `iterations`, `total_us`, and
`avg_us`, so performance history can be collected without scraping prose.
List the available scenarios without running them with:

```powershell
cargo bench --bench perf_harness -- --list
```

It currently covers:

- `layout_deep_nesting`, `layout_wrap_1k`, `layout_virtualized_10k`,
  `layout_virtualized_fixed_10k`, `layout_virtualized_fixed_scroll_10k`, and
  `layout_mark_dirty_subtree_10k`
- `app_virtual_list_projection_10k`,
  `app_virtual_list_projection_generated_child_ids_10k`,
  `app_virtual_selectable_list_projection_10k`, and
  `app_virtual_list_window_projection_10k`
- `runtime_surface_large_tree`, `runtime_text_paint_plan_1k`,
  `runtime_horizontal_scroll_paint_1k`, `runtime_virtualized_list_wheel_10k`,
  `runtime_virtualized_list_hover_10k`,
  `runtime_virtualized_list_stable_hover_10k`,
  `runtime_virtualized_list_hover_paint_10k`,
  `runtime_pointer_overlay_paint_10k`,
  `runtime_virtualized_nested_scroll_hover_10k`,
  `runtime_refresh_large_tree`, `runtime_resize_large_tree`,
  `runtime_command_flattening_512`, `runtime_command_drain_1k`, and
  `runtime_nested_command_drain_1k`
- `resource_slot_stale_completions_1k`
- `text_line_cache_1k`
- `gpu_signal_summary`, `gpu_surface_projection`, and
  `gpu_custom_shader_projection`

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
apps can collect frame diagnostics without parsing logs. The profile separates
paint-plan primitive counts, Vello scene encode categories, retained-surface
bridge/cache/miss counts, custom-surface fallback counts, GPU-surface
render/cache counts, transient-overlay primitive counts, and timing for surface
refresh, paint-plan generation, Vello render-to-texture, composed-base refresh
or cache hits for transient overlays, transient-overlay paint callbacks,
GPU-surface composition, and presentation.
`NativeRunOptions::retained_surface_cache` accepts `RetainedSurfaceCachePolicy`
for apps that need to tune or disable retained custom-surface frame reuse during
profiling.
`NativeFrameDiagnostics::text` exposes native text layout-cache hits, misses,
and evictions, text atom-cache activity, and fallback/missing glyph counts so
hosts can detect repeated text measurement, cache churn, or font coverage gaps
without parsing renderer logs.

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
| State, commands, and background work | `todo_list`, `message_routing`, `background_loading`, `status_bar` |
| Layout, scrolling, and virtualization | `layout_rows_columns`, `grid_gallery`, `scroll`, `sizing`, `virtualized_list` |
| Styling, theming, and reusable widgets | `styling`, `theme_playground`, `widget_gallery`, `toolbar_icons` |
| Input, focus, menus, and editor interactions | `focus_controls`, `context_menu`, `tree_and_details`, `folder_browser` |
| Custom widgets and retained GPU surfaces | `custom_widget`, `gpu_surface`, `custom_shader_surface`, `gpu_surface_stack_overlay`, `waveform_view` |
| Advanced creative-tool surfaces | `node_editor`, `timeline_editor`, `plugin_panel`, `split_workspace` |
| Text, diagnostics, and performance inspection | `typography`, `layout_diagnostics`, `rendering_benchmark`, `host_surface_frame` |
| Window and host integration | `multi_window_manifest`, `popup_window`, `host_surface_frame` |

Run an example interactively with `cargo run --example <name>`. Showcase
examples use portable defaults, with optional inputs for real local data:

```powershell
cargo run --example folder_browser -- C:\path\to\root
$env:RADIANT_FOLDER_BROWSER_ROOT = "C:\path\to\root"
cargo run --example waveform_view -- C:\path\to\sample.wav
$env:RADIANT_WAVEFORM_PATH = "C:\path\to\sample.wav"
```

If no folder root is supplied, `folder_browser` uses a temp-directory demo
root. If no WAV path is supplied, `waveform_view` uses a generated synthetic
signal while exercising the same waveform summary and GPU-surface projection
path as real input. The waveform view keeps the dense signal body in a retained
`GpuSurfaceContent::SignalSummaryBands` surface and uses
`.animated_transient_overlay_at(...)` for the playback playhead, anchoring the
moving line through `SurfacePaintPlan::first_widget_rect`. That keeps playback
on the paint-only presentation path instead of queueing app frame messages,
reprojecting the declarative surface, or rebuilding the waveform scene for each
playhead tick.
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
backend-neutral `GpuShaderSurfaceDescriptor`. Native backends without a matching
shader pipeline report the surface through
`NativeGpuSurfaceDiagnostics::unsupported_custom_shader_surfaces` and the
related skipped vertex/uniform/storage counters rather than creating a separate
WGPU-facing application API.
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
`virtual_list_window(...)`. Use the windowed helper for large fixed-height
lists so projection stays bounded to a `VirtualListWindow`; reserve
`virtual_list(...)` for smaller lists where eagerly building every row remains
acceptable.
Run `cargo run --example inspector_panel` for a compact inspector/property
panel sandbox that uses `PropertyRow`, `property_panel(...)`, and
`selectable_property_panel(...)` on the same application-builder path as other
stateful examples.
Run `cargo run --example context_menu` for a generic menu/context-menu sandbox
that composes `MenuItem`, `menu(...)`, and `context_menu_overlay(...)` with
normal state callbacks.
Run `cargo run --example split_workspace` for an editor-style split workspace
that uses `SplitPaneSidebarState`, `SplitPaneSlot`, and generic Radiant views
without adding docking-specific runtime concepts.
Run `cargo run --example node_editor` for a node-editor-style workspace that
composes retained canvas metadata, connection markers, draggable card stacks,
selectables, and port rewiring through public application builders.
Run `cargo run --example timeline_editor` for a timeline-editor-style sandbox
that projects `TimelineSurfaceState`, `TimelineMotionState`, retained canvas
metadata, marker selection, and transport controls through normal app views.
Run `cargo run --example animation_showcase` for a frame-driven UI sandbox that
uses `.animation(...)` and `.on_frame(...)` through the stateful application
builder.
Run `cargo run --example gpu_surface_stack_overlay` for a retained GPU surface
with normal widget overlays plus a transient animated blob that repaints every
frame through `.animated_transient_overlay_at(...)` without refreshing the
declarative surface, rebuilding the cached Vello scene, or recompositing the
stable retained GPU surface on every overlay-only frame. The overlay caps its
paint-only cadence to 60 FPS and anchors to the cached GPU-surface rectangle
through `SurfacePaintPlan::first_widget_rect`.
Run `cargo run --example background_loading` for a background-work sandbox that
uses `ResourceSlot`, `ResourceCompletion`, and `UpdateContext::spawn_resource(...)`
to route worker resource results back into the normal state update path.
Run `cargo run --example typography` for a focused text sandbox that exercises
wrapping, truncation, fixed text heights, fill sizing, and explicit baselines
through the application-builder API.
Run `cargo run --example widget_gallery` for a reusable-widget gallery that
shows `badge(...)`, `selectable(...)`, and passive `card()` composition through
the prelude builders.
Run `cargo run --example custom_widget` for a custom widget authoring sandbox
that implements paint and input dispatch through the public widget trait.
Run `cargo run --example volume_slider` for a focused parameter-control sandbox
that uses the prelude `slider(...)` builder, horizontal value changes, and a
checkbox-backed mute state through direct state callbacks.
Run `cargo run --example sample_source_list` for a compact stateful list
sandbox that emulates a sample-source picker with selectable rows, stable row
IDs, and small `+` / `-` row actions.
Run `cargo run --example toolbar_icons` for a horizontal SVG-icon toolbar
sandbox that uses custom toggle buttons, state-driven active highlights, and
muted inactive vector icons.
Run `cargo run --example status_bar` for a bottom status-bar sandbox that shows
button actions, toggle state, animation updates, and background worker progress
flowing into a one-line log and retained-canvas progress strip.
Run `cargo run --example layout_rows_columns` for a compact row/column layout
sandbox with padding and fill sizing.
Run `cargo run --example grid_gallery` for a fixed-column gallery sandbox that
uses `grid_with_gaps(...)` with normal nested views and styling.
Run `cargo run --example tree_and_details` for tree-list and sortable details
list composition with drag-aware row controls.
Run `cargo run --example theme_playground` for a theme-token sandbox that
compares density scale, tone, prominence, and interactive state through normal
application views. It is intended to make theme policy visually inspectable, not
only to prove that token colors resolve.
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
Run `cargo run --example message_routing` for command-returning update flows,
runtime messages, and repaint requests.
Run `cargo run --example keys` for stable keys and reversed list identity.
Run `cargo run --example focus_controls` for an input/focus sandbox that uses
`UpdateContext::focus(...)` and app-level `.shortcuts(...)` to move keyboard
focus from normal app messages.
Run `cargo run --example plugin_panel` for a dense plugin-style control panel
that stays on generic Radiant layout, style, focus, and state-callback APIs;
host/plugin SDK integration remains outside Radiant.

## Quality Gate

Radiant's normal local quality lane includes Clippy across library, tests,
examples, and benches:

```powershell
cargo clippy --all-targets --all-features -- -D warnings
```

This command is a code-quality baseline, not a performance benchmark. Keep new
lint exceptions local and specific instead of adding broad crate-level Clippy
allows.

## Automation

`radiant::gui::automation` owns the serializable automation snapshot contract:
`AutomationNodeId`, `AutomationRole`, `AutomationBounds`,
`AutomationNodeSnapshot`, and `GuiAutomationSnapshot`. Backends and test tools
can consume this semantic tree without depending on a host application's state
types or reducer.

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
`StatusSegments` and `ContentViewChrome`. `radiant::gui::feedback` contains
compact feedback models such as `StatusLineLog` and `StatusLineEntry` for
bounded one-line status messages from buttons, background workers, animations,
and other app-owned systems. Host applications map product-specific copy into
these slots; Radiant defaults stay product-neutral.

`radiant::gui::panel` contains generic split-pane and sidebar models such as
`SplitPaneSlot`, `SplitPaneAssignedRow`, `SplitPaneTreePanel`, and
`SplitPaneSidebarState`, plus `anchored_panel_rect` for clamped popup/panel
placement. Host applications map product-specific navigation, workspace,
project, or asset concepts onto these reusable panel structures.

`radiant::gui::badge` contains compact label and pill primitives such as
`SelectablePill`, `PillEditorPanel`, `InlineBadgeMetrics`,
`inline_badge_rects_for_labels`, and `inline_badge_text_origin`. Repeated layout
or paint paths can use `inline_badge_labels_owned_into`,
`inline_badge_rects_for_labels_into`, and `inline_badge_rects_into` to reuse
caller-owned buffers. Hosts can use these to render dense badge clusters for
metadata, filters, status chips, or other product-specific labels without
embedding domain terms in Radiant.

`radiant::gui::form` contains reusable form and picker models such as
`DecimalTextInputPolicy`, `SummaryField`, `OptionItem`, `PairedPickerTarget`,
`PairedPickerValue`, `PairedStatusPanel`, and `PreferencePanelState`.
`PairedStatusPanel` models a two-sided status/picker surface with summary rows,
active picker identity, and option lists while leaving the meaning of those
options to the host. `PreferencePanelState` models generic settings-panel
visibility, a primary text value, fixed-size toggle state, and an auxiliary
label without owning product-specific preference names.

`radiant::gui::text_layout` contains retained text-line placement helpers such
as `TextLineInsets`, `centered_text_line`, `top_text_line`, and
`TextLineLayoutCache`. The plain placement helpers are deterministic and
side-effect free; renderer adapters that need retention can pass an owned cache
and font-family cache key to `centered_text_line_with_cache` or
`top_text_line_with_cache`. That keeps hot-path text geometry reuse explicit,
backend-owned, and free of hidden global synchronization while avoiding
host-domain text semantics.

`radiant::gui::visualization` contains generic visualization models such as
`TimelineViewport`, `TimelineTransportState`, `TimelineEditPreview`,
`TimelineFeedbackEvents`, `TimelinePresentationState`, `SignalRasterPreview`,
`TimelineSurfaceParts`, `TimelineSurfaceState`, `TimelineMotionState`, and
`normalized_milli_point_in_rect`. Hosts can map product-specific media or
spatial surfaces into these reusable visualization slots while keeping domain
workflow state outside Radiant.

## Invalidation And Lifecycle

Hosts project immutable surface snapshots. Radiant compares widget identity,
layout inputs, style tokens, and paint data to keep redraw work focused. Generic
invalidation primitives such as `InvalidationMask`, `RetainedSegmentMask`,
`RetainedSegmentRevisions`, `StableFingerprint`, repaint signals, and frame
feedback exist so backend runtimes can avoid unnecessary full-tree rebuilds and
full redraws while still falling back conservatively when a host cannot provide
fine-grained hints.

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
