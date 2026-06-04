use super::*;
use crate::{
    gui::types::{Point, Rgba8, Vector2},
    layout::LayoutOutput,
    runtime::PaintPrimitive,
    theme::ThemeTokens,
    widgets::{PointerButton, PointerModifiers, WidgetInput},
};

#[test]
fn dense_visual_state_merges_host_and_interaction_state() {
    let mut row = InteractiveRowWidget::new(7, WidgetSizing::fixed(Vector2::new(120.0, 22.0)));
    row.common.state.hovered = true;
    row.common.state.pressed = true;

    assert_eq!(
        row.dense_visual_state(InteractiveRowVisualStateParts {
            selected: true,
            active_target: false,
            candidate: true,
        }),
        DenseRowVisualState {
            selected: true,
            hovered: true,
            pressed: true,
            active_target: false,
            candidate: true,
        }
    );
}

#[test]
fn dense_visual_state_preserves_default_host_state() {
    let row = InteractiveRowWidget::new(7, WidgetSizing::fixed(Vector2::new(120.0, 22.0)));

    assert_eq!(
        row.dense_visual_state(InteractiveRowVisualStateParts::default()),
        DenseRowVisualState::default()
    );
}

#[test]
fn push_dense_fill_uses_row_state_and_identity() {
    let mut row = InteractiveRowWidget::new(7, WidgetSizing::fixed(Vector2::new(120.0, 22.0)));
    row.common.state.hovered = true;
    let bounds = Rect::from_size(120.0, 22.0);
    let color = Rgba8::new(8, 9, 10, 180);
    let mut primitives = Vec::new();

    assert!(
        row.push_dense_fill(
            &mut primitives,
            bounds,
            InteractiveRowVisualStateParts {
                selected: true,
                ..InteractiveRowVisualStateParts::default()
            },
            DenseRowPalette::new()
                .selected(Rgba8::new(1, 2, 3, 120))
                .hovered(color),
        )
    );

    assert!(matches!(
        primitives.as_slice(),
        [PaintPrimitive::FillRect(fill)]
            if fill.widget_id == row.id() && fill.rect == bounds && fill.color == color
    ));
}

#[test]
fn paint_plan_with_defaults_exposes_query_helpers_for_one_widget() {
    let mut row = InteractiveRowWidget::new(7, WidgetSizing::fixed(Vector2::new(120.0, 22.0)));
    row.common.state.hovered = true;

    let plan = row.paint_plan_with_defaults(Rect::from_size(120.0, 22.0));

    assert_eq!(
        plan.fill_rects().next().map(|fill| fill.widget_id),
        Some(row.id())
    );
}

#[test]
fn accessors_expose_identity_and_common_contract_for_custom_row_wrappers() {
    let mut row = InteractiveRowWidget::new(13, WidgetSizing::fixed(Vector2::new(120.0, 22.0)));

    assert_eq!(row.id(), 13);
    assert_eq!(row.common().id, 13);

    row.common_mut().state.hovered = true;
    assert!(row.common().state.hovered);
}

#[test]
fn drop_target_mode_configures_hover_and_drop_only_states() {
    let inactive = InteractiveRowWidget::new(7, WidgetSizing::fixed(Vector2::new(120.0, 22.0)))
        .with_drop_target(true)
        .with_drop_target_mode(false, true);
    assert!(!inactive.props.droppable);
    assert!(!inactive.props.drag_active);
    assert!(!inactive.props.drop_hover);

    let hover = InteractiveRowWidget::new(8, WidgetSizing::fixed(Vector2::new(120.0, 22.0)))
        .with_drop_target_mode(true, true);
    assert!(hover.props.droppable);
    assert!(hover.props.drag_active);
    assert!(hover.props.drop_hover);

    let drop_only = InteractiveRowWidget::new(9, WidgetSizing::fixed(Vector2::new(120.0, 22.0)))
        .with_drop_target_mode(true, false);
    assert!(drop_only.props.droppable);
    assert!(drop_only.props.drag_active);
    assert!(!drop_only.props.drop_hover);
}

#[test]
fn handle_input_mapped_routes_custom_row_output() {
    let bounds = Rect::from_size(120.0, 22.0);
    let mut row = InteractiveRowWidget::new(10, WidgetSizing::fixed(Vector2::new(120.0, 22.0)));
    let pointer = Point::new(4.0, 4.0);

    let press = row.handle_input_mapped(
        bounds,
        WidgetInput::PointerPress {
            position: pointer,
            button: PointerButton::Primary,
            modifiers: PointerModifiers::default(),
        },
        |_| Some("activated"),
    );
    assert!(press.is_none());

    let release = row
        .handle_input_mapped(
            bounds,
            WidgetInput::PointerRelease {
                position: pointer,
                button: PointerButton::Primary,
                modifiers: PointerModifiers::default(),
            },
            |message| message.is_single_activation().then_some("activated"),
        )
        .expect("release maps to typed widget output");
    assert_eq!(release.typed_ref::<&'static str>(), Some(&"activated"));
}

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
    assert!(host.accepts_pointer_move());

    let _ = host.handle_input(bounds, WidgetInput::primary_press(pointer));
    let output = host
        .handle_input(bounds, WidgetInput::primary_release(pointer))
        .expect("embedded row host should emit mapped row output");
    assert!(output.typed_ref::<InteractiveRowMessage>().is_some());
}

#[test]
fn embedded_interactive_row_widget_routes_configured_actions_by_default() {
    let bounds = Rect::from_size(120.0, 22.0);
    let pointer = Point::new(4.0, 4.0);
    let mut host = ActionRowHost {
        row: InteractiveRowWidget::new(12, WidgetSizing::fixed(Vector2::new(120.0, 22.0))),
        actions: InteractiveRowActions::new().activate(|| "activated"),
    };

    let _ = host.handle_input(bounds, WidgetInput::primary_press(pointer));
    let output = host
        .handle_input(bounds, WidgetInput::primary_release(pointer))
        .expect("embedded row host should emit configured action output");

    assert_eq!(output.typed_ref::<&'static str>(), Some(&"activated"));
}
