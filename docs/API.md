# Radiant Core API

Radiant is a reusable declarative GUI library. Host applications own domain
state and side effects; Radiant owns view-tree identity, layout, input routing,
focus, style resolution, invalidation, and renderer-facing paint plans.

## Dependency Boundary

The dependency direction is host application to Radiant. Radiant default builds
must not depend on host crates, host modules, product assets, or product-domain
model names. In short: host -> Radiant, never Radiant -> host. Transitional
compatibility code is isolated behind the `legacy-shell` feature and is not part
of the default standalone API.

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

An application is host-owned state plus a projection function. Radiant does not
define the domain model. Hosts provide a `RuntimeBridge<Message>` or use
`declarative_runtime_bridge(state, project, reduce)` to project an immutable
`UiSurface<Message>` from state and reduce messages back into state.

## View, Element, And Widget

`UiSurface` is the root declarative view snapshot. `SurfaceNode` is the generic
element tree: container nodes hold `SurfaceChild` entries and widget nodes hold
`WidgetSpec` leaves. Widget primitives such as `ButtonWidget`, `TextWidget`,
`TextInputWidget`, `ToggleWidget`, `ScrollbarWidget`, `CanvasWidget`, and
`ListItemWidget` describe reusable UI behavior without host-domain semantics.

Widget identity is explicit through stable `WidgetId` values. Stable identity is
required for focus, input capture, message routing, and efficient updates.

## Message And Command

Radiant routes widget outputs into host-defined `Message` values through
`WidgetMessageMapper`. `SurfaceRuntime` dispatches input, emits mapped messages,
calls the host reducer, and requests a fresh surface snapshot. `Command<Message>`
is the generic runtime-visible follow-up value for host reducers that need to
queue messages, batch runtime-visible work, or request repaint.

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
`ContentListPanel<Row, Editor>` provides retained, product-neutral state for
searchable and filterable large-list panels while hosts own row and editor
payload semantics.
`ContentListActions` provides product-neutral action availability for
content-list toolbars and context menus.
Timeline and signal visualizations can use `SignalChromeState` for reusable
status/reference/channel chrome, `SignalToolState` for generic enabled/visible
tool flags, `SignalRasterPreview` for retained raster image payloads and
loading state, `TimelineViewport` for normalized viewport bounds,
`TimelineTransportState` for cursor/playhead/selection positions,
`TimelineEditPreview` for editable range and fade/curve handles, and
`TimelineMarkerPreview` for retained marker overlays.

## Style And Theme

`radiant::theme::ThemeTokens` and widget visual-token resolution provide
domain-neutral colors, spacing, borders, typography scale, and interaction
states. Product visual identity should be supplied by the host or translated
through generic tokens instead of baked into Radiant primitives.

## Renderer

Radiant's generic runtime produces a backend-neutral `SurfacePaintPlan` made of
`PaintPrimitive` values. Native Vello support is an adapter that consumes this
paint plan through `run_native_vello_runtime`. Renderers should consume paint
plans and report frame results without owning host state.

Native runtime entry points return `RuntimeRunReport<Artifacts>` when artifact
capture is requested. The report envelope is generic: Radiant owns the
success/error transport while each runtime path chooses its artifact payload.
This keeps compatibility diagnostics and generic runtime diagnostics on the same
mechanism without coupling the public runtime API to a host application model.

## Context

Runtime context is split deliberately:

- Host context lives in the host application state and reducer.
- Layout context is the viewport and resolved `LayoutOutput`.
- Style context is the active `ThemeTokens`.
- Runtime context is held by `SurfaceRuntime`, including the current surface,
  focus target, widget hit testing, and message dispatch.

## Event And Focus

Backend input is normalized into Radiant input primitives such as
`WidgetInput`, `PointerButton`, and `WidgetKey`. The runtime performs hit
testing, focus changes, pointer press/release routing, keyboard routing to the
focused widget, and message mapping. Focus behavior is declared by widget
contracts rather than by host-domain code.

## Automation

`radiant::gui::automation` owns the serializable automation snapshot contract:
`AutomationNodeId`, `AutomationRole`, `AutomationBounds`,
`AutomationNodeSnapshot`, and `GuiAutomationSnapshot`. Backends and test tools
can consume this semantic tree without depending on a host application's state
types or reducer.

`radiant::gui::snapshot` owns deterministic rendered-frame snapshot primitives:
`VisualSnapshot`, `SnapshotPrimitive`, `SnapshotTextRun`, `SnapshotRect`,
`SnapshotPoint`, `SnapshotColor`, and `SnapshotTextAlign`. These types are for
fixture generation, renderer verification, and visual regression tooling. Host
or compatibility adapters may build these snapshots from their own frame models,
but the serializable snapshot schema is generic Radiant API.

## Generic Panels And Forms

`radiant::gui::chrome` contains generic chrome/status models such as
`StatusSegments` and `ContentViewChrome`. Host applications map product-specific
copy into these slots; Radiant defaults stay product-neutral.

`radiant::gui::form` contains reusable form and picker models such as
`DecimalTextInputPolicy`, `SummaryField`, `OptionItem`, `PairedPickerTarget`,
`PairedPickerValue`, and `PairedStatusPanel`. `PairedStatusPanel` models a
two-sided status/picker surface with summary rows, active picker identity, and
option lists while leaving the meaning of those options to the host.

## Invalidation And Lifecycle

Hosts project immutable surface snapshots. Radiant compares widget identity,
layout inputs, style tokens, and paint data to keep redraw work focused. Generic
invalidation primitives such as `InvalidationMask`, repaint signals, and frame
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
