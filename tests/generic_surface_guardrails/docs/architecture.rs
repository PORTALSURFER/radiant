use super::*;

#[test]
fn architecture_map_documents_target_aligned_boundaries() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let api_docs = fs::read_to_string(manifest_dir.join("docs/API.md"))
        .expect("docs/API.md should be readable");
    let architecture = fs::read_to_string(manifest_dir.join("docs/ARCHITECTURE.md"))
        .expect("docs/ARCHITECTURE.md should be readable");
    let normalized_architecture = architecture
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    assert!(
        api_docs.contains("docs/ARCHITECTURE.md"),
        "docs/API.md should point contributors to the architecture map"
    );

    for required in [
        "# Radiant Architecture Map",
        "`docs/TARGET.md`",
        "`docs/API.md` remains the application-facing contract",
        "Application code owns domain state",
        "Radiant owns declarative view construction",
        "Native or embedded hosts own the platform event loop",
        "The explicit runtime and widget modules are supported control surfaces",
        "`src/application` owns the application-builder runtime",
        "`src/runtime` owns backend-neutral retained surfaces",
        "`src/gui_runtime` owns native runtime integration and renderer adapters",
        "`examples` owns maintained public-API sandboxes",
        "`benches/perf_harness` owns opt-in performance scenarios",
        "## Rendering Boundary",
        "direct WGPU paths for retained GPU surfaces",
        "not split into separate \"Vello apps\" and \"WGPU apps\"",
        "explicit vertex/fragment entry points",
        "WGPU-specific details should stay there or behind explicit GPU-surface contracts",
        "## Text Boundary",
        "`src/gui/text_layout` owns deterministic text-line placement helpers",
        "`src/gui_runtime/native_vello/text_renderer` owns native text rendering",
        "`src/gui_runtime/native_vello/text_edit` owns native text-edit state",
        "`NativeTextOptions` and `EmbeddedFont`",
        "## Platform Boundary",
        "Windows-first today",
        "typed `PlatformRequest` commands",
        "Current target-specific seams are intentionally narrow",
        "`src/gui_runtime/native_vello/generic_runtime/window/platform.rs`",
        "`src/gui_runtime/native_vello/generic_runtime/external_drag/platform.rs`",
        "`src/gui_runtime/native_vello/text_renderer/font.rs`",
        "`examples/popup_window/platform.rs`",
        "`examples/popup_window/platform/readiness.rs`",
        "`examples/popup_window/host/child.rs`",
        "`examples/popup_window/host/prewarm.rs`",
        "`examples/popup_window/host/process.rs`",
        "explicit non-target fallback",
        "Do not add raw Windows imports",
        "## Validation Map",
        "cargo test --test generic_surface_guardrails",
        "cargo clippy --all-targets --all-features -- -D warnings",
        "cargo test -j 1 --lib --tests",
        "cargo bench --bench perf_harness -- --list",
        "Performance benchmarks are trend and profiling tools",
        "text-line layout caching",
        "## Current Non-Goals",
        "Radiant should not own VST SDK integration",
        "accessibility systems in the current phase",
    ] {
        assert!(
            normalized_architecture.contains(required),
            "docs/ARCHITECTURE.md should document `{required}`"
        );
    }
}
