# Radiant Architecture Map

This map explains how the current Radiant codebase is organized against
`docs/TARGET.md`. It is a contributor guide, not a second public API reference:
`docs/API.md` remains the application-facing contract.

Radiant's architecture should keep one external mental model while allowing
focused internal modules. The main ownership rule is:

- Application code owns domain state, side effects, files, audio/plugin hosts,
  and product-specific naming.
- Radiant owns declarative view construction, stable widget identity, layout,
  input routing, focus, styling, invalidation, paint planning, diagnostics, and
  renderer-facing surface contracts.
- Native or embedded hosts own the platform event loop and decide how to attach
  Radiant surfaces to windows, popup surfaces, or host-controlled render
  targets.

## Public Surface

Normal application code should start through `radiant::prelude`,
`radiant::window(...)`, or `radiant::app(...)`. These builders lower into the
same `UiSurface`, `SurfaceNode`, `WidgetId`, `Command`, and `RuntimeBridge`
contracts exposed through the explicit runtime modules.

The explicit runtime and widget modules are supported control surfaces, not a
competing framework. They exist for custom hosts, tests, advanced widgets,
diagnostics, and embedded integration where the application needs to drive a
surface without the native window runner.

## Core Subsystems

- `src/application` owns the application-builder runtime: state projection,
  update callbacks, runtime messages, subscriptions, timers, and background
  work delivery back into the UI-first runtime.
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

Current target-specific seams are intentionally narrow:

- `src/gui_runtime/native_vello/generic_runtime/window/platform.rs` owns native
  window attribute extensions such as Windows drag/drop and popup taskbar
  policy. Non-Windows targets keep the same runtime options and no-op the
  unsupported window hints.
- `src/gui_runtime/native_vello/generic_runtime/external_drag/platform.rs` owns
  external drag-out platform selection. Windows delegates to the native drag
  implementation; other targets report an explicit unsupported result through
  the normal runtime command path.
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
- Examples: `cargo test --examples`, or the focused example target when a
  change is local to one sandbox.
- Formatting and linting: `cargo fmt --check` and
  `cargo clippy --all-targets --all-features -- -D warnings`.
- Broad regression lane: `cargo test -j 1 --lib --tests`.
- Performance investigation: `cargo bench --bench perf_harness -- --list` to
  inspect scenarios, then `cargo bench --bench perf_harness <scenario>` for a
  focused trend run.

Performance benchmarks are trend and profiling tools, not portable timing
pass/fail gates. They should still cover hot paths that matter to the target:
large layout trees, virtualized lists, paint-plan generation, command drainage,
runtime refreshes, pointer overlays, text-line layout caching, and GPU-surface
data preparation.

## Current Non-Goals

Radiant should not own VST SDK integration, audio-domain host behavior,
application-specific asset models, product-specific state, or accessibility
systems in the current phase. Those concerns can integrate with Radiant through
host-owned state, platform services, custom widgets, runtime commands, and
embedded-host surfaces without becoming Radiant core.

Avoid new architecture that creates parallel application models, leaks renderer
internals into normal app code, couples core modules to Windows-only behavior,
or makes examples the only proof of a public feature. A feature is aligned when
it has a coherent API, clean module ownership, tests or guardrails where
practical, and an example or documentation path that shows how application code
is expected to use it.
