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
`list_row`, `empty`, `toggle`, `text_input`, `dropdown_trigger`, `custom_widget`, `IntoView`, `View`,
`StateView`, `Command`, `EmbeddedFont`, `StatusSegments`, `StatusLineLog`,
`StatusLineEntry`, `ContentViewChrome`, common custom-widget authoring contracts such as
`Widget`, `WidgetCommon`, `WidgetSizing`, `WidgetInput`, `WidgetOutput`,
`PointerButton`, `FocusBehavior`, `ActivationInputPolicy`,
`handle_activation_input`, and backend-neutral paint primitives such as
`PaintPrimitive`, `PaintClipStart`, `PaintClipEnd`, `PaintFillRect`,
`PaintFillRectBatch`,
`PaintFillPath`, `PaintPathCommand`, `PaintTransform`, and `PaintTextRun`. It
also includes the geometry, layout, image, color, and theme
types needed in widget method signatures, including `Rect`, `Point`, `Vector2`,
`LayoutOutput`, `ImageRgba`, `ImageRgbaError`, `Rgba8`, and `ThemeTokens`, plus
app-facing asset helpers such as `SvgIcon` and `SvgIconTintCache`, common
feedback geometry helpers such as `horizontal_progress_fill_rect`, paint geometry helpers such as
`horizontal_line_rect` and `vertical_line_rect`, plus the builder types needed by method chains. These
builders lower into the same `UiSurface`, `SurfaceNode`, `SurfaceChild`,
`WidgetSizing`, and `RuntimeBridge` contracts available through the explicit
runtime modules.

Custom widgets can use `Rgba8::new`, `Rgba8::with_alpha`,
`Rgba8::blend_toward`, and `Rgba8::blend_opaque_toward` for common color
manipulation. Use `Rect::from_size(width, height)` for origin-based widget,
viewport, and test bounds, or `Rect::from_xy_size(x, y, width, height)` for
positioned widget bounds, instead of repeating `Point` plus `Vector2`
construction. Dense visualizations can use `ColorRamp` and `ColorRampStop` for
normalized heatmap and intensity palettes without local interpolation helpers.
Custom canvas, image, GPU surface, and overlay widgets can use
`WidgetCommon::without_default_chrome()` when they still need Radiant's sizing,
focus, hit testing, and style contracts but draw their own focus and state
affordances. Use `WidgetCommon::is_hovered()`, `is_pressed()`, `is_focused()`,
`is_selected()`, `is_active()`, `is_disabled()`, and `is_read_only()`, or the
matching `WidgetState` helpers, when tests, custom widgets, or automation need
to query shared interaction state without reading the raw state fields. Use
`InteractiveRowWidget::paints_interaction_fill()` when custom dense-row
painters need hover/pressed fills to follow Radiant's hover suppression and
active-drag policy. Use
`Widget::paint_plan(...)` or
`paint_plan_with_defaults(...)` when focused custom-widget tests or previews
need the same `SurfacePaintPlan` query helpers available from full view
frames. Dynamic custom widgets and row input layers can use
`stable_widget_id(...)` to derive deterministic widget IDs from host-owned
scopes and durable text app keys instead of duplicating local hashing helpers.
Use `stable_widget_id_u64(...)` when dynamic rows or controls are keyed by
durable numeric app IDs or enum indexes and projection should avoid allocating
temporary strings. `interactive_row_underlay(content)` can use
`.stable_input_id(scope, key)` or `.stable_u64_input_id(scope, key)` to bind
those stable IDs directly to the backing interactive row.
Custom matrix or heatmap widgets can use `DenseGridLayout` and `DenseGridCell`
for reusable row/column cell projection and hit testing.
For paint-plan emission,
`WidgetPaint`, `push_fill_rect`, `push_fill_rect_batch`, `push_stroke_rect`,
`push_stroke_rect_batch`, `push_fill_polygon`, `push_stroke_polyline`,
`push_text`, `PaintTextMetrics`, and `push_text_run_with_metrics` provide the
reusable primitive construction path used by complex examples and custom
widgets. Dense custom widgets can use `push_visible_fill_rect` when derived or
clipped geometry should only enter the paint plan if it has finite positive area.
Use `WidgetPaint::new(...)` when several primitives are emitted for the same
custom widget and local code would otherwise thread the same primitive buffer and
widget id through every helper call. Timeline, waveform, progress, and
scrubber-style custom widgets can use `push_horizontal_value_range_fill`,
`push_horizontal_value_range_edge_fills`, and
`push_horizontal_value_cursor_fill`, or the matching `WidgetPaint` methods, to
append guarded normalized range, range edge, and cursor fills without repeating
local geometry-to-paint boilerplate.
Editor-style widgets that draw sampled curves such as EQ responses,
automation curves, fade curves, and analysis overlays can use
`SampledCurveStrokeParts`, `sampled_curve_points`, and
`push_sampled_curve_stroke` to keep finite-point filtering, bounds clamping,
point-buffer allocation, and stroke emission on Radiant's generic paint path
while the host owns the curve math.
Tests, automation, and embedded hosts that inspect paint plans can use
`SurfacePaintPlan::text_runs()`, `text_labels()`, `text_label_strings()`,
`first_text_run(...)`, `contains_text(...)`, `first_text_run_after_x(...)`,
`contains_text_after_x(...)`, `first_text_rect(...)`, `first_text_color(...)`, `text_inputs()`,
`first_text_input()`, `contains_text_input()`,
`paint_primitives()`, `contains_paint_primitives()`, `clip_starts()`,
`rects()`, `contains_rect_matching(...)`, `paint_rects()`,
`contains_paint_rect_matching(...)`, `fill_rects()`, `stroke_rects()`, `fill_polygons()`,
`stroke_polylines()`, `svgs()`, and `gpu_surfaces()`. Widget-specific query
helpers such as `fill_rects_for_widget(...)`,
`visible_fill_rects_for_widget(...)`,
`contains_visible_fill_rect_for_widget(...)`,
`fill_polygons_for_widget(...)`, `visible_fill_polygons_for_widget(...)`,
`contains_visible_fill_polygon_for_widget(...)`, `svgs_for_widget(...)`, and
`first_svg_rect_for_widget(...)` cover common automation assertions without
app-local primitive filtering. Transient overlays can use `first_widget_rect(...)`
or `first_widget_rect_by_priority(...)` to anchor frame-time paint to a cached
paint plan. Use
`PaintPrimitive::text_run()`, `text_input()`, `clip_start()`, `fill_rect()`,
`stroke_rect()`, `fill_polygon()`, `stroke_polyline()`, `svg()`, and
`gpu_surface()`, to query common paint primitives without app-local exhaustive
primitive matches.

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
Use `empty()` for optional branches that must return a view without
contributing visible layout size; use `spacer()` when the view should reserve a
non-painting fixed or flexible gap. Use `text_line(label, height)` for fixed-height
single-line labels that should fill their parent width and truncate rather than
wrap.
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
`Command<Message>` values directly. Interactive row and badge builders can use
`InteractiveRowActions` when they only need common activation, secondary-click,
drag, drop, or hover-drop routing without hand-written enum filtering. Use
`InteractiveRowBuilder::tracked_drag_source(...)` when host-owned row drag
state should configure the common draggable, drag-active, drag-source, and
pointer-motion policy together. Use
`InteractiveRowBuilder::tracked_drag_source_with_motion(...)` when the active
source is retained from host state and should keep emitting pointer movement
after projection. Use
`InteractiveRowUnderlayBuilder::tracked_drop_target(...)` when arbitrary
visible row content should keep its own paint tree while the transparent
interactive-row underlay owns standard tracked drop-target behavior.
Reducers that need the full app runtime can
use `.update_with(...)` and an `UpdateContext<Message>` to emit messages,
request repaint, move focus, start background work, schedule delayed messages,
or request runtime exit. `PlatformResponse` exposes helpers such as
`path()`, `into_path()`, `into_path_or_canceled()`, `is_canceled()`,
`is_completed()`, `into_completed()`, `confirmation()`, and
`into_confirmation()`, while the `PlatformResultExt` prelude trait provides the
same common decoders directly on platform-service callback results so reducers
can propagate platform errors and reject wrong response shapes without local
adapter code. Use `LatestTask` with
`UpdateContext::spawn_latest(...)`
for one-resource background loads where a newer selection should invalidate an
older completion. The resulting message receives a `TaskCompletion<Output>`;
call `LatestTask::finish(completion.ticket)` before applying the output so stale
work is rejected consistently without host-specific task-id plumbing. Use
`UpdateContext::spawn_cancellable_latest_with_priority(...)` when replace-latest
work also needs cooperative cancellation and a scheduling priority. Use
`UpdateContext::after_latest(...)` for debounced one-resource work when a
selection, search query, or inspector target should only start after it remains
current for a short delay; the delayed message carries the same ticket type and
should be accepted through the same `LatestTask` methods. Use
`UpdateContext::spawn_with_priority(...)` or
`spawn_cancellable_with_priority(...)` when background work should carry a
best-effort `TaskPriority` hint, such as idle-priority indexing or interactive
preview preparation. Use
`KeyedLatestTasks` with `UpdateContext::spawn_latest_for(...)` when the same
replace-latest behavior is needed independently for many keys, such as row
previews, folder scans, or document-local workers.
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
`CompactDetailsAnchoredCellParts` with
`compact_details_anchored_cell_from_parts(...)` when a compact cell needs a
fixed-size anchored child such as a badge, status marker, or compact action
without rebuilding the anchored-layer and cell-sizing composition locally.
Custom details-list headers can use
  `compact_details_header_row(...)`, `compact_resizable_details_header_cell(...)`,
  and `details_sort_label(...)` to share Radiant's compact header chrome,
  sortable click-or-drag behavior, resize handles, and sort marker copy while
  still composing app-specific menus or column policies. Use
  `compact_resizable_details_header_cell_with_ids(...)` with
  `CompactDetailsHeaderCellIds` when dynamic header cells need stable explicit
  widget ids for retained focus, drag, or resize state.
Resizable and reorderable details headers can keep interaction state in
`DetailsColumnResizeDrag` and `DetailsColumnReorderDrag`, using
`update_details_column_resize_drag(...)`,
`update_details_column_reorder_drag(...)`,
`details_column_drag_content_left(...)`, `details_column_reorder_index(...)`,
`details_column_drag_feedback(...)`, and `reorder_details_columns_by_id(...)`
for stable framework-owned column geometry and drag-lifecycle behavior.
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
`InteractiveRowWidget::handle_input_mapped(...)` and
`synchronize_from_previous_embedded(...)` when a custom row widget embeds an
interactive row for generic input behavior but exposes host-specific messages
and custom paint outside the trait shape. Use `InteractiveRowWidget::id()`, `common()`, and
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
`secondary_position()`, `drag_message()`, `hover_drop_position()`, and
`is_drop()` when custom row widgets need to map Radiant row interactions into
host-specific row messages without repeating exhaustive event-shape matches.
`InteractiveRowActions` is a widget-layer router; use
`InteractiveRowActions::route(...)` when custom row wrappers need the same
activation, modifier-aware activation, secondary-click, drag, drop, and
hover-drop routing table that `interactive_row().actions(...)` and
`interactive_row_underlay(...).actions(...)` use. Use the keyed variants
(`activate_key(...)`, `activate_or_double_key(...)`,
`activate_with_modifiers_key(...)`, `double_activate_key(...)`,
`secondary_key(...)`, `drag_key(...)`, and `drop_target_key(...)`) when row
interactions should route through the same host-owned item key without
duplicating capture closures at each row, chip, or tree item. Use
`activate_or_double(...)` or `activate_or_double_key(...)` when primary release
and double-click should route to the same host action. Use
`activate_or_double_with_modifiers(...)` when primary release should preserve
modifier state but double-click still maps to the same action with default
modifiers. Use `activate_secondary_key(...)` when a row or chip should route
primary activation and secondary context-menu activation through the same
host-owned key. Use
`activate_or_double_with_modifiers_secondary_drag_key(...)` for selectable
dense rows where primary release preserves modifiers and activation,
double-click, secondary context-menu activation, and drag lifecycle events all
belong to one durable row key. Use
`activate_or_double_secondary_drag_drop_target_key(...)` for tree, outline,
layer, folder, or lane rows where activation, secondary context-menu
activation, drag lifecycle, committed drop, and hover-drop updates all share one
durable row key.
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
native external-drag payload can call `UpdateContext::end_drag_session()` instead
of ending those runtime surfaces separately. Use
`UpdateContext::begin_drag_session(...)` or `Command::begin_drag_session(...)`
when one gesture may have an in-window preview, a native external-drag payload,
both, or neither. Use `UpdateContext::begin_drag_with_external(...)` or
`Command::begin_drag_with_external(...)` when both requests are already known to
exist and should be started together.
Dense custom row painters can use `push_dense_row_chrome(...)` with
`DenseRowChromeParts`, `DenseRowMarkerStyle`, and `DenseRowOutlineStyle` when
one row needs standard fill, leading/trailing markers, and optional outline
composition from one app-neutral paint descriptor. Use `push_dense_row_fill`,
`push_dense_row_label`, `push_dense_row_vertical_marker`, and
`push_dense_row_inset_stroke` when a row needs individual state-prioritized
fills, centered labels, edge markers, or outlines from Radiant's generic
dense-row geometry helpers without repeating paint-plan guard code. Use
`DenseRowLabelParts` when custom dense rows need row-height-aware label sizing,
text insets, alignment, and wrapping without constructing `PaintTextRun`
manually. Use `DenseRowMarkerParts::leading(width)` and `trailing(width)` for
common selection, status, and activity edge markers instead of repeating raw
marker geometry fields.
Tree and outline rows that need continuous descendant guide lines can use
`TreeGuideRow`, `TreeGuideStyle`, `tree_guide_segments(...)`,
`tree_guide_overlay(...)`, `tree_guide_indent(...)`, and
`virtual_tree_list_window(...)`. Applications should map their domain rows into
depth plus `starts_descendant_group` metadata while Radiant owns segment
projection, paint clipping for materialized virtual-list windows, passive
indent sizing, and the standard fixed-row virtual tree body composition.
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
previously active target.
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
composition in application code.
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
`NormalizedRange::with_edge_fraction(...)`,
`NormalizedRange::shifted_by_fraction(...)`, `NormalizedRangeDrag`,
`NormalizedRangeEdge`, `normalized_fraction_to_milli(...)`,
`normalized_fraction_to_micros(...)`, and `normalized_fraction_to_nanos(...)`
convert floating point interaction ratios into the stable normalized units used
by timeline, canvas, and retained visualization APIs while keeping common
range creation, edge dragging, and clamped movement behavior out of host code.
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
while app code owns the resulting domain messages.
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
and drops, so applications can block interaction during modal/loading states or
clear stale drag-hover state without app-local invisible hit-test widgets.
Convenience constructors such as `.pointer_move_only(...)` and
`.pointer_drop_only(...)` cover common transparent overlay policies.
Popover and menu stacks can use `dismiss_layer(message)` as a transparent
full-surface activation layer behind foreground content, avoiding app-local
empty input-only buttons for outside-click dismissal.
When the caller has separate base content and foreground overlay content,
`dismissible_overlay(base, overlay, message)` composes the standard
base/dismiss/foreground stack so apps do not repeat the ordering required for
outside-click dismissal.
Base content with optional overlays can use `stack_layers(...)` to avoid
application-local `if len > 1 { stack(...) } else { base }` branching. It
returns `empty()` for zero layers, returns the only layer unchanged for one
layer, and builds a normal `stack(...)` for multiple layers.
Dropdown menus rendered as stack-level overlays can use
`dropdown_menu_overlay_below_trigger(...)` when the menu is anchored below
Radiant's standard dropdown trigger, avoiding app-local calls to
`dropdown_height(...)` just to recover the trigger height.
Composite controls can use `input_overlay(content, input)` when visible content
and a transparent input surface should share bounds without repeating a local
two-child stack. Use `input_underlay(content, input)` when the input surface
should stay below visible content so it can paint hover, selection, drag, or
drop-target feedback behind custom row contents.
Passive visual feedback layers can use `FeedbackOverlayWidget` for background
tints, determinate progress fills, and edge-band accents without app-local
paint-only custom widgets.
Status surfaces and background-job indicators can use `ProgressBarWidget` for
theme-backed determinate or indeterminate horizontal progress, with optional
pointer activation when the bar should open details. Use
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
`window.icon`, `frame.target_fps`, and whether native file drag-and-drop is
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
`RuntimeAnimationActivity` and `RuntimeAnimationDemand`, distinguishing
frame-message animation from paint-only presentation work and optionally
carrying a per-activity target FPS.
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
Applications that need custom capability flags, runtime pointer-line policies,
or lightweight backend-composited overlays can use
`GpuSurfaceConfiguredParts` with `gpu_surface_configured_from_parts(...)`.
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
repaint, schedule delayed messages, run background work, move focus, override
native DPI through `Command::set_dpi_scale(...)`, request a native-window
logical viewport size through `Command::set_window_logical_size(...)`, or
request runtime exit. Hosts that inspect only the immediate messages in a command can use
`Command::into_messages_into(...)` to reuse caller-owned storage, while
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
and `WidgetOutput::typed_cloned::<T>()`, `typed_copied::<T>()`,
`custom_cloned::<T>()`, and `custom_copied::<T>()` provide owned payload
extraction for tests, automation, and custom-widget adapters without repeating
manual downcast chains. `WidgetMessageMapper::dynamic(...)` is available when a
host needs manual downcast or filtering behavior. Adding a widget should not
require adding a central output enum variant.

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
Editable list/tree projections use named construction parts such as
`EditableTreeRowParts` and `EditableTreeDraftInputParts` so selection,
hierarchy, draft text, validation, and focus policy remain explicit at call
sites instead of being encoded as positional boolean lists.
Application-builder code that owns a resolved logical window can use
`virtual_list_window(...)` for fixed-height rows; it preserves full scroll
extent with spacer rows while only projecting the materialized item range.
Use `virtual_tree_list_window(...)` for fixed-height tree or outline rows when
the same materialized range should include a standard tree-guide overlay.
Use `virtual_list_window_body(...)` when the materialized range needs to be
composed as one body, such as row groups, table overlays, guide overlays, or
other decoration spanning several fixed-height rows, while Radiant still owns
the full-scroll spacer geometry.
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
messages. Use `CompactOptionListFloatingAboveParts` and
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
Application-builder toolbars can use `ToolbarParts`, `ToolbarAlignment`,
`toolbar(...)`, and `toolbar_from_parts(...)` when the app owns the ordered
controls but Radiant should own the compact row chrome, spacing, padding,
height, start/end alignment, and two-ended strips with
`ToolbarParts::trailing(...)` or `ToolbarParts::trailing_controls(...)`.
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
progress-track fill geometry, `horizontal_progress_activity_rect` for
indeterminate progress segments, `horizontal_progress_track_rect` for switching
between determinate and indeterminate progress tracks, `horizontal_meter_fill_rect` and
`horizontal_discrete_meter_fill_rect` for reusable meter geometry,
`horizontal_value_range_rect`, `horizontal_value_range_edge_rects`, and
`horizontal_wrapped_value_range_rects` for normalized horizontal track ticks,
top/bottom range rails, and wrapped phase/activity segments,
`horizontal_value_cursor_rect` for pixel-stable full-height cursors on
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
`CanvasSelectionTrailingControlHitTestParts`, `CanvasSelectionTrailingControlPaintParts`,
`CanvasSelectionTrailingControlStyle`,
`canvas_selection_body_handle_rect`,
`canvas_selection_trailing_control_rect`, `canvas_selection_edge_handles`,
`canvas_selection_edge_visual_rect`, and `horizontal_resize_edge_bracket_rects`
for generic retained-canvas layering, selection, control, resize handle geometry,
selection affordance hit testing, and guarded selection-affordance paint emission,
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
host-supplied colors. Use `TimelineEditCurveStrokeParts`,
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
`gpu_surface_configured_from_parts(...)`, or `gpu_surface_input(...)`, or
through `GpuSurfaceWidget` in lower-level host code. GPU surfaces are still
normal Radiant widgets: they own stable identity, receive layout bounds, can
route widget input, and paint through the same `SurfacePaintPlan` as
Vello-backed widgets.

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
instead of reparsing formatted SVG strings. `svg_with_current_color(...)`
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
`StatusSegments::new(...)`, `StatusSegments::primary(...)`, and the
`with_left(...)` / `with_center(...)` / `with_right(...)` builders provide a
structured left/center/right status-bar model for application chrome.
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
Application builders can register host-owned shortcut catalogs with
`.shortcuts(...)`. The runtime supplies pending chord state, normalized
`KeyPress`, and `FocusSurface`; returning `ShortcutResolution::action(message)`
dispatches a normal app message before focused-widget key routing, while
`ShortcutResolution::handled()` suppresses the fallback without coupling Radiant
to an application command model. `ShortcutLayer` maps normalized
`ShortcutGesture` values to host actions, supports modal layers that consume
unmatched keys, and offers `resolve_or_else(...)` for dynamic fallbacks such as
shifted navigation. Use `ShortcutLayer::bind_all(...)` when several equivalent
gestures should dispatch the same host action. Use
`ShortcutLayer::modal_escape(...)` for modal surfaces whose Escape key dismisses
the surface while other keys should remain shielded from lower-priority
shortcuts. This keeps modal shortcut shielding and simple global accelerators
declarative while still leaving command catalogs and focus policy in the host
application.

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

Each JSON line includes `type`, `scenario`, `category`, `iterations`,
`total_us`, and `avg_us`, so performance history can be collected without
scraping prose or losing which target area the scenario validates.
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

Metric lines and list output both include each scenario's target-area category
and default iteration count, so reviewers can quickly spot whether a run covered
layout, runtime, text, resource, or GPU-facing work.
Run a whole target-area category without spelling every scenario with
`--category`:

```powershell
cargo bench --bench perf_harness -- --category runtime_virtualized --jsonl
```

It currently covers:

- Layout scenarios: `layout_deep_nesting`, `layout_wrap_1k`, `layout_virtualized_10k`,
  `layout_virtualized_fixed_10k`, `layout_virtualized_fixed_scroll_10k`, and
  `layout_mark_dirty_subtree_10k`
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
  `runtime_command_flattening_512`, `runtime_command_drain_1k`, and
  `runtime_nested_command_drain_1k`
- Resource lifecycle scenarios: `resource_slot_stale_completions_1k`
- Text scenarios: `text_line_cache_1k`, `text_word_selection_1k`, and
  `text_word_deletion_1k`
- GPU data and surface scenarios: `gpu_signal_summary`, `gpu_surface_projection`, and
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
| State, commands, and background work | `todo_list`, `message_routing`, `background_loading`, `status_bar`, `sample_source_list`, `animation_showcase` |
| Layout, scrolling, and virtualization | `layout_rows_columns`, `grid_gallery`, `scroll`, `sizing`, `list`, `virtualized_list` |
| Styling, theming, and reusable widgets | `styling`, `theme_playground`, `widget_gallery`, `toolbar_icons`, `svg`, `form`, `volume_slider`, `passive_widgets` |
| Input, focus, menus, and editor interactions | `focus_controls`, `keys`, `context_menu`, `floating_overlay`, `tree_and_details`, `folder_browser`, `paint_helpers` |
| Custom widgets and retained GPU surfaces | `custom_widget`, `gpu_surface`, `custom_shader_surface`, `gpu_surface_stack_overlay`, `waveform_view` |
| Advanced creative-tool surfaces | `node_editor`, `timeline_editor`, `inspector_panel`, `plugin_panel`, `eq_editor`, `spectrogram`, `mixer_console`, `piano_roll`, `modulation_matrix`, `arrangement_shell`, `split_workspace` |
| Text, diagnostics, and performance inspection | `typography`, `layout_diagnostics`, `rendering_benchmark`, `host_surface_frame` |
| Window and host integration | `multi_window_manifest`, `popup_window`, `host_surface_frame`, `dpi_scaling` |

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
`virtual_list_window(...)`. Use the windowed helper for large fixed-height
lists so projection stays bounded to a `VirtualListWindow`; use
`virtual_tree_list_window(...)` when a fixed-height tree or outline should
compose materialized rows with standard guide overlays; use
`virtual_list_window_body(...)` when the materialized window needs a shared
overlay or grouped row body outside the standard tree-guide case; reserve
`virtual_list(...)` for smaller lists where eagerly building every row remains
acceptable.
Run `cargo run --example inspector_panel` for a compact inspector/property
panel sandbox that uses `PropertyRow`, `property_rows(...)`,
`property_panel(...)`, and `selectable_property_panel(...)` on the same
application-builder path as other stateful examples. `property_rows(...)`
builds read-only property rows without adding a titled panel shell, so host
applications can embed standard inspector rows inside app-owned panel sections.
`property_panel(...)` is read-only and can be used with any host message type;
use `selectable_property_panel(...)` when property rows should emit state
callbacks. Compact titled panels with optional header actions can use
`PanelSectionParts`, `panel_section(...)`, `panel_section_from_parts(...)`, and
`closeable_panel_section_from_parts(...)` instead of rebuilding title rows,
close buttons, padding, spacing, and neutral panel chrome in application code.
Use `PanelSectionGeometry` when app-owned resize constraints or fixed-content
panels need the same panel padding, title-height, and spacing calculations
without constructing view parts.
Use `PanelSectionParts::trailing_resize_handle(...)` when a resizable titled
panel should use Radiant's standard compact drag handle while the host reducer
keeps owning durable size, constraints, and resize messages.
Compact control panels can use `LabeledControlParts`,
`labeled_control(...)`, `labeled_control_from_parts(...)`, and
`labeled_control_control_offset(...)` for label-over-control groups and overlay
anchors without repeating label text styling and stacked spacing.
Use `FormRowParts`, `form_row(...)`, or `form_row_from_parts(...)` when a
panel needs compact horizontal label/control rows with Radiant-owned label
width, row padding, spacing, hover behavior, and optional row styling.
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
dismiss layer while preserving the base content underneath. Fixed-size titled
popovers and inspector panels can use `PanelSectionLayerParts`,
`panel_section_layer_from_parts(...)`, or
`closeable_panel_section_layer_from_parts(...)` to keep panel sizing and
full-surface anchored placement in one generic contract.
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
These helpers avoid app-local `message_menu_height(...)` sizing and hard-coded
context-menu width constants.
Run `cargo run --example context_menu` for a generic menu/context-menu sandbox
that composes `MenuItem`, `menu(...)`, and `context_menu_overlay(...)` with
normal state callbacks.
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
should keep emitting pointer movement after projection.
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
muted inactive vector icons. Compact action strips can use `ToolbarParts`,
`ToolbarAlignment`, `toolbar(...)`, and `toolbar_from_parts(...)` when host code
owns the controls and Radiant should own alignment, row height, spacing,
padding, and optional trailing controls.
Run `cargo run --example svg` for a focused SVG icon sandbox that parses
inline vector assets through `SvgIcon::from_svg(...)` and paints them through
the standard `icon_button(...)` builder. For common compact controls, use
`close_button()` and `disclosure_button(expanded)` so apps do not repeat literal
text labels or parse their own standard close/disclosure icons. Icon-button
builders support both message-style `.message(...)` routing and direct
state-callback `.on_click(...)` routing. Use `icon_button(...).passive()`,
`close_button().passive()`, or `disclosure_button(expanded).passive()` when a
standard icon should paint as decorative chrome while another parent surface
owns interaction routing. Button reducers can use
`ButtonMessage::is_activate()`, `secondary_position()`, and `drag_message()` to
route primary activation, context-menu clicks, or drag lifecycle events without
repeating the raw button enum shape. Button-backed drags emit `Cancelled` when
focus loss aborts an active drag before release.
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
input widget id for dispatch and tests. Use `.stable_input_id(...)` or
`.stable_u64_input_id(...)` on dynamic underlay rows instead of creating
app-local row input-id helper functions.
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
Run `cargo run --example message_routing` for command-returning update flows,
runtime messages, and repaint requests.
Run `cargo run --example keys` for stable keys and reversed list identity.
Run `cargo run --example focus_controls` for an input/focus sandbox that uses
`UpdateContext::focus(...)` and app-level `.shortcuts(...)` to move keyboard
focus from normal app messages.
Run `cargo run --example plugin_panel` for a dense plugin-style control panel
that stays on generic Radiant layout, style, focus, and state-callback APIs;
host/plugin SDK integration remains outside Radiant.
Run `cargo run --example eq_editor` for a graphical plugin-style EQ editor
surface that paints a visual response curve, analyzer-style overlay, editable
band handles, and parameter-routing messages without modeling DSP or audio
processing.
Run `cargo run --example spectrogram` for a DAW-style realtime spectrogram
surface that scrolls deterministic synthetic spectrum data through frame-driven
messages, heatmap painting, hover readout, and transport controls without
modeling DSP or audio processing.
Run `cargo run --example mixer_console` for a dense 32-channel DAW-style mixer
panel with deterministic synthetic meter levels, stereo decibel meters, faders,
send controls, group tinting, Shift/Ctrl multi-selection, grouped fader
adjustments, drag-and-drop strip reordering with insertion-line previews,
mute/solo/arm buttons, and paint-only hover overlays without
modeling DSP or audio processing.
Run `cargo run --example piano_roll` for a piano-roll editor sandbox with a
keyboard lane, beat grid, synthetic notes, drag-painted note creation,
move/resize previews, overlap cutting, horizontal and vertical zoom/pan, an
Ableton-style piano key lane, keyboard delete, Select/Paint tool switching, a
4096-note stress mode, marquee multi-selection, a dense editable velocity lane
linked to selected notes, and paint-only hover, drag, and playhead overlays
without modeling MIDI, DSP, or audio processing.
Run `cargo run --example modulation_matrix` for a dense modulation-routing
matrix with source and destination labels, bipolar amount editing, clear/delete
behavior, synthetic activity markers, and paint-only hover overlays without
modeling synth, DSP, or audio processing.
Run `cargo run --example arrangement_shell` for a DAW-style workspace shell with
transport controls, a browser pane, arrangement overview, inspector, compact
mixer/status strip, synthetic clips and meters, and paint-only arrangement hover
and playhead overlays without modeling audio, DSP, or plugin behavior.

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
`FlowTrailingItemParts` and `pack_flow_rows_with_trailing_item` when a trailing
input/control should stay on the current row only if enough editing room
remains. Use `push_flow_row_group` when several flow items, such as a prefix
token plus its editor, should wrap atomically instead of splitting across rows.
Use `pack_flow_rows_with_trailing_group` when callers need the common form of
packing existing items and appending one such atomic trailing group.
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
messages. Its `affordance_at_point(...)` helper can resolve optional body,
edge, and trailing-control hit targets while host applications keep their own
command mapping and domain-specific priority. Use
`CanvasSelectionAffordanceStyle` when the same selection should expose a reusable
set of optional body, edge, and trailing-control affordances from one grouped
style instead of rebuilding low-level hit-test parts in app code. Use
`CanvasSelectionBodyHandleStyle`, `CanvasSelectionEdgeVisualStyle`, and
`CanvasSelectionTrailingControlStyle` when hit testing and painting the same
canvas affordance should share one reusable dimension policy without duplicating
constants across input and paint code. Use `TimelineEditPreview` with
`TimelineEditHandleGeometry` and `TimelineEditRegionGeometry` when timeline
editors need standard handle hit rectangles and leading/trailing region paint
rectangles without duplicating viewport projection math. Use `TimelineEditRamp`
and `TimelineEditPreview::from_normalized_ramps(...)` when a host already has a
normalized selected range plus leading/trailing ramp lengths, outer extensions,
and optional curve values. Use
`TimelineEditPreview::push_standard_region_fills(...)` and
`push_standard_handle_fills(...)` when Radiant should own standard edit-preview
paint emission while the host owns colors and domain commands. Use
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
