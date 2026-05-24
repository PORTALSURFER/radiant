use super::{
    AppMessage, DATA_SOURCE_NOTE, DESTINATION_COUNT, MATRIX_WIDGET_ID, MatrixCell, MatrixMessage,
    ModulationMatrixState, SOURCE_COUNT, STATUS_WIDGET_ID, project_surface, update,
    widget::ModulationMatrixWidget,
};
use radiant::prelude::*;
use radiant::runtime::{RuntimeBridge, SurfaceRuntime};

#[test]
fn modulation_matrix_tick_advances_synthetic_activity_without_synth_or_dsp() {
    let mut state = ModulationMatrixState::default();
    let initial = state.activity_phase;

    state.tick();

    assert_eq!(state.frame, 1);
    assert!(state.activity_phase > initial);
    assert_eq!(DATA_SOURCE_NOTE, "without_synth_or_dsp");
}

#[test]
fn modulation_matrix_widget_paints_sources_destinations_and_amounts() {
    let state = ModulationMatrixState::default();
    let widget = ModulationMatrixWidget::new(state.amounts, state.selected, state.activity_phase);
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 390.0));
    let mut primitives = Vec::new();

    widget.append_paint(
        &mut primitives,
        bounds,
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );

    assert!(
        primitives
            .iter()
            .filter(|primitive| matches!(primitive, PaintPrimitive::FillRect(_)))
            .count()
            >= SOURCE_COUNT * DESTINATION_COUNT
    );
    assert!(primitives.iter().any(
        |primitive| matches!(primitive, PaintPrimitive::Text(text) if text.text.as_str() == "LFO 1")
    ));
    assert!(
        primitives
            .iter()
            .any(|primitive| matches!(primitive, PaintPrimitive::Text(text) if text.text.as_str() == "Cutoff"))
    );
}

#[test]
fn modulation_matrix_drag_routes_bipolar_amount_change() {
    let state = ModulationMatrixState::default();
    let mut widget =
        ModulationMatrixWidget::new(state.amounts, state.selected, state.activity_phase);
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 390.0));
    let matrix = widget.matrix_rect(bounds);
    let cell = MatrixCell {
        source: 2,
        destination: 4,
    };
    let rect = widget.cell_rect(matrix, cell);

    let output = widget.handle_input(
        bounds,
        WidgetInput::PointerPress {
            position: Point::new(rect.center().x, rect.min.y + 1.0),
            button: PointerButton::Primary,
            modifiers: Default::default(),
        },
    );

    assert_eq!(
        output.and_then(|output| output.typed_ref::<MatrixMessage>().copied()),
        Some(MatrixMessage::SetAmount {
            cell,
            amount: widget.amount_for_position(rect, Point::new(rect.center().x, rect.min.y + 1.0))
        })
    );
    assert!(!widget.prefers_pointer_move_paint_only());
}

#[test]
fn modulation_matrix_hover_uses_paint_only_runtime_overlay() {
    let state = ModulationMatrixState::default();
    let mut widget =
        ModulationMatrixWidget::new(state.amounts, state.selected, state.activity_phase);
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 390.0));
    let matrix = widget.matrix_rect(bounds);
    let cell = MatrixCell {
        source: 1,
        destination: 3,
    };

    let output = widget.handle_input(
        bounds,
        WidgetInput::PointerMove {
            position: widget.cell_rect(matrix, cell).center(),
        },
    );

    assert!(output.is_none());
    assert_eq!(widget.hover_cell, Some(cell));
    assert!(widget.prefers_pointer_move_paint_only());
    let mut overlay = Vec::new();
    widget.append_runtime_overlay_paint(
        &mut overlay,
        bounds,
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );
    assert!(
        overlay
            .iter()
            .any(|primitive| matches!(primitive, PaintPrimitive::StrokeRect(_))),
        "hovered route should paint as a lightweight runtime overlay"
    );
}

#[test]
fn modulation_matrix_runtime_hover_does_not_refresh_surface() {
    let bridge = modulation_matrix_test_bridge(ModulationMatrixState::default());
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(1040.0, 620.0));
    let bounds = runtime.layout().rects[&MATRIX_WIDGET_ID];
    let first = runtime
        .dispatch_pointer_move_with_outcome(Point::new(bounds.min.x + 180.0, bounds.center().y));
    let second = runtime
        .dispatch_pointer_move_with_outcome(Point::new(bounds.min.x + 280.0, bounds.center().y));

    assert!(first.needs_scene_rebuild());
    assert!(second.paint_only_requested);
    assert!(
        !second.needs_scene_rebuild(),
        "stable modulation-matrix hover should avoid reprojection and full scene rebuilds"
    );
}

#[test]
fn modulation_matrix_runtime_frame_messages_advance_status() {
    let bridge = modulation_matrix_test_bridge(ModulationMatrixState::default());
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(1040.0, 620.0));
    let initial_status = status_text(&runtime);

    assert!(runtime.bridge_mut().needs_animation());
    assert!(runtime.bridge_mut().queue_animation_frame());
    let outcome = runtime.drain_runtime_messages();

    assert_eq!(outcome.messages_dispatched, 1);
    assert_ne!(status_text(&runtime), initial_status);
}

fn modulation_matrix_test_bridge(state: ModulationMatrixState) -> impl RuntimeBridge<AppMessage> {
    radiant::app(state)
        .view(project_surface)
        .animation(|state| state.running)
        .on_frame(|| AppMessage::Frame)
        .update(update)
        .into_bridge()
}

fn status_text<Bridge>(runtime: &SurfaceRuntime<Bridge, AppMessage>) -> String
where
    Bridge: RuntimeBridge<AppMessage>,
{
    runtime
        .paint_plan(&ThemeTokens::default())
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            PaintPrimitive::Text(text) if text.widget_id == STATUS_WIDGET_ID => {
                Some(text.text.as_str().to_string())
            }
            _ => None,
        })
        .expect("status text should be painted")
}
