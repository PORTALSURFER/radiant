use super::*;

#[test]
fn api_docs_describe_the_structural_boundary_strategy() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let crate_docs =
        fs::read_to_string(manifest_dir.join("src/lib.rs")).expect("src/lib.rs should be readable");
    let docs = fs::read_to_string(manifest_dir.join("docs/API.md"))
        .expect("docs/API.md should be readable");
    let normalized_crate_docs = crate_docs
        .lines()
        .map(|line| line.trim_start().strip_prefix("//!").unwrap_or(line))
        .collect::<Vec<_>>()
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let normalized_docs = docs.split_whitespace().collect::<Vec<_>>().join(" ");

    for required in [
        "Radiant exposes one public API with progressive control",
        "use radiant::prelude::*;",
        "radiant::window(\"Radiant Hello World\").run(text(\"Hello, world!\"))",
        "not a separate framework",
        "Start with `README.md`",
        "`docs/API.md`",
        "checked public API boundary",
        "`docs/ARCHITECTURE.md`",
        "ownership boundaries",
        "`docs/TARGET.md`",
        "long-term standalone GUI library direction",
        "`widget_gallery`",
        "`waveform_view`",
        "`timeline_editor`",
    ] {
        assert!(
            normalized_crate_docs.contains(required),
            "crate docs should document `{required}`"
        );
    }

    for required in [
        "# Radiant Core API",
        "Dependency Boundary",
        "host -> Radiant, never Radiant -> host",
        "Boundary tests prove that dependency direction, public exports, examples, and",
        "they intentionally avoid enforcing product",
        "Radiant now exposes only generic GUI and native runtime APIs",
        "Radiant exposes one public API with progressive control",
        "Application builders and explicit runtime objects are part of the same API surface",
        "same model with more explicit control",
        "Radiant's application API is designed to be easy to read without hiding the runtime model",
        "View, Element, And Widget",
        "VirtualListWindow",
        "virtual_list_view_start_after_scroll_delta",
        "SignalChromeState",
        "SignalToolFlags",
        "SignalToolState",
        "SignalRasterPreview",
        "TimelineViewport",
        "TimelineTransportState",
        "TimelineEditPreview",
        "TimelineEditPreviewParts",
        "TimelineFeedbackEvents",
        "TimelinePresentationState",
        "TimelineSurfaceParts",
        "TimelineSurfaceState",
        "TimelineMotionState",
        "UiSurface",
        "SurfaceNode",
        "WidgetId",
        "Command<Message>",
        "Soft-Deprecated First-Use Boilerplate",
        "not a Rust `#[deprecated]` attribute on the explicit control objects",
        "RuntimeRunReport<Artifacts, Error>",
        "RuntimeBridge",
        "ThemeTokens",
        "SurfacePaintPlan",
        "SurfaceRuntime::borrowed_frame_into(...)",
        "reuse `SurfacePaintPlan` primitive storage",
        "SurfaceRuntime::dispatch_pointer_move_with_outcome(...)",
        "SurfaceRuntime::dispatch_pointer_move_deferred_refresh_with_outcome(...)",
        "PointerMoveOutcome",
        "paint-only overlay requests",
        "Native popup windows are revealed as soon as the window surface and initial Radiant scene are prepared",
        "instant transient UI surface",
        "prewarm one offscreen visible popup surface",
        "NativePopupOptions::prewarmed_at(...)",
        "prime the non-focusing show/hide path",
        "park the prepared surface visible at the offscreen prewarm position",
        "first post-hide native reveal",
        "request foreground activation after the already-rendered surface is visible",
        "first native show",
        "first presented frame",
        "InvalidationMask",
        "RetainedSegmentMask",
        "VisualSnapshot",
    ] {
        assert!(
            normalized_docs.contains(required),
            "docs/API.md should document `{required}`"
        );
    }
}

#[test]
fn crate_layout_facade_uses_explicit_exports() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let crate_source =
        fs::read_to_string(manifest_dir.join("src/lib.rs")).expect("src/lib.rs should be readable");

    assert!(
        crate_source.contains("pub mod layout {")
            && crate_source.contains("pub use crate::gui::layout_core::{")
            && crate_source.contains("LayoutEngine")
            && crate_source.contains("layout_tree_with_state")
            && crate_source.contains("StackedRowRectsParts")
            && crate_source.contains("VirtualizationPolicy"),
        "crate layout facade should name the stable layout API exports explicitly"
    );
    assert!(
        !crate_source.contains("pub use crate::gui::layout_core::*;"),
        "crate layout facade should not wildcard-export layout internals"
    );
}

#[test]
fn api_docs_soft_deprecate_only_first_use_runtime_boilerplate() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let docs = fs::read_to_string(manifest_dir.join("docs/API.md"))
        .expect("docs/API.md should be readable");
    let runtime = fs::read_to_string(manifest_dir.join("src/runtime/mod.rs"))
        .expect("runtime module should be readable");

    for first_use_boilerplate in [
        "constructing `NativeRunOptions` directly for a hello-world app",
        "hand-writing a closure bridge before the app has meaningful state",
        "wrapping one label in `Arc<UiSurface<_>>`",
        "manually composing `SurfaceNode`, `SurfaceChild`, explicit numeric IDs, and",
    ] {
        assert!(
            docs.contains(first_use_boilerplate),
            "docs/API.md should soft-deprecate `{first_use_boilerplate}` for first-use application code"
        );
    }

    for explicit_control in [
        "The `radiant::runtime` module",
        "`RuntimeBridge`",
        "`UiSurface`",
        "`SurfaceNode`",
        "`SurfaceChild`",
        "`NativeRunOptions`",
        "`WidgetSizing`",
        "remain supported and non-deprecated for hosts",
    ] {
        assert!(
            docs.contains(explicit_control),
            "docs/API.md should preserve explicit-control API guidance for `{explicit_control}`"
        );
    }
    assert!(
        !runtime.contains("#[deprecated"),
        "radiant::runtime should remain supported, not a blanket-deprecated module"
    );
}
