use super::*;

fn browser_drag_model() -> AppModel {
    let mut model = browser_model_with_rows(4, 0);
    model.sources = SourcesPanelModel {
        folder_rows: vec![
            crate::app::FolderRowModel::new("Root", "", 0, false, false, true, true, true)
                .with_source_index(0),
            crate::app::FolderRowModel::new("Drums", "drums", 1, false, false, false, true, true)
                .with_source_index(7),
        ],
        ..SourcesPanelModel::default()
    };
    model
}

fn browser_row_point(layout: &ShellLayout) -> Point {
    let style = StyleTokens::for_viewport_width(layout.root.rect.width());
    Point::new(
        layout.browser_rows.min.x + 24.0,
        layout.browser_rows.min.y + (style.sizing.browser_row_height * 0.5),
    )
}

fn folder_row_point(
    shell_state: &mut NativeShellState,
    layout: &ShellLayout,
    model: &AppModel,
    row_index: usize,
) -> Point {
    for x in layout.sidebar_rows.min.x as i32..=layout.sidebar_rows.max.x as i32 {
        for y in layout.sidebar_rows.min.y as i32..=layout.sidebar_rows.max.y as i32 {
            let point = Point::new(x as f32, y as f32);
            if shell_state.folder_row_at_point(layout, model, point) == Some(row_index) {
                return point;
            }
        }
    }
    panic!("expected hittable folder row at index {row_index}");
}

#[test]
fn browser_row_drag_can_render_folder_drag_highlight() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(layout.root.rect.width());
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    runner.model = Arc::new(browser_drag_model());
    runner.shell_layout = Some(Arc::new(layout.clone()));
    let press_point = browser_row_point(&layout);
    let drag_point = folder_row_point(&mut runner.shell_state, &layout, &runner.model, 1);
    let hover_row = rendered_folder_row_rects(&layout, &style, &runner.model)[1];
    runner.last_cursor = Some(press_point);

    assert!(
        runner.handle_pointer_press_action(UiAction::FocusBrowserRow { visible_row: 0 }, false)
    );
    runner.handle_cursor_moved_for_tests(drag_point);

    assert_eq!(
        runner.bridge.actions,
        vec![
            UiAction::StartBrowserSampleDrag {
                visible_row: 0,
                pointer_x: drag_point.x.round() as u16,
                pointer_y: drag_point.y.round() as u16,
            },
            UiAction::UpdateBrowserSampleDrag {
                pointer_x: drag_point.x.round() as u16,
                pointer_y: drag_point.y.round() as u16,
                hovered_folder_row: Some(7),
                over_folder_panel: true,
                shift_down: false,
                alt_down: false,
            },
        ]
    );
    assert_eq!(runner.shell_state.hovered_folder_row_index(), Some(1));

    let mut drag_model = Arc::unwrap_or_clone(runner.model.clone());
    drag_model.drag_overlay = crate::app::DragOverlayModel {
        active: true,
        label: String::from("row_0000"),
        target_label: String::from("Folder: drums"),
        valid_target: true,
        pointer_x: Some(drag_point.x.round() as u16),
        pointer_y: Some(drag_point.y.round() as u16),
    };
    runner.model = Arc::new(drag_model);

    let mut frame = NativeViewFrame::default();
    runner
        .shell_state
        .build_state_overlay_into(&layout, &style, &runner.model, &mut frame);

    let overlay_color = frame
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Rect(rect) if rect.rect == hover_row => Some(rect.color),
            _ => None,
        })
        .expect("drag-hovered folder row should emit a fill rectangle");

    assert_eq!(overlay_color, folder_drag_hover_fill(&style, true));
}
