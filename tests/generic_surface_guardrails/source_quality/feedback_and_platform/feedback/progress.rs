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
