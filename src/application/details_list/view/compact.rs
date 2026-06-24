use crate::{
    application::{LayerHorizontalAnchor, LayerVerticalAnchor, View, anchored_layer, row},
    layout::Vector2,
};

/// Fluent builder for a compact details-list cell with a fixed-size anchored child.
pub struct CompactDetailsAnchoredCellBuilder<Message> {
    parts: CompactDetailsAnchoredCellParts<Message>,
}

/// Named inputs for a compact details-list cell with a fixed-size anchored child.
pub struct CompactDetailsAnchoredCellParts<Message> {
    child: View<Message>,
    width: Option<f32>,
    size: Vector2,
    horizontal: LayerHorizontalAnchor,
    vertical: LayerVerticalAnchor,
    inset_x: f32,
    inset_y: f32,
}

impl<Message> CompactDetailsAnchoredCellBuilder<Message> {
    /// Use a fixed cell width.
    pub fn width(mut self, width: f32) -> Self {
        self.parts.width = Some(width);
        self
    }

    /// Fill the remaining details-row width.
    pub fn fill_width(mut self) -> Self {
        self.parts.width = None;
        self
    }

    /// Place the fixed-size child along the compact cell's horizontal axis.
    pub fn horizontal(mut self, horizontal: LayerHorizontalAnchor) -> Self {
        self.parts.horizontal = horizontal;
        self
    }

    /// Place the fixed-size child along the compact cell's vertical axis.
    pub fn vertical(mut self, vertical: LayerVerticalAnchor) -> Self {
        self.parts.vertical = vertical;
        self
    }

    /// Offset the child from its anchor by the given x/y inset.
    pub fn inset(mut self, x: f32, y: f32) -> Self {
        self.parts.inset_x = x;
        self.parts.inset_y = y;
        self
    }

    /// Build this anchored compact details-cell view.
    pub fn view(self) -> View<Message>
    where
        Message: 'static,
    {
        compact_details_anchored_cell_from_parts(self.parts)
    }
}

impl<Message> CompactDetailsAnchoredCellParts<Message> {
    /// Create compact anchored-cell inputs with centered placement and no inset.
    pub fn new(child: View<Message>, size: Vector2) -> Self {
        Self {
            child,
            width: None,
            size,
            horizontal: LayerHorizontalAnchor::Center,
            vertical: LayerVerticalAnchor::Center,
            inset_x: 0.0,
            inset_y: 0.0,
        }
    }

    /// Use a fixed width, or fill the remaining details-row width when `None`.
    pub fn width(mut self, width: Option<f32>) -> Self {
        self.width = width;
        self
    }

    /// Place the fixed-size child along the compact cell's horizontal axis.
    pub fn horizontal(mut self, horizontal: LayerHorizontalAnchor) -> Self {
        self.horizontal = horizontal;
        self
    }

    /// Place the fixed-size child along the compact cell's vertical axis.
    pub fn vertical(mut self, vertical: LayerVerticalAnchor) -> Self {
        self.vertical = vertical;
        self
    }

    /// Offset the child from its anchor by the given x/y inset.
    pub fn inset(mut self, x: f32, y: f32) -> Self {
        self.inset_x = x;
        self.inset_y = y;
        self
    }
}

/// Build a compact horizontal details-row layout.
///
/// This is the same dense row frame used by Radiant's built-in details list:
/// fixed 22px row height, small vertical padding, left/right chrome, and
/// compact cell spacing. Host apps can reuse it when they need custom row
/// content but still want details-list density and alignment.
pub fn compact_details_row<Message>(
    children: impl IntoIterator<Item = View<Message>>,
) -> View<Message> {
    row(children)
        .fill_width()
        .height(22.0)
        .padding_x(8.0)
        .padding_y(1.0)
        .spacing(10.0)
}

/// Size one compact details-list cell.
///
/// This matches the cell sizing used by Radiant's built-in details lists:
/// fixed-width columns get a 20px-tall fixed cell, while flexible columns fill
/// the remaining row width at the same height. Host apps can use it for custom
/// cell content without repeating details-list sizing policy.
pub fn compact_details_cell<Message>(cell: View<Message>, width: Option<f32>) -> View<Message> {
    match width {
        Some(width) => cell.width(width).height(20.0),
        None => cell.fill_width().height(20.0),
    }
}

/// Build a compact details-list cell with a fixed-size anchored child.
///
/// The returned builder keeps the normal path fluent for app code while
/// preserving [`CompactDetailsAnchoredCellParts`] for advanced named-field
/// construction.
pub fn compact_details_anchored_cell<Message>(
    child: View<Message>,
    size: Vector2,
) -> CompactDetailsAnchoredCellBuilder<Message> {
    CompactDetailsAnchoredCellBuilder {
        parts: CompactDetailsAnchoredCellParts::new(child, size),
    }
}

/// Build a compact details-list cell with a fixed-size anchored child.
///
/// This preserves the standard compact cell sizing policy while letting hosts
/// place badges, status markers, compact actions, or other fixed-size content
/// inside the cell without rebuilding the full-size anchored layer and then
/// applying details-cell sizing separately.
pub fn compact_details_anchored_cell_from_parts<Message>(
    parts: CompactDetailsAnchoredCellParts<Message>,
) -> View<Message>
where
    Message: 'static,
{
    let CompactDetailsAnchoredCellParts {
        child,
        width,
        size,
        horizontal,
        vertical,
        inset_x,
        inset_y,
    } = parts;
    compact_details_cell(
        anchored_layer(child, size, horizontal, vertical, inset_x, inset_y),
        width,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        application::{IntoView, text},
        widgets::{WidgetStyle, WidgetTone},
    };

    #[test]
    fn compact_details_anchored_cell_preserves_cell_size_and_places_child() {
        let frame = compact_details_anchored_cell::<()>(
            text("K").style(WidgetStyle::subtle(WidgetTone::Warning)),
            Vector2::new(24.0, 14.0),
        )
        .width(64.0)
        .horizontal(LayerHorizontalAnchor::End)
        .vertical(LayerVerticalAnchor::Start)
        .inset(2.0, 3.0)
        .view()
        .view_frame_at_size_with_default_theme(Vector2::new(64.0, 20.0));

        let text_rect = frame
            .paint_plan
            .first_text_run("K")
            .expect("anchored child text should paint")
            .rect;

        assert!(text_rect.min.x >= 38.0, "{text_rect:?}");
        assert!(text_rect.min.y >= 3.0, "{text_rect:?}");
        assert!(text_rect.max.x <= 64.0, "{text_rect:?}");
        assert!(text_rect.max.y <= 20.0, "{text_rect:?}");
    }
}
