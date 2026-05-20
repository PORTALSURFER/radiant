//! Theme-token, tone, prominence, and density-policy playground.

#[path = "theme_playground/model.rs"]
mod model;
#[path = "theme_playground/report.rs"]
mod report;
#[path = "theme_playground/view.rs"]
mod view;

use model::{PlaygroundMessage, PlaygroundState};
use report::theme_report;
use view::project_surface;

#[cfg(test)]
#[path = "theme_playground/tests.rs"]
mod tests;

fn main() -> radiant::Result {
    let report = theme_report(1280.0, 1.25);
    println!(
        "radiant_theme_playground tier={:?} scale={:.3} accent_fill={:?} danger_fill={:?}",
        report.tier, report.scale, report.accent_fill, report.danger_fill
    );

    radiant::app(PlaygroundState::default())
        .title("Radiant Theme Playground")
        .size(760, 560)
        .min_size(520, 420)
        .view(project_surface)
        .update(|state, message| match message {
            PlaygroundMessage::SelectTone(tone) => state.selected_tone = tone,
            PlaygroundMessage::ToggleActive(active) => state.active_preview = active,
        })
        .run()
}
