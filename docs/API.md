# Radiant Core API

Radiant is a reusable declarative GUI library. Host applications own domain
state and side effects; Radiant owns view-tree identity, layout, input routing,
focus, style resolution, invalidation, and renderer-facing paint plans.

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
`StateView`, `Command`, and the builder types needed by method chains. These
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
or request runtime exit.
Higher-level application helpers follow the same logical-coordinate sizing
model as view modifiers: fixed details-list columns use `f32` logical widths
through `DetailsColumn::fixed(...)`, matching `.size(...)`, `.fixed(...)`, and
other layout builders instead of introducing a separate integer sizing model.
`NativeRunOptions` keeps platform/window integration policy behind Radiant's
native runtime boundary. Common launch code can stay platform-neutral while
still configuring window title, logical size, minimum size, maximized state,
decorations, icon, target frame rate, and whether native file drag-and-drop is
requested on platforms that support it. Window launch and manifest builders
provide integer `.size(...)` convenience methods plus `.logical_size(...)` and
`.min_logical_size(...)` when hosts need fractional logical dimensions.
`NativeGpuOptions` and `NativeGpuBackend` keep WGPU backend selection explicit
without exposing normal app code to raw WGPU setup; the default remains WGPU's
environment-aware adapter selection, while diagnostics or platform work can
request a specific backend such as DX12, Vulkan, Metal, GL, or browser WebGPU.
`NativeTextOptions` lets hosts provide preferred font files before Radiant falls
back to environment or system fonts, keeping text/font policy explicit without
moving application asset loading into the renderer.
`WindowSpec` describes one host-managed window without opening the platform
runtime. `WindowManifest` stores ordered specs and rejects duplicate stable
keys, so multi-window or embedded hosts can validate a window set and attach a
separate bridge or view to each spec. `radiant::window(...).spec("main")`
converts the no-state launch builder into the same manifest shape.

Serious apps use the same builder API. `radiant::app(...)` supports
`.subscriptions(...)` for interval and worker-message sources, `.animation(...)`
with `.on_frame(...)` for frame-driven UI, `.on_scroll(...)` for observing
runtime-owned scroll offsets, `.on_startup(...)`, `.on_shutdown(...)`,
`.on_close_requested(...)`, `.run_with_artifacts()`, and retained-surface
painters registered through `.retained_painter(...)`.
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
move. Emit a `WidgetOutput` only when the host-owned model changes, such as
seek, create, move, resize, or delete.

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
runtime exit.
`ResourceSlot<T>`, `ResourceRequest`, `ResourceLoad<T>`, and
`ResourceLoadState` provide a small runtime-level state contract for host-owned
background resource work. Radiant does not own the filesystem or asset decoder,
but examples and apps can use the same key/state/result shape for loading
images, previews, manifests, fonts, or other resources through
`Command::perform(...)` or `UpdateContext::spawn(...)`. Use
`ResourceSlot::begin_load()` and `ResourceSlot::apply_for(...)` when repeated
loads for the same key can overlap; stale worker completions are ignored instead
of replacing the current result.

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
status/reference/channel chrome, `SignalToolState` for generic enabled/visible
tool flags, `SignalRasterPreview` for retained raster image payloads and
loading state, `horizontal_progress_fill_rect` for resolving normalized
progress-track fill geometry, `horizontal_progress_activity_rect` for
indeterminate progress segments, `horizontal_progress_track_rect` for switching
between determinate and indeterminate progress tracks, `horizontal_meter_fill_rect` and
`horizontal_discrete_meter_fill_rect` for reusable meter geometry, and
`inline_indicator_layout` for compact text-relative status indicator clusters,
`TimelineViewport` for normalized viewport bounds,
`TimelineTransportState` for cursor/playhead/selection positions,
`TimelineEditPreview` for editable range and fade/curve handles,
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
cloning the resolved layout maps every frame. `SurfaceRuntime::frame(...)`
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

`PaintGpuSurface` currently supports the built-in v1 content payloads
`GpuSurfaceContent::RgbaAtlas`, `SignalBands`, and `SignalSummaryBands`.
`GpuSurfaceContent::is_renderable()` and `signal_render_shape()` define the
shared payload contract used by widget projection and native renderers, so
invalid signal shapes or empty texture sources do not leak into backend work.
Runtime behavior is declared explicitly through `GpuSurfaceCapabilities`:
`fast_pointer_move` allows pointer-motion overlay updates without reprojecting
the app surface, `coalesce_vertical_wheel` allows vertical wheel deltas to be
batched until redraw, and `native_hover_cursor` lets the native runtime compose
a lightweight vertical cursor. These capabilities are part of the GPU-surface
contract, not side effects inferred from overlays. Future custom shader or
program support should extend this contract rather than adding backend-specific
runtime special cases.

Native runtime entry points return `RuntimeRunReport<Artifacts>` when artifact
capture is requested. The report envelope is generic: Radiant owns the
success/error transport while each runtime path chooses its artifact payload.
This keeps compatibility diagnostics and generic runtime diagnostics on the same
mechanism without coupling the public runtime API to a host application model.
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
`radiant::gui::svg::rasterize_svg_icon(...)` converts the supported filled SVG
subset into a square `ImageRgba` for retained icon controls and toolbar-style
widgets.
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
Pointer dispatch through `dispatch_input_at` can assign focus from hit testing;
keyboard dispatch through `dispatch_focused_input` routes input to the focused
widget by stable `WidgetId`.
Application builders can register host-owned shortcut catalogs with
`.shortcuts(...)`. The runtime supplies pending chord state, normalized
`KeyPress`, and `FocusSurface`; returning `ShortcutResolution::action(message)`
dispatches a normal app message before focused-widget key routing, while
`ShortcutResolution::handled()` suppresses the fallback without coupling Radiant
to an application command model.

## Performance Harness

Radiant includes a standalone performance harness for trend and profiling
evidence. Run it with:

```powershell
cargo bench --bench perf_harness
```

The harness prints parseable `radiant_perf` metric lines for layout, runtime
surface, and GPU-surface data preparation scenarios. It currently covers deep
layout trees, 1k wrap layout, 10k virtualized scroll layout, fixed-size 10k
virtualized scroll layout, eager and windowed 10k application-list projection,
large declarative surface layout plus paint-plan generation, GPU signal-summary
construction, and GPU-surface primitive projection. The harness performs sanity
assertions, but it does not enforce machine-dependent pass/fail timing thresholds; use the output for local
comparisons, profiling runs, and regression investigation.
Run `cargo run --example rendering_benchmark` for a checked public-API sandbox
that builds a large declarative surface, runs layout plus paint-plan generation,
and prints parseable primitive-count diagnostics.
Run `cargo run --example host_surface_frame` for a checked embedded-host
sandbox that drives `SurfaceRuntime` with backend-neutral events, requests a
`SurfaceFrame`, and reports `SurfacePaintStats` without opening the native
window runtime.

For interactive native runs, set `RADIANT_NATIVE_RENDER_PROFILE=1` to emit a
per-frame `radiant native render profile` tracing line. The profile separates
paint-plan primitive counts, Vello scene encode categories, retained-surface
bridge/cache counts, GPU-surface render/cache counts, and timing for surface
refresh, paint-plan generation, Vello render-to-texture, composition, and
presentation. This is a development diagnostic, not a stable public telemetry
schema.

## Examples And Sandboxes

Radiant examples are maintained API and sandbox contracts. They should compile
as checked example targets and participate in normal example validation:

```powershell
cargo test --examples
```

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
path as real input.
Run `cargo run --example gpu_surface` for a small retained-GPU-surface sandbox
that uses the prelude `gpu_surface(...)` application builder with generated
demo atlas data.
Run `cargo run --example multi_window_manifest` for a checked manifest sandbox
that uses `WindowManifest` to describe multiple windows and separate views
without expanding the native runtime event loop.
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
Run `cargo run --example background_loading` for a background-work sandbox that
uses `ResourceSlot`, `ResourceLoad`, and `UpdateContext::spawn(...)` to route
worker resource results back into the normal state update path.
Run `cargo run --example busy_progress` for a background-work progress sandbox
that starts slow tasks with `UpdateContext::spawn(...)`, uses animation frames
while work is active, and paints aggregate progress through a retained canvas.
Run `cargo run --example typography` for a focused text sandbox that exercises
wrapping, truncation, fixed text heights, fill sizing, and explicit baselines
through the application-builder API.
Run `cargo run --example widget_gallery` for a reusable-widget gallery that
shows `badge(...)`, `selectable(...)`, and passive `card()` composition through
the prelude builders.
Run `cargo run --example volume_slider` for a focused parameter-control sandbox
that uses the prelude `slider(...)` builder, horizontal value changes, and a
checkbox-backed mute state through direct state callbacks.
Run `cargo run --example sample_source_list` for a compact stateful list
sandbox that emulates a sample-source picker with selectable rows, stable row
IDs, and small `+` / `-` row actions.
Run `cargo run --example toolbar_icons` for a horizontal SVG-icon toolbar
sandbox that uses custom toggle buttons, state-driven active highlights, and
muted inactive icon rasterization.
Run `cargo run --example status_bar` for a bottom status-bar sandbox that shows
button actions, toggle state, and background worker progress flowing into a
thin persistent status strip.
Run `cargo run --example grid_gallery` for a fixed-column gallery sandbox that
uses `grid_with_gaps(...)` with normal nested views and styling.
Run `cargo run --example theme_playground` for a theme-token sandbox that
compares density scale, tone, prominence, and interactive state through normal
application views. It is intended to make theme policy visually inspectable, not
only to prove that token colors resolve.
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
`StatusSegments` and `ContentViewChrome`. Host applications map product-specific
copy into these slots; Radiant defaults stay product-neutral.

`radiant::gui::panel` contains generic split-pane and sidebar models such as
`SplitPaneSlot`, `SplitPaneAssignedRow`, `SplitPaneTreePanel`, and
`SplitPaneSidebarState`, plus `anchored_panel_rect` for clamped popup/panel
placement. Host applications map product-specific navigation, workspace,
project, or asset concepts onto these reusable panel structures.

`radiant::gui::badge` contains compact label and pill primitives such as
`SelectablePill`, `PillEditorPanel`, `InlineBadgeMetrics`,
`inline_badge_rects_for_labels`, and `inline_badge_text_origin`. Hosts can use
these to render dense badge clusters for metadata, filters, status chips, or
other product-specific labels without embedding domain terms in Radiant.

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
`TimelineSurfaceState`, `TimelineMotionState`, and
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
runtime preserve layout measurement and virtualization caches while still
accepting fresh immutable `UiSurface` snapshots from the host. Direct
`UiSurface::frame(...)` calls remain one-shot by design for embedded hosts that
want a single packaged frame without owning runtime state.

The lifecycle is:

1. Host state is projected into `UiSurface<Message>`.
2. Radiant measures layout and builds a paint plan.
3. Backend input is routed to widgets.
4. Widget outputs are mapped to host messages.
5. The host reducer mutates host state and may request repaint.
6. Radiant refreshes the surface and rebuilds only the necessary runtime data.
