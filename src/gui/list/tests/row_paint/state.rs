use super::fixtures::*;

#[test]
fn dense_row_fill_color_prioritizes_active_interaction_states() {
    assert_eq!(
        dense_row_fill_color(
            DenseRowVisualState {
                selected: true,
                hovered: true,
                pressed: true,
                active_target: true,
                candidate: true,
            },
            palette(),
        ),
        Some(ACTIVE)
    );
    assert_eq!(
        dense_row_fill_color(
            DenseRowVisualState {
                selected: true,
                hovered: true,
                candidate: true,
                ..DenseRowVisualState::default()
            },
            palette(),
        ),
        Some(CANDIDATE)
    );
    assert_eq!(
        dense_row_fill_color(
            DenseRowVisualState {
                selected: true,
                hovered: true,
                pressed: true,
                ..DenseRowVisualState::default()
            },
            palette(),
        ),
        Some(PRESSED)
    );
}

#[test]
fn dense_row_fill_color_uses_selection_as_base_state() {
    assert_eq!(
        dense_row_fill_color(
            DenseRowVisualState {
                selected: true,
                ..DenseRowVisualState::default()
            },
            palette(),
        ),
        Some(SELECTED)
    );
    assert_eq!(
        dense_row_fill_color(DenseRowVisualState::default(), palette()),
        None
    );
}

#[test]
fn dense_row_visual_state_reports_label_emphasis() {
    assert!(
        DenseRowVisualState {
            selected: true,
            ..DenseRowVisualState::default()
        }
        .emphasizes_label()
    );
    assert!(
        DenseRowVisualState {
            active_target: true,
            ..DenseRowVisualState::default()
        }
        .emphasizes_label()
    );
    assert!(
        DenseRowVisualState {
            hovered: true,
            candidate: true,
            ..DenseRowVisualState::default()
        }
        .emphasizes_label()
    );
    assert!(
        !DenseRowVisualState {
            hovered: true,
            ..DenseRowVisualState::default()
        }
        .emphasizes_label()
    );
}

#[test]
fn dense_row_label_font_size_tracks_compact_row_height() {
    assert_eq!(dense_row_label_font_size(22.0), 13.0);
    assert_eq!(dense_row_label_font_size(28.0), 14.0);
    assert_eq!(dense_row_label_font_size(38.0), 18.0);
}
