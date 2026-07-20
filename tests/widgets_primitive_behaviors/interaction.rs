use super::*;

#[test]
fn list_item_invocation_is_public_and_deterministic() {
    let mut item = ListItemWidget::new(
        9,
        "Document",
        WidgetSizing::fixed(Vector2::new(120.0, 28.0)),
    );
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 28.0));

    assert_eq!(
        item.handle_input(
            bounds,
            WidgetInput::PointerPress {
                position: Point::new(12.0, 10.0),
                button: PointerButton::Primary,
                modifiers: Default::default(),
            },
        ),
        None
    );
    assert_eq!(
        item.handle_input(
            bounds,
            WidgetInput::PointerRelease {
                position: Point::new(12.0, 10.0),
                button: PointerButton::Primary,
                modifiers: Default::default(),
            },
        ),
        Some(ListItemMessage::Invoked)
    );

    let _ = item.handle_input(bounds, WidgetInput::FocusChanged(true));
    assert_eq!(
        item.handle_input(bounds, WidgetInput::KeyPress(WidgetKey::Enter)),
        Some(ListItemMessage::Invoked)
    );
}

#[test]
fn interactive_row_emits_secondary_activation() {
    let mut row = InteractiveRowWidget::new(31, WidgetSizing::fixed(Vector2::new(120.0, 18.0)));
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 18.0));

    assert_eq!(
        row.handle_input(
            bounds,
            WidgetInput::PointerPress {
                position: Point::new(16.0, 8.0),
                button: PointerButton::Secondary,
                modifiers: Default::default(),
            },
        ),
        Some(InteractiveRowMessage::SecondaryActivate {
            position: Point::new(16.0, 8.0)
        })
    );
    assert!(row.common.state.hovered);
    assert!(!row.common.state.pressed);
}

#[test]
fn interactive_row_emits_double_activation() {
    let mut row = InteractiveRowWidget::new(37, WidgetSizing::fixed(Vector2::new(120.0, 18.0)));
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 18.0));

    assert_eq!(
        row.handle_input(
            bounds,
            WidgetInput::PointerDoubleClick {
                position: Point::new(16.0, 8.0),
                button: PointerButton::Primary,
                modifiers: Default::default(),
            },
        ),
        Some(InteractiveRowMessage::DoubleActivate)
    );
    assert!(row.common.state.hovered);
    assert!(!row.common.state.pressed);
}

#[test]
fn interactive_row_can_emit_modifier_aware_pointer_activation() {
    let mut row = InteractiveRowWidget::new(39, WidgetSizing::fixed(Vector2::new(120.0, 18.0)))
        .with_activation_modifiers();
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 18.0));
    let modifiers = PointerModifiers {
        shift: true,
        ..PointerModifiers::default()
    };

    assert_eq!(
        row.handle_input(
            bounds,
            WidgetInput::PointerPress {
                position: Point::new(16.0, 8.0),
                button: PointerButton::Primary,
                modifiers: PointerModifiers::default(),
            },
        ),
        None
    );
    assert_eq!(
        row.handle_input(
            bounds,
            WidgetInput::PointerRelease {
                position: Point::new(16.0, 8.0),
                button: PointerButton::Primary,
                modifiers,
            },
        ),
        Some(InteractiveRowMessage::ActivateWithModifiers { modifiers })
    );
}

#[test]
fn interactive_row_message_helpers_project_common_custom_row_intents() {
    let modifiers = PointerModifiers {
        shift: true,
        ..PointerModifiers::default()
    };
    let drag = DragHandleMessage::Moved {
        position: Point::new(18.0, 9.0),
    };

    assert_eq!(
        InteractiveRowMessage::Activate.activation_modifiers(),
        Some(PointerModifiers::default())
    );
    assert_eq!(
        InteractiveRowMessage::Activate.single_activation_modifiers(),
        Some(PointerModifiers::default())
    );
    assert_eq!(
        InteractiveRowMessage::DoubleActivate.activation_modifiers(),
        Some(PointerModifiers::default())
    );
    assert_eq!(
        InteractiveRowMessage::DoubleActivate.single_activation_modifiers(),
        None
    );
    assert_eq!(
        InteractiveRowMessage::ActivateWithModifiers { modifiers }.activation_modifiers(),
        Some(modifiers)
    );
    assert_eq!(
        InteractiveRowMessage::ActivateWithModifiers { modifiers }.single_activation_modifiers(),
        Some(modifiers)
    );
    assert!(InteractiveRowMessage::Activate.is_activation());
    assert!(InteractiveRowMessage::Activate.is_single_activation());
    assert!(!InteractiveRowMessage::Activate.is_double_activation());
    assert!(InteractiveRowMessage::DoubleActivate.is_activation());
    assert!(!InteractiveRowMessage::DoubleActivate.is_single_activation());
    assert!(InteractiveRowMessage::DoubleActivate.is_double_activation());
    assert_eq!(
        InteractiveRowMessage::SecondaryActivate {
            position: Point::new(12.0, 4.0)
        }
        .secondary_position(),
        Some(Point::new(12.0, 4.0))
    );
    assert_eq!(InteractiveRowMessage::Drag(drag).drag_message(), Some(drag));
    assert_eq!(
        InteractiveRowMessage::HoverDropTarget {
            position: Point::new(20.0, 10.0)
        }
        .hover_drop_position(),
        Some(Point::new(20.0, 10.0))
    );
    assert_eq!(
        InteractiveRowMessage::ClearDropTarget {
            position: Point::new(22.0, 12.0)
        }
        .clear_drop_position(),
        Some(Point::new(22.0, 12.0))
    );
    assert!(InteractiveRowMessage::Drop.is_drop());
    assert_eq!(
        InteractiveRowMessage::SecondaryActivate {
            position: Point::new(12.0, 4.0)
        }
        .activation_modifiers(),
        None
    );
}

#[test]
fn drag_handle_message_helpers_project_phase_and_position() {
    let origin = Point::new(8.0, 2.0);
    let start = DragHandleMessage::started_from(origin, Point::new(10.0, 4.0));
    let moved = DragHandleMessage::Moved {
        position: Point::new(12.0, 8.0),
    };
    let ended = DragHandleMessage::Ended {
        position: Point::new(14.0, 9.0),
    };

    assert_eq!(start.phase(), DragHandlePhase::Started);
    assert_eq!(moved.phase(), DragHandlePhase::Moved);
    assert_eq!(ended.phase(), DragHandlePhase::Ended);

    assert_eq!(start.position(), Point::new(10.0, 4.0));
    assert_eq!(moved.position(), Point::new(12.0, 8.0));
    assert_eq!(ended.position(), Point::new(14.0, 9.0));

    assert_eq!(start.started_position(), Some(Point::new(10.0, 4.0)));
    assert_eq!(start.started_origin(), Some(origin));
    assert_eq!(start.moved_position(), None);
    assert_eq!(start.ended_position(), None);
    assert_eq!(moved.started_position(), None);
    assert_eq!(moved.moved_position(), Some(Point::new(12.0, 8.0)));
    assert_eq!(moved.ended_position(), None);
    assert_eq!(ended.started_position(), None);
    assert_eq!(ended.moved_position(), None);
    assert_eq!(ended.ended_position(), Some(Point::new(14.0, 9.0)));

    assert!(start.is_started());
    assert!(!start.is_moved());
    assert!(!start.is_ended());
    assert!(moved.is_moved());
    assert!(ended.is_ended());
}

#[test]
fn text_input_message_helpers_project_value_and_event_kind() {
    let changed = TextInputMessage::Changed {
        value: String::from("draft"),
    };
    let submitted = TextInputMessage::Submitted {
        value: String::from("commit"),
    };
    let completion = TextInputMessage::CompletionRequested {
        value: String::from("pre"),
    };

    assert_eq!(changed.value(), "draft");
    assert_eq!(submitted.value(), "commit");
    assert_eq!(completion.value(), "pre");

    assert!(changed.is_changed());
    assert!(!changed.is_submitted());
    assert!(!changed.is_completion_requested());
    assert!(submitted.is_submitted());
    assert!(completion.is_completion_requested());
    assert_eq!(submitted.into_value(), "commit");
}

#[test]
fn widget_input_helpers_project_pointer_positions_and_start_bounds() {
    let bounds = Rect::from_min_size(Point::new(10.0, 20.0), Vector2::new(100.0, 40.0));
    let inside = Point::new(20.0, 30.0);
    let outside = Point::new(4.0, 30.0);

    let press = WidgetInput::PointerPress {
        position: inside,
        button: PointerButton::Primary,
        modifiers: Default::default(),
    };
    assert_eq!(press.pointer_position(), Some(inside));
    assert_eq!(press.pointer_start_position(), Some(inside));
    assert!(press.pointer_start_inside(bounds));
    assert!(!press.pointer_start_outside(bounds));

    let wheel = WidgetInput::Wheel {
        position: outside,
        delta: Vector2::new(0.0, 1.0),
        modifiers: Default::default(),
    };
    assert_eq!(wheel.pointer_position(), Some(outside));
    assert_eq!(wheel.pointer_start_position(), Some(outside));
    assert!(wheel.pointer_start_outside(bounds));
    assert!(!wheel.pointer_start_inside(bounds));

    let release = WidgetInput::PointerRelease {
        position: outside,
        button: PointerButton::Primary,
        modifiers: Default::default(),
    };
    assert_eq!(release.pointer_position(), Some(outside));
    assert_eq!(release.pointer_start_position(), None);
    assert!(!release.pointer_start_outside(bounds));

    assert_eq!(WidgetInput::FocusChanged(true).pointer_position(), None);
}

#[test]
fn interactive_row_clears_stale_hover_when_requested() {
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 18.0));
    let mut previous =
        InteractiveRowWidget::new(32, WidgetSizing::fixed(Vector2::new(120.0, 18.0)));
    assert_eq!(
        previous.handle_input(
            bounds,
            WidgetInput::PointerMove {
                position: Point::new(16.0, 8.0),
            },
        ),
        None
    );
    assert!(previous.common.state.hovered);

    let mut row = InteractiveRowWidget::new(32, WidgetSizing::fixed(Vector2::new(120.0, 18.0)))
        .clear_hover_on_sync();
    row.synchronize_from_previous(&previous);

    assert!(!row.common.state.hovered);
}

#[test]
fn interactive_row_active_drag_source_releases_after_refresh() {
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 18.0));
    let mut row = InteractiveRowWidget::new(33, WidgetSizing::fixed(Vector2::new(120.0, 18.0)))
        .with_drag()
        .with_drag_source(true);

    assert_eq!(
        row.handle_input(
            bounds,
            WidgetInput::PointerMove {
                position: Point::new(34.0, 8.0),
            },
        ),
        None,
        "runtime drag previews should own movement while the row is only the retained source"
    );
    assert_eq!(
        row.handle_input(
            bounds,
            WidgetInput::PointerRelease {
                position: Point::new(220.0, 90.0),
                button: PointerButton::Primary,
                modifiers: Default::default(),
            },
        ),
        Some(InteractiveRowMessage::Drag(DragHandleMessage::Ended {
            position: Point::new(220.0, 90.0),
        }))
    );
}

#[test]
fn interactive_row_can_report_active_drag_source_motion_after_refresh() {
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 18.0));
    let mut row = InteractiveRowWidget::new(35, WidgetSizing::fixed(Vector2::new(120.0, 18.0)))
        .with_drag()
        .with_drag_source(true)
        .with_drag_source_motion(true);

    assert_eq!(
        row.handle_input(
            bounds,
            WidgetInput::PointerMove {
                position: Point::new(34.0, 8.0),
            },
        ),
        Some(InteractiveRowMessage::Drag(DragHandleMessage::Moved {
            position: Point::new(34.0, 8.0),
        }))
    );
}

#[test]
fn interactive_row_can_limit_pointer_motion_to_active_interactions() {
    let mut row = InteractiveRowWidget::new(40, WidgetSizing::fixed(Vector2::new(120.0, 18.0)))
        .with_pointer_motion_during_interaction();
    assert!(!row.accepts_pointer_move());

    row.common.state.pressed = true;
    assert!(row.accepts_pointer_move());

    row.common.state.pressed = false;
    row = row.with_pointer_motion_active(true);
    assert!(row.accepts_pointer_move());
}

#[test]
fn interactive_row_suppresses_hover_during_external_drag() {
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 18.0));
    let mut row = InteractiveRowWidget::new(34, WidgetSizing::fixed(Vector2::new(120.0, 18.0)))
        .with_drag_active(true);
    row.common.state.hovered = true;

    assert_eq!(
        row.handle_input(
            bounds,
            WidgetInput::PointerMove {
                position: Point::new(16.0, 8.0),
            },
        ),
        None
    );
    assert!(!row.common.state.hovered);
}

#[test]
fn interactive_row_drop_only_accepts_release_without_hover_notification() {
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 18.0));
    let mut row = InteractiveRowWidget::new(36, WidgetSizing::fixed(Vector2::new(120.0, 18.0)))
        .with_drop_only(true);

    assert_eq!(
        row.handle_input(
            bounds,
            WidgetInput::PointerMove {
                position: Point::new(16.0, 8.0),
            },
        ),
        None
    );
    assert_eq!(
        row.handle_input(
            bounds,
            WidgetInput::PointerRelease {
                position: Point::new(16.0, 8.0),
                button: PointerButton::Primary,
                modifiers: Default::default(),
            },
        ),
        Some(InteractiveRowMessage::Drop)
    );
}

#[test]
fn interactive_row_drop_hover_reports_pointer_position() {
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 18.0));
    let mut row = InteractiveRowWidget::new(38, WidgetSizing::fixed(Vector2::new(120.0, 18.0)))
        .with_drop_target(true);

    assert_eq!(
        row.handle_input(
            bounds,
            WidgetInput::PointerMove {
                position: Point::new(16.0, 8.0),
            },
        ),
        Some(InteractiveRowMessage::HoverDropTarget {
            position: Point::new(16.0, 8.0),
        })
    );
}

#[test]
fn pointer_shield_blocks_configured_pointer_events_inside_bounds() {
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 18.0));
    let shield = PointerShieldWidget::pointer_move_only(true).with_pointer_release(true);

    assert_eq!(
        shield.handle_input(
            bounds,
            WidgetInput::PointerMove {
                position: Point::new(16.0, 8.0),
            },
        ),
        Some(PointerShieldMessage::PointerMove {
            position: Point::new(16.0, 8.0),
        })
    );
    assert_eq!(
        shield.handle_input(
            bounds,
            WidgetInput::PointerRelease {
                position: Point::new(18.0, 8.0),
                button: PointerButton::Primary,
                modifiers: Default::default(),
            },
        ),
        Some(PointerShieldMessage::PointerRelease {
            position: Point::new(18.0, 8.0),
            button: PointerButton::Primary,
            modifiers: Default::default(),
        })
    );
    assert_eq!(
        shield.handle_input(
            bounds,
            WidgetInput::PointerPress {
                position: Point::new(18.0, 8.0),
                button: PointerButton::Primary,
                modifiers: Default::default(),
            },
        ),
        None
    );
}

#[test]
fn pointer_shield_drop_only_reports_only_captured_drops() {
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 18.0));
    let shield = PointerShieldWidget::pointer_drop_only(true);

    assert_eq!(
        shield.handle_input(
            bounds,
            WidgetInput::PointerMove {
                position: Point::new(16.0, 8.0),
            },
        ),
        None
    );
    assert_eq!(
        shield.handle_input(
            bounds,
            WidgetInput::PointerDrop {
                position: Point::new(16.0, 8.0),
                button: PointerButton::Primary,
                modifiers: Default::default(),
            },
        ),
        Some(PointerShieldMessage::PointerDrop {
            position: Point::new(16.0, 8.0),
            button: PointerButton::Primary,
            modifiers: Default::default(),
        })
    );
}

#[test]
fn pointer_shield_consumes_wheel_when_enabled() {
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 18.0));
    let shield = PointerShieldWidget::fill(true);
    let delta = Vector2::new(0.0, -18.0);

    assert!(shield.accepts_wheel_input());
    assert_eq!(
        shield.handle_input(
            bounds,
            WidgetInput::Wheel {
                position: Point::new(16.0, 8.0),
                delta,
                modifiers: Default::default(),
            },
        ),
        Some(PointerShieldMessage::Wheel {
            position: Point::new(16.0, 8.0),
            delta,
            modifiers: Default::default(),
        })
    );
}

#[test]
fn pointer_shield_ignores_wheel_when_disabled() {
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 18.0));
    let shield = PointerShieldWidget::fill(true).with_wheel(false);

    assert!(!shield.accepts_wheel_input());
    assert_eq!(
        shield.handle_input(
            bounds,
            WidgetInput::Wheel {
                position: Point::new(16.0, 8.0),
                delta: Vector2::new(0.0, -18.0),
                modifiers: Default::default(),
            },
        ),
        None
    );
}

#[test]
fn pointer_shield_stays_quiet_when_inactive_or_outside_bounds() {
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 18.0));
    let inactive = PointerShieldWidget::fill(false);
    let active = PointerShieldWidget::fill(true);

    assert!(!inactive.accepts_pointer_move());
    assert!(inactive.common.state.disabled);
    assert_eq!(
        inactive.handle_input(
            bounds,
            WidgetInput::PointerMove {
                position: Point::new(16.0, 8.0),
            },
        ),
        None
    );
    assert_eq!(
        active.handle_input(
            bounds,
            WidgetInput::PointerMove {
                position: Point::new(160.0, 8.0),
            },
        ),
        None
    );
}

#[test]
fn feedback_overlay_paints_background_progress_and_edge_bands() {
    let widget = FeedbackOverlayWidget::fill()
        .with_background(radiant::gui::types::Rgba8::new(1, 2, 3, 40))
        .with_progress(0.25, radiant::gui::types::Rgba8::new(4, 5, 6, 80))
        .with_edge(
            radiant::gui::types::Rgba8::new(7, 8, 9, 120),
            3.0,
            BorderSides {
                top: true,
                bottom: true,
                left: false,
                right: false,
            },
        );
    let bounds = Rect::from_min_size(Point::new(10.0, 20.0), Vector2::new(100.0, 40.0));
    let mut primitives = Vec::new();

    widget.append_paint(
        &mut primitives,
        bounds,
        &Default::default(),
        &Default::default(),
    );

    let fills: Vec<_> = primitives
        .iter()
        .filter_map(|primitive| match primitive {
            radiant::runtime::PaintPrimitive::FillRect(fill) => Some(fill),
            _ => None,
        })
        .collect();
    let fill_batches: Vec<_> = primitives
        .iter()
        .filter_map(|primitive| match primitive {
            radiant::runtime::PaintPrimitive::FillRectBatch(batch) => Some(batch),
            _ => None,
        })
        .collect();
    assert_eq!(fills.len(), 2);
    assert_eq!(fills[0].rect, bounds);
    assert_eq!(fills[1].rect.width(), 25.0);
    assert_eq!(fill_batches.len(), 1);
    assert_eq!(fill_batches[0].rects.as_ref().len(), 2);
    assert_eq!(fill_batches[0].rects[0], bounds.top_edge_strip(3.0));
    assert_eq!(fill_batches[0].rects[1], bounds.bottom_edge_strip(3.0));
}

#[test]
fn progress_bar_paints_determinate_track_and_fill() {
    let widget = ProgressBarWidget::determinate(0.4)
        .with_colors(
            radiant::gui::types::Rgba8::new(10, 20, 30, 210),
            radiant::gui::types::Rgba8::new(220, 120, 40, 210),
        )
        .with_max_track_height(8.0);
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(100.0, 20.0));
    let mut primitives = Vec::new();

    widget.append_paint(
        &mut primitives,
        bounds,
        &Default::default(),
        &Default::default(),
    );

    let fills: Vec<_> = primitives
        .iter()
        .filter_map(|primitive| match primitive {
            radiant::runtime::PaintPrimitive::FillRect(fill) => Some(fill),
            _ => None,
        })
        .collect();
    assert_eq!(fills.len(), 2);
    assert_eq!(fills[0].rect.height(), 8.0);
    assert_eq!(fills[0].rect.min.y, 6.0);
    assert_eq!(fills[1].rect.width(), 40.0);
}

#[test]
fn progress_bar_can_emit_activation_when_enabled() {
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(100.0, 12.0));
    let mut widget = ProgressBarWidget::indeterminate(0.5).with_activation();

    assert_eq!(
        widget.handle_input(
            bounds,
            WidgetInput::PointerPress {
                position: Point::new(20.0, 6.0),
                button: PointerButton::Primary,
                modifiers: Default::default(),
            },
        ),
        None
    );
    assert_eq!(
        widget.handle_input(
            bounds,
            WidgetInput::PointerRelease {
                position: Point::new(20.0, 6.0),
                button: PointerButton::Primary,
                modifiers: Default::default(),
            },
        ),
        Some(ProgressBarMessage::Activate)
    );
}

#[test]
fn text_widget_can_use_muted_foreground_role() {
    let widget = TextWidget::new(45, "Muted", WidgetSizing::fixed(Vector2::new(100.0, 20.0)))
        .with_color(TextColorRole::Muted);
    let theme = radiant::theme::ThemeTokens::default();
    let mut primitives = Vec::new();

    widget.append_paint(
        &mut primitives,
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(100.0, 20.0)),
        &Default::default(),
        &theme,
    );

    assert!(primitives.iter().any(|primitive| matches!(
        primitive,
        radiant::runtime::PaintPrimitive::Text(text) if text.color == theme.text_muted
    )));
}

#[test]
fn text_widget_can_paint_accent_background_and_inset_text() {
    let widget = TextWidget::new(46, "Ghost", WidgetSizing::fixed(Vector2::new(100.0, 18.0)))
        .with_background(TextBackgroundRole::Accent)
        .with_color(TextColorRole::OnAccent)
        .with_inset(Vector2::new(3.0, 0.0));
    let theme = radiant::theme::ThemeTokens::default();
    let bounds = Rect::from_min_size(Point::new(10.0, 20.0), Vector2::new(100.0, 18.0));
    let mut primitives = Vec::new();

    widget.append_paint(&mut primitives, bounds, &Default::default(), &theme);

    assert!(primitives.iter().any(|primitive| matches!(
        primitive,
        radiant::runtime::PaintPrimitive::FillRect(fill)
            if fill.rect == bounds
                && fill.color == theme.accent_mint.blend_toward(theme.bg_primary, 0.12)
    )));
    assert!(primitives.iter().any(|primitive| matches!(
        primitive,
        radiant::runtime::PaintPrimitive::Text(text)
            if text.rect.min.x == bounds.min.x + 3.0 && text.color == theme.bg_primary
    )));
}

#[test]
fn color_marker_paints_right_aligned_square() {
    let color = radiant::gui::types::Rgba8::new(12, 34, 56, 200);
    let widget = ColorMarkerWidget::new(Some(color));
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(80.0, 20.0));
    let mut primitives = Vec::new();

    widget.append_paint(
        &mut primitives,
        bounds,
        &Default::default(),
        &Default::default(),
    );

    let fills: Vec<_> = primitives
        .iter()
        .filter_map(|primitive| match primitive {
            radiant::runtime::PaintPrimitive::FillRect(fill) => Some(fill),
            _ => None,
        })
        .collect();
    assert_eq!(fills.len(), 1);
    assert_eq!(fills[0].color, color);
    assert_eq!(
        fills[0].rect,
        Rect::from_min_max(Point::new(66.0, 5.0), Point::new(76.0, 15.0))
    );
}

#[test]
fn selectable_paints_configured_color_marker_without_overlapping_label() {
    let color = radiant::gui::types::Rgba8::new(30, 140, 90, 220);
    let widget = SelectableWidget::new(
        44,
        "Choice",
        true,
        WidgetSizing::fixed(Vector2::new(80.0, 20.0)),
    )
    .with_color_marker_props(
        ColorMarkerProps::new(Some(color))
            .side(6)
            .inset(2)
            .align(ColorMarkerAlign::Right),
    );
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(80.0, 20.0));
    let mut primitives = Vec::new();

    widget.append_paint(
        &mut primitives,
        bounds,
        &Default::default(),
        &Default::default(),
    );

    assert!(primitives.iter().any(|primitive| matches!(
        primitive,
        radiant::runtime::PaintPrimitive::FillRect(fill)
            if fill.color == color
                && fill.rect
                    == Rect::from_min_max(Point::new(72.0, 7.0), Point::new(78.0, 13.0))
    )));
    assert!(primitives.iter().any(|primitive| matches!(
        primitive,
        radiant::runtime::PaintPrimitive::Text(text)
            if text.text.as_str() == "Choice" && text.rect.max.x == 68.0
    )));
}

#[test]
fn drag_handle_emits_captured_drag_lifecycle() {
    let mut handle = DragHandleWidget::new(12, WidgetSizing::fixed(Vector2::new(24.0, 24.0)));
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(24.0, 24.0));

    assert_eq!(
        handle.handle_input(
            bounds,
            WidgetInput::PointerMove {
                position: Point::new(12.0, 12.0),
            },
        ),
        None
    );
    assert!(handle.common.state.hovered);
    assert_eq!(
        handle.handle_input(
            bounds,
            WidgetInput::PointerPress {
                position: Point::new(12.0, 12.0),
                button: PointerButton::Primary,
                modifiers: Default::default(),
            },
        ),
        Some(DragHandleMessage::started(Point::new(12.0, 12.0)))
    );
    assert!(handle.common.state.pressed);
    assert!(handle.common.state.active);
    assert_eq!(
        handle.handle_input(
            bounds,
            WidgetInput::PointerMove {
                position: Point::new(12.0, 70.0),
            },
        ),
        Some(DragHandleMessage::Moved {
            position: Point::new(12.0, 70.0),
        })
    );
    assert_eq!(
        handle.handle_input(
            bounds,
            WidgetInput::PointerRelease {
                position: Point::new(12.0, 70.0),
                button: PointerButton::Primary,
                modifiers: Default::default(),
            },
        ),
        Some(DragHandleMessage::Ended {
            position: Point::new(12.0, 70.0),
        })
    );
    assert!(!handle.common.state.pressed);
    assert!(!handle.common.state.active);
}

#[test]
fn selectable_toggles_selected_state_with_pointer_and_keyboard() {
    let mut selectable = SelectableWidget::new(
        11,
        "Choice",
        false,
        WidgetSizing::fixed(Vector2::new(120.0, 28.0)),
    );
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 28.0));

    assert!(!selectable.common.state.selected);
    assert_eq!(
        selectable.handle_input(
            bounds,
            WidgetInput::PointerPress {
                position: Point::new(12.0, 10.0),
                button: PointerButton::Primary,
                modifiers: Default::default(),
            },
        ),
        None
    );
    assert_eq!(
        selectable.handle_input(
            bounds,
            WidgetInput::PointerRelease {
                position: Point::new(12.0, 10.0),
                button: PointerButton::Primary,
                modifiers: Default::default(),
            },
        ),
        Some(SelectableMessage::SelectionChanged { selected: true })
    );
    assert!(selectable.common.state.selected);

    let _ = selectable.handle_input(bounds, WidgetInput::FocusChanged(true));
    assert_eq!(
        selectable.handle_input(bounds, WidgetInput::KeyPress(WidgetKey::Space)),
        Some(SelectableMessage::SelectionChanged { selected: false })
    );
    assert!(!selectable.common.state.selected);
}
