use super::*;

#[derive(Clone, Debug)]
struct RowHost {
    row: InteractiveRowWidget,
}

impl EmbeddedInteractiveRowWidget for RowHost {
    type Message = InteractiveRowMessage;

    fn interactive_row(&self) -> &InteractiveRowWidget {
        &self.row
    }

    fn interactive_row_mut(&mut self) -> &mut InteractiveRowWidget {
        &mut self.row
    }

    fn map_interactive_row_message(&self, message: InteractiveRowMessage) -> Option<Self::Message> {
        Some(message)
    }

    fn append_interactive_row_paint(
        &self,
        _primitives: &mut Vec<PaintPrimitive>,
        _bounds: Rect,
        _layout: &LayoutOutput,
        _theme: &ThemeTokens,
    ) {
    }
}

#[derive(Clone, Debug)]
struct ActionRowHost {
    row: InteractiveRowWidget,
    actions: InteractiveRowActions<&'static str>,
}

impl EmbeddedInteractiveRowWidget for ActionRowHost {
    type Message = &'static str;

    fn interactive_row(&self) -> &InteractiveRowWidget {
        &self.row
    }

    fn interactive_row_mut(&mut self) -> &mut InteractiveRowWidget {
        &mut self.row
    }

    fn interactive_row_actions(&self) -> Option<&InteractiveRowActions<Self::Message>> {
        Some(&self.actions)
    }

    fn append_interactive_row_paint(
        &self,
        _primitives: &mut Vec<PaintPrimitive>,
        _bounds: Rect,
        _layout: &LayoutOutput,
        _theme: &ThemeTokens,
    ) {
    }
}

#[test]
fn synchronize_from_previous_embedded_preserves_custom_row_state() {
    let bounds = Rect::from_size(120.0, 22.0);
    let pointer = Point::new(4.0, 4.0);
    let mut previous = RowHost {
        row: InteractiveRowWidget::new(11, WidgetSizing::fixed(Vector2::new(120.0, 22.0))),
    };
    let _ = previous
        .row
        .handle_input(bounds, WidgetInput::PointerMove { position: pointer });

    let mut next = RowHost {
        row: InteractiveRowWidget::new(11, WidgetSizing::fixed(Vector2::new(120.0, 22.0))),
    };

    assert!(
        next.row
            .synchronize_from_previous_embedded::<RowHost>(&previous, |previous| &previous.row)
    );
    assert!(next.row.common.state.hovered);
}

#[test]
fn embedded_interactive_row_widget_routes_widget_contract() {
    let bounds = Rect::from_size(120.0, 22.0);
    let pointer = Point::new(4.0, 4.0);
    let mut host = RowHost {
        row: InteractiveRowWidget::new(12, WidgetSizing::fixed(Vector2::new(120.0, 22.0))),
    };

    assert_eq!(Widget::common(&host).id, 12);
    assert!(!host.accepts_pointer_move());

    let _ = host.handle_input(bounds, WidgetInput::primary_press(pointer));
    assert!(host.accepts_pointer_move());
    let output = host
        .handle_input(bounds, WidgetInput::primary_release(pointer))
        .expect("embedded row host should emit mapped row output");
    assert!(output.typed_ref::<InteractiveRowMessage>().is_some());
    assert!(!host.accepts_pointer_move());
}

#[test]
fn embedded_interactive_row_widget_routes_configured_actions_by_default() {
    let bounds = Rect::from_size(120.0, 22.0);
    let pointer = Point::new(4.0, 4.0);
    let mut host = ActionRowHost {
        row: InteractiveRowWidget::new(12, WidgetSizing::fixed(Vector2::new(120.0, 22.0))),
        actions: InteractiveRowActions::new().primary(|| "activated"),
    };

    let _ = host.handle_input(bounds, WidgetInput::primary_press(pointer));
    let output = host
        .handle_input(bounds, WidgetInput::primary_release(pointer))
        .expect("embedded row host should emit configured action output");

    assert_eq!(output.typed_ref::<&'static str>(), Some(&"activated"));
}
