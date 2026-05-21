# Radiant

Radiant is a Windows-first Rust GUI library for serious desktop applications.
It exposes one public API for declarative views, application state updates,
layout, input, focus, styling, Vello-backed rendering, and retained GPU
surfaces for dense realtime visuals.

Radiant is intended to stay application-independent. Product state, files,
audio/plugin host behavior, and domain-specific naming belong in the host
application; Radiant owns generic GUI primitives, runtime surfaces, paint plans,
diagnostics, examples, and validation.

## Start Here

Use the prelude and application builders for normal apps:

```rust
use radiant::prelude::*;

fn main() -> radiant::Result {
    radiant::window("Radiant Hello World").run(text("Hello, world!"))
}
```

Low-level runtime objects such as `RuntimeBridge`, `UiSurface`, `SurfaceNode`,
and `NativeRunOptions` remain supported for custom hosts, embedded surfaces,
diagnostics, and advanced widgets. They are explicit control surfaces, not a
second framework.

## Documentation Map

- `docs/API.md`: application-facing API contract, examples, runtime model, and
  validation lane.
- `docs/ARCHITECTURE.md`: contributor map for ownership boundaries, platform
  seams, rendering boundaries, and validation slices.
- `docs/TARGET.md`: long-term project direction for a standalone,
  high-performance general-purpose GUI library.

Radiant uses the workspace CC0 1.0 license.

## Examples

Examples are maintained API sandboxes and validation targets:

```powershell
cargo run --example hello_world
cargo run --example generic_native
cargo run --example widget_gallery
cargo run --example waveform_view
cargo run --example timeline_editor
```

Run all checked examples with:

```powershell
cargo test --examples
```

## Validation

The normal local quality lane is documented in `docs/API.md`. The short version:

```powershell
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --lib --tests
cargo test --examples
cargo doc --no-deps
cargo test --doc
```

Portable library checks for the documented future Linux and macOS targets and
the perf-harness smoke lane are also part of the full validation path.
