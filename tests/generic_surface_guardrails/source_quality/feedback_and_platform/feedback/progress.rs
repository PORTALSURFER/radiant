use super::*;

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
    let throttle = fs::read_to_string(manifest_dir.join("src/gui/feedback/progress/throttle.rs"))
        .expect("progress update gate module should be readable");
    let throttle_tests =
        fs::read_to_string(manifest_dir.join("src/gui/feedback/progress/throttle/tests.rs"))
            .expect("progress update gate tests should be readable");
    let track = fs::read_to_string(manifest_dir.join("src/gui/feedback/progress/track.rs"))
        .expect("progress track module should be readable");
    let progress_track =
        fs::read_to_string(manifest_dir.join("src/gui/feedback/progress/track/progress.rs"))
            .expect("progress track facade should be readable");
    let progress_track_scalar =
        fs::read_to_string(manifest_dir.join("src/gui/feedback/progress/track/progress/scalar.rs"))
            .expect("progress track scalar geometry module should be readable");
    let progress_track_range =
        fs::read_to_string(manifest_dir.join("src/gui/feedback/progress/track/progress/range.rs"))
            .expect("progress track range geometry module should be readable");
    let progress_track_cursor =
        fs::read_to_string(manifest_dir.join("src/gui/feedback/progress/track/progress/cursor.rs"))
            .expect("progress track cursor geometry module should be readable");
    let progress_track_paint =
        fs::read_to_string(manifest_dir.join("src/gui/feedback/progress/track/progress/paint.rs"))
            .expect("progress track paint adapter module should be readable");
    let progress_track_tests =
        fs::read_to_string(manifest_dir.join("src/gui/feedback/progress/track/progress/tests.rs"))
            .expect("progress track geometry tests should be readable");
    let progress_track_scalar_tests = fs::read_to_string(
        manifest_dir.join("src/gui/feedback/progress/track/progress/tests/scalar.rs"),
    )
    .expect("progress scalar geometry tests should be readable");
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
        "mod throttle;",
        "mod track;",
        "pub use overlay::{ProgressOverlay, ProgressSnapshot};",
        "pub use throttle::ProgressUpdateGate;",
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
            && overlay.contains("pub struct ProgressSnapshot")
            && overlay.contains("pub visible: bool")
            && overlay.contains("pub cancel_requested: bool")
            && overlay.contains("#[path = \"overlay/tests.rs\"]")
            && !overlay.contains("fn progress_overlay_defaults_to_hidden_and_empty"),
        "progress overlay state should live in progress/overlay.rs while behavior tests stay delegated"
    );
    assert!(
        overlay_tests.contains("fn progress_overlay_defaults_to_hidden_and_empty")
            && overlay_tests.contains("ProgressOverlay::default()")
            && overlay_tests.contains("fn progress_snapshot_reports_indeterminate_progress"),
        "progress overlay behavior tests should live in progress/overlay/tests.rs"
    );
    assert!(
        throttle.contains("pub struct ProgressUpdateGate")
            && throttle.contains("pub fn accept_at")
            && throttle.contains("#[path = \"throttle/tests.rs\"]")
            && !throttle.contains("fn progress_update_gate_coalesces_tight_fraction_updates"),
        "progress update throttling should live in progress/throttle.rs while behavior tests stay delegated"
    );
    assert!(
        throttle_tests.contains("fn progress_update_gate_coalesces_tight_fraction_updates")
            && throttle_tests
                .contains("fn progress_update_gate_accepts_terminal_fraction_immediately"),
        "progress update gate behavior tests should live in progress/throttle/tests.rs"
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
        progress_track.contains("mod cursor;")
            && progress_track.contains("mod paint;")
            && progress_track.contains("mod range;")
            && progress_track.contains("mod scalar;")
            && progress_track.contains("pub use cursor::horizontal_value_cursor_rect;")
            && progress_track.contains("pub use paint::{")
            && progress_track.contains("pub use range::{")
            && progress_track.contains("pub use scalar::{")
            && progress_track.contains("#[path = \"progress/tests.rs\"]")
            && !progress_track.contains("fn horizontal_progress_fill_rect_clamps_to_track"),
        "progress track facade should re-export focused geometry and paint modules while behavior tests stay delegated"
    );
    assert!(
        progress_track_scalar.contains("pub fn horizontal_progress_fill_rect")
            && progress_track_scalar.contains("pub fn horizontal_progress_activity_rect")
            && progress_track_scalar.contains("pub fn horizontal_progress_track_rect"),
        "scalar progress fill and activity geometry should live in progress/track/progress/scalar.rs"
    );
    assert!(
        progress_track_range.contains("pub fn horizontal_value_range_rect")
            && progress_track_range.contains("pub fn horizontal_value_range_edge_rects")
            && progress_track_range.contains("pub fn horizontal_wrapped_value_range_rects")
            && progress_track_range.contains("fn wrapped_fraction"),
        "value range and wraparound geometry should live in progress/track/progress/range.rs"
    );
    assert!(
        progress_track_cursor.contains("pub fn horizontal_value_cursor_rect"),
        "cursor geometry should live in progress/track/progress/cursor.rs"
    );
    assert!(
        progress_track_paint.contains("pub fn push_horizontal_value_range_fill")
            && progress_track_paint.contains("pub fn push_horizontal_value_cursor_fill")
            && progress_track_paint.contains("impl WidgetPaint<'_>"),
        "progress paint adapters should live in progress/track/progress/paint.rs"
    );
    assert!(
        progress_track_tests.contains("mod scalar;")
            && progress_track_scalar_tests
                .contains("fn horizontal_progress_fill_rect_clamps_to_track")
            && progress_track_scalar_tests
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
            && feedback.contains("ProgressUpdateGate")
            && feedback.contains("horizontal_progress_fill_rect")
            && feedback.contains("horizontal_meter_fill_rect"),
        "feedback facade should continue exporting progress overlay and track helpers"
    );
}
