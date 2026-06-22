pub(super) const DEFAULT_PANEL_SECTION_PADDING: f32 = 6.0;
pub(super) const DEFAULT_PANEL_SECTION_SPACING: f32 = 4.0;
pub(super) const DEFAULT_PANEL_SECTION_TITLE_HEIGHT: f32 = 20.0;

/// Reusable geometry for compact titled panel sections.
///
/// This keeps panel chrome math reusable for app-owned resize constraints,
/// popover anchors, and fixed-content sizing without requiring callers to build
/// spacer rows or duplicate padding/title/spacing arithmetic.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PanelSectionGeometry {
    /// Inner container padding.
    pub padding: f32,
    /// Vertical spacing between the header and content.
    pub spacing: f32,
    /// Fixed title/header row height.
    pub title_height: f32,
}

impl PanelSectionGeometry {
    /// Build panel-section geometry with Radiant's compact defaults.
    pub const fn new() -> Self {
        Self {
            padding: DEFAULT_PANEL_SECTION_PADDING,
            spacing: DEFAULT_PANEL_SECTION_SPACING,
            title_height: DEFAULT_PANEL_SECTION_TITLE_HEIGHT,
        }
    }

    /// Override inner container padding.
    pub const fn padding(mut self, padding: f32) -> Self {
        self.padding = padding;
        self
    }

    /// Override vertical spacing between the header and content.
    pub const fn spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing;
        self
    }

    /// Override fixed title/header row height.
    pub const fn title_height(mut self, height: f32) -> Self {
        self.title_height = height;
        self
    }

    /// Return the section height needed when only the header row is visible.
    pub const fn header_only_height(self) -> f32 {
        sanitized_panel_metric(self.padding) * 2.0 + sanitized_panel_metric(self.title_height)
    }

    /// Return the vertical offset from the panel's top edge to the content area.
    pub const fn content_top_offset(self) -> f32 {
        sanitized_panel_metric(self.padding)
            + sanitized_panel_metric(self.title_height)
            + sanitized_panel_metric(self.spacing)
    }

    /// Return the vertical inset from the panel's bottom edge to the content top.
    pub const fn content_top_inset_from_bottom(self, panel_height: f32) -> f32 {
        let inset = sanitized_panel_metric(panel_height) - self.content_top_offset();
        if inset > 0.0 { inset } else { 0.0 }
    }

    /// Return the vertical inset from the panel's bottom edge to the content bottom.
    pub const fn content_bottom_inset(self) -> f32 {
        sanitized_panel_metric(self.padding)
    }

    /// Return the total section height needed for a fixed content height.
    pub const fn section_height_for_content_height(self, content_height: f32) -> f32 {
        self.content_top_offset()
            + sanitized_panel_metric(content_height)
            + self.content_bottom_inset()
    }

    /// Return the content height available inside a fixed section height.
    pub const fn content_height_for_section_height(self, section_height: f32) -> f32 {
        let height = sanitized_panel_metric(section_height)
            - self.content_top_offset()
            - self.content_bottom_inset();
        if height > 0.0 { height } else { 0.0 }
    }
}

impl Default for PanelSectionGeometry {
    fn default() -> Self {
        Self::new()
    }
}

const fn sanitized_panel_metric(value: f32) -> f32 {
    if value.is_finite() {
        if value > 0.0 { value } else { 0.0 }
    } else {
        0.0
    }
}
