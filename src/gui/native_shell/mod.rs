//! Backend-neutral native shell model used by the Vello runtime.
//!
//! The design mirrors a retained view tree (inspired by Xilem): build a
//! deterministic layout tree, run hit testing against that tree, then derive
//! backend-neutral paint primitives (shapes + text runs).

mod layout;
mod paint;
mod state;
mod style;

pub(crate) use layout::ShellLayout;
pub(crate) use layout::ShellNodeKind;
pub(crate) use paint::{Primitive, TextAlign, TextRun};
pub(crate) use state::NativeShellState;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::{
        input::KeyCode,
        types::{Point, Vector2},
    };

    #[test]
    fn layout_exposes_non_overlapping_columns() {
        let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
        assert!(layout.columns[0].max.x <= layout.columns[1].min.x);
        assert!(layout.columns[1].max.x <= layout.columns[2].min.x);
        assert!(layout.columns.iter().all(|column| column.width() > 40.0));
    }

    #[test]
    fn hit_test_prefers_column_node_inside_content() {
        let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
        let center = Point::new(
            (layout.columns[0].min.x + layout.columns[0].max.x) * 0.5,
            (layout.columns[0].min.y + layout.columns[0].max.y) * 0.5,
        );
        assert_eq!(
            layout.hit_test(center),
            Some(layout::ShellNodeKind::TriageColumn(0))
        );
    }

    #[test]
    fn primary_click_selects_clicked_column() {
        let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
        let mut state = NativeShellState::new();
        let point = Point::new(
            (layout.columns[2].min.x + layout.columns[2].max.x) * 0.5,
            (layout.columns[2].min.y + layout.columns[2].max.y) * 0.5,
        );
        assert!(state.handle_primary_click(&layout, point));
        let frame = state.build_frame(&layout, &crate::app::AppModel::default());
        assert!(frame.primitives.len() > 10);
        assert!(!frame.text_runs.is_empty());
    }

    #[test]
    fn arrow_keys_wrap_selection() {
        let mut state = NativeShellState::new();
        assert!(state.handle_key(KeyCode::ArrowRight));
        assert!(state.handle_key(KeyCode::ArrowRight));
        assert!(state.handle_key(KeyCode::ArrowRight));
        assert!(state.handle_key(KeyCode::ArrowLeft));
    }

    #[test]
    fn browser_row_hit_test_resolves_visible_row() {
        let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
        let mut state = NativeShellState::new();
        let mut model = crate::app::AppModel::default();
        model
            .browser
            .rows
            .push(crate::app::BrowserRowModel::new(7, "kick", 0, false, true));
        let style = style::StyleTokens::for_viewport_width(layout.root.rect.width());
        let row_center_y = layout.column_rows[0].min.y + (style.sizing.browser_row_height * 0.5);
        let point = Point::new(
            (layout.columns[0].min.x + layout.columns[0].max.x) * 0.5,
            row_center_y,
        );
        assert_eq!(state.browser_row_at_point(&layout, &model, point), Some(7));
        state.sync_from_model(&model);
        let frame = state.build_frame(&layout, &model);
        assert!(frame.text_runs.iter().any(|run| run.text == "kick"));
    }

    #[test]
    fn compact_layout_keeps_tight_header_and_footer_bands() {
        let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
        assert!(layout.top_bar.height() <= 40.0);
        assert!(layout.status_bar.height() <= 24.0);
        assert!(layout.waveform_card.height() >= 120.0);
    }

    #[test]
    fn compact_layout_preserves_content_on_narrow_viewports() {
        let layout = ShellLayout::build(Vector2::new(820.0, 520.0));
        assert!(layout.sidebar.width() >= 160.0);
        assert!(layout.content.width() >= 200.0);
        assert!(layout.columns.iter().all(|column| column.width() >= 40.0));
    }

    #[test]
    fn viewport_tier_sizing_changes_row_density() {
        let narrow = style::StyleTokens::for_viewport_width(820.0);
        let wide = style::StyleTokens::for_viewport_width(1900.0);
        assert!(narrow.sizing.browser_row_height < wide.sizing.browser_row_height);
        assert!(narrow.sizing.source_row_height < wide.sizing.source_row_height);
    }

    #[test]
    fn layout_bands_stay_within_panel_bounds() {
        let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
        assert!(layout.sidebar_header.max.y <= layout.sidebar_rows.min.y);
        assert!(layout.sidebar_rows.max.y <= layout.sidebar_footer.min.y);
        assert!(layout.waveform_header.max.y <= layout.waveform_plot.min.y);
        for index in 0..3 {
            assert!(layout.column_headers[index].max.y <= layout.column_rows[index].min.y);
            assert!(layout.column_rows[index].min.x >= layout.columns[index].min.x);
            assert!(layout.column_rows[index].max.x <= layout.columns[index].max.x);
        }
    }

    #[test]
    fn layout_uses_tokenized_shell_heights() {
        let width = 1280.0;
        let height = 720.0;
        let layout = ShellLayout::build(Vector2::new(width, height));
        let tokens = style::StyleTokens::for_viewport_width(width);
        assert!((layout.top_bar.height() - tokens.sizing.top_bar_height).abs() < 0.001);
        assert!((layout.status_bar.height() - tokens.sizing.status_bar_height).abs() < 0.001);
    }

    #[test]
    fn action_strip_hit_test_emits_browser_action() {
        let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
        let state = NativeShellState::new();
        let mut model = crate::app::AppModel::default();
        model.browser_actions.can_delete = true;
        let style = style::StyleTokens::for_viewport_width(layout.root.rect.width());
        let sizing = style.sizing;
        let button_count = 5.0;
        let total_width =
            (sizing.action_button_width * button_count) + (sizing.action_button_gap * 4.0);
        let start_x = (layout.top_bar.max.x - (sizing.text_inset_x + 32.0) - total_width)
            .max(layout.top_bar.min.x + 180.0);
        let y = (layout.top_bar.max.y - sizing.action_button_height - sizing.text_inset_y)
            .max(layout.top_bar.min.y + 1.0);
        let point = Point::new(
            start_x
                + ((sizing.action_button_width + sizing.action_button_gap) * 4.0)
                + (sizing.action_button_width * 0.5),
            y + (sizing.action_button_height * 0.5),
        );
        assert_eq!(
            state.browser_action_at_point(&layout, &model, point),
            Some(crate::app::UiAction::DeleteBrowserSelection)
        );
    }

    #[test]
    fn prompt_hit_test_emits_confirm_and_cancel() {
        let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
        let state = NativeShellState::new();
        let mut model = crate::app::AppModel::default();
        model.confirm_prompt.visible = true;
        let style = style::StyleTokens::for_viewport_width(layout.root.rect.width());
        let dialog = {
            let sizing = style.sizing;
            let width = sizing
                .prompt_width
                .min(layout.content.width() - (sizing.overlay_padding * 2.0))
                .max(260.0);
            let height = sizing
                .prompt_min_height
                .min(layout.content.height() - (sizing.overlay_padding * 2.0))
                .max(108.0);
            let x = layout.content.min.x + (layout.content.width() - width).max(0.0) * 0.5;
            let y = layout.content.min.y + (layout.content.height() - height).max(0.0) * 0.35;
            crate::gui::types::Rect::from_min_max(
                Point::new(x, y),
                Point::new(x + width, y + height),
            )
        };
        let cancel = {
            let sizing = style.sizing;
            crate::gui::types::Rect::from_min_max(
                Point::new(
                    dialog.max.x - sizing.overlay_button_width - sizing.text_inset_x,
                    dialog.max.y - sizing.overlay_button_height - sizing.text_inset_y,
                ),
                Point::new(
                    dialog.max.x - sizing.text_inset_x,
                    dialog.max.y - sizing.text_inset_y,
                ),
            )
        };
        let confirm = {
            let sizing = style.sizing;
            crate::gui::types::Rect::from_min_max(
                Point::new(
                    cancel.min.x - sizing.overlay_button_width - sizing.action_button_gap,
                    cancel.min.y,
                ),
                Point::new(cancel.min.x - sizing.action_button_gap, cancel.max.y),
            )
        };
        let confirm_point = Point::new(
            (confirm.min.x + confirm.max.x) * 0.5,
            (confirm.min.y + confirm.max.y) * 0.5,
        );
        let cancel_point = Point::new(
            (cancel.min.x + cancel.max.x) * 0.5,
            (cancel.min.y + cancel.max.y) * 0.5,
        );
        assert_eq!(
            state.prompt_action_at_point(&layout, &model, confirm_point),
            Some(crate::app::UiAction::ConfirmPrompt)
        );
        assert_eq!(
            state.prompt_action_at_point(&layout, &model, cancel_point),
            Some(crate::app::UiAction::CancelPrompt)
        );
    }
}
