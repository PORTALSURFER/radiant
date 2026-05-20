use super::*;

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
