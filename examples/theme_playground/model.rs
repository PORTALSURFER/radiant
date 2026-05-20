use radiant::prelude::WidgetTone;

#[derive(Clone, Debug, PartialEq)]
pub(super) enum PlaygroundMessage {
    SelectTone(WidgetTone),
    ToggleActive(bool),
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct PlaygroundState {
    pub(super) selected_tone: WidgetTone,
    pub(super) active_preview: bool,
}

impl Default for PlaygroundState {
    fn default() -> Self {
        Self {
            selected_tone: WidgetTone::Accent,
            active_preview: true,
        }
    }
}
