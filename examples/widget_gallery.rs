//! Gallery of reusable application-builder widgets.

use radiant::prelude::*;
use radiant::widgets::ColorMarkerAlign;

#[derive(Clone, Debug, PartialEq, Eq)]
enum GalleryMessage {
    SetSelected(bool),
    SetArmed(bool),
    SetStatus(&'static str),
    Reset,
}

#[derive(Clone, Debug)]
struct GalleryState {
    selected: bool,
    armed: bool,
    status: String,
}

impl Default for GalleryState {
    fn default() -> Self {
        Self {
            selected: true,
            armed: false,
            status: "ready".to_string(),
        }
    }
}

fn main() -> radiant::Result {
    radiant::app(GalleryState::default())
        .title("Radiant Widget Gallery")
        .size(640, 380)
        .min_size(480, 280)
        .view(project_surface)
        .update(update)
        .run()
}

fn project_surface(state: &GalleryState) -> View<GalleryMessage> {
    column([
        row([
            text("Widget Gallery").height(30.0).fill_width(),
            badge(format!("Status: {}", state.status))
                .primary()
                .message(GalleryMessage::SetStatus("acknowledged"))
                .size(148.0, 28.0),
        ])
        .fill_width()
        .spacing(10.0),
        grid_with_gaps(
            [
                control_tile(
                    "Badges",
                    "Compact labels can still emit activation messages.",
                    row([
                        badge("Ready").message(GalleryMessage::SetStatus("ready")),
                        badge("Warning")
                            .danger()
                            .message(GalleryMessage::SetStatus("warning")),
                    ])
                    .spacing(8.0),
                ),
                control_tile(
                    "Selectables",
                    "Selectable surfaces emit value messages for the update path.",
                    row([
                        selectable("Primary", state.selected)
                            .primary()
                            .message(GalleryMessage::SetSelected),
                        selectable("Armed", state.armed)
                            .subtle()
                            .color_marker(Some(Rgba8::new(255, 178, 76, 230)))
                            .color_marker_side(8)
                            .color_marker_align(ColorMarkerAlign::Left)
                            .message(GalleryMessage::SetArmed),
                    ])
                    .spacing(8.0),
                ),
                stack([
                    card().fill(),
                    column([
                        text("Card").height(24.0).fill_width(),
                        text("Cards provide passive panel chrome for composed content.")
                            .wrap()
                            .height(48.0)
                            .fill_width(),
                        button("Reset").subtle().message(GalleryMessage::Reset),
                    ])
                    .padding(12.0)
                    .spacing(8.0)
                    .fill(),
                ])
                .height(148.0)
                .fill_width(),
            ],
            3,
            12.0,
            12.0,
        )
        .fill_width(),
        text(summary_text(state)).height(26.0).fill_width(),
    ])
    .style(WidgetStyle::default())
    .padding(16.0)
    .spacing(12.0)
    .fill()
}

fn control_tile(
    title: &'static str,
    description: &'static str,
    controls: View<GalleryMessage>,
) -> View<GalleryMessage> {
    column([
        text(title).height(24.0).fill_width(),
        text(description).wrap().height(48.0).fill_width(),
        controls.fill_width(),
    ])
    .style(WidgetStyle {
        tone: WidgetTone::Neutral,
        prominence: WidgetProminence::Subtle,
    })
    .padding(12.0)
    .spacing(8.0)
    .height(148.0)
    .fill_width()
}

fn update(state: &mut GalleryState, message: GalleryMessage) {
    match message {
        GalleryMessage::SetSelected(selected) => {
            state.selected = selected;
            state.status = if selected { "selected" } else { "cleared" }.to_string();
        }
        GalleryMessage::SetArmed(armed) => state.armed = armed,
        GalleryMessage::SetStatus(status) => state.status = status.to_string(),
        GalleryMessage::Reset => *state = GalleryState::default(),
    }
}

fn summary_text(state: &GalleryState) -> String {
    format!(
        "selected={} armed={} status={}",
        state.selected, state.armed, state.status
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use radiant::prelude::IntoView;

    #[test]
    fn widget_gallery_lowers_public_widgets() {
        let mut state = GalleryState::default();
        let surface = project_surface(&mut state).into_surface();
        let layout = radiant::layout::layout_tree(
            &surface.layout_node(),
            radiant::gui::types::Rect::from_min_size(
                radiant::gui::types::Point::new(0.0, 0.0),
                radiant::gui::types::Vector2::new(640.0, 380.0),
            ),
        );

        assert_eq!(surface.root().id(), 1);
        assert!(layout.rects.contains_key(&2));
        assert!(surface.keyboard_focus_order().len() >= 6);
    }
}
