use super::*;

fn sidebar_focus_background_point(
    layout: &ShellLayout,
    model: &AppModel,
    shell_state: &mut NativeShellState,
    expected: UiAction,
) -> Point {
    let mut y = layout.sidebar_rows.min.y + 1.0;
    while y < layout.sidebar_rows.max.y {
        let mut x = layout.sidebar_rows.min.x + 1.0;
        while x < layout.sidebar_rows.max.x {
            let point = Point::new(x, y);
            if shell_state
                .source_row_at_point(layout, model, point)
                .is_none()
                && shell_state
                    .folder_row_at_point(layout, model, point)
                    .is_none()
                && shell_state.sidebar_focus_action_at_point(layout, model, point)
                    == Some(expected.clone())
            {
                return point;
            }
            x += 4.0;
        }
        y += 4.0;
    }
    panic!("failed to find sidebar background point for {expected:?}");
}

fn populated_sidebar_model() -> AppModel {
    populated_sidebar_model_with_search("")
}

fn populated_sidebar_model_with_search(query: &str) -> AppModel {
    AppModel {
        sources: SourcesPanelModel {
            header: String::from("2 sources"),
            folder_search_query: String::from(query),
            selected_row: Some(0),
            focused_folder_row: Some(1),
            rows: vec![
                crate::app::SourceRowModel::new("Source A", "ready", true, false),
                crate::app::SourceRowModel::new("Source B", "ready", false, false),
            ],
            folder_rows: vec![
                crate::app::FolderRowModel::new("Root", "", 0, true, false, true, true, true),
                crate::app::FolderRowModel::new("Drums", "", 1, false, true, false, true, true),
                crate::app::FolderRowModel::new("Kicks", "", 2, false, false, false, false, false),
            ],
            ..SourcesPanelModel::default()
        },
        ..AppModel::default()
    }
}

mod map_and_toolbar_routes;
mod sidebar_focus_routes;
