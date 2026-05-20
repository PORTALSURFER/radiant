use radiant::{
    prelude::{IntoView, WidgetTone},
    theme::ViewportScaleTier,
};

use crate::{model::PlaygroundState, report::theme_report, view::project_surface};

#[test]
fn theme_playground_resolves_density_and_visual_tokens() {
    let compact = theme_report(720.0, 0.5);
    let wide = theme_report(2400.0, 1.6);

    assert_eq!(compact.tier, ViewportScaleTier::Compact);
    assert_eq!(wide.tier, ViewportScaleTier::Wide);
    assert!(wide.scale > compact.scale);
    assert_ne!(wide.accent_fill, wide.danger_fill);

    let mut state = PlaygroundState::default();
    let surface = project_surface(&mut state).into_surface();
    assert!(surface.keyboard_focus_order().len() >= 8);
}

#[test]
fn theme_playground_projects_distinct_tone_and_state_controls() {
    let mut state = PlaygroundState {
        selected_tone: WidgetTone::Danger,
        active_preview: false,
    };

    let surface = project_surface(&mut state).into_surface();

    assert!(
        surface.keyboard_focus_order().len() >= 8,
        "tone buttons, prominence controls, and state controls should all be interactive"
    );
}
