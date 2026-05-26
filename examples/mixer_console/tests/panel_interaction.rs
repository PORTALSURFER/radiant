use super::super::panel::MixerDragTarget;
use super::*;

#[test]
fn mixer_panel_hover_uses_paint_only_runtime_overlay() {
    let state = MixerState::default();
    let mut widget = mixer_widget(&state);
    let bounds = mixer_bounds();
    let strip = widget.strip_rect(bounds, 2);

    let output = widget.handle_input(
        bounds,
        WidgetInput::PointerMove {
            position: strip.center(),
        },
    );

    assert!(output.is_none());
    assert_eq!(widget.interaction.hover_channel, Some(2));
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
            .any(|primitive| matches!(primitive, PaintPrimitive::StrokeRect(_)))
    );
}

#[test]
fn mixer_panel_fader_drag_routes_gain_change() {
    let state = MixerState::default();
    let mut widget = mixer_widget(&state);
    let bounds = mixer_bounds();
    let strip = widget.strip_rect(bounds, 4);
    let fader = widget.fader_rect(strip);

    let output = widget.handle_input(
        bounds,
        WidgetInput::PointerPress {
            position: Point::new(fader.center().x, fader.min.y),
            button: PointerButton::Primary,
            modifiers: Default::default(),
        },
    );

    assert_eq!(
        output.and_then(|output| output.typed_ref::<MixerPanelMessage>().copied()),
        Some(MixerPanelMessage::SetGain {
            channel: 4,
            ratio: 1.0,
            selection: Some(ListSelectionModifiers::new()),
        })
    );
    assert!(widget.prefers_pointer_move_paint_only());
}

#[test]
fn mixer_panel_supports_shift_and_control_multi_channel_selection() {
    let state = MixerState::default();
    let mut widget = mixer_widget(&state);
    let bounds = mixer_bounds();

    press_strip_label(&mut widget, bounds, 2, PointerModifiers::default());
    press_strip_label(
        &mut widget,
        bounds,
        5,
        PointerModifiers {
            shift: true,
            ..Default::default()
        },
    );

    assert_eq!(widget.selection.selected_indices(), &[2, 3, 4, 5]);

    press_strip_label(
        &mut widget,
        bounds,
        7,
        PointerModifiers {
            command: true,
            ..Default::default()
        },
    );

    assert_eq!(widget.selection.selected_indices(), &[2, 3, 4, 5, 7]);
}

#[test]
fn mixer_strip_drag_paints_insertion_line_without_spreading_strips() {
    let state = MixerState::default();
    let mut widget = mixer_widget(&state);
    let bounds = mixer_bounds();
    let source = widget.strip_rect(bounds, 2);
    let target_line = widget.insertion_line_rect(bounds, 7);

    widget.handle_input(
        bounds,
        WidgetInput::PointerPress {
            position: Point::new(source.center().x, source.min.y + 22.0),
            button: PointerButton::Primary,
            modifiers: Default::default(),
        },
    );
    let move_output = widget.handle_input(
        bounds,
        WidgetInput::PointerMove {
            position: target_line.center(),
        },
    );

    assert!(move_output.is_none());
    assert_eq!(
        widget.interaction.drag_target,
        Some(MixerDragTarget::Strip(2))
    );
    assert_eq!(widget.interaction.reorder_insert, Some(7));
    let mut overlay = Vec::new();
    widget.append_runtime_overlay_paint(
        &mut overlay,
        bounds,
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );
    assert!(overlay.iter().any(|primitive| {
        matches!(
            primitive,
            PaintPrimitive::FillRect(fill)
                if fill.rect == target_line
                    && fill.color == ThemeTokens::default().highlight_cyan.with_alpha(235)
        )
    }));
}

#[test]
fn mixer_strip_drag_drop_emits_reorder_message() {
    let state = MixerState::default();
    let mut widget = mixer_widget(&state);
    let bounds = mixer_bounds();
    let source = widget.strip_rect(bounds, 2);
    let target_line = widget.insertion_line_rect(bounds, 7);

    widget.handle_input(
        bounds,
        WidgetInput::PointerPress {
            position: Point::new(source.center().x, source.min.y + 22.0),
            button: PointerButton::Primary,
            modifiers: Default::default(),
        },
    );
    widget.handle_input(
        bounds,
        WidgetInput::PointerMove {
            position: target_line.center(),
        },
    );
    let output = widget.handle_input(
        bounds,
        WidgetInput::PointerRelease {
            position: target_line.center(),
            button: PointerButton::Primary,
            modifiers: Default::default(),
        },
    );

    assert_eq!(
        output.and_then(|output| output.typed_ref::<MixerPanelMessage>().copied()),
        Some(MixerPanelMessage::Reorder { from: 2, insert: 7 })
    );
}

#[test]
fn mixer_panel_send_drag_routes_dense_aux_control_change() {
    let state = MixerState::default();
    let mut widget = mixer_widget(&state);
    let bounds = mixer_bounds();
    let strip = widget.strip_rect(bounds, 17);
    let send = widget.send_rect(strip, 2);

    let output = widget.handle_input(
        bounds,
        WidgetInput::PointerPress {
            position: Point::new(send.min.x + send.width() * 0.75, send.center().y),
            button: PointerButton::Primary,
            modifiers: Default::default(),
        },
    );

    assert_eq!(
        output.and_then(|output| output.typed_ref::<MixerPanelMessage>().copied()),
        Some(MixerPanelMessage::SetSend {
            channel: 17,
            send: 2,
            ratio: 0.75
        })
    );
    assert_eq!(
        widget.interaction.drag_target,
        Some(MixerDragTarget::Send {
            channel: 17,
            send: 2
        })
    );
}
