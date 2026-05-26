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
