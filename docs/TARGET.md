# Radiant Project Target: A High-Performance General-Purpose Rust GUI Library

`docs/DESIGN_DIRECTION.md` is the normative, detailed target-state architecture
contract for Radiant's node model, application API, scheduling, and rendering
boundaries. This document remains the broader product boundary and incremental
delivery guide: it explains what Radiant is trying to become and what belongs
in the library, but it does not assert that every target-state API or runtime
behavior is shipped today. For current API names and compatibility status, see
`docs/API.md`; for current module ownership, see `docs/ARCHITECTURE.md`.

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

Radiant's target support scope is macOS, Windows, and Linux through a native
Wayland session. X11 sessions and a direct X11 backend are explicit
non-goals. The public API and core architecture remain platform-neutral while
native host adapters own windowing, input, text, accessibility, surface, and
presentation details.

Implementation and acceptance may be phased, but the target is not complete
until all three in-scope platforms satisfy the feature Definition of Done.
The authoritative modern-system matrix is:

- macOS: the current supported macOS on the M5 Pro development host;
- Linux: Ubuntu 26.04 LTS Desktop with its default GNOME Wayland session;
- Windows: Windows 11 25H2 as the broad-deployment baseline.

Exact versions, runner images, hardware, and validation results belong in
review and CI evidence rather than in the public API contract. Current
Linux/Windows repository CI evidence is limited to the portable, build,
compile, and check jobs actually present; it does not include the target
integration or headless Wayland/native-host smoke lanes. Those lanes must
eventually be added through GitHub Actions where runners permit. Until then,
Linux/Windows CI establishes no host, IME, accessibility, presentation,
latency, GPU, or performance acceptance.

Platform-specific code should be isolated behind clear boundaries. The public
and core Radiant APIs should remain as platform-neutral as practical.

The goal is:

- Keep core GUI, layout, input, text, scheduling, and rendering contracts
  platform-neutral.
- Isolate Wayland, Windows, and macOS host integration behind explicit
  adapters.
- Validate the native macOS path on the M5 Pro; eventually validate
  Linux/Windows paths through the required GitHub Actions lanes.
- Make platform support an extension of the architecture, not a rewrite.

## Windowing and Platform Integration

Radiant should clearly separate GUI architecture from platform/windowing integration.

Radiant may use an existing Rust windowing/event-loop solution where appropriate, but the public API should not force normal application code to depend directly on low-level platform details.

The architecture should make it possible to support additional targets without
rewriting core systems such as layout, widgets, styling, state, or rendering.

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

Accessibility is a target requirement. Radiant owns one backend-neutral
semantic and automation model; macOS, Wayland, and Windows adapters expose that
model through their native accessibility systems. The current implementation
may still be incomplete, but it must not introduce a second platform-specific
UI model. Native macOS acceptance is hardware-backed. Current Linux and
Windows CI does not establish accessibility acceptance; the required future
native-host lanes are target smoke evidence, not hardware-backed acceptance.

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
  dialogs, reveal/open, clipboard text/file-list reads and writes,
  confirmation prompts, and native handoffs, are requested through typed
  Radiant platform services.
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

Internally, runtime diagnostics keep public snapshot and policy models separate
from synchronized lifecycle recording. Surface frame context keeps borrowed
frame models, paint projection, and tooltip shaping in focused owners. Business
update requests keep lane/resource selection, command dispatch, cooperative
cancellation, and latest-stream closure separate while preserving one stable
`UiUpdateContext::business()` API.

The qualified public owner boundary includes
`CancellableBusinessRequest::run_for_owner_with_receipt(owner, work, map)` for
token-cancellable ordinary owner one-shots, and
`BusinessRequest::stream_for_owner_with_receipt(owner, work, map_event,
map_final)` for ordinary ordered streams and
`BusinessRequest::stream_latest_for_owner_with_receipt(owner, work, map_event,
map_final)` for ordinary coalesced streams, and
`BusinessLatestRequest::stream_for_owner_with_receipt(owner, work, map_event,
map_final)` for ordered latest-task streams, plus
`BusinessLatestRequest::stream_latest_for_owner_with_receipt(owner, work,
map_event, map_final)` for coalesced latest-task streams, and
`CancellableBusinessRequest::stream_for_owner_with_receipt(owner, work,
map_event, map_final)` for cancellation-aware ordinary ordered streams, and
`CancellableBusinessRequest::stream_latest_for_owner_with_receipt(owner, work,
map_event, map_final)` for cancellation-aware ordinary coalesced streams, and
`BusinessRequest::latest_for(&mut keyed_tasks, key).stream_for_owner_with_receipt(owner, work, map_event, map_final)`
for ordered application-owned keyed-latest streams, plus
`BusinessRequest::latest_for(&mut keyed_tasks, key).stream_latest_for_owner_with_receipt(owner, work, map_event, map_final)`
for coalesced application-owned keyed-latest streams. All twelve routes reuse the
accepted-surface owner projection and generation ledger, the worker registry,
the existing bounded ingress, the admission receipt, and the
controller-composed cancellation probe. Ordinary and latest-task ordered
events remain FIFO; coalesced ordinary and latest-task streams keep at most one pending
intermediate payload and one queued latest marker before UI drain, replacing a
newer pending event and recording the existing coalescing diagnostic. In every
case the final is delivered once after the last accepted intermediate event,
and event/final mappers remain UI-local. The ordered latest-task route also
retains the exact latest ticket and rolls back its predecessor on invalid owner
or host admission; the coalesced latest-task route retains that same ticket and
fences by latest supersession and owner generation. The coalesced keyed-latest
owner route retains the exact host key, keyed ticket, replacement transaction,
owner generation, and receipt; it keeps only the newest pending intermediate
payload before UI drain, delivers the uncoalesced final exactly once after the
retained event, and passes exact `KeyedTaskCompletion<Key, _>` values to
UI-local/non-`Send` mappers. Keyed supersession and owner retirement
independently fence worker, mapping, and reduction. Invalid, removed,
ambiguous, unkeyed, incompatible, stale, host, capacity, closing, and
same-update admissions fail closed without `Application` fallback and restore
only the affected key's predecessor; sibling keys remain unchanged. Cancellable
ordinary owner-stream cancellation composes the explicit token and declarative
owner probes with OR semantics; token cancellation and owner retirement
independently fence cooperative work, FIFO events, final delivery, mapping, and
reduction. Its coalesced route retains one pending intermediate payload and one
queued marker, replaces older pending events, and records the existing
coalescing diagnostic; events separated by a UI drain map separately. Its receipt
remains admission-only and its mappers remain UI-local/non-`Send`.
Invalid, removed, ambiguous, unkeyed, incompatible, stale, same-update, host,
capacity, and closing admissions reject atomically without spawn, mapping, retry,
or `Application` fallback. The cancellable ordinary owner one-shot composes the
explicit token and declarative owner probes with OR semantics for cooperative
work, deferred mapping, and reduction; only that token-cancellable owner
one-shot defers mapping, while application-owned and non-cancellable owner
one-shots remain eager. Its receipt is admission-only and its UI-local mapper
need not be `Send` or `Sync`. Invalid, removed, ambiguous, unkeyed,
incompatible, stale, same-update, host, capacity, and closing admissions reject
without spawn, mapping, retry, or `Application` fallback. Keyed-latest resource ownership, `ResourceTasks`, platform ownership, renderer, scheduler,
native, and product wiring remain deferred.

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

The same qualified owner boundary now includes the application-owned keyed
latest one-shot route
`BusinessRequest::latest_for(&mut keyed_tasks, key).run_for_owner_with_receipt(owner, work, map)`.
It retains the exact host key, keyed ticket and replacement transaction,
declarative owner generation, and admission receipt; its mapper receives one
`KeyedTaskCompletion<Key, Output>` only while both keyed-latest and owner
generation fences remain current. Owner retirement and keyed supersession are
OR-composed cancellation and late-publication fences. Invalid owner, lifecycle,
host, or capacity admission rejects without spawn, mapping, reduction, retry,
or fallback to `Application`, and restores only the affected key's eligible
predecessor. `ResourceTasks` remains application-owned and has no owner-scoped
route.

The ordered keyed-latest stream retains the exact host key, keyed ticket, and
replacement transaction for every accepted FIFO intermediate event and the
single final output through `KeyedTaskCompletion<Key, Event>` and
`KeyedTaskCompletion<Key, Output>`. Keyed supersession and owner retirement
independently cancel and fence stale worker, mapping, and reduction work.
Invalid owner, lifecycle, host, or capacity admission fails closed without
fallback and restores only the affected key's eligible predecessor.

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

## 60Hz Retained Presentation Target

Radiant should target steady 60Hz presentation for interactive desktop surfaces.
Missing the cadence is a runtime-diagnostics event that should be logged or
reported with enough context to investigate the cause.

The 60Hz target must not mean full UI work every frame. Frame cadence and frame
work are separate contracts:

- The runtime should be able to wake and present at 60Hz when a surface is
  visible and expected to feel live.
- If application state, layout inputs, paint inputs, retained GPU payloads, text
  runs, scroll windows, and transient overlays are unchanged, the frame should
  reuse existing work and avoid surface reprojection, layout, paint-plan
  rebuilds, Vello scene re-encoding, retained GPU-surface uploads, and text
  shaping.
- Frame-clock messages that only update presentation state should resolve to
  `PaintOnly` when the cached base surface remains valid.
- Hosts with deterministic invalidation keys should expose explicit structural,
  layout, and projection revisions. The runtime may then select typed stages
  from paint-only through projection-with-layout-reuse, relayout, and the
  correctness-first full-surface fallback; it must never infer reuse safety from
  message names.
- Startup, resize, widget identity changes, and unknown custom-host projections
  must retain the full-surface fallback. Layout reuse is valid only while both
  the structural topology and geometry revision remain unchanged.
- Transient cursor, hover, drag, playhead, and progress overlays should prefer
  overlay or paint-only paths over structural surface refreshes.
- Retained surfaces should expose stable keys, revisions, dirty masks, and
  diagnostics so hosts can prove that unchanged segments are reused.
- Virtualized lists and dense trees should retain measurement and window metrics
  across scroll and hover frames, invalidating only when their dependencies
  change.

Radiant tests and diagnostics should make this contract hard to regress. The
runtime should expose counters for surface refreshes, paint-only presents, scene
rebuilds, paint-plan rebuilds, layout cache hits/misses, text cache hits/misses,
retained-surface hits/misses, GPU upload/rebuild counts, transient-overlay work,
and missed 60Hz frame deadlines. Stress tests should assert that stable idle
frames and overlay-only motion reuse cached work, while targeted invalidation
rebuilds only the affected regions or retained segments.

### Measured staged-refresh consumer boundary

The visible state has exactly one complete `CommittedFrameState`/last-complete
frame. A private invisible `PreparedSurfaceRefresh` candidate may contain
candidate surface and traversal, source projection, layout root, view-delta
decision, candidate layout, candidate paint plan, damage, and timing evidence.
Preparation may mutate candidate-owned storage only; it never mutates active
focus/capture/composition/wheel ownership, the declarative owner,
accessibility/automation projection, active layout, retiring-widget ownership,
or the last-complete frame.

Immediately before irreversible replacement cleanup, revalidate runtime
identity, lifecycle-transition generation, active-surface generation,
layout-state generation, viewport, window environment, requested refresh
revision, and the existing native window/adapter/target/stage/owner/revision
fences. On mismatch, stale generation, lifecycle transition, resize/recovery,
newer visual work, unsupported/ambiguous/incomplete evidence, or failure before
commit, drop the candidate with no active mutation, callback, terminal message,
or presentation and retain the combined correctness-first fallback. After
validation, perform irreversible replacement cleanup once, atomically publish
complete candidate state, then dispatch terminal messages. No scheduler yield
is permitted after cleanup begins; a panic then is terminal recovery/shutdown,
not rollback.

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

In this private implementation phase, exact native `DiscreteInput` completion
is the first authoritative native soft-budget consumer. Every successful exact
`DiscreteInput` admission binds the current effective-FPS `input_transient`
budget returned by `SchedulerSoftBudgets::for_effective_fps` and captures
admission and completion clocks independently of diagnostics, profiling, or
frame observation. `ImmediateTransient` retains its observation-only behavior:
it may bind the same budget only when private frame observation is enabled and
otherwise binds no budget or additional budget-timing clock. The typed exact
completion result is either `Completed` with `NotBudgeted`, `Within`, or
`Exceeded`, or `Mismatch`; rejected, stale, vetoed, lifecycle-invalidated,
wrong, and repeated tickets produce no policy result or lower-stage fallback.
A timed sample uses saturating elapsed time and is `Exceeded` only when elapsed
is strictly greater than its bound budget; equality is `Within`. `NotBudgeted`
and `Within` map to `ContinueNow`, while exact `Exceeded` maps to
`DeferLowerPriority`; every exact completion remains successful.

An exact exceeded `DiscreteInput` route defers only that event's lower-priority
due `Deadline` and visual follow-up through the existing bounded state at the
next safe native boundary. It does not defer `Exit` or terminal intent, replay
or roll back semantic input, add an event/message queue, or change fairness,
promotion, cadence, visual-packet, coalescing, resize, refresh/rebuild, repaint,
auxiliary-synchronization, or route-outcome policy. `FrameWork::None` does not
synthesize redraw or wake. No policy budget is consumed for `ImmediateTransient`,
`Projection`, `Layout`, `PaintPlan`, `EncodePresent`, `Maintenance`, `Deadline`,
or `Lifecycle` in this phase, and this slice adds no public diagnostics/API.

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

The target keeps the declarative UI tree, input/layout state, and native
renderer state confined to their owning window or UI runtime. It should not
turn that live state into a cross-thread object merely to parallelize work.
Expensive preparation, immutable payload production, and other bounded work may
run off-thread and return owned, generation-checked results through the runtime
boundary; a separate physical render thread remains platform-dependent.

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

The current split-pane boundary is intentionally narrower than the complete
input/accessibility target. The public `SplitPaneCollapsePolicy::{FirstPane,
SecondPane}` and additive `SplitPaneBuilder::collapse_policy(...)` opt-in apply
only to runtime-owned split panes. An admitted divider double activation
discretely resolves the selected pane to its declared minimum through the
authoritative current geometry/quantization rules; the next accepted
activation restores the last finite normalized expanded ratio, including the
latest committed drag ratio. Capacity-limited or undersized geometry is
rejected before collapse. Static and controlled modes, active drags, stale or
incompatible mounted state, unavailable capacity, invalid evidence, and no-ops
remain inert. Meaningful collapse/restore updates mounted state first,
requests the existing layout work, and emits one settled ratio after cleanup;
restore evidence is bounded and retired with the existing lifecycle.
Passive separator automation may report final geometry/value, but remains
non-focusable, actionless, non-interactive, and native-omitted. Focus ownership,
Tab/spatial traversal, keyboard/arrow-key resizing, semantic actions, native
adapters, and paint/cursor/renderer behavior remain future work.

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

Radiant does own backend-neutral automation snapshots and flattened automation
target projections for tests, devtools, direct-manipulation sidecars, and native
adapters. These may carry generic roles, labels, values, bounds, center points,
stable action names, focus state, and metadata when that information already
belongs to reusable widgets or runtime layout.

Native adapters consume this model; they do not replace it or expose raw host
handles through ordinary application APIs. Accessibility actions use the same
focus, identity, virtualization, and edit-transaction contracts as pointer and
keyboard input.

Virtual-layout semantic providers may declare a qualified application-owned
custom-coordinate transform through `VirtualLayoutParts`. The synchronous
`Rc` resolver receives only finite source geometry, runtime-validated logical
destination context, host revisions, and its exact transform revision, and
returns a conservative logical-window AABB directly. The runtime owns exact
admission, clipping, panic/reentry containment, retention, and publication;
ordinary snapshots and passive paths remain resolver-free. The private primary-
window AppKit consumer admits only qualified current Custom attachments and
consumes the compositor's normalized logical bounds plus exact sidecar
witness/publication authority. It performs the existing logical-to-AppKit
conversion and never invokes or reconstructs the custom resolver, assumes an
affine mapping, maps corners, inverts, or uses identity fallback.

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

The maintained `examples/arrangement_shell` is consumed directly by three
deterministic `standalone_gui` lanes: frame update plus current combined refresh
and paint-plan materialization; browser/inspector structural toggle plus full
refresh and relayout; and existing hover movement plus paint-only output with
zero application projection, runtime projection, widget-state synchronization,
and layout. Exact counter deltas and repeated-run counter identity are part of
the workload contract. The harness reports finite nearest-rank `p50_us`,
`p95_us`, and `p99_us` in text, JSONL, and baseline JSONL, while baseline
comparison remains average-based and old baseline files remain readable. These
measurements establish a consumer contract only; they do not claim production
staged execution or earn design-only credit.

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
- What is currently out of scope, including VST SDK integration, a direct X11
  backend, and replacing Vello

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

The GitHub Actions platform lanes are part of the target evidence contract, not
current repository evidence. Current Linux/Windows jobs provide only the
portable, build, compile, and check evidence present in `.github/workflows/ci.yml`;
they do not provide the target integration or native-host smoke evidence. The
target lanes must eventually add Linux headless Wayland and Linux/Windows
native-host smoke coverage where runners permit. Until those lanes exist, no
Linux/Windows host, IME, accessibility, presentation, latency, GPU, or
performance acceptance is established.

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
10. Extend platform support across the remaining desktop targets when the core architecture is ready.

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

- Exact text shaping/rendering stack.
- Exact plugin-host integration adapter design.
- Which additional performance workloads should become formal release gates;
  the cross-platform scheduler workload and stage budgets are already
  normative.
- Whether Vello should ever be replaced or supplemented by a custom full renderer in the future.

These are implementation or future-product details, not permission to weaken
the current three-platform target or its accessibility, CI, and evidence
requirements.

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
- Does this assume a specific platform unnecessarily?
- Would this design make additional platform support difficult?
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
- Native macOS, Windows, and Linux/Wayland support without unnecessary
  platform-specific assumptions in core code
- GitHub Actions portable/build/compile/check evidence and, where runners
  permit, the required Linux/Windows integration and headless Wayland/native-host
  lanes, plus native M5 Pro acceptance for macOS
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
