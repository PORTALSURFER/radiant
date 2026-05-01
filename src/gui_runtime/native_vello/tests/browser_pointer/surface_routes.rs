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
    let tree_rows = vec![
        crate::compat_app_contract::FolderRowModel::new(
            "Root", "", 0, true, false, true, true, true,
        ),
        crate::compat_app_contract::FolderRowModel::new(
            "Drums", "", 1, false, true, false, true, true,
        ),
        crate::compat_app_contract::FolderRowModel::new(
            "Kicks", "", 2, false, false, false, false, false,
        ),
    ];
    let mut source_a =
        crate::compat_app_contract::SourceRowModel::new("Source A", "ready", false, false);
    source_a.assigned_to_upper_pane = true;
    let mut source_b =
        crate::compat_app_contract::SourceRowModel::new("Source B", "ready", false, false);
    source_b.assigned_to_lower_pane = true;
    AppModel {
        sources: SourcesPanelModel {
            header: String::from("2 sources"),
            tree_search_query: String::from(query),
            selected_row: Some(0),
            focused_tree_row: Some(1),
            rows: vec![source_a, source_b].into(),
            tree_rows: tree_rows.clone().into(),
            upper_folder_pane: crate::compat_app_contract::FolderPaneModel {
                pane: crate::compat_app_contract::FolderPaneIdModel::Upper,
                title: String::from("Upper"),
                item_label: String::from("Source A"),
                item_detail: String::from("ready"),
                active: true,
                has_item: true,
                tree_search_query: String::from(query),
                focused_tree_row: Some(1),
                tree_rows: tree_rows.clone().into(),
                ..crate::compat_app_contract::FolderPaneModel::default()
            },
            lower_folder_pane: crate::compat_app_contract::FolderPaneModel {
                pane: crate::compat_app_contract::FolderPaneIdModel::Lower,
                title: String::from("Lower"),
                item_label: String::from("Source B"),
                item_detail: String::from("ready"),
                active: false,
                has_item: true,
                focused_tree_row: Some(1),
                tree_rows: tree_rows.into(),
                ..crate::compat_app_contract::FolderPaneModel::default()
            },
            ..SourcesPanelModel::default()
        },
        ..AppModel::default()
    }
}

mod map_and_toolbar_routes;
mod sidebar_focus_routes;
