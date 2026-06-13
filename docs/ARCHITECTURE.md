# Radiant Architecture Map

This map explains how the current Radiant codebase is organized against
`docs/TARGET.md`. It is a contributor guide, not a second public API reference:
`docs/API.md` remains the application-facing contract, while
`docs/API_STYLE.md` defines the preferred application-facing API style and
cleanup-ticket target shape.

Radiant's architecture should keep one external mental model while allowing
focused internal modules. The main ownership rule is:

- Application code owns domain state, business logic, files, audio/plugin
  hosts, and product-specific naming.
- Radiant owns declarative view construction, stable widget identity, layout,
  input routing, focus, styling, invalidation, paint planning, diagnostics, and
  renderer-facing surface contracts.
- Radiant owns the scheduling boundary for UI-safe follow-up work. Normal app
  update handlers must not run business work directly; they schedule it through
  `UiUpdateContext::business()` or request typed Radiant platform services.
- Radiant additions must pass the primitive-boundary test in `docs/TARGET.md`:
  they should be generic UI primitives or reusable GUI building blocks, not
  product-shaped composite widgets or application workflows.
- Native or embedded hosts own the platform event loop and decide how to attach
  Radiant surfaces to windows, popup surfaces, or host-controlled render
  targets.

## Public Surface

Normal application code should start through `radiant::prelude`,
`radiant::window(...)`, or `radiant::app(...)`. These builders lower into the
same `UiSurface`, `SurfaceNode`, `WidgetId`, `Command`, and `RuntimeBridge`
contracts exposed through the explicit runtime modules.

The normal app-facing surface is intentionally non-blocking. Update handlers
may mutate durable UI/application state, apply business or platform-service
results, emit messages, request repaint/focus/timers, request typed platform
services, and schedule business work. Filesystem, database, decode/load,
network/process work, sleeps, blocking waits or joins, thread creation, cache
hydration, and long CPU transforms belong behind the business runtime or a
platform adapter. Command-returning and command-injection paths are migration or
advanced surfaces, not the target ordinary application model.

The explicit runtime and widget modules are supported control surfaces, not a
competing framework. They exist for custom hosts, tests, advanced widgets,
diagnostics, and embedded integration where the application needs to drive a
surface without the native window runner.

### Prelude Export Hygiene

`src/prelude.rs` is only the small facade for common application imports. It
must not accumulate direct subsystem export lists. The first-level prelude
modules are facades too: if a grouped file grows into a broad import block, split
it into smaller owning modules under a directory with the same name, as
`src/prelude/application/**`, `src/prelude/gui/**`, and
`src/prelude/runtime/**` do today. Add new prelude exports to the smallest
owning grouped file:

- `application.rs` for app builders, view builders, control builders, menu
  builders, panel builders, task/update types, and stateful app helpers.
- `gui.rs` for backend-neutral GUI models, layout helpers, selection,
  shortcuts, visualization, text-layout, paint geometry, and form/panel state
  helpers.
- `layout.rs` for direct layout result/types that are part of common app-facing
  signatures.
- `runtime.rs` for commands, runtime services, paint primitives, native run
  options, resources, drag/drop, windows, diagnostics, and retained surfaces.
- `theme.rs` for theme-token exports.
- `widgets.rs` for widget contracts, primitive widget parts, input/output
  messages, interaction helpers, and widget visual tokens.

Prelude export files should stay cohesive enough to scan without horizontal
ownership hunting. As a rule of thumb, split a file before it mixes unrelated
areas such as controls plus menus plus tasks, or before it needs a very large
`pub use crate::<subsystem>::{...}` block just to remain formatted. A facade may
`pub use` its focused child modules, but the child modules should encode the API
area being exported: controls, details lists, overlays, resources, paint,
visualization, and so on.

Keep exports explicit inside those grouped files. Do not replace them with
wildcard re-exports from owning subsystems, because that would let unrelated
future public items leak into `radiant::prelude::*` without an API decision.
When an item is useful but not common enough for normal app code, leave it on
its owning explicit module such as `radiant::runtime`, `radiant::widgets`,
`radiant::layout`, `radiant::theme`, or `radiant::gui`.

Import and export size is treated as design feedback, not as a formatting
problem. A prelude export leaf should stay below the source-quality guardrail of
32 lines. When a leaf approaches that limit, first decide whether the API area is
too broad, whether a new focused prelude child should own part of the surface, or
whether the item should stay on its explicit subsystem module instead of entering
the common prelude. Do not add catch-all prelude modules, wildcard subsystem
exports, or local application preludes to make a large list disappear.

New prelude exports should follow this checklist:

- Add the item to the smallest existing prelude leaf module that matches the
  owning API area.
- If no focused leaf exists, add one under the matching first-level prelude
  directory instead of lengthening a facade.
- Keep first-level split groups such as `application`, `gui`, and `runtime` as
  child-module facades. They should not contain direct `pub use crate::...`
  lists once they have been split.
- Treat a large import or export block as a module-boundary smell. First decide
  whether the code is mixing ownership areas or whether a reusable GUI primitive
  belongs in Radiant, then add the import only after the boundary is clear.
- If an application needs many low-level widget, paint, layout, and runtime
  types together, prefer introducing a generic Radiant primitive or splitting the
  app-side custom widget into smaller paint/input/state modules over expanding
  `radiant::prelude`.

The source-quality guardrails assert that the root prelude and split first-level
prelude groups remain facades. If those tests fail, the fix should normally be a
new focused export leaf or a module split, not a formatting workaround.

## Core Subsystems

- `src/application` owns the application-builder runtime: state projection,
  update callbacks, runtime messages, subscriptions, timers, and business-work
  delivery back into the UI-first runtime.
- `src/runtime` owns backend-neutral retained surfaces, runtime commands,
  widget traversal, input dispatch, focus, scroll state, resource slots,
  platform requests, paint plans, diagnostics, GPU-surface payload contracts,
  and the `RuntimeBridge` boundary.
- `src/widgets` owns built-in widget contracts and named-part construction for
  primitive widgets.
- `src/gui` owns reusable backend-neutral GUI models: layout, forms, feedback,
  panels, lists, selection, shortcuts, text-line placement, visualization
  helpers, automation snapshots, and visual snapshots.
- `src/gui_runtime` owns native runtime integration and renderer adapters. Its
  native Vello runtime is the Windows-first implementation path and keeps WGPU,
  Vello, font loading, scene caching, native input, window policy, and popup
  behavior behind Radiant-owned runtime options.
- `examples` owns maintained public-API sandboxes. Examples are validation
  surfaces as well as documentation.
- `benches/perf_harness` owns opt-in performance scenarios for layout,
  application projection, runtime surface work, command drainage, and
  GPU-surface data preparation.
- `tests` owns public API, behavior, source-quality, example, and documentation
  guardrails.

## Rendering Boundary

Radiant uses Vello for normal UI primitives and direct WGPU paths for retained
GPU surfaces where dense realtime rendering benefits from custom GPU resources.
The public application model should not split into separate "Vello apps" and
"WGPU apps". A custom GPU surface is still a Radiant widget/surface: it
participates in layout, input routing, paint planning, diagnostics, and normal
runtime invalidation.

Built-in GPU-surface payloads cover atlas and signal rendering. Advanced shader
surfaces use a backend-neutral custom shader descriptor for stable shader
identity, optional WGSL source, explicit vertex/fragment entry points, and
opaque payload bytes. The native WGPU adapter executes the source-backed subset
with Radiant's surface-uniform ABI plus optional app uniform and read-only
storage payload bindings; descriptors missing the required shader handoff
report that through frame diagnostics instead of introducing a parallel
application-facing WGPU API. The same diagnostics separate rendered custom
shader surfaces, pipeline rebuilds, bind-group rebuilds, and bind-group cache
hits so native shader setup is visible without exposing raw WGPU handles.
Validation errors from shader modules, render pipelines, and bind groups are
counted separately from unsupported descriptors and logged through tracing with
the backend error message.

Backend-neutral paint plans live under `src/runtime/paint`. Native Vello scene
construction, retained scene caching, post-GPU overlays, GPU-surface pipelines,
and frame presentation live under
`src/gui_runtime/native_vello/generic_runtime`. WGPU-specific details should
stay there or behind explicit GPU-surface contracts, not leak into normal
application-builder code.

## Text Boundary

Radiant treats text as a first-class GUI concern but keeps the responsibilities
separate:

- `src/gui/text_layout` owns deterministic text-line placement helpers and a
  small owned cache for renderer-side reuse.
- `src/gui_runtime/native_vello/text_renderer` owns native text rendering,
  font fallback, glyph/cache behavior, and scene text encoding.
- `src/gui_runtime/native_vello/text_edit` owns native text-edit state,
  cursor stops, selection, and single-line edit layout.
- `src/runtime/paint/primitives/text.rs` owns backend-neutral text paint
  primitives and shared text storage for paint plans.

Application code should configure portable font policy through
`NativeTextOptions` and `EmbeddedFont` instead of depending on installed fonts
or renderer internals.

## Platform Boundary

Radiant is Windows-first today, but core GUI, runtime, widget, layout, and
paint-plan code should stay platform-neutral. Windows-specific integration
belongs in native runtime/windowing modules or explicitly named platform
adapters. Platform services such as file dialogs and URL opening flow through
typed `PlatformRequest` commands and `RuntimeBridge::request_platform_service`.
Application update handlers request those services through Radiant context
helpers instead of calling platform APIs directly. The portable library
boundary should keep compiling for future Linux and macOS targets even while
native runtime behavior is validated Windows-first.

Current target-specific seams are intentionally narrow:

- `src/application/runtime/threading/platform.rs` owns native thread-priority
  hints for background business workers. The application runtime keeps a
  platform-neutral worker-pool contract, while unsupported targets use the same
  worker loop without priority changes.
- `src/application/runtime/bridge/adapter/platform_services.rs` owns app-runtime
  platform service dispatch for file dialogs, reveal/open, clipboard text and
  file-list reads/writes, and confirmation prompts. The bridge exposes typed
  `PlatformRequest` values while target-specific reveal and clipboard behavior
  stays inside this adapter.
- `src/gui_runtime/native_vello/generic_runtime/window/platform.rs` owns native
  window attribute extensions such as Windows drag/drop and popup taskbar
  policy. Non-Windows targets keep the same runtime options and no-op the
  unsupported window hints.
- `src/gui_runtime/native_vello/generic_runtime/external_drag/platform.rs` owns
  external drag-out platform selection. Windows delegates to the native drag
  implementation; other targets report an explicit unsupported result through
  the normal runtime command path.
- `src/gui_runtime/native_vello/generic_runtime/external_drag/windows.rs`,
  `src/gui_runtime/native_vello/generic_runtime/external_drag/data_object.rs`,
  `src/gui_runtime/native_vello/generic_runtime/external_drag/drop_source.rs`,
  `src/gui_runtime/native_vello/generic_runtime/external_drag/payload.rs`,
  `src/gui_runtime/native_vello/generic_runtime/external_drag/preview.rs`,
  `src/gui_runtime/native_vello/generic_runtime/external_drag/data_object/formats.rs`,
  `src/gui_runtime/native_vello/generic_runtime/external_drag/data_object/medium.rs`,
  and
  `src/gui_runtime/native_vello/generic_runtime/external_drag/payload/dropfiles.rs`
  are the Windows-only OLE file-drag implementation behind that selector. These
  modules must stay reachable only through the cfg-gated platform adapter.
- `src/gui_runtime/native_vello/text_renderer/font.rs` owns native fallback
  font discovery after application-provided embedded fonts and font paths.
  Platform-specific font candidates stay inside that renderer adapter rather
  than leaking installed-font assumptions into widgets or layout.
- `examples/popup_window/platform.rs`,
  `examples/popup_window/platform/readiness.rs`,
  `examples/popup_window/host/child.rs`,
  `examples/popup_window/host/prewarm.rs`, and
  `examples/popup_window/host/process.rs` own the popup example's optional
  Windows window-control proof. The public popup and multi-window APIs remain
  platform-neutral, and non-Windows example paths degrade through local no-op
  or unsupported host behavior.

New target-specific code should either fit one of these seams or introduce a
similarly named adapter with a neutral public contract and an explicit
non-target fallback. Do not add raw Windows imports, `target_os` branches, or
installed-font/path assumptions to core widget, layout, styling, runtime
surface, or paint-plan modules.

## Validation Map

Use the smallest validation slice that proves the edited boundary, then run the
normal quality lane before merging meaningful changes.

- Public API/runtime behavior: `cargo test --test runtime_surface_public_api`,
  `cargo test --test runtime_bridge_public_api`, or the specific public API
  integration test touched by the change.
- Source-quality and documentation guardrails:
  `cargo test --test generic_surface_guardrails`.
- Reusable host-app non-blocking scans:
  `radiant::guardrails::NonBlockingGuardrail::app_update_paths()` from a
  consumer test. Applications should scan app-facing update/action/view roots,
  add host-domain forbidden tokens such as decode or database entry points, and
  allowlist only explicit worker or platform-adapter modules.
- Examples: `cargo test --examples`, or the focused example target when a
  change is local to one sandbox.
- Documentation: `cargo doc --no-deps` to verify the public docs build and
  catch broken intra-doc links before they reach CI.
- Doctests: `cargo test --doc` to keep public documentation snippets compiling
  against the real crate API.
- Formatting and linting: `cargo fmt -- --check` and
  `cargo clippy --all-targets --all-features -- -D warnings`.
- Broad regression lane: `cargo test --lib --tests`, matching CI. Use
  `cargo test -j 1 --lib --tests` when diagnosing order-sensitive failures or
  reducing concurrent resource pressure locally.
- Example compile checks: `cargo test --examples`.
- Portable library boundary: after installing the targets with
  `rustup target add x86_64-unknown-linux-gnu x86_64-apple-darwin`, run
  `cargo check --lib --no-default-features --target x86_64-unknown-linux-gnu`
  and
  `cargo check --lib --no-default-features --target x86_64-apple-darwin`.
  These checks do not prove native Linux/macOS runtime behavior, but they do
  catch accidental Windows-only imports, target-specific dependency leakage, and
  public/core API drift that would make future platform support a rewrite.
- Performance smoke: `cargo bench --bench perf_harness -- --list`, then a
  focused JSONL baseline round trip such as
  `cargo bench --bench perf_harness runtime_virtualized_list_hover -- --jsonl --write-baseline-jsonl .\target\perf-baseline.jsonl`
  followed by
  `cargo bench --bench perf_harness runtime_virtualized_list_hover -- --jsonl --baseline-jsonl .\target\perf-baseline.jsonl --fail-on-missing-baseline`.
- Performance investigation: run `cargo bench --bench perf_harness <scenario>`
  or a filtered `--category` pass for focused trend work.

Performance benchmarks are trend and profiling tools, not portable timing
pass/fail gates. They should still cover hot paths that matter to the target:
large layout trees, virtualized lists, paint-plan generation, command drainage,
runtime refreshes, pointer overlays, text-line layout caching, and GPU-surface
data preparation.

## Current Non-Goals

Radiant should not own VST SDK integration, audio-domain host behavior,
application-specific asset models, product-specific state, or accessibility
systems in the current phase. Those concerns can integrate with Radiant through
host-owned state, platform services, custom widgets, business-runtime requests,
and embedded-host surfaces without becoming Radiant core.

Avoid new architecture that creates parallel application models, leaks renderer
internals into normal app code, couples core modules to Windows-only behavior,
or makes examples the only proof of a public feature. A feature is aligned when
it has a coherent API, clean module ownership, tests or guardrails where
practical, and an example or documentation path that shows how application code
is expected to use it.
