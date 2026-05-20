use super::*;

#[test]
fn status_line_entries_use_named_parts_for_source_and_message() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = manifest_dir.join("src/gui/feedback/status/line.rs");
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));
    let source_tests =
        fs::read_to_string(manifest_dir.join("src/gui/feedback/status/line/tests.rs"))
            .expect("status line tests should be readable");
    let status = fs::read_to_string(manifest_dir.join("src/gui/feedback/status.rs"))
        .expect("feedback status module should be readable");
    let recovery = fs::read_to_string(manifest_dir.join("src/gui/feedback/status/recovery.rs"))
        .expect("feedback recovery status module should be readable");
    let health = fs::read_to_string(manifest_dir.join("src/gui/feedback/status/health.rs"))
        .expect("feedback health status module should be readable");
    let drag_overlay =
        fs::read_to_string(manifest_dir.join("src/gui/feedback/status/drag_overlay.rs"))
            .expect("feedback drag-overlay status module should be readable");
    let update = fs::read_to_string(manifest_dir.join("src/gui/feedback/status/update.rs"))
        .expect("feedback update status module should be readable");
    let prompt = fs::read_to_string(manifest_dir.join("src/gui/feedback/status/prompt.rs"))
        .expect("feedback prompt status module should be readable");
    let tests = fs::read_to_string(manifest_dir.join("src/gui/feedback/status/tests.rs"))
        .expect("feedback status tests should be readable");
    let feedback = fs::read_to_string(manifest_dir.join("src/gui/feedback.rs"))
        .expect("feedback module should be readable");
    let lib = fs::read_to_string(manifest_dir.join("src/lib.rs"))
        .expect("library module should be readable");

    assert!(
        source.contains("pub struct StatusLineEntryParts")
            && source.contains("pub fn from_parts(parts: StatusLineEntryParts) -> Self")
            && source.contains("#[path = \"line/tests.rs\"]")
            && !source.contains("fn status_line_log_keeps_latest_bounded_message"),
        "status-line entries should expose named parts for source and message text while delegating behavior tests"
    );
    assert!(
        source_tests.contains("fn status_line_log_keeps_latest_bounded_message")
            && source_tests.contains("fn status_line_entry_supports_named_parts_construction"),
        "status-line behavior tests should live in status/line/tests.rs"
    );
    assert!(
        source.contains("Self::from_parts(StatusLineEntryParts {")
            && status.contains("StatusLineEntryParts")
            && feedback.contains("StatusLineEntryParts")
            && lib.contains("StatusLineEntryParts"),
        "status-line entry compatibility constructor and public exports should keep the named-parts path available"
    );
    for required in [
        "mod drag_overlay;",
        "mod health;",
        "mod prompt;",
        "mod recovery;",
        "mod update;",
        "#[path = \"status/tests.rs\"]",
        "pub use drag_overlay::DragOverlay;",
        "pub use health::HealthState;",
        "pub use prompt::{ConfirmPrompt, PromptIntent};",
        "pub use recovery::RecoverySummary;",
        "pub use update::{UpdatePanel, UpdateStatus};",
    ] {
        assert!(
            status.contains(required),
            "feedback status facade should delegate `{required}`"
        );
    }
    assert!(
        !status.contains("pub struct RecoverySummary")
            && !status.contains("pub enum HealthState")
            && !status.contains("pub struct DragOverlay")
            && !status.contains("pub struct UpdatePanel")
            && !status.contains("pub struct ConfirmPrompt")
            && !status.contains("fn recovery_summary_defaults_to_idle_and_empty"),
        "feedback status root should re-export focused models and delegate behavior tests instead of owning them"
    );
    assert!(
        recovery.contains("pub struct RecoverySummary")
            && health.contains("pub enum HealthState")
            && drag_overlay.contains("pub struct DragOverlay")
            && update.contains("pub enum UpdateStatus")
            && update.contains("pub struct UpdatePanel")
            && prompt.contains("pub enum PromptIntent")
            && prompt.contains("pub struct ConfirmPrompt"),
        "feedback status models should live in their focused status child modules"
    );
    assert!(
        tests.contains("fn recovery_summary_defaults_to_idle_and_empty")
            && tests.contains("fn prompt_intent_exposes_generic_confirmation_categories"),
        "feedback status behavior tests should live in status/tests.rs"
    );
}

#[test]
fn inline_feedback_indicators_keep_model_geometry_and_sanitizers_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/gui/feedback/inline.rs"))
        .expect("inline feedback root should be readable");
    let model = fs::read_to_string(manifest_dir.join("src/gui/feedback/inline/model.rs"))
        .expect("inline feedback model should be readable");
    let geometry = fs::read_to_string(manifest_dir.join("src/gui/feedback/inline/geometry.rs"))
        .expect("inline feedback geometry should be readable");
    let sanitize = fs::read_to_string(manifest_dir.join("src/gui/feedback/inline/sanitize.rs"))
        .expect("inline feedback sanitizers should be readable");
    let tests = fs::read_to_string(manifest_dir.join("src/gui/feedback/inline/tests.rs"))
        .expect("inline feedback tests should be readable");
    let feedback = fs::read_to_string(manifest_dir.join("src/gui/feedback.rs"))
        .expect("feedback facade should be readable");

    assert!(
        root.contains("mod geometry;")
            && root.contains("mod model;")
            && root.contains("mod sanitize;")
            && root.contains("#[path = \"inline/tests.rs\"]")
            && root.contains("pub use geometry::{inline_indicator_layout")
            && root.contains("pub use model::{InlineIndicatorAnchor")
            && !root.contains("pub struct InlineIndicatorMetrics")
            && !root.contains("fn finite_nonnegative")
            && !root.contains("fn inline_indicator_reserved_width_includes_text_gap"),
        "inline feedback root should re-export focused model and geometry modules while delegating behavior tests"
    );
    assert!(
        model.contains("pub struct InlineIndicatorMetrics")
            && model.contains("pub struct InlineIndicatorAnchor")
            && model.contains("pub struct InlineIndicatorLayout"),
        "inline feedback public DTOs should live in inline/model.rs"
    );
    assert!(
        geometry.contains("pub fn inline_indicator_reserved_width")
            && geometry.contains("pub fn inline_indicator_layout")
            && geometry.contains("finite_nonnegative")
            && geometry.contains("finite_or")
            && !geometry.contains("fn finite_nonnegative")
            && !geometry.contains("pub struct InlineIndicatorLayout"),
        "inline feedback geometry should consume model DTOs and sanitizer helpers"
    );
    assert!(
        sanitize.contains("fn finite_nonnegative") && sanitize.contains("fn finite_or"),
        "inline feedback numeric sanitizers should live in inline/sanitize.rs"
    );
    assert!(
        tests.contains("fn inline_indicator_reserved_width_includes_text_gap_and_unit_gaps")
            && tests.contains("fn inline_indicator_layout_rejects_nonfinite_content_rect"),
        "inline feedback behavior tests should live in inline/tests.rs"
    );
    assert!(
        feedback.contains("InlineIndicatorMetrics") && feedback.contains("inline_indicator_layout"),
        "inline feedback public API should remain available through the feedback facade"
    );
}

#[test]
fn progress_feedback_keeps_overlay_state_and_track_geometry_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/gui/feedback/progress.rs"))
        .expect("progress feedback root should be readable");
    let overlay = fs::read_to_string(manifest_dir.join("src/gui/feedback/progress/overlay.rs"))
        .expect("progress overlay module should be readable");
    let overlay_tests =
        fs::read_to_string(manifest_dir.join("src/gui/feedback/progress/overlay/tests.rs"))
            .expect("progress overlay tests should be readable");
    let track = fs::read_to_string(manifest_dir.join("src/gui/feedback/progress/track.rs"))
        .expect("progress track module should be readable");
    let progress_track =
        fs::read_to_string(manifest_dir.join("src/gui/feedback/progress/track/progress.rs"))
            .expect("progress track geometry module should be readable");
    let progress_track_tests =
        fs::read_to_string(manifest_dir.join("src/gui/feedback/progress/track/progress/tests.rs"))
            .expect("progress track geometry tests should be readable");
    let meter_track =
        fs::read_to_string(manifest_dir.join("src/gui/feedback/progress/track/meter.rs"))
            .expect("progress meter geometry module should be readable");
    let meter_track_tests =
        fs::read_to_string(manifest_dir.join("src/gui/feedback/progress/track/meter/tests.rs"))
            .expect("progress meter geometry tests should be readable");
    let sanitize =
        fs::read_to_string(manifest_dir.join("src/gui/feedback/progress/track/sanitize.rs"))
            .expect("progress track sanitizer module should be readable");
    let feedback = fs::read_to_string(manifest_dir.join("src/gui/feedback.rs"))
        .expect("feedback module should be readable");

    for required in [
        "mod overlay;",
        "mod track;",
        "pub use overlay::ProgressOverlay;",
        "pub use track::{",
    ] {
        assert!(
            root.contains(required),
            "progress feedback root should delegate `{required}`"
        );
    }
    assert!(
        !root.contains("pub struct ProgressOverlay")
            && !root.contains("fn horizontal_progress_fill_rect"),
        "progress feedback root should re-export public primitives without owning implementation"
    );
    assert!(
        overlay.contains("pub struct ProgressOverlay")
            && overlay.contains("pub visible: bool")
            && overlay.contains("pub cancel_requested: bool")
            && overlay.contains("#[path = \"overlay/tests.rs\"]")
            && !overlay.contains("fn progress_overlay_defaults_to_hidden_and_empty"),
        "progress overlay state should live in progress/overlay.rs while behavior tests stay delegated"
    );
    assert!(
        overlay_tests.contains("fn progress_overlay_defaults_to_hidden_and_empty")
            && overlay_tests.contains("ProgressOverlay::default()"),
        "progress overlay behavior tests should live in progress/overlay/tests.rs"
    );
    assert!(
        track.contains("mod meter;")
            && track.contains("mod progress;")
            && track.contains("mod sanitize;")
            && track.contains(
                "pub use meter::{horizontal_discrete_meter_fill_rect, horizontal_meter_fill_rect};"
            )
            && track.contains("pub use progress::{")
            && !track.contains("pub fn horizontal_progress_fill_rect")
            && !track.contains("pub fn horizontal_meter_fill_rect"),
        "progress track root should re-export focused geometry modules without owning implementation"
    );
    assert!(
        progress_track.contains("pub fn horizontal_progress_fill_rect")
            && progress_track.contains("pub fn horizontal_progress_activity_rect")
            && progress_track.contains("pub fn horizontal_progress_track_rect")
            && progress_track.contains("#[path = \"progress/tests.rs\"]")
            && !progress_track.contains("fn horizontal_progress_fill_rect_clamps_to_track"),
        "progress track fill and activity geometry should live in progress/track/progress.rs while behavior tests stay delegated"
    );
    assert!(
        progress_track_tests.contains("fn horizontal_progress_fill_rect_clamps_to_track")
            && progress_track_tests
                .contains("fn horizontal_progress_track_rect_switches_between_activity_and_fill"),
        "progress track behavior tests should live in progress/track/progress/tests.rs"
    );
    assert!(
        meter_track.contains("pub fn horizontal_meter_fill_rect")
            && meter_track.contains("pub fn horizontal_discrete_meter_fill_rect")
            && meter_track.contains("#[path = \"meter/tests.rs\"]")
            && !meter_track
                .contains("fn horizontal_meter_fill_rect_clamps_level_and_minimum_width"),
        "progress meter geometry should live in progress/track/meter.rs while behavior tests stay delegated"
    );
    assert!(
        meter_track_tests.contains("fn horizontal_meter_fill_rect_clamps_level_and_minimum_width")
            && meter_track_tests
                .contains("fn horizontal_discrete_meter_fill_rect_rounds_and_clamps_byte_levels"),
        "progress meter behavior tests should live in progress/track/meter/tests.rs"
    );
    assert!(
        sanitize.contains("fn normalized_fraction") && sanitize.contains("fn finite_nonnegative"),
        "progress track geometry sanitizers should live in progress/track/sanitize.rs"
    );
    assert!(
        feedback.contains("ProgressOverlay")
            && feedback.contains("horizontal_progress_fill_rect")
            && feedback.contains("horizontal_meter_fill_rect"),
        "feedback facade should continue exporting progress overlay and track helpers"
    );
}

#[test]
fn window_specs_use_named_parts_for_manifest_identity_and_options() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let spec_path = manifest_dir.join("src/gui_runtime/window_manifest/spec.rs");
    let builder_path = manifest_dir.join("src/gui_runtime/window_manifest/spec/builders.rs");
    let spec = fs::read_to_string(&spec_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", spec_path.display()));
    let builders = fs::read_to_string(&builder_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", builder_path.display()));
    let manifest = fs::read_to_string(manifest_dir.join("src/gui_runtime/window_manifest.rs"))
        .expect("window manifest module should be readable");
    let runtime = fs::read_to_string(manifest_dir.join("src/runtime/mod.rs"))
        .expect("runtime module should be readable");
    let lib = fs::read_to_string(manifest_dir.join("src/lib.rs"))
        .expect("library module should be readable");

    assert!(
        spec.contains("pub struct WindowSpecParts")
            && builders.contains("pub fn from_parts(parts: WindowSpecParts) -> Self"),
        "window specs should expose named parts for stable key and native options"
    );
    assert!(
        builders.contains("Self::from_parts(WindowSpecParts {")
            && manifest.contains("WindowSpecParts")
            && runtime.contains("WindowSpecParts")
            && lib.contains("WindowSpecParts"),
        "window spec compatibility constructors and public exports should keep the named-parts path available"
    );
}

#[test]
fn status_segments_use_named_parts_for_chrome_slots() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = manifest_dir.join("src/gui/chrome.rs");
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));
    let tests = fs::read_to_string(manifest_dir.join("src/gui/chrome/tests.rs"))
        .expect("chrome behavior tests should be readable");
    let lib = fs::read_to_string(manifest_dir.join("src/lib.rs"))
        .expect("library module should be readable");

    assert!(
        source.contains("pub struct StatusSegmentsParts")
            && source.contains("pub fn from_parts(parts: StatusSegmentsParts) -> Self")
            && source.contains("#[path = \"chrome/tests.rs\"]")
            && !source.contains("fn status_segments_default_to_empty_text"),
        "status segments should expose named parts for left, center, and right chrome slots while delegating behavior tests"
    );
    assert!(
        tests.contains("fn status_segments_default_to_empty_text")
            && tests.contains("fn content_view_chrome_defaults_to_product_neutral_copy"),
        "chrome behavior coverage should live in gui/chrome/tests.rs"
    );
    assert!(
        source.contains("Self::from_parts(StatusSegmentsParts {")
            && lib.contains("StatusSegmentsParts"),
        "status segment compatibility constructor and prelude export should keep the named-parts path available"
    );
}

#[test]
fn image_rgba_buffer_keeps_diagnostics_and_tests_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source = fs::read_to_string(manifest_dir.join("src/gui/types/image.rs"))
        .expect("image buffer source should be readable");
    let tests = fs::read_to_string(manifest_dir.join("src/gui/types/image/tests.rs"))
        .expect("image buffer behavior tests should be readable");

    assert!(
        source.contains("pub struct ImageRgba")
            && source.contains("pub struct ImageRgbaError")
            && source.contains("pub fn try_new")
            && source.contains("#[path = \"image/tests.rs\"]")
            && !source.contains("fn image_rgba_try_new_reports_length_mismatch"),
        "RGBA image buffer and diagnostics should live in gui/types/image.rs while behavior tests stay delegated"
    );
    assert!(
        tests.contains("fn image_rgba_try_new_reports_length_mismatch")
            && tests.contains("fn image_rgba_try_new_reports_dimension_overflow"),
        "image buffer behavior coverage should live in gui/types/image/tests.rs"
    );
}

#[test]
fn timeline_visualization_state_uses_named_parts_for_large_projection_buckets() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let timeline_dir = manifest_dir.join("src/gui/visualization/timeline");
    let mut violations = Vec::new();

    for path in [
        timeline_dir.join("edit.rs"),
        timeline_dir.join("surface.rs"),
    ] {
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
        if source.contains("too_many_arguments") {
            violations.push(relative_path(&manifest_dir, &path));
        }
    }

    assert!(
        violations.is_empty(),
        "timeline visualization state should use named projection parts instead of suppressing long positional constructors:\n{}",
        violations.join("\n")
    );
}
