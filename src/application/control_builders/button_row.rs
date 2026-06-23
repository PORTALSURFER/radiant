use crate::application::{ViewNode, row};

/// Default fixed height for compact horizontal button rows.
pub const DEFAULT_BUTTON_ROW_HEIGHT: f32 = 26.0;
/// Default fixed height applied to each button inside a compact button row.
pub const DEFAULT_BUTTON_ROW_BUTTON_HEIGHT: f32 = 24.0;
/// Default horizontal spacing between compact button-row items.
pub const DEFAULT_BUTTON_ROW_SPACING: f32 = 6.0;

/// Named construction fields for compact horizontal button groups.
///
/// Button rows are useful in dialogs, popovers, inspectors, and utility panels
/// where application code owns each button but Radiant should own consistent
/// horizontal group spacing and height.
pub struct ButtonRowParts<Message> {
    /// Buttons or button-like controls shown in declaration order.
    pub buttons: Vec<ViewNode<Message>>,
    /// Fixed row height.
    pub height: f32,
    /// Fixed height applied to each child button.
    pub button_height: f32,
    /// Horizontal spacing between buttons.
    pub spacing: f32,
}

impl<Message> ButtonRowParts<Message> {
    /// Build button-row parts with Radiant's compact dialog/control defaults.
    pub fn new(buttons: impl IntoIterator<Item = ViewNode<Message>>) -> Self {
        Self {
            buttons: buttons.into_iter().collect(),
            height: DEFAULT_BUTTON_ROW_HEIGHT,
            button_height: DEFAULT_BUTTON_ROW_BUTTON_HEIGHT,
            spacing: DEFAULT_BUTTON_ROW_SPACING,
        }
    }

    /// Override fixed row height.
    pub const fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    /// Override fixed child button height.
    pub const fn button_height(mut self, height: f32) -> Self {
        self.button_height = height;
        self
    }

    /// Override horizontal button spacing.
    pub const fn spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing;
        self
    }
}

/// Build a compact horizontal button group with Radiant's standard spacing.
pub fn button_row<Message: 'static>(
    buttons: impl IntoIterator<Item = ViewNode<Message>>,
) -> ViewNode<Message> {
    button_row_from_parts(ButtonRowParts::new(buttons))
}

/// Build a compact horizontal button group from named parts.
pub fn button_row_from_parts<Message: 'static>(
    parts: ButtonRowParts<Message>,
) -> ViewNode<Message> {
    let vertical_padding = ((parts.height - parts.button_height) * 0.5).max(0.0);
    row(parts
        .buttons
        .into_iter()
        .map(|button| button.height(parts.button_height)))
    .spacing(parts.spacing)
    .padding_y(vertical_padding)
    .fill_width()
    .height(parts.height)
}

#[cfg(test)]
mod tests {
    use super::{
        ButtonRowParts, DEFAULT_BUTTON_ROW_BUTTON_HEIGHT, DEFAULT_BUTTON_ROW_HEIGHT,
        DEFAULT_BUTTON_ROW_SPACING, button_row_from_parts,
    };
    use crate::{
        application::{IntoView, button, column},
        layout::Vector2,
        widgets::{ButtonMessage, WidgetOutput},
    };

    #[derive(Clone, Debug, PartialEq)]
    enum Message {
        Confirm,
        Cancel,
    }

    #[test]
    fn button_row_applies_compact_default_geometry() {
        let layout = column([button_row_from_parts(ButtonRowParts::new([
            button("Confirm")
                .message(Message::Confirm)
                .id(10)
                .width(72.0),
            button("Cancel").message(Message::Cancel).id(11).width(64.0),
        ]))
        .id(1)])
        .view_layout_at_size(Vector2::new(180.0, 40.0));

        let confirm = layout.rects.get(&10).expect("confirm button rect");
        let cancel = layout.rects.get(&11).expect("cancel button rect");

        assert_eq!(confirm.height(), DEFAULT_BUTTON_ROW_BUTTON_HEIGHT);
        assert_eq!(cancel.height(), DEFAULT_BUTTON_ROW_BUTTON_HEIGHT);
        assert_eq!(layout.rects[&1].height(), DEFAULT_BUTTON_ROW_HEIGHT);
        assert!((cancel.min.x - confirm.max.x - DEFAULT_BUTTON_ROW_SPACING).abs() < 0.01);
    }

    #[test]
    fn button_row_routes_child_button_messages() {
        let surface = button_row_from_parts(ButtonRowParts::new([
            button("Confirm")
                .message(Message::Confirm)
                .id(10)
                .width(72.0),
            button("Cancel").message(Message::Cancel).id(11).width(64.0),
        ]))
        .into_surface();

        assert_eq!(
            surface.dispatch_widget_output(10, WidgetOutput::typed(ButtonMessage::Activate)),
            Some(Message::Confirm)
        );
        assert_eq!(
            surface.dispatch_widget_output(11, WidgetOutput::typed(ButtonMessage::Activate)),
            Some(Message::Cancel)
        );
    }

    #[test]
    fn button_row_accepts_custom_metrics() {
        let layout = column([button_row_from_parts(
            ButtonRowParts::new([
                button("A").message(Message::Confirm).id(10).width(32.0),
                button("B").message(Message::Cancel).id(11).width(32.0),
            ])
            .height(30.0)
            .button_height(22.0)
            .spacing(3.0),
        )
        .id(1)])
        .view_layout_at_size(Vector2::new(120.0, 40.0));

        let first = layout.rects.get(&10).expect("first button rect");
        let second = layout.rects.get(&11).expect("second button rect");

        assert_eq!(layout.rects[&1].height(), 30.0);
        assert_eq!(first.height(), 22.0);
        assert!((second.min.x - first.max.x - 3.0).abs() < 0.01);
    }
}
