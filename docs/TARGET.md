# Radiant Project Target: A High-Performance General-Purpose Rust GUI Library

## Vision

Radiant should become a clean, reusable, high-performance, general-purpose GUI library for building serious desktop applications in Rust.

Radiant is currently used for a sample-manager-style application, but it must not be designed around that use case alone. It should be broad enough to support many kinds of software, including:

- Sample managers
- DAWs
- Plugin interfaces
- Project managers
- Todo/planning tools
- Editors
- Creative tools
- Technical tools
- Control panels
- Realtime visual tools
- High-performance desktop applications

Radiant should feel like a real standalone GUI library, not an application-specific UI folder.

The target is a library that is pleasant to use, highly performant, architecturally coherent, cleanly modular, and powerful enough for complex creative and technical applications.

## Scope and Interpretation

This document is a project target and architecture direction for Radiant, not a single one-shot implementation task.

Do not attempt to implement every goal in one large rewrite. Use this document as the standard for reviewing, designing, refactoring, and extending Radiant over time.

When making changes, prefer incremental improvements that move Radiant closer to this target while keeping the library compiling, examples working, and the public API coherent.

The target architecture matters more than cosmetic cleanup. Avoid large rewrites unless they clearly reduce complexity, improve the public API, unlock important performance work, or remove architectural blockers.

## Core Product Goals

Radiant should provide:

1. A clean public API
2. One unified API surface
3. A declarative GUI model
4. Strong rendering and layout performance
5. Vello-based rendering for standard UI widgets and primitives
6. Direct WGPU/custom shader rendering for specialized GPU-heavy widgets where useful
7. Modern CPU/GPU utilization
8. Clean widget, layout, styling, event, input, focus, and state systems
9. Strong application independence
10. Small, focused modules and functions
11. Maintained examples and sandboxes for all major features
12. Tests, diagnostics, and benchmarks that support long-term development
13. Clear documentation for application developers and contributors

Radiant should scale from simple “hello world” interfaces to advanced applications such as DAWs, plugin UIs, node editors, timeline editors, waveform views, inspectors, large virtualized lists, and GPU-heavy realtime interfaces.

## Platform Target

Radiant should be developed Windows-first.

The initial primary development and testing target is:

- Windows

However, Radiant should be architected so it can support these platforms in the future:

- Linux
- macOS

Cross-platform support does not need to be fully implemented immediately, but the architecture should avoid unnecessary Windows-only assumptions in core library code.

Platform-specific code should be isolated behind clear boundaries. The public Radiant API should remain as platform-neutral as practical.

The goal is:

- Build and validate Radiant on Windows first.
- Avoid hardcoding Windows-specific assumptions into core systems.
- Keep windowing, surface creation, platform event handling, file/resource behavior, and backend integration modular.
- Make future Linux and macOS support an extension of the architecture, not a rewrite.

## Windowing and Platform Integration

Radiant should clearly separate GUI architecture from platform/windowing integration.

Radiant may use an existing Rust windowing/event-loop solution where appropriate, but the public API should not force normal application code to depend directly on low-level platform details.

The architecture should make it possible to support future Linux and macOS targets without rewriting core systems such as layout, widgets, styling, state, or rendering.

Platform-specific code should be kept out of generic widget, layout, styling, and state systems unless there is a clear reason.

## Rendering Stack Decision

Radiant currently uses Vello as its primary renderer for standard UI rendering.

Vello uses WGPU under the hood, so Radiant’s rendering foundation is still WGPU-based. However, Radiant should clearly distinguish between:

1. Vello-based rendering for standard UI widgets and vector-style UI primitives.
2. Direct WGPU rendering for specialized GPU-heavy widgets or surfaces where custom shader pipelines make more sense.

The current rendering target is:

- Use Vello for standard UI widgets where Vello is a good fit.
- Use WGPU directly for custom GPU surfaces, shader-driven widgets, waveform views, scopes, timelines, meters, visualizers, or other dense realtime rendering cases where Vello is not the best tool.
- Keep both paths integrated into one coherent Radiant rendering architecture.
- Keep normal application-facing code independent from low-level Vello or WGPU details unless explicitly using advanced rendering features.

Radiant should not currently replace Vello or build a full custom renderer from scratch.

A future custom renderer or replacement for Vello may be considered later, but that is out of scope for the current target. For now, the goal is to use Vello well, use WGPU directly where it clearly makes sense, and keep the architecture clean enough that future rendering changes remain possible.

## WGPU Backend Strategy

Radiant should use WGPU as the GPU foundation.

WGPU should be allowed to select the appropriate backend by default, unless an explicit backend/device configuration is needed for debugging, testing, platform work, or advanced control.

The architecture should:

- Use Vello as the primary standard UI renderer.
- Use WGPU directly for custom GPU rendering where appropriate.
- Rely on WGPU’s normal adapter/backend selection by default.
- Allow explicit backend/device configuration where useful.
- Keep WGPU-specific implementation details behind clean Radiant abstractions.
- Avoid leaking low-level WGPU details into normal application code.
- Allow advanced rendering features through the unified Radiant API.

Radiant should not build several competing rendering backends at this stage.

The target is not to make rendering backend abstraction the main project. The target is to build a strong Vello + WGPU GUI rendering architecture with clean boundaries.

## Non-Goals

Radiant should not become:

- A GUI layer tightly coupled to one application
- A sample-manager-specific UI framework
- A DAW-specific UI framework
- A plugin-only UI framework
- A VST SDK wrapper
- A collection of disconnected APIs
- A thin wrapper around unrelated systems without a coherent architecture
- A large monolithic codebase with god files and god objects
- A framework that requires application code to know too much about internals
- A system that only works well for simple apps but breaks down for advanced tools

At this stage, Radiant should not include VST SDK integration directly.

At this stage, Radiant should also not replace Vello or attempt to build a full custom renderer from scratch. Vello is the current primary renderer for standard UI widgets. Direct WGPU rendering should be used where it clearly fits better, not as an excuse to rewrite the whole rendering stack.

VST/plugin integration belongs to the application or plugin framework using Radiant. Radiant should provide the GUI/window/surface/rendering/event APIs that make plugin UI integration possible, but the plugin-domain layer should own VST-specific behavior.

Accessibility is also not a priority for the current stage. Full accessibility support is a future concern, not a current implementation target.

Radiant may support sample managers, DAWs, plugins, todo apps, editors, and other tools, but it should do so through general-purpose GUI primitives and extensible architecture.

## Design Principles

Radiant should be:

- General-purpose
- Declarative
- High-performance
- Vello-based for standard UI rendering
- WGPU-capable for custom GPU rendering
- GPU-friendly
- Modular internally
- Unified externally
- Strongly typed where useful
- Explicit in data flow
- Predictable in behavior
- Easy to extend
- Easy to test
- Easy to profile
- Application-independent
- Pleasant to use from Rust application code

Prefer simple, clear APIs that scale to advanced use cases.

Avoid clever abstractions unless they clearly improve usability, correctness, performance, or maintainability.

During the current API refinement phase, Radiant may break public API
compatibility to remove mixed patterns, weak names, or migration debt. Because
Radiant is vendor-owned by this workspace and consumed by Wavecrate, prefer one
clean API plus Wavecrate call-site updates over compatibility aliases, parallel
old/new builders, or half-finished migration layers.

## Primitive Boundary

Radiant is a generic UI system, not an application toolkit for one product
category. Its public API should provide the primitive widgets, layout tools,
interaction contracts, rendering hooks, and runtime services that host
applications use to build their own product-specific interfaces.

The default question for adding code to Radiant is:

> Is this a generic UI primitive or reusable GUI building block that several
> unrelated applications could reasonably use?

If the answer is no, the code belongs in the host application. This rule applies
even when moving the code into Radiant would make one current application call
site shorter.

Appropriate Radiant-owned primitives include:

- Basic controls such as buttons, icon buttons, checkboxes, toggles, sliders,
  dropdown triggers, text inputs, labels, badges, and status indicators.
- Layout and composition primitives such as rows, columns, stacks, grids,
  split panes, scroll areas, panels, cards, overlays, menus, popovers, and
  modals.
- Generic interaction primitives such as activation, secondary activation,
  focus traversal, keyboard shortcuts, pointer capture, drag, drop, resize
  handles, selection, hover, and disabled/read-only state.
- Generic large-data UI primitives such as virtualized lists, tables, trees,
  outline rows, property panels, inspectors, and details rows.
- Generic visualization and editor building blocks such as timelines,
  waveform-like value displays, meters, parameter controls, curves, grids,
  markers, and retained GPU surfaces when they are domain-neutral and driven by
  host-provided data.
- Backend-neutral paint, geometry, theme, image, text, invalidation, resource,
  window, and runtime coordination primitives.

Radiant should not own:

- Product workflows such as sample extraction, tagging, rating, library
  scanning, plugin preset management, DAW arrangement logic, todo workflows, or
  project planning behavior.
- Product domain models such as sample IDs, tag categories, track models,
  plugin parameters, task records, file-library entities, or application
  command catalogs.
- Product naming, copy, icons, colors, persistence keys, file formats, storage
  policies, recovery behavior, or telemetry/logging semantics.
- Composite widgets whose behavior only makes sense for one product. A host may
  build a `SampleBrowser`, `TagLibrary`, `PluginPresetPanel`, or
  `TodoFilterBar` from Radiant primitives, but those named product components
  should not become Radiant primitives.
- Side effects such as file I/O, audio playback, metadata writes, network
  calls, plugin host operations, database updates, or product-specific
  background jobs.

A specialized visual surface can belong in Radiant only when it is expressed as
a generic, host-data-driven primitive. For example, a reusable timeline ruler,
range selection layer, virtualized tree row, waveform-style scalar display, or
GPU surface host can be Radiant-owned. A sample-extraction timeline that knows
about Wavecrate source files, tags, ratings, extraction success, or audio
library persistence must remain Wavecrate-owned.

When a host repeats layout math, interaction routing, hit testing, overlay
placement, stable identity handling, virtual-list viewport logic, paint-plan
construction, or widget-state styling across multiple surfaces, evaluate whether
Radiant is missing a generic primitive. When a host repeats product policy,
domain vocabulary, persistence rules, or workflow decisions, keep the code in
the host and clean up the host abstraction instead.

## Unified Public API

Radiant should expose one coherent public API surface for building applications.

Applications should interact with Radiant through one unified system for:

- Creating and managing windows
- Running the application lifecycle
- Building UI declaratively
- Layout
- Styling and theming
- Input and event handling
- Focus management
- State updates
- Widget composition
- Animation
- Rendering
- Vello-backed UI rendering
- WGPU-backed custom rendering features
- Custom widgets
- Resource management
- Menus, panels, inspectors, editors, and other UI structures

Radiant should not be split into:

- A “simple API” and an “advanced API”
- Multiple competing UI paradigms
- Disconnected subsystems that feel like different frameworks
- APIs that require bypassing normal architecture for common advanced use cases

There may be internal modules, backend layers, advanced features, and low-level escape hatches, but they should all fit into one coherent public API and one consistent mental model.

Advanced use cases should grow naturally from the same patterns used for simple applications. A developer should not feel like they are switching frameworks when building DAWs, plugin interfaces, node editors, timeline editors, complex inspector panels, large virtualized lists, GPU-heavy custom visualizations, or realtime interfaces.

Low-level access is acceptable where necessary, but it must integrate cleanly into the Radiant architecture rather than becoming a separate parallel API ecosystem.

## Public API Boundary

Radiant should have a unified public API, but this does not mean the internals must be flat or monolithic.

Internally, Radiant may have separate modules for runtime, windowing, layout, widgets, rendering, Vello integration, WGPU integration, text, input, styling, diagnostics, examples, tests, and benchmarks.

Externally, application developers should experience Radiant as one coherent library with one mental model.

Internal modules should serve the public API. They should not become separate competing frameworks or disconnected ways of building UI.

Advanced functionality may exist, but it should still feel like part of Radiant, not a different API family.

## Application and Host Model

Radiant should support normal desktop applications and, where practical, embedded or hosted UI contexts.

This is important because Radiant may be used for:

- Standalone desktop applications
- Creative tools
- DAWs
- Plugin interfaces
- Tool panels
- Embedded editor views
- Multi-window applications
- High-performance realtime UI surfaces

The architecture should distinguish between:

- The application/runtime layer
- Window and surface management
- Platform event-loop constraints
- Render backend integration
- Application state
- UI description
- Widget/event/layout/render systems

Radiant should provide a clean public model for common app setup while allowing more specialized hosting scenarios without requiring a separate framework.

Radiant should support proper multi-window hosting as a first-class runtime capability. A multi-window application should not need to launch a separate independent app runtime or create one native event loop per auxiliary panel. The preferred model is one application/runtime host that can manage multiple OS windows from the platform event loop that owns windowing.

This should include:

- A main application window
- Additional top-level windows such as preferences, inspectors, tool palettes, floating panels, and document windows
- Owned or child-associated windows where the operating system supports that model
- Window lifetime rules that keep auxiliary windows tied to their owner when appropriate
- Explicit close, focus, show/hide, minimize, restore, and z-order behavior
- Platform-native ownership semantics such as Windows owned windows, where useful
- Clear fallback behavior on platforms with different window ownership models
- Public APIs that let application state open, update, and close secondary windows without depending on low-level HWND, NSWindow, Wayland, X11, or winit details in ordinary app code

For Windows specifically, Radiant should prefer owned top-level windows for floating app panels that must remain associated with the main application window. These windows should be separate OS windows, not widgets drawn inside the main surface, but they should minimize, close, and stay ordered with their owner according to normal Windows owned-window behavior.

Threading should be explicit. Radiant may support helper-thread window runtimes as a narrow compatibility or transitional path, but the target architecture should be proper multi-window hosting inside a single application runtime/event-loop model wherever the platform and backend allow it.

For plugin-style use cases, Radiant should not include VST SDK integration directly. Instead, the application or plugin framework should own VST-specific integration, host callbacks, plugin lifecycle behavior, and any audio-domain concerns.

Radiant should provide the GUI-side capabilities needed for that integration, such as:

- Creating or attaching to a render surface where appropriate
- Rendering widgets into a host-controlled UI context where possible
- Handling UI events passed in from an application or host integration layer
- Allowing application-owned state to drive the UI
- Avoiding assumptions that only work for standalone desktop windows

Radiant is a GUI library, not an audio engine or plugin framework. It should be suitable for building audio/plugin interfaces, but plugin-domain logic belongs outside Radiant.

## Non-Blocking Application Runtime Contract

Radiant applications should be structurally non-blocking by default. The
UI/event/render path owns input, focus, layout, repaint, presentation, and short
host state reducers. It must not be the place where host-owned business work is
performed.

The target app-facing model is:

- Views project host state into a declarative UI and emit host messages.
- Update handlers synchronously apply lightweight state changes and UI/runtime
  requests.
- Host-owned business work is scheduled only through Radiant's business runtime
  lanes, such as `context.business().interactive(...)`,
  `.background(...)`, or `.idle(...)`.
- Platform side effects that belong to the GUI/runtime boundary, such as file
  dialogs, reveal/open, clipboard, confirmation prompts, and native handoffs,
  are requested through typed Radiant platform services.
- Worker closures receive business context and return results through the
  normal message path; they do not mutate UI state directly.

Forbidden work on the normal update-handler path includes filesystem and
database access, decoding/loading, cache hydration, network or process work,
sleeps, blocking waits or joins, thread creation, long CPU transforms, and
helper calls that hide those operations. Rust cannot prove every possible
blocking call through the type system, so the final architecture should combine
API removal, capability-limited contexts, reusable static guardrails, runtime
slow-handler diagnostics, and CI enforcement.

Radiant may break public compatibility during this phase. Wavecrate is the
current consumer, so the desired final shape is more important than preserving
old app-facing command, task, or spawn escape hatches. Low-level runtime command
machinery can remain internal or advanced-only where custom hosts and tests
need it, but the normal app path should make the business runtime the only
practical way to run host business work off the UI path.

## Declarative GUI Model

Radiant should move toward a clean declarative GUI model.

The API should make it easy to describe:

- What UI should exist
- How UI elements are composed
- How state maps to UI
- How events request state changes
- How layout is described
- How styling is applied
- How reusable UI components are defined

Application code should not need to micromanage rendering, invalidation, dirty state, or low-level layout behavior unless explicitly using advanced capabilities.

Useful architectural concepts may include:

- Component/view functions
- Declarative widget trees
- Stable identity for dynamic elements
- Incremental updates or reconciliation
- Dirty tracking
- Memoization and caching
- Local widget state where needed
- Predictable widget/component lifecycle
- Clear separation between app state, UI description, layout, events, rendering, and backend details

Radiant can learn from Xilem, egui, iced, SwiftUI, React, and retained/immediate hybrid GUI systems, but it should not blindly copy any single framework.

## API Ergonomics

Radiant’s API should be designed from the perspective of an application developer.

The API should be:

- Small enough to learn
- Consistent
- Declarative
- Strongly typed where useful
- Composable
- Discoverable
- Easy to test
- Easy to extend
- Independent from internal implementation details

Common tasks should feel straightforward:

- Creating an application
- Opening a window
- Creating layout containers
- Adding widgets
- Binding or reading state
- Handling events
- Styling components
- Reusing UI fragments
- Building dynamic lists or panels
- Building editor-like interfaces
- Building timeline-like interfaces
- Building control-heavy interfaces
- Building audio/plugin-style interfaces
- Updating only what changed
- Composing complex interfaces from smaller pieces

Application code should not need to know too much about Radiant internals.

## Performance Goals

Radiant should be designed for high-performance application UIs.

It should support:

- Large widget trees
- Large lists and grids
- Frequent UI updates
- Smooth scrolling
- Animation
- Editor-like tools
- Audio/plugin interfaces
- Realtime visual interfaces
- Dense visual widgets such as waveforms, meters, graphs, timelines, and scopes

The architecture should avoid:

- Unnecessary allocations
- Excessive cloning
- Redundant layout recalculation
- Full-tree rebuilds when partial updates are possible
- Unnecessary render command regeneration
- CPU-heavy rendering paths where GPU acceleration would help
- Excessive dynamic dispatch in hot paths unless justified
- Cache-unfriendly data layouts
- Large monolithic update/render passes
- Unnecessary string allocation or formatting in hot paths
- Repeated expensive text/layout measurement
- Unnecessary locking or synchronization
- Per-frame work when nothing relevant changed

Important hot paths should have benchmarks, profiling notes, diagnostics, or stress-test examples where useful.

Performance should be treated as an architectural concern, not as an afterthought.

## Modern CPU/GPU Architecture

Radiant should be designed from the ground up to take advantage of modern CPU and GPU capabilities.

The architecture should support:

- Multi-threaded work scheduling
- Parallel layout, measurement, scene preparation, or resource processing where practical
- Clear separation between main-thread/window/event-loop requirements and work that can safely happen off-thread
- Async or background resource loading
- Efficient Vello scene preparation where applicable
- Direct WGPU rendering paths for specialized widgets where useful
- Compute shaders where they provide a real benefit
- SIMD-friendly data layouts and algorithms where useful
- Efficient batching
- Cache-friendly memory access
- Minimal synchronization overhead
- Safe Rust concurrency patterns

Radiant should not assume all UI work must happen on one thread unless required by platform, windowing, host, Vello, WGPU, or backend constraints.

Expensive work should be structured so it can be parallelized, cached, incrementally updated, or moved to the GPU where appropriate.

Areas to evaluate include:

- Text shaping, measurement, and caching
- Layout calculation
- Large list/grid virtualization
- Vello scene building
- Render command generation
- Geometry generation
- Image/resource processing
- Waveform, timeline, graph, meter, and scope rendering
- Animation updates
- Hit testing and spatial indexing
- GPU resource uploads
- Shader-based effects
- Compute-based preprocessing or rendering support

The goal is not complexity for its own sake. Multi-threading, SIMD, compute shaders, Vello optimization, and direct WGPU rendering should be used where they materially improve performance, scalability, latency, or responsiveness.

The public API should stay clean and unified even when Radiant internally uses advanced rendering, CPU, or GPU techniques.

## Rendering Architecture: Vello + WGPU

Radiant should use a hybrid rendering architecture built around Vello and WGPU.

Vello should be used for standard UI rendering where it fits well, such as:

- General UI primitives
- Panels
- Buttons
- Text-related UI surfaces where applicable
- Borders
- Backgrounds
- Shapes
- Clipping
- Layering
- Transforms
- Vector-style interface elements
- Normal widget rendering

Direct WGPU rendering should be available for cases where custom GPU pipelines are more appropriate, such as:

- Waveform views
- Timeline views
- Scopes
- Meters
- Graphs
- Spectral views
- Dense realtime visualizations
- Shader-driven effects
- Custom GPU-rendered widgets
- High-frequency animated surfaces
- Large visual datasets
- Compute-based preprocessing or rendering support

The goal is not to force every visual element through one rendering path. The goal is to use the right rendering path for the job while keeping the public Radiant API coherent.

The rendering architecture should clearly separate:

- UI description
- Layout
- Paint command generation
- Vello scene construction
- Custom WGPU surface/widget rendering
- GPU resource management
- Render scheduling
- Backend-specific details

Application-facing code should not need to know whether a normal widget is rendered through Vello or a custom WGPU path. That should be an internal implementation detail unless the application is intentionally creating a custom GPU-rendered widget.

Custom WGPU widgets should still integrate with Radiant’s normal systems:

- Layout
- Events
- Focus where relevant
- Styling where relevant
- State updates
- Invalidation
- Resource management
- Examples and diagnostics

Radiant should avoid creating two disconnected rendering worlds. Vello-rendered widgets and direct-WGPU widgets should feel like part of the same GUI library.

## Custom GPU Widget Model

Radiant should support custom GPU-rendered widgets without fragmenting the API.

A custom GPU-rendered widget should behave like a normal Radiant widget from the application’s perspective.

It should participate in:

- Layout
- State updates
- Event handling
- Hit testing where relevant
- Focus where relevant
- Styling where relevant
- Invalidation
- Resource lifetime management
- Diagnostics
- Examples and tests

The difference should be in the rendering implementation, not in the application-facing mental model.

A waveform view, for example, should be a Radiant widget. Internally it may use direct WGPU buffers, shaders, textures, compute passes, or custom render passes, but externally it should fit the same widget model as other Radiant UI elements.

Custom GPU rendering should be used for clear performance, visual, or architectural reasons, not as a default replacement for normal Vello-rendered UI.

## Layout System

Radiant should have a clear, flexible, predictable, and performant layout system.

It should make common layouts easy:

- Rows
- Columns
- Grids
- Stacks
- Panels
- Scroll areas
- Split panes
- Absolute overlays where needed
- Responsive sizing
- Fixed, flexible, and content-driven sizing
- Dock-like layouts
- Inspector/sidebar layouts
- Editor layouts
- Timeline/arrangement layouts
- Dense control layouts for plugin-style UIs

The layout system should support:

- Clear constraint behavior
- Efficient measurement passes
- Layout caching
- Dirty layout invalidation
- Predictable overflow behavior
- Scroll handling
- Alignment
- Padding, margin, and gap APIs
- Nested layout performance
- Virtualization for large lists or grids
- Stability under dynamic content
- Debug tooling for layout issues

Layout behavior should be easy to reason about and easy to inspect when something goes wrong.

## Styling and Theming

Radiant should make coherent application styling easy.

The styling system should support:

- Theme objects
- Design tokens
- Colors
- Spacing
- Radius
- Borders
- Shadows
- Typography
- State-based styles
- Hover, active, focused, and disabled variants
- Widget-specific overrides
- Application-level theming
- Scoped themes where useful
- Compact/dense UI modes

Styling should be composable and shared across widgets.

Each widget should not invent its own styling model.

## Input, Events, and Focus

Radiant should have a predictable input, event, and focus model.

The system should handle:

- Mouse input
- Keyboard input
- Focus
- Hover
- Active/pressed state
- Text input
- Shortcuts/hotkeys
- Dragging
- Scrolling
- Selection
- Pointer capture where needed
- Context menus
- Multi-select behavior

Event flow should be clear:

- Where events enter the system
- How events are routed
- How widgets receive events
- How events are consumed or propagated
- How state changes are requested
- How redraw/layout invalidation is triggered

Input handling should not be scattered across unrelated rendering or layout code.

## Widget System

Radiant should make widgets easy to create, compose, style, test, and update.

Widgets should have clear responsibilities and reusable behavior where appropriate.

The system should support both simple and advanced widgets:

- Buttons
- Toggles
- Sliders
- Knobs
- Text inputs
- Lists
- Trees
- Tables
- Panels
- Inspectors
- Meters
- Waveform views
- Timeline views
- Arrangement/editor views
- Property controls
- Custom GPU-rendered widgets

Advanced widgets should not require breaking the architecture or bypassing normal layout/event/render systems unless using an intentional low-level escape hatch integrated into the unified API.

Every public widget should have:

- A clear API
- A focused example
- Basic tests where practical
- Styling behavior that fits the shared styling system
- Event behavior that fits the shared input/event model
- Documentation or example usage

## State and Update Model

Radiant should have a clear state and update model.

The architecture should make clear:

- Where application state lives
- How UI reads state
- How UI events request state changes
- How changes trigger layout/render updates
- How partial invalidation works
- How widget identity is preserved
- How dynamic UI lists avoid unnecessary rebuilds
- How local widget state is handled
- How transient interaction state is handled
- How app-level commands are emitted or processed

Avoid unclear bidirectional state flow, hidden mutable state, or global mutable state that makes behavior difficult to reason about.

Prefer explicit, understandable data flow.

## Text and Fonts

Radiant should treat text as a first-class GUI concern.

The text system should consider:

- Font loading
- Text shaping
- Text measurement
- Text caching
- Selection
- Cursor behavior
- Text input
- Keyboard editing behavior
- High-DPI rendering
- Unicode correctness where practical

Text handling is often a performance-sensitive and correctness-sensitive part of GUI systems. It should be designed intentionally rather than treated as a small rendering detail.

Full internationalization can be a future concern, but the core text system should avoid obviously fragile assumptions where practical.

## Accessibility

Accessibility is not a current priority for Radiant.

Full accessibility support is a non-goal for the current phase.

Do not spend implementation effort on accessibility-specific systems unless they are directly needed for another core feature. Accessibility can be revisited later once the core library architecture is stronger.

However, avoid unnecessary design choices that would make future accessibility impossible if there is no meaningful cost to keeping the architecture flexible.

## Application Independence

Radiant must remain independent from any specific application domain.

It may currently be used by a sample-manager-style application, but it should provide generic GUI primitives that can support many application types.

Good generic primitives include:

- Panels
- Lists
- Buttons
- Waveform views
- Timeline or arrangement views
- Meters
- Knobs, sliders, toggles, and parameter controls
- Metadata display widgets
- Command/event systems
- Styling primitives
- Keyboard shortcut and focus systems
- Drag-and-drop systems
- Editor primitives
- Menu/context-menu primitives
- Inspector/property-panel primitives
- Virtualized list/grid primitives

Avoid:

- Application-specific models
- Application-specific naming
- Sample-manager-specific assumptions in core Radiant logic
- DAW-specific assumptions in core Radiant logic
- Plugin-specific assumptions in core Radiant logic
- Todo-manager-specific assumptions in core Radiant logic
- VST SDK integration inside Radiant
- Hardcoded workflows from one application
- Abstractions that only make sense for one application
- Rendering, layout, or event logic coupled to one product’s data model
- Artificial tests that only check for forbidden application names

Application independence should be enforced through architecture, module boundaries, generic API design, and clear library/application separation.

## Module and Code Organization

Radiant should be cleanly modular internally while presenting a unified public API externally.

Files and modules should be small, focused, and organized by responsibility.

Potential module areas include:

- Public API/facade
- Core app/runtime logic
- Window/surface integration
- Platform integration
- Vello rendering integration
- Custom WGPU rendering integration
- Widget definitions
- Layout system
- Styling/theme system
- Event handling
- Input handling
- Focus/navigation
- Text handling
- Geometry/types
- State/update/reconciliation logic
- Animation/timing
- Diagnostics
- Examples
- Tests
- Benchmarks

Avoid god files and god objects.

Each file should have a clear reason to exist. Each module should expose a clean surface and hide internal implementation details where possible.

Module boundaries should match real architectural responsibilities, not arbitrary file splitting.

## Code Quality Standards

Radiant code should be simple, focused, and maintainable.

Guidelines:

- Keep functions small and single-purpose.
- Keep structs focused on one responsibility.
- Keep traits minimal and meaningful.
- Split complex methods into named helpers.
- Separate large impl blocks where it improves clarity.
- Expose only intentional public API.
- Keep internal types internal.
- Make error handling explicit and understandable.
- Keep control flow readable.
- Document or encode invariants in types.
- Keep state mutation clear and predictable.
- Make side effects easy to identify.
- Keep hot-path code simple and efficient.
- Prefer clear composition over large inheritance-like trait systems.
- Avoid premature abstraction.
- Avoid cleverness where straightforward code is better.
- Prefer explicit data flow.
- Minimize global mutable state.
- Remove dead code.
- Remove unused experiments unless intentionally preserved and documented.
- Make every abstraction earn its place.
- Avoid large rewrites unless they clearly reduce complexity or unlock important architecture.

Code smells to avoid:

- God objects
- Long functions
- Deep nesting
- Repeated logic
- Ambiguous names
- Hidden side effects
- Unclear ownership
- Overly broad traits
- Tight coupling between unrelated modules
- Application-specific assumptions
- Temporary hacks that become architecture
- Internal details leaking into application code

## Error Handling and Diagnostics

Radiant should provide clear errors and useful diagnostics.

Diagnostics should help with:

- Invalid layout states
- Rendering failures
- Missing resources
- Backend initialization problems
- Invalid widget usage
- Broken invariants
- Unexpected input/event states
- Failed text/font/resource handling
- Vello rendering issues
- WGPU/backend errors
- Performance hotspots during development

Development-only diagnostics may include:

- Layout bounds visualization
- Repaint/invalidation visualization
- Widget tree inspection
- Event routing inspection
- Frame timing
- Render command counts
- Vello scene statistics where practical
- Allocation hotspots
- Layout pass counts
- GPU timing where practical
- Resource/cache inspection where practical

Debug assertions and tracing should improve development without hurting release performance.

## Tests

Radiant should have tests that validate real behavior and protect useful architectural guarantees.

Good test targets include:

- Layout calculations
- Widget behavior
- Event propagation
- Focus behavior
- State update behavior
- Render command generation
- Dirty invalidation
- Public API examples
- Regression cases for actual bugs
- Virtualized list behavior
- Styling/theme resolution
- Text measurement where practical
- Widget identity and dynamic list behavior
- Resource/cache behavior where practical
- Custom GPU widget integration where practical

Avoid tests that only lock in names, file layout, or incidental implementation details.

Tests should support refactoring, not prevent it.

## Benchmarks and Performance Validation

Radiant should include benchmarks or performance validation tools for important hot paths.

Benchmark or stress-test areas may include:

- Large widget trees
- Large virtualized lists/grids
- Layout recalculation
- Text measurement and rendering
- Vello scene building
- Render command generation
- GPU upload behavior
- Waveform/timeline rendering
- Custom WGPU widget rendering
- Animation-heavy interfaces
- High-frequency UI updates
- Multi-threaded resource or scene preparation

Performance examples should make it possible to see whether Radiant feels smooth under realistic load.

Performance work should be measured where possible, not guessed.

## Examples, Applications, and Sandboxes

Radiant should include a strong set of example applications and interactive sandboxes.

These examples are not only for documentation. They are also validation tools for architecture, usability, rendering behavior, interaction quality, and performance.

Examples should act as:

- Documentation
- Usage references
- API demonstrations
- Feature validation tools
- Performance testbeds
- Interaction/layout/rendering sandboxes
- Regression detection tools
- Visual QA tools

Every major Radiant system should have at least one focused example demonstrating intended usage and behavior.

Examples should cover:

- Hello-world applications
- Basic window/application setup
- Layout systems
- Styling/theming
- Widget composition
- State-driven UI
- Dynamic lists
- Virtualized lists/grids
- Input and focus handling
- Menus and context menus
- Drag-and-drop
- Animation systems
- Vello-rendered UI widgets
- WGPU/custom-rendered widgets
- Waveform/timeline-style views
- High-frequency rendering
- Realtime UI updates
- Large-scale UI stress tests
- Multi-threaded systems where applicable
- Async/background resource loading
- Text rendering and typography
- Docking/editor-style interfaces
- Inspector/property panels
- Plugin-style interfaces
- Custom widget creation
- Rendering diagnostics/debug tools
- Performance benchmarks or profiling views

Useful example applications may include:

- Hello world
- Counter app
- Todo app
- Layout playground
- Styling/theme playground
- Widget gallery
- Timeline editor demo
- Waveform viewer demo
- Inspector/property editor demo
- Plugin-style UI demo
- Node editor demo
- Virtualized list stress test
- Rendering benchmark demo
- Animation showcase
- Multi-window demo
- Multi-threaded rendering/resource demo
- Custom WGPU widget demo
- Vello/WGPU composition demo

Examples should be:

- Small and focused where possible
- Easy to understand
- Well-structured
- Representative of intended API usage
- Maintained alongside the core library
- Kept working as the architecture evolves
- Included in CI/build checks where practical

Avoid examples that become outdated, abandoned, or architecturally inconsistent.

Examples and sandboxes are part of the Radiant development workflow, not optional extras.

## Documentation Goals

Radiant documentation should clarify:

- What Radiant is
- What belongs in Radiant
- What belongs in an application using Radiant
- How to create UI declaratively
- How the unified API is meant to be used
- How application/window setup works
- How platform support is structured
- How Vello rendering is used for standard UI
- How WGPU rendering is used for custom GPU widgets
- How Vello and direct WGPU rendering fit into one architecture
- How layout works
- How events work
- How styling works
- How state updates work
- How rendering works at a high level
- How to create custom widgets
- How to create custom GPU-rendered widgets
- How to avoid common performance mistakes
- How to structure applications built with Radiant
- How examples map to supported features
- What is currently out of scope, including VST SDK integration, accessibility, and replacing Vello

Documentation should stay aligned with the examples and the actual public API.

## Feature Definition of Done

A Radiant feature is not complete just because the code compiles.

For each meaningful public feature, widget, layout primitive, rendering feature, or interaction system, completion should usually include:

- A clear public API
- Internal implementation with clean module boundaries
- Tests where practical
- At least one focused example or sandbox
- Documentation or example comments explaining intended usage
- Styling/theming integration where relevant
- Event/focus/input integration where relevant
- Performance consideration where relevant
- Diagnostics or benchmark coverage if performance-sensitive
- No unnecessary application-specific assumptions
- No unnecessary platform-specific assumptions in core code
- No VST/plugin SDK coupling inside Radiant
- No unnecessary leakage of Vello or WGPU details into normal application code

This keeps Radiant coherent as a library rather than becoming a pile of isolated features.

## Validation and CI Expectations

Radiant should be validated continuously as it evolves.

Where practical, CI or local validation should cover:

- `cargo fmt`
- `cargo clippy` where available and useful
- Unit tests
- Integration tests where practical
- Example builds
- Documentation builds where useful
- Benchmarks or performance examples for manual/profiling runs

Examples should not be treated as throwaway demos. They should compile and remain aligned with the intended public API.

Performance benchmarks do not need to run on every normal CI pass if they are expensive or machine-dependent, but they should exist for important hot paths and be easy to run intentionally.

## Development Approach

Radiant should be improved incrementally toward this target architecture.

Before broad changes, produce an implementation plan identifying:

- Current architectural issues
- API pain points
- Performance bottlenecks
- Large files/modules to split
- Code smells
- Application-specific leakage
- Platform-specific assumptions
- Rendering architecture issues
- Vello/WGPU boundary issues
- Missing abstractions
- Overcomplicated abstractions
- Suggested implementation order
- Areas that should not be changed yet
- Risks of large rewrites
- Tests or examples needed to protect the work

Prefer small, coherent commits that each improve one area.

After each meaningful change:

- Run formatting
- Run linting where available
- Run tests
- Run relevant examples where practical
- Add or update tests where useful
- Add or update examples where useful
- Verify examples still work
- Keep the codebase compiling
- Commit changes with a clear message

Do not turn this work into endless renaming. Renaming is only useful when it improves API clarity, architectural understanding, or developer experience.

## Milestone Strategy

Radiant should move toward the target through clear milestones.

A reasonable milestone order is:

1. Establish the unified public API direction.
2. Clarify module boundaries and split obvious god files.
3. Clarify the Vello rendering path for standard UI.
4. Clarify the direct WGPU rendering path for custom GPU widgets.
5. Improve the declarative UI model.
6. Improve layout, invalidation, and widget composition.
7. Build and maintain core examples and sandboxes.
8. Add diagnostics, profiling tools, and performance validation.
9. Improve advanced rendering, multi-threading, caching, and GPU-backed features where they clearly help.
10. Extend platform support beyond Windows when the core architecture is ready.

Each milestone should leave the codebase in a better, working state.

## Suggested Implementation Order

Use the actual codebase review to determine the final order, but prefer an approach like this:

1. Review architecture, public API, module layout, examples, and performance-sensitive paths.
2. Identify the clearest architectural seams.
3. Establish or clarify the unified public API facade.
4. Clarify the Vello rendering boundary, direct WGPU rendering boundary, and platform integration boundary.
5. Split the largest files into focused modules without changing behavior.
6. Clean up obvious dead code and duplicated logic.
7. Improve public API ergonomics where the current design is clearly awkward.
8. Improve declarative UI structure and component composition.
9. Improve layout and invalidation behavior.
10. Improve Vello scene construction and standard UI rendering paths.
11. Improve custom WGPU rendering paths for specialized widgets.
12. Improve modern CPU/GPU utilization where it clearly helps.
13. Improve widget internals and shared widget primitives.
14. Improve styling/theme APIs.
15. Improve event, input, and focus handling.
16. Add or improve examples that demonstrate the intended API.
17. Add useful tests and benchmarks for behavior and performance-sensitive paths.
18. Improve documentation around public API, examples, architecture, platform support, Vello rendering, custom WGPU rendering, and performance.
19. Do a final cleanup pass for code smells, module boundaries, docs, and public API consistency.

Avoid combining unrelated changes in one commit.

## Deferred Decisions

The following decisions do not need to be finalized immediately, but should remain visible:

- Exact windowing/event-loop strategy.
- Exact text shaping/rendering stack.
- Exact Linux and macOS support timeline.
- Exact plugin-host integration adapter design.
- Whether any optional accessibility foundation should be added later.
- Which performance benchmarks should become formal release gates.
- Whether Vello should ever be replaced or supplemented by a custom full renderer in the future.

Do not block current work on these decisions unless a change would make a future decision much harder.

## Review Checklist

When evaluating Radiant, ask:

- Is this part of a general GUI library, or is it application-specific?
- Does the public API feel unified?
- Does this feature fit the declarative model?
- Does application code need to know too much about internals?
- Is this normal UI rendering that should go through Vello?
- Is this a specialized visual widget that would benefit from direct WGPU/shader rendering?
- Are Vello and direct WGPU rendering integrated into one coherent Radiant rendering model?
- Does this leak Vello or WGPU details into normal application code unnecessarily?
- Does this preserve the option to change or replace rendering internals later without redesigning the public API?
- Is platform-specific code isolated?
- Does this assume Windows unnecessarily?
- Would this design make future Linux/macOS support difficult?
- Does this accidentally couple Radiant to VST/plugin SDK concepts?
- Is the module boundary clear?
- Is this function or struct too large?
- Is this abstraction earning its place?
- Is there unnecessary allocation, cloning, locking, or per-frame work?
- Can this work be cached, parallelized, incrementally updated, or moved to the GPU?
- Does this feature have an example?
- Does this feature have tests where practical?
- Does this feature have documentation or clear example usage?
- Does this design scale to DAW/plugin/editor-style applications?
- Does this preserve Radiant as a standalone reusable GUI library?

## Completion Criteria

Radiant is moving toward the target when it has:

- A cleaner public API
- One unified API surface instead of fragmented simple/advanced APIs
- A more declarative usage model
- Strong independence from any single application domain
- Vello-based rendering for standard UI widgets
- Direct WGPU/custom shader rendering for specialized GPU-heavy widgets where useful
- Clean integration between Vello-rendered UI and direct-WGPU custom surfaces
- No unnecessary leakage of Vello or WGPU internals into normal application code
- Rendering architecture that can evolve later without requiring a public API rewrite
- Windows-first support without unnecessary Windows-only assumptions in core code
- Architecture that can extend to Linux and macOS later
- No direct VST SDK integration inside Radiant
- A plugin-friendly GUI architecture that can be integrated by application/plugin frameworks
- Clean internal module structure
- Small, focused files
- Small, focused functions
- Clear structs and traits
- Reduced code smells
- Strong rendering and layout performance
- Strong support for modern CPU/GPU performance techniques
- Multi-threading support where useful
- SIMD-friendly internals where useful
- GPU acceleration and compute-shader paths where useful
- Clean widget, layout, style, event, input, focus, and state systems
- Text/font handling designed as a first-class concern
- Maintained examples and sandboxes covering major systems
- Tests that validate important behavior without locking in incidental implementation details
- Benchmarks or profiling tools for important hot paths
- Clear documentation
- A clear distinction between library code, examples, and application-specific code

The target is for Radiant to become a real standalone Rust GUI library that can cleanly support sample managers, DAWs, plugin interfaces, todo/planning tools, editors, control panels, and other high-performance desktop applications.
