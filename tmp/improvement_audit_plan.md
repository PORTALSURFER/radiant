# Radiant Improvement Audit Plan

Date: 2026-05-08
Branch: next
Scope: Radiant as a reusable GUI library for applications like Sempal.

## Audit Read

Radiant is now directionally correct: the app-facing API is centered on
`radiant::app(...).view(...).update_with(...).run()`, retained canvas support
exists, and low-level `RuntimeBridge` is no longer the normal public path.

The next improvements should make that single API production-grade instead of
only ergonomic:

1. Runtime effects need lifecycle ownership, cancellation, and backpressure.
2. New command semantics need to be executed consistently in startup,
   subscription, update, and runtime-drain paths.
3. Retained and animated surfaces need invalidation hints that avoid full
   projection/layout work for small dynamic changes.
4. Retained canvas and app-builder APIs should use descriptive builder steps
   rather than positional primitive arguments.
5. Large files should be split by responsibility after behavior is covered by
   tests, preserving public re-exports.

## Evidence

- `src/application/runtime.rs` spawns interval subscriptions with an infinite
  sleep/enqueue loop and no stop handle.
- `src/application/runtime.rs` runs startup hooks by flattening commands through
  `Command::into_messages`, which drops repaint/focus/exit/perform and turns
  delayed messages into immediate messages.
- `src/runtime/command.rs` still exposes `into_messages` for all command shapes
  even though `Command` now represents effects, focus, exit, delayed work, and
  background work.
- `src/runtime/controller.rs` refreshes by pulling a full surface, restoring
  widget state, rebuilding hit-order metadata, and relayouting.
- `src/application/view_node.rs` collects reserved IDs and lowers the full view
  tree on every projection.
- `src/application/builders.rs` contains many public builder families in one
  file; `retained_canvas(key, revision, dirty_mask, volatile)` is positional.
- Large files remain in important public/internal areas:
  `tests/runtime_surface_public_api.rs`, `examples/folder_browser.rs`,
  `examples/waveform_view.rs`, `src/gui/list.rs`,
  `src/gui/visualization.rs`,
  `src/gui_runtime/native_vello/generic_runtime/gpu_surface.rs`,
  `src/runtime/controller.rs`, `src/runtime/surface.rs`,
  `src/application/builders.rs`, and `src/runtime/paint.rs`.

## Ordered Backlog

1. Fix app-runtime command execution and startup semantics.
2. Add lifecycle-owned runtime effects with cancellation and bounded/coalesced
   queues.
3. Add app-level invalidation and refresh planning so common runtime messages do
   not force full projection/layout.
4. Polish retained canvas API into a descriptive builder with typed handles.
5. Split application builder modules by family.
6. Split GPU-surface rendering and backend-neutral paint primitives by
   responsibility.
7. Split generic GUI data/model modules and large runtime-surface API tests into
   behavior-focused modules.

## Validation Lanes

- `cargo test`
- `cargo test --example waveform_view`
- focused tests for startup command execution, subscription cancellation,
  delayed/background commands, repaint coalescing, retained canvas metadata, and
  animation scheduling.
