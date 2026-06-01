use crate::{
    application::{ViewNode, row, text},
    gui::chrome::StatusSegments,
    widgets::WidgetStyle,
};

/// Named construction fields for a compact application status bar.
pub struct StatusBarParts<Message> {
    /// Left, center, and right status labels supplied by the host app.
    pub segments: StatusSegments,
    /// Optional trailing control, progress bar, or compact action.
    pub trailing: Option<ViewNode<Message>>,
    /// Optional fixed width for the left segment. When omitted, left fills.
    pub left_width: Option<f32>,
    /// Optional fixed width for the center segment. When omitted, center fills.
    pub center_width: Option<f32>,
    /// Optional fixed width for the right segment. When omitted, right fills.
    pub right_width: Option<f32>,
    /// Total status-bar height.
    pub height: f32,
    /// Fixed height for text segments.
    pub segment_height: f32,
    /// Horizontal gap between segments.
    pub spacing: f32,
    /// Horizontal status-bar padding.
    pub padding_x: f32,
    /// Vertical status-bar padding.
    pub padding_y: f32,
    /// Semantic style applied to the bar background.
    pub style: WidgetStyle,
}

impl<Message> StatusBarParts<Message> {
    /// Build status-bar parts from generic left, center, and right labels.
    pub fn new(segments: StatusSegments) -> Self {
        Self {
            segments,
            trailing: None,
            left_width: None,
            center_width: None,
            right_width: None,
            height: 30.0,
            segment_height: 20.0,
            spacing: 8.0,
            padding_x: 12.0,
            padding_y: 4.0,
            style: WidgetStyle::default(),
        }
    }

    /// Add trailing content such as a progress bar or compact action.
    pub fn trailing(mut self, trailing: ViewNode<Message>) -> Self {
        self.trailing = Some(trailing);
        self
    }

    /// Set a fixed width for the left segment.
    pub fn left_width(mut self, width: f32) -> Self {
        self.left_width = Some(width);
        self
    }

    /// Set a fixed width for the center segment.
    pub fn center_width(mut self, width: f32) -> Self {
        self.center_width = Some(width);
        self
    }

    /// Set a fixed width for the right segment.
    pub fn right_width(mut self, width: f32) -> Self {
        self.right_width = Some(width);
        self
    }

    /// Set total status-bar height.
    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    /// Set fixed text-segment height.
    pub fn segment_height(mut self, height: f32) -> Self {
        self.segment_height = height;
        self
    }

    /// Set horizontal gap between segments.
    pub fn spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing;
        self
    }

    /// Set horizontal and vertical padding.
    pub fn padding(mut self, x: f32, y: f32) -> Self {
        self.padding_x = x;
        self.padding_y = y;
        self
    }

    /// Set the semantic bar style.
    pub fn style(mut self, style: WidgetStyle) -> Self {
        self.style = style;
        self
    }
}

/// Build a compact application status bar from generic status segments.
pub fn status_bar<Message: 'static>(segments: StatusSegments) -> ViewNode<Message> {
    status_bar_from_parts(StatusBarParts::new(segments))
}

/// Build a compact application status bar from named parts.
pub fn status_bar_from_parts<Message: 'static>(
    parts: StatusBarParts<Message>,
) -> ViewNode<Message> {
    let mut children = vec![
        status_segment(parts.segments.left, parts.left_width, parts.segment_height),
        status_segment(
            parts.segments.center,
            parts.center_width,
            parts.segment_height,
        ),
    ];
    if !parts.segments.right.is_empty() || parts.right_width.is_some() {
        children.push(status_segment(
            parts.segments.right,
            parts.right_width,
            parts.segment_height,
        ));
    }
    if let Some(trailing) = parts.trailing {
        children.push(trailing);
    }
    row(children)
        .style(parts.style)
        .spacing(parts.spacing)
        .padding_x(parts.padding_x)
        .padding_y(parts.padding_y)
        .fill_width()
        .height(parts.height)
}

fn status_segment<Message: 'static>(
    label: String,
    width: Option<f32>,
    height: f32,
) -> ViewNode<Message> {
    let segment = text(label).truncate().height(height);
    if let Some(width) = width {
        segment.width(width)
    } else {
        segment.fill_width()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        application::{IntoView, button},
        layout::{LayoutNode, SizeModeMain},
    };

    #[test]
    fn status_bar_projects_segments_and_trailing_content() {
        let bar = status_bar_from_parts(
            StatusBarParts::new(StatusSegments::new("2 samples", "Ready", "Idle"))
                .left_width(120.0)
                .right_width(76.0)
                .trailing(button("!").message(()).width(24.0).height(20.0)),
        );

        let layout = bar.into_surface().layout_node();
        let LayoutNode::Container(row) = layout else {
            panic!("status bar should lower to a row");
        };
        assert_eq!(row.children.len(), 4);
        assert!(matches!(
            row.children[0].slot.size_main,
            SizeModeMain::Fixed(width) if (width - 120.0).abs() < 0.01
        ));
        assert!(matches!(
            row.children[2].slot.size_main,
            SizeModeMain::Fixed(width) if (width - 76.0).abs() < 0.01
        ));
    }
}
