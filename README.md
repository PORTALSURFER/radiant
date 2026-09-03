# Radiant

Radiant is a macOS-first Rust GUI library for serious desktop applications,
designed to grow into a cross-platform library. It exposes one public API for
declarative views, application state updates, layout, input, focus, styling,
Vello-backed rendering, and retained GPU surfaces for dense realtime visuals.

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

Low-level runtime objects such as the minimal `RuntimeBridge`, cached
`RuntimeHostCapabilities`, `UiSurface`, `SurfaceNode`, and `NativeRunOptions`
remain supported for custom hosts, embedded surfaces, diagnostics, and advanced
widgets. They are explicit control surfaces, not a second framework.

## Documentation Map

- `docs/DESIGN_DIRECTION.md`: normative target-state architecture contract for
  the node model, application API, scheduling, and rendering boundaries.
- `docs/API.md`: application-facing API contract, examples, runtime model, and
  validation lane.
- `docs/API_STYLE.md`: preferred public API style, example style, and cleanup
  ticket criteria.
- `docs/migrations/TIMER_API_MIGRATION.md`: custom-host timer wake migration
  and the UI-owned mapper contract.
- `docs/ARCHITECTURE.md`: contributor map for ownership boundaries, platform
  seams, rendering boundaries, and validation slices.
- [Platform Acceptance and Evidence Policy](docs/PLATFORM_ACCEPTANCE.md):
  canonical three-platform acceptance, lanes, outcomes, baselines, gates, and
  artifact rules.
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
cargo run --example curve_area_fill
cargo run --example timeline_editor
cargo run --example eq_editor
cargo run --example spectrogram
```

Run all checked examples with:

```powershell
cargo test --examples
```

## macOS Dev App Bundles

Direct Unix binaries with windows are not always visible as normal applications
to macOS LaunchServices or app-level UI automation tools. Radiant ships a
generic helper for host applications that need a LaunchServices-visible
development `.app` wrapper around a prebuilt binary. From the root of a
standalone Radiant checkout, run:

```bash
cargo build --release --example hello_world
RADIANT_DEV_APP_NAME="Radiant Hello World" \
RADIANT_DEV_APP_BINARY="$PWD/target/release/examples/hello_world" \
RADIANT_DEV_APP_BUNDLE_ID="com.example.radiant.hello-world.dev" \
./scripts/dev_app_bundle.sh --log
```

When Radiant is vendored under `vendor/radiant` in a host repository, run
`vendor/radiant/scripts/dev_app_bundle.sh` from that host repository's root
instead.

The helper stages `target/dev-app/My App.app`, writes a minimal `Info.plist`,
copies the binary into `Contents/MacOS`, ad-hoc signs when possible, and
launches through `open`. Set `RADIANT_DEV_APP_PREPARE_ONLY=1` to stage without
launching. This supports app-level automation attachment by app name or bundle
id; Radiant's backend-neutral automation snapshots and flattened target
projection remain the semantic UI source for tests, devtools, Computer Use
sidecars, and future accessibility adapters.

For direct dev-app sidecars, set `RADIANT_AUTOMATION_TARGET_EXPORT` to a JSON
path before launching. The native runtime writes the latest flattened target
snapshot after surface refreshes, using atomic replacement and skipping
unchanged payloads. Set `RADIANT_AUTOMATION_TARGET_EXPORT_PRETTY=1` for readable
JSON during manual automation work.

## Validation

The [Platform Acceptance and Evidence Policy](docs/PLATFORM_ACCEPTANCE.md)
classifies this portable quality procedure as lane C, with lane A applying to
deterministic runtime behavior. It does not establish H, N, or M native
acceptance.

The normal local quality lane is documented in `docs/API.md`. The short version:

```powershell
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --lib --tests
cargo test --examples
cargo doc --no-deps
cargo test --doc
```

Portable library checks preserve the cross-platform boundary without asserting
a current platform-support matrix, and the perf-harness smoke lane is also
part of the full validation path.
