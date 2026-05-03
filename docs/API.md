# Radiant Core API

Radiant is a reusable declarative GUI library. Host applications own domain
state and side effects; Radiant owns view-tree identity, layout, input routing,
focus, style resolution, invalidation, and renderer-facing paint plans.

## Dependency Boundary

The dependency direction is host application to Radiant. Radiant default builds
must not depend on host crates, host modules, product assets, or product-domain
model names. In short: host -> Radiant, never Radiant -> host. Transitional
compatibility code is isolated behind the `legacy-shell` feature and is not part
of the default standalone API. The `compat::legacy_shell` contract is temporary
host-shaped migration glue with its final host-owned disposition recorded in
`domain_extraction_inventory.tsv`; generic Radiant code must not grow new
dependencies on it.

New host applications should use:

- `radiant::runtime` for application projection, message dispatch, and native
  runtime entry points.
- `radiant::widgets` for reusable leaf widgets and widget interaction
  contracts.
- `radiant::layout` for container measurement and placement.
- `radiant::theme` for shared style tokens.
- `radiant::gui` for lower-level geometry, input, invalidation, frame, and
  repaint primitives.

## App

An application is host-owned state plus a projection function and reducer.
Radiant does not define the domain model. The public `App<Message>` contract is
implemented by every `RuntimeBridge<Message>`: hosts can provide a custom bridge
or use `declarative_runtime_bridge(state, project, reduce)` to project an
immutable `UiSurface<Message>` from state and reduce messages back into state.
Hosts whose update flow returns runtime-visible follow-up work can use
`declarative_command_runtime_bridge(state, project, update)`; its update closure
returns `Command<Message>` while keeping side effects and domain state host-owned.

## View, Element, And Widget

`View<Message>` is the root declarative view snapshot and is a public alias for
`UiSurface<Message>`. `Element<Message>` is the generic element tree and is a
public alias for `SurfaceNode<Message>`: container nodes hold `SurfaceChild`
entries and widget nodes hold `WidgetSpec` leaves. Widget primitives such as
`ButtonWidget`, `BadgeWidget`, `TextWidget`, `TextInputWidget`, `ToggleWidget`,
`ScrollbarWidget`, `SelectableWidget`, `CardWidget`, `ImageWidget`,
`CanvasWidget`, and `ListItemWidget` describe reusable UI behavior without
host-domain semantics.

Common declarative composition should use `SurfaceNode::row`,
`SurfaceNode::column`, `SurfaceNode::grid`, `SurfaceChild::fill`, and
`SurfaceNode::static_widget` when a host only needs ordered row/column/grid
structure, fill slots, and display widgets that do not emit messages.
`SurfaceNode::text`, `SurfaceNode::button`,
`SurfaceNode::button_mapped`, `SurfaceNode::badge`, `SurfaceNode::badge_mapped`,
`SurfaceNode::text_input`,
`SurfaceNode::text_input_mapped`, `SurfaceNode::toggle`, and
`SurfaceNode::toggle_mapped`, `SurfaceNode::scrollbar`,
`SurfaceNode::scrollbar_mapped`, `SurfaceNode::list_item`, and
`SurfaceNode::list_item_action`, `SurfaceNode::list_item_mapped`,
`SurfaceNode::selectable`, `SurfaceNode::selectable_mapped`,
`SurfaceNode::card`, `SurfaceNode::image`, and `SurfaceNode::canvas` cover
common leaf widgets without requiring hosts to manually wrap `WidgetSpec`.
`SurfaceNode::stack` overlays children in slot order so hosts can compose a card
background with nested rows, columns, labels, and controls. Lower-level
`SurfaceNode::container` plus `ContainerPolicy` and `SlotParams` remains
available for custom layout policy.
`SurfaceNode::scroll_area` wraps one content child in a generic scroll viewport;
`SurfaceNode::virtual_scroll_area` adds a `VirtualizationPolicy` for large
linear lists without tying the framework API to any host content-list model.

Widget identity is explicit through stable `WidgetId` values. Stable identity is
required for focus, input capture, message routing, and efficient updates.

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
reducers that need to queue messages, batch runtime-visible work, or request
repaint.

Asynchronous side effects remain host-owned. A host reducer may start work,
enqueue IO, or signal background tasks, but Radiant's generic runtime only
observes `Command<Message>` values, repaint requests, projected surface
snapshots, and host-defined messages.

## Layout

`radiant::layout` provides slot-based measurement and placement. Containers use
`ContainerPolicy`, `ContainerKind`, `SlotParams`, and `LayoutNode` to describe
rows, columns, overlays, fixed sizing, fill behavior, spacing, padding, and
stable output rectangles. Layout is deterministic and independent from any
renderer backend.

Large item-indexed lists can use `VirtualListWindowRequest` and
`VirtualListWindow` from `radiant::gui::list` before projecting widgets. This
keeps host-side list projection bounded while `layout::VirtualizationPolicy`
continues to handle pixel-based scroll-container virtualization.
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
Declarative views can use `SurfaceNode::scroll_area` and
`SurfaceNode::virtual_scroll_area` for the scroll viewport itself, then project
generic rows, cards, images, badges, selectables, or host-defined canvas cells as
children.
Dense card or tile grids can use `VirtualGridWindowRequest` and
`VirtualGridWindow` from the same module to resolve an allocation-free
row-major item window before projecting visible grid cells into
`SurfaceNode::grid` or a virtual scroll area.
`ContentListPanel<Row, Editor>` provides retained, product-neutral state for
searchable and filterable large-list panels while hosts own row and editor
payload semantics.
`ContentListActions` provides product-neutral action availability for
content-list toolbars and context menus.
Timeline and signal visualizations can use `SignalChromeState` for reusable
status/reference/channel chrome, `SignalToolState` for generic enabled/visible
tool flags, `SignalRasterPreview` for retained raster image payloads and
loading state, `horizontal_progress_fill_rect` for resolving normalized
progress-track fill geometry, `TimelineViewport` for normalized viewport bounds,
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
`SurfaceRuntime::focus_widget`, `SurfaceRuntime::clear_focus`,
`SurfaceRuntime::focused_widget`, `SurfaceRuntime::traverse_focus`, and
`FocusTraversal` expose deterministic keyboard focus ownership and traversal.
Pointer dispatch through `dispatch_input_at` can assign focus from hit testing;
keyboard dispatch through `dispatch_focused_input` routes input to the focused
widget by stable `WidgetId`.

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
`SplitPaneSidebarState`. Host applications map product-specific navigation,
workspace, project, or asset concepts onto these reusable panel structures.

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
as `TextLineInsets`, `centered_text_line`, and `top_text_line`. These helpers
provide deterministic cached geometry for renderer adapters that need to place
single-line labels without owning host-domain text semantics.

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

The lifecycle is:

1. Host state is projected into `UiSurface<Message>`.
2. Radiant measures layout and builds a paint plan.
3. Backend input is routed to widgets.
4. Widget outputs are mapped to host messages.
5. The host reducer mutates host state and may request repaint.
6. Radiant refreshes the surface and rebuilds only the necessary runtime data.
