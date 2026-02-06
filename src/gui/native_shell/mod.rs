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
        let point = Point::new(
            (layout.columns[0].min.x + layout.columns[0].max.x) * 0.5,
            (layout.columns[0].min.y + layout.columns[0].max.y) * 0.5,
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
}
