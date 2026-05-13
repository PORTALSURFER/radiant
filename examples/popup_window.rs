//! Floating popup window sandbox for drag previews and tooltip-style surfaces.

use radiant::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PopupMode {
    DragPreview,
    Tooltip,
}

#[derive(Clone, Debug)]
struct PopupDemoState {
    mode: PopupMode,
    pinned: bool,
}

impl Default for PopupDemoState {
    fn default() -> Self {
        Self {
            mode: PopupMode::DragPreview,
            pinned: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PopupMessage {
    ShowDragPreview,
    ShowTooltip,
    TogglePinned,
}

fn main() -> radiant::Result {
    radiant::app(PopupDemoState::default())
        .title("Radiant Floating Popup")
        .size(300, 132)
        .floating_popup()
        .popup_policy(popup_policy())
        .view(popup_view)
        .update(update)
        .run()
}

fn popup_policy() -> NativePopupOptions {
    NativePopupOptions::default()
        .position(420.0, 260.0)
        .transparent(true)
        .always_on_top(true)
        .initially_focused(false)
        .skip_taskbar(true)
}

#[cfg(test)]
fn popup_spec() -> WindowSpec {
    WindowSpec::popup("drag-preview-popup", "Radiant Floating Popup")
        .logical_size(300.0, 132.0)
        .popup_policy(popup_policy())
}

fn popup_view(state: &mut PopupDemoState) -> View<PopupMessage> {
    let (title, detail) = match state.mode {
        PopupMode::DragPreview => (
            "Drag preview",
            "Audio Clip 04 is being dragged outside the main window.",
        ),
        PopupMode::Tooltip => (
            "Tooltip",
            "Transient help can use the same borderless popup surface.",
        ),
    };
    let status_badge = if state.pinned {
        badge("Pinned")
            .primary()
            .message(PopupMessage::TogglePinned)
            .id(10)
            .size(86.0, 26.0)
    } else {
        badge("Floating")
            .subtle()
            .message(PopupMessage::TogglePinned)
            .id(10)
            .size(86.0, 26.0)
    };

    column([
        row([status_badge, text(title).id(11).height(26.0).fill_width()])
            .spacing(8.0)
            .fill_width(),
        text(detail).id(12).wrap().height(38.0).fill_width(),
        row([
            button("Drag")
                .message(PopupMessage::ShowDragPreview)
                .primary()
                .id(20)
                .size(72.0, 30.0),
            button("Tip")
                .message(PopupMessage::ShowTooltip)
                .subtle()
                .id(21)
                .size(62.0, 30.0),
            toggle("Pin", state.pinned)
                .message(|_| PopupMessage::TogglePinned)
                .id(22)
                .size(72.0, 30.0),
        ])
        .spacing(8.0)
        .fill_width(),
    ])
    .style(WidgetStyle::default())
    .padding(12.0)
    .spacing(8.0)
    .fill()
}

fn update(state: &mut PopupDemoState, message: PopupMessage) {
    match message {
        PopupMessage::ShowDragPreview => state.mode = PopupMode::DragPreview,
        PopupMessage::ShowTooltip => state.mode = PopupMode::Tooltip,
        PopupMessage::TogglePinned => state.pinned = !state.pinned,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use radiant::{
        runtime::UiSurface,
        widgets::{TextWidget, WidgetId},
    };

    #[test]
    fn popup_policy_describes_non_focused_transient_window() {
        let policy = popup_policy();

        assert_eq!(policy.position, Some([420.0, 260.0]));
        assert!(policy.transparent);
        assert!(policy.always_on_top);
        assert!(!policy.initially_focused);
        assert!(policy.skip_taskbar);
        assert!(!policy.resizable);
    }

    #[test]
    fn popup_spec_uses_borderless_popup_window_options() {
        let spec = popup_spec();

        assert!(spec.is_popup());
        assert_eq!(spec.key, "drag-preview-popup");
        assert_eq!(spec.inner_size(), Some([300.0, 132.0]));
        assert_eq!(
            spec.popup_options().and_then(|popup| popup.position),
            Some([420.0, 260.0])
        );
        assert!(!spec.native_options().decorations);
        assert!(!spec.drag_and_drop_enabled());
    }

    #[test]
    fn popup_view_switches_between_drag_preview_and_tooltip_copy() {
        let mut state = PopupDemoState::default();
        let drag_view = popup_view(&mut state).into_surface();
        assert_eq!(text(&drag_view, 11).text, "Drag preview");

        update(&mut state, PopupMessage::ShowTooltip);
        let tooltip_view = popup_view(&mut state).into_surface();
        assert_eq!(text(&tooltip_view, 11).text, "Tooltip");
    }

    fn text<Message>(surface: &UiSurface<Message>, id: WidgetId) -> &TextWidget {
        surface
            .find_widget(id)
            .expect("text widget should exist")
            .widget()
            .as_any()
            .downcast_ref::<TextWidget>()
            .expect("widget should be text")
    }
}
