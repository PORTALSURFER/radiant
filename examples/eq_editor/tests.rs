use super::model::{EqEditorMessage, EqEditorState, EqMessage, selected_band, update};
use super::widget::{EqEditorWidget, x_for_freq, y_for_gain};
use radiant::prelude::*;
use radiant::runtime::{PaintPrimitive, PaintStrokePolyline};
use radiant::widgets::{PointerModifiers, Widget};

#[test]
fn eq_widget_paints_curve_analyzer_and_band_handles() {
    let state = EqEditorState::default();
    let widget = EqEditorWidget::new(state.bands, state.selected_band, true);
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(700.0, 300.0));
    let mut primitives = Vec::new();

    widget.append_paint(
        &mut primitives,
        bounds,
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );

    assert!(
        primitives.iter().any(|primitive| matches!(
            primitive,
            PaintPrimitive::StrokePolyline(PaintStrokePolyline { points, width, .. })
                if points.len() == 160 && *width == 3.0
        )),
        "EQ response should paint as a sampled visual curve"
    );
    assert!(
        primitives
            .iter()
            .any(|primitive| matches!(primitive, PaintPrimitive::FillPolygon(_))),
        "analyzer overlay should be a normal GUI paint primitive"
    );
    assert!(
        primitives
            .iter()
            .filter(|primitive| matches!(primitive, PaintPrimitive::StrokeRect(_)))
            .count()
            >= 5,
        "plot and band handles should produce visible chrome"
    );
}

#[test]
fn eq_widget_routes_select_and_drag_messages_without_dsp() {
    let state = EqEditorState::default();
    let mut widget = EqEditorWidget::new(state.bands.clone(), state.selected_band, true);
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(700.0, 300.0));
    let plot = widget.plot_rect(bounds);
    let band = state.bands[1];
    let center = widget.handle_center(plot, band);

    let select = widget
        .handle_input(
            bounds,
            WidgetInput::PointerPress {
                position: center,
                button: PointerButton::Primary,
                modifiers: PointerModifiers::default(),
            },
        )
        .expect("pressing a band should emit selection");
    assert_eq!(
        select.typed_ref::<EqEditorMessage>(),
        Some(&EqEditorMessage::SelectBand(band.id))
    );

    let drag = widget
        .handle_input(
            bounds,
            WidgetInput::PointerMove {
                position: Point::new(x_for_freq(plot, 1_000.0), y_for_gain(plot, 6.0)),
            },
        )
        .expect("dragging a band should emit a parameter-style GUI message");
    assert!(matches!(
        drag.typed_ref::<EqEditorMessage>(),
        Some(EqEditorMessage::MoveBand {
            id,
            freq_hz,
            gain_db,
        }) if *id == band.id && (*freq_hz - 1_000.0).abs() < 2.0 && (*gain_db - 6.0).abs() < 0.1
    ));
}

#[test]
fn eq_update_applies_gui_parameter_messages() {
    let mut state = EqEditorState::default();

    update(
        &mut state,
        EqMessage::Editor(EqEditorMessage::MoveBand {
            id: 2,
            freq_hz: 1_200.0,
            gain_db: 8.0,
        }),
    );

    let band = selected_band(&state).expect("moved band should remain selected");
    assert_eq!(band.id, 2);
    assert_eq!(band.freq_hz, 1_200.0);
    assert_eq!(band.gain_db, 8.0);
    assert!(state.status.contains("moved"));
}
