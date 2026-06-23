use crate::application::{ViewNode, ViewNodeKind, primary_style, row, view_node_from_widget};
use crate::gui::types::{Point, Rect, Rgba8};
use crate::layout::{LayoutOutput, Vector2};
use crate::runtime::{PaintPrimitive, push_visible_fill_rect};
use crate::theme::ThemeTokens;
use crate::widgets::{Widget, WidgetCommon, WidgetInput, WidgetOutput, WidgetSizing};

/// Build a floating drop marker in surface coordinates.
pub fn drop_marker<Message>(x: f32, y: f32, width: f32, height: f32) -> ViewNode<Message> {
    ViewNode::new(ViewNodeKind::OverlayPanel {
        rect: Rect::from_min_size(Point::new(x, y), Vector2::new(width, height)),
        label: None,
    })
    .style(primary_style())
}

/// Build a non-interactive insertion marker positioned in local layout coordinates.
///
/// This is useful inside stack layers where the marker should align to sibling
/// content such as list rows, table headers, or local drop targets rather than
/// using surface coordinates.
pub fn local_drop_marker<Message: 'static>(
    x: f32,
    color: Rgba8,
    width: f32,
    height: f32,
) -> ViewNode<Message> {
    row([
        view_node_from_widget(LocalDropMarkerWidget::new(x, color, width, height))
            .fill_width()
            .height(finite_nonnegative(height).max(1.0)),
    ])
    .spacing(0.0)
}

#[derive(Clone, Debug, PartialEq)]
struct LocalDropMarkerWidget {
    common: WidgetCommon,
    x: f32,
    color: Rgba8,
    width: f32,
    height: f32,
}

impl LocalDropMarkerWidget {
    fn new(x: f32, color: Rgba8, width: f32, height: f32) -> Self {
        let x = finite_nonnegative(x);
        let width = finite_nonnegative(width).max(1.0);
        let height = finite_nonnegative(height).max(1.0);
        Self {
            common: WidgetCommon::new(0, WidgetSizing::fixed(Vector2::new(x + width, height))),
            x,
            color,
            width,
            height,
        }
    }
}

impl Widget for LocalDropMarkerWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, _bounds: Rect, _input: WidgetInput) -> Option<WidgetOutput> {
        None
    }

    fn needs_state_synchronization(&self) -> bool {
        false
    }

    fn accepts_pointer_move(&self) -> bool {
        false
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        _theme: &ThemeTokens,
    ) {
        if !bounds.has_finite_positive_area() {
            return;
        }
        let width = self.width.min(bounds.width()).max(0.0);
        let height = self.height.min(bounds.height()).max(0.0);
        if width <= 0.0 || height <= 0.0 {
            return;
        }
        let max_x = (bounds.width() - width).max(0.0);
        let x = self.x.min(max_x);
        let rect = Rect::from_min_size(
            Point::new(bounds.min.x + x, bounds.min.y),
            Vector2::new(width, height),
        );
        push_visible_fill_rect(primitives, self.common.id, rect, self.color);
    }
}

fn finite_nonnegative(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}
