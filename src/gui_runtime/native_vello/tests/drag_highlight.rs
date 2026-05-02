use super::*;
use crate::gui::types::Rect;

fn browser_drag_model() -> AppModel {
    let mut model = browser_model_with_rows(4, 0);
    let tree_rows = vec![
        crate::compat_app_contract::FolderRowModel::new(
            "Root", "", 0, false, false, true, true, true,
        )
        .with_backing_index(0),
        crate::compat_app_contract::FolderRowModel::new(
            "Drums", "drums", 1, false, false, false, true, true,
        )
        .with_backing_index(7),
    ];
    model.sources = SourcesPanelModel {
        tree_rows: tree_rows.clone().into(),
        upper_folder_pane: crate::compat_app_contract::FolderPaneModel {
            pane: crate::compat_app_contract::FolderPaneIdModel::Upper,
            title: String::from("Upper"),
            active: true,
            has_item: true,
            tree_rows: tree_rows.clone().into(),
            ..crate::compat_app_contract::FolderPaneModel::default()
        },
        lower_folder_pane: crate::compat_app_contract::FolderPaneModel {
            pane: crate::compat_app_contract::FolderPaneIdModel::Lower,
            title: String::from("Lower"),
            has_item: true,
            tree_rows: tree_rows.into(),
            ..crate::compat_app_contract::FolderPaneModel::default()
        },
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
    let row_rect = shell_state.rendered_folder_row_rects(layout, model)[row_index];
    let seam_stroke = StyleTokens::for_viewport_width(layout.root.rect.width())
        .sizing
        .border_width
        .max(1.0);
    Point::new(
        row_rect.min.x + seam_stroke + 2.0,
        (row_rect.min.y + row_rect.max.y) * 0.5,
    )
}

fn folder_panel_background_point(
    shell_state: &mut NativeShellState,
    layout: &ShellLayout,
    model: &AppModel,
) -> Point {
    for x in layout.sidebar_rows.min.x as i32..=layout.sidebar_rows.max.x as i32 {
        for y in layout.sidebar_rows.min.y as i32..=layout.sidebar_rows.max.y as i32 {
            let point = Point::new(x as f32, y as f32);
            if shell_state.folder_panel_contains_point(layout, model, point)
                && shell_state
                    .folder_row_at_point(layout, model, point)
                    .is_none()
                && shell_state
                    .source_row_at_point(layout, model, point)
                    .is_none()
            {
                return point;
            }
        }
    }
    panic!("expected hittable folder-panel background point");
}

fn valid_folder_drag_hover_fill(style: &StyleTokens) -> Rgba8 {
    let expected_alpha = (style.state_hover_soft * 2.1).clamp(0.22, 0.46);
    let mix = |from: u8, to: u8| -> u8 {
        ((from as f32) + ((to as f32 - from as f32) * expected_alpha))
            .round()
            .clamp(0.0, 255.0) as u8
    };
    Rgba8 {
        r: mix(style.bg_tertiary.r, style.accent_mint.r),
        g: mix(style.bg_tertiary.g, style.accent_mint.g),
        b: mix(style.bg_tertiary.b, style.accent_mint.b),
        a: (expected_alpha
            * (style.bg_tertiary.a as f32 / 255.0)
            * (style.accent_mint.a as f32 / 255.0)
            * 255.0)
            .round()
            .clamp(0.0, 255.0) as u8,
    }
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
    let hover_row = runner
        .shell_state
        .rendered_folder_row_rects(&layout, &runner.model)[1];
    let seam_stroke = style.sizing.border_width.max(1.0);
    let hover_visual_rect = Rect::from_min_max(
        Point::new(hover_row.min.x + seam_stroke, hover_row.min.y),
        Point::new(hover_row.max.x - seam_stroke, hover_row.max.y),
    );
    runner.last_cursor = Some(press_point);

    assert!(
        runner.handle_pointer_press_action(UiAction::FocusBrowserRow { visible_row: 0 }, false)
    );
    runner.handle_cursor_moved_for_tests(drag_point);

    assert_eq!(
        runner.bridge.actions,
        vec![
            UiAction::StartContentItemDrag {
                visible_row: 0,
                pointer_x: drag_point.x.round() as u16,
                pointer_y: drag_point.y.round() as u16,
            },
            UiAction::UpdateContentItemDrag {
                pointer_x: drag_point.x.round() as u16,
                pointer_y: drag_point.y.round() as u16,
                hovered_folder_pane: Some(crate::compat_app_contract::FolderPaneIdModel::Upper),
                hovered_folder_row: Some(7),
                over_folder_panel: Some(crate::compat_app_contract::FolderPaneIdModel::Upper),
                shift_down: false,
                alt_down: false,
            },
        ]
    );
    assert_eq!(runner.shell_state.hovered_folder_row_index(), Some(1));

    let mut drag_model = Arc::unwrap_or_clone(runner.model.clone());
    drag_model.drag_overlay = crate::compat_app_contract::DragOverlayModel {
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
            Primitive::Rect(rect) if rect.rect == hover_visual_rect => Some(rect.color),
            _ => None,
        })
        .expect("drag-hovered folder row should emit a fill rectangle");

    assert_eq!(overlay_color, valid_folder_drag_hover_fill(&style));
}

#[test]
fn browser_row_drag_over_folder_panel_background_does_not_highlight_row() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(layout.root.rect.width());
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    runner.model = Arc::new(browser_drag_model());
    runner.shell_layout = Some(Arc::new(layout.clone()));
    let press_point = browser_row_point(&layout);
    let drag_point = folder_panel_background_point(&mut runner.shell_state, &layout, &runner.model);
    let seam_stroke = style.sizing.border_width.max(1.0);
    let folder_visual_rects: Vec<Rect> = runner
        .shell_state
        .rendered_folder_row_rects(&layout, &runner.model)
        .into_iter()
        .map(|row| {
            Rect::from_min_max(
                Point::new(row.min.x + seam_stroke, row.min.y),
                Point::new(row.max.x - seam_stroke, row.max.y),
            )
        })
        .collect();
    runner.last_cursor = Some(press_point);

    assert!(
        runner.handle_pointer_press_action(UiAction::FocusBrowserRow { visible_row: 0 }, false)
    );
    runner.handle_cursor_moved_for_tests(drag_point);

    assert_eq!(
        runner.bridge.actions,
        vec![
            UiAction::StartContentItemDrag {
                visible_row: 0,
                pointer_x: drag_point.x.round() as u16,
                pointer_y: drag_point.y.round() as u16,
            },
            UiAction::UpdateContentItemDrag {
                pointer_x: drag_point.x.round() as u16,
                pointer_y: drag_point.y.round() as u16,
                hovered_folder_pane: None,
                hovered_folder_row: None,
                over_folder_panel: Some(crate::compat_app_contract::FolderPaneIdModel::Upper),
                shift_down: false,
                alt_down: false,
            },
        ]
    );
    assert_eq!(runner.shell_state.hovered_folder_row_index(), None);

    let mut drag_model = Arc::unwrap_or_clone(runner.model.clone());
    drag_model.drag_overlay = crate::compat_app_contract::DragOverlayModel {
        active: true,
        label: String::from("row_0000"),
        target_label: String::from("Folder panel"),
        valid_target: true,
        pointer_x: Some(drag_point.x.round() as u16),
        pointer_y: Some(drag_point.y.round() as u16),
    };
    runner.model = Arc::new(drag_model);

    let mut frame = NativeViewFrame::default();
    runner
        .shell_state
        .build_state_overlay_into(&layout, &style, &runner.model, &mut frame);

    let expected_hover = valid_folder_drag_hover_fill(&style);
    assert!(frame.primitives.iter().all(|primitive| {
        !matches!(
            primitive,
            Primitive::Rect(rect)
                if folder_visual_rects.contains(&rect.rect) && rect.color == expected_hover
        )
    }));
}
