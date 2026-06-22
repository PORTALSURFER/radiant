use crate::gui::{
    list::{DenseRowOutlineStyle, DenseRowPalette, TreeGuideMetrics, TreeGuideStyle},
    types::Rgba8,
};

pub(super) const DEFAULT_TREE_ROW_HEIGHT: f32 = 22.0;
pub(super) const DEFAULT_TREE_EXPANDER_WIDTH: f32 = 28.0;
const DEFAULT_TREE_DEPTH_INDENT: f32 = 12.0;
const DEFAULT_TREE_GUIDE_COLOR: Rgba8 = Rgba8 {
    r: 116,
    g: 130,
    b: 148,
    a: 128,
};
const DEFAULT_SELECTED_FILL: Rgba8 = Rgba8 {
    r: 95,
    g: 130,
    b: 170,
    a: 96,
};
const DEFAULT_INTERACTION_FILL: Rgba8 = Rgba8 {
    r: 110,
    g: 138,
    b: 170,
    a: 78,
};
const DEFAULT_ACTIVE_TARGET_FILL: Rgba8 = Rgba8 {
    r: 100,
    g: 150,
    b: 190,
    a: 190,
};
const DEFAULT_CANDIDATE_HOVER_FILL: Rgba8 = Rgba8 {
    r: 110,
    g: 150,
    b: 190,
    a: 130,
};
const DEFAULT_OUTLINE_COLOR: Rgba8 = Rgba8 {
    r: 150,
    g: 185,
    b: 220,
    a: 220,
};
pub(super) const DEFAULT_HIGHLIGHTED_LABEL_COLOR: Rgba8 = Rgba8 {
    r: 245,
    g: 248,
    b: 252,
    a: 255,
};

pub(super) fn default_guide_style() -> TreeGuideStyle {
    TreeGuideStyle::new(
        DEFAULT_TREE_DEPTH_INDENT,
        DEFAULT_TREE_ROW_HEIGHT,
        DEFAULT_TREE_GUIDE_COLOR,
    )
}

pub(super) fn default_guide_metrics() -> TreeGuideMetrics {
    default_guide_style().metrics()
}

pub(super) fn default_palette() -> DenseRowPalette {
    DenseRowPalette::new()
        .selected(DEFAULT_SELECTED_FILL)
        .interaction_fills(DEFAULT_INTERACTION_FILL, DEFAULT_INTERACTION_FILL)
        .active_target(DEFAULT_ACTIVE_TARGET_FILL)
        .candidate_hovered(DEFAULT_CANDIDATE_HOVER_FILL)
}

pub(super) fn default_drop_target_outline() -> DenseRowOutlineStyle {
    DenseRowOutlineStyle::new(0.5, DEFAULT_OUTLINE_COLOR, 1.5)
}
