//! Bottom status bar with one-line updates and progress-aware background work.

#[path = "status_bar/model.rs"]
mod model;
#[path = "status_bar/paint.rs"]
mod paint;
#[path = "status_bar/update.rs"]
mod update;
#[path = "status_bar/view.rs"]
mod view;

#[cfg(test)]
#[path = "status_bar/tests.rs"]
mod tests;

use model::{StatusBarState, StatusMessage};
use paint::{STATUS_PROGRESS_KEY, progress_frame};
use radiant::theme::ThemeTokens;
use update::update;
use view::project_surface;

fn main() -> radiant::Result {
    radiant::app(StatusBarState::default())
        .title("Radiant Status Bar")
        .size(660, 340)
        .min_size(520, 260)
        .view(project_surface)
        .animation(|state| state.active_count() > 0 || state.animation_running)
        .on_frame(|| StatusMessage::Frame)
        .retained_painter(
            STATUS_PROGRESS_KEY,
            |state, _descriptor, rect, _viewport| {
                Some(progress_frame(state, rect, &ThemeTokens::default()))
            },
        )
        .handle_message(update)
        .run()
}
