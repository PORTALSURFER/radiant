use super::core::view_node_from_widget;
use crate::{
    application::{ViewNode, ViewNodeKind},
    runtime::PaintText,
    widgets::{BadgeWidget, ButtonWidget, CardWidget, TextInputWidget, TextWidget, ToggleWidget},
};

use super::super::defaults::{
    default_badge_sizing, default_button_sizing, default_card_sizing, default_text_input_sizing,
    default_text_sizing, default_toggle_sizing,
};

/// Build a non-interactive text view with generated identity and default sizing.
pub fn text<Message: 'static>(value: impl Into<String>) -> ViewNode<Message> {
    view_node_from_widget(TextWidget::new(
        0,
        PaintText::from(value.into()),
        default_text_sizing(),
    ))
}

/// Build a fixed-height single-line text view that fills available width.
pub fn text_line<Message: 'static>(value: impl Into<String>, height: f32) -> ViewNode<Message> {
    text(value).fill_width().height(height).truncate()
}

/// Build a passive button view for retained surfaces that need button chrome
/// without host messages.
pub fn passive_button<Message: 'static>(label: impl Into<String>) -> ViewNode<Message> {
    view_node_from_widget(ButtonWidget::new(
        0,
        PaintText::from(label.into()),
        default_button_sizing(""),
    ))
}

/// Build a passive badge view for retained surfaces that need badge chrome
/// without host messages.
pub fn passive_badge<Message: 'static>(label: impl Into<String>) -> ViewNode<Message> {
    let label = PaintText::from(label.into());
    let sizing = default_badge_sizing(&label);
    view_node_from_widget(BadgeWidget::new(0, label, sizing))
}

/// Build a passive toggle view for retained surfaces that need toggle chrome
/// without host messages.
pub fn passive_toggle<Message: 'static>(
    label: impl Into<String>,
    checked: bool,
) -> ViewNode<Message> {
    view_node_from_widget(
        ToggleWidget::new(
            0,
            PaintText::from(label.into()),
            default_toggle_sizing("", true),
        )
        .with_checked(checked),
    )
}

/// Build a passive single-line text input view for retained surfaces that need
/// input chrome without host messages.
pub fn passive_text_input<Message: 'static>(
    value: impl Into<String>,
    placeholder: impl Into<String>,
) -> ViewNode<Message> {
    let mut input = TextInputWidget::new(0, value, default_text_input_sizing());
    let placeholder = placeholder.into();
    if !placeholder.is_empty() {
        input.props.placeholder = Some(placeholder.into());
    }
    view_node_from_widget(input)
}

/// Build a passive card or panel view.
pub fn card<Message: 'static>() -> ViewNode<Message> {
    view_node_from_widget(CardWidget::new(0, default_card_sizing()))
}

/// Build a passive view with no visible extent.
///
/// Use this for optional branches that need to return a view but should not
/// contribute layout size. Use [`spacer`] when the layout needs a visible
/// flexible or fixed gap.
pub fn empty<Message: 'static>() -> ViewNode<Message> {
    super::media::canvas().size(0.0, 0.0)
}

/// Build a minimal passive spacer view.
pub fn spacer<Message: 'static>() -> ViewNode<Message> {
    ViewNode::new(ViewNodeKind::Row {
        spacing: 0.0,
        children: Vec::new(),
    })
    .size(1.0, 1.0)
}

#[cfg(test)]
mod tests {
    use crate::{
        application::{IntoView, empty, spacer},
        gui::types::{Point, Rect},
        layout::{LayoutNode, Vector2},
        runtime::UiSurface,
    };

    #[test]
    fn empty_lowers_to_zero_extent_widget() {
        let layout = empty::<()>().into_surface().layout_node();

        let LayoutNode::Widget(widget) = layout else {
            panic!("empty should lower to a widget leaf");
        };
        assert_eq!(widget.intrinsic, Vector2::new(0.0, 0.0));
    }

    #[test]
    fn spacer_lowers_to_non_painting_container() {
        let layout = spacer::<()>().into_surface().layout_node();

        let LayoutNode::Container(container) = layout else {
            panic!("spacer should lower to a non-painting layout container");
        };
        assert!(container.children.is_empty());
    }

    #[test]
    fn spacer_reserves_space_without_painting() {
        let view = spacer::<()>().fill_width().height(20.0);
        let frame = UiSurface::new(view.into_node()).frame(
            Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(240.0, 20.0)),
            &Default::default(),
        );

        assert!(frame.paint_plan.primitives.is_empty());
    }
}
