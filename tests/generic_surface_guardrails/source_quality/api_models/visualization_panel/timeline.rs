use super::*;

#[test]
fn timeline_metadata_state_uses_named_parts_for_projection_fields() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let timeline_dir = manifest_dir.join("src/gui/visualization/timeline");

    for (file, parts, from_parts, wrapper) in [
        (
            "transport.rs",
            "pub struct TimelineTransportParts",
            "pub fn from_parts(parts: TimelineTransportParts) -> Self",
            "Self::from_parts(TimelineTransportParts {",
        ),
        (
            "feedback.rs",
            "pub struct TimelineFeedbackParts",
            "pub fn from_parts(parts: TimelineFeedbackParts) -> Self",
            "Self::from_parts(TimelineFeedbackParts {",
        ),
        (
            "presentation.rs",
            "pub struct TimelinePresentationParts",
            "pub fn from_parts(parts: TimelinePresentationParts) -> Self",
            "Self::from_parts(TimelinePresentationParts {",
        ),
    ] {
        let source_path = timeline_dir.join(file);
        let source = fs::read_to_string(&source_path)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));

        assert!(
            source.contains(parts) && source.contains(from_parts),
            "timeline metadata in {file} should expose named parts for readable public construction"
        );
        assert!(
            source.contains(wrapper),
            "timeline metadata compatibility constructor in {file} should delegate through named parts"
        );
    }
}

#[test]
fn timeline_viewport_uses_named_parts_for_precision_bounds() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = manifest_dir.join("src/gui/visualization/timeline/viewport.rs");
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));

    assert!(
        source.contains("pub struct TimelineViewportParts")
            && source.contains("pub fn from_parts(parts: TimelineViewportParts) -> Self"),
        "timeline viewport should expose named parts for readable multi-precision bounds"
    );
    assert!(
        source.contains("Self::from_parts(TimelineViewportParts {")
            && source.contains("Self::from_parts(TimelineViewportParts::default())"),
        "timeline viewport compatibility/default constructors should delegate through named parts"
    );
}

#[test]
fn timeline_item_layout_uses_named_parts_for_clip_geometry() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = manifest_dir.join("src/gui/visualization/timeline/item.rs");
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));

    assert!(
        source.contains("pub struct TimelineItemLayoutParts")
            && source.contains("pub const fn from_parts(parts: TimelineItemLayoutParts) -> Self"),
        "timeline item layout should expose named parts for readable reusable item geometry"
    );
    assert!(
        source.contains("Self::from_parts(TimelineItemLayoutParts::new(")
            && source.contains("pub fn item_rect(self, lane: usize, start: f32, end: f32) -> Rect"),
        "timeline item layout compatibility constructors should delegate through named parts"
    );
}

#[test]
fn timeline_lane_layout_owns_label_gutter_projection() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = manifest_dir.join("src/gui/visualization/timeline/lanes.rs");
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));

    assert!(
        source.contains("pub struct TimelineLaneLayoutParts")
            && source.contains("pub const fn from_parts(parts: TimelineLaneLayoutParts) -> Self"),
        "timeline lane layout should expose named parts for reusable lane geometry"
    );
    assert!(
        source.contains("pub fn lane_label_rect(self, label_bounds: Rect, lane: usize) -> Rect"),
        "timeline lane layout should own aligned lane label gutter projection"
    );
}

#[test]
fn timeline_panel_layout_uses_named_parts_for_editor_chrome() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = manifest_dir.join("src/gui/visualization/timeline/panel.rs");
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));

    assert!(
        source.contains("pub struct TimelinePanelLayoutParts")
            && source.contains("pub fn from_parts(parts: TimelinePanelLayoutParts) -> Self"),
        "timeline panel layout should expose named parts for reusable editor chrome splits"
    );
    assert!(
        source.contains("pub header: Rect")
            && source.contains("pub ruler: Rect")
            && source.contains("pub lanes: Rect"),
        "timeline panel layout should own header, ruler, and lane rectangles"
    );
}

#[test]
fn timeline_pitch_layout_uses_named_parts_for_note_editor_rows() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = manifest_dir.join("src/gui/visualization/timeline/pitch.rs");
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));

    assert!(
        source.contains("pub struct TimelinePitchLayoutParts")
            && source.contains("pub const fn from_parts(parts: TimelinePitchLayoutParts) -> Self"),
        "timeline pitch layout should expose named parts for readable note-editor row geometry"
    );
    assert!(
        source.contains("pub fn pitch_rect(self, pitch: i32) -> Rect")
            && source.contains("pub fn pitch_at(self, position: Point) -> Option<i32>"),
        "timeline pitch layout should own pitch row projection and hit testing"
    );
    assert!(
        source.contains("pub struct TimelinePitchItemLayoutParts")
            && source
                .contains("pub const fn from_parts(parts: TimelinePitchItemLayoutParts) -> Self"),
        "timeline pitch item layout should expose named parts for readable note-item geometry"
    );
    assert!(
        source.contains("pub fn item_rect(self, pitch: i32, start: f32, end: f32) -> Rect")
            && source.contains(
                "pub fn item_rect_unclamped(self, pitch: i32, start: f32, end: f32) -> Rect"
            ),
        "timeline pitch item layout should own clamped and unclamped note-item projection"
    );
}

#[test]
fn timeline_value_marker_layout_uses_named_parts_for_automation_geometry() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = manifest_dir.join("src/gui/visualization/timeline/value_marker.rs");
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));

    assert!(
        source.contains("pub struct TimelineValueMarkerLayoutParts")
            && source
                .contains("pub const fn from_parts(parts: TimelineValueMarkerLayoutParts) -> Self"),
        "timeline value marker layout should expose named parts for readable automation geometry"
    );
    assert!(
        source.contains("pub fn marker(self, timeline_value: f32, value: f32)")
            && source.contains("pub fn marker_unclamped(self, timeline_value: f32, value: f32)"),
        "timeline value marker layout should own clamped and unclamped value-marker projection"
    );
}
