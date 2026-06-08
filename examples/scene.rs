//! Declarative scene root example.

use radiant::layout::{Point, Vector2};
use radiant::prelude::*;

#[derive(Clone, Debug)]
struct SceneExampleState {
    local_overlay_open: bool,
    floating_open: bool,
    popover_open: bool,
    modal_open: bool,
    context_menu_open: bool,
    tooltip_open: bool,
    drag_preview_open: bool,
}

impl Default for SceneExampleState {
    fn default() -> Self {
        Self {
            local_overlay_open: true,
            floating_open: true,
            popover_open: true,
            modal_open: true,
            context_menu_open: true,
            tooltip_open: true,
            drag_preview_open: true,
        }
    }
}

#[derive(Clone, Debug)]
enum SceneExampleMessage {
    ToggleLocalOverlay,
    ToggleFloating,
    TogglePopover,
    ToggleModal,
    ToggleContextMenu,
    CloseContextMenu,
    ToggleTooltip,
    ToggleDragPreview,
}

impl SceneExampleState {
    fn toggle_local_overlay(&mut self) {
        self.local_overlay_open = !self.local_overlay_open;
    }

    fn toggle_floating(&mut self) {
        self.floating_open = !self.floating_open;
    }

    fn toggle_popover(&mut self) {
        self.popover_open = !self.popover_open;
    }

    fn toggle_modal(&mut self) {
        self.modal_open = !self.modal_open;
    }

    fn toggle_context_menu(&mut self) {
        self.context_menu_open = !self.context_menu_open;
    }

    fn close_context_menu(&mut self) {
        self.context_menu_open = false;
    }

    fn toggle_tooltip(&mut self) {
        self.tooltip_open = !self.tooltip_open;
    }

    fn toggle_drag_preview(&mut self) {
        self.drag_preview_open = !self.drag_preview_open;
    }
}

fn main() -> radiant::Result {
    radiant::app(SceneExampleState::default())
        .title("Radiant Scene")
        .size(560, 360)
        .min_size(460, 280)
        .view(|state| {
            scene(base_layout(state))
                .layer_opt(
                    state
                        .floating_open
                        .then(|| Layer::floating(floating_layer_slot())),
                )
                .layer_opt(state.popover_open.then(|| Layer::popover(popover_slot())))
                .layer_opt(
                    state
                        .modal_open
                        .then(|| Layer::modal(modal_slot()).block_input()),
                )
                .layer_opt(state.context_menu_open.then(|| {
                    Layer::context_menu(context_menu_slot())
                        .dismiss_on_outside_click(SceneExampleMessage::CloseContextMenu)
                }))
                .layer_opt(
                    state
                        .tooltip_open
                        .then(|| Layer::tooltip(tooltip_slot()).pass_through()),
                )
                .layer_opt(
                    state
                        .drag_preview_open
                        .then(|| Layer::drag_preview(drag_preview_slot()).pass_through()),
                )
                .into_view()
                .fill()
        })
        .update(|state, message| match message {
            SceneExampleMessage::ToggleLocalOverlay => state.toggle_local_overlay(),
            SceneExampleMessage::ToggleFloating => state.toggle_floating(),
            SceneExampleMessage::TogglePopover => state.toggle_popover(),
            SceneExampleMessage::ToggleModal => state.toggle_modal(),
            SceneExampleMessage::ToggleContextMenu => state.toggle_context_menu(),
            SceneExampleMessage::CloseContextMenu => state.close_context_menu(),
            SceneExampleMessage::ToggleTooltip => state.toggle_tooltip(),
            SceneExampleMessage::ToggleDragPreview => state.toggle_drag_preview(),
        })
        .run()
}

fn base_layout(state: &SceneExampleState) -> ViewNode<SceneExampleMessage> {
    column([
        text("Scene").height(28.0).fill_width(),
        text("Each toggle changes state; Radiant assembles the root scene order.")
            .height(24.0)
            .fill_width(),
        local_overlay_demo(state),
        toggle_button(
            "Local overlay",
            state.local_overlay_open,
            SceneExampleMessage::ToggleLocalOverlay,
        ),
        toggle_button(
            "Floating",
            state.floating_open,
            SceneExampleMessage::ToggleFloating,
        ),
        toggle_button(
            "Popover",
            state.popover_open,
            SceneExampleMessage::TogglePopover,
        ),
        toggle_button("Modal", state.modal_open, SceneExampleMessage::ToggleModal),
        toggle_button(
            "Context menu",
            state.context_menu_open,
            SceneExampleMessage::ToggleContextMenu,
        ),
        toggle_button(
            "Tooltip",
            state.tooltip_open,
            SceneExampleMessage::ToggleTooltip,
        ),
        toggle_button(
            "Drag preview",
            state.drag_preview_open,
            SceneExampleMessage::ToggleDragPreview,
        ),
    ])
    .padding(16.0)
    .spacing(8.0)
    .fill_width()
    .fill_height()
}

fn local_overlay_demo(state: &SceneExampleState) -> ViewNode<SceneExampleMessage> {
    overlay_stack(
        panel(
            "Bounded content",
            "Local overlays share this region rather than the root scene.",
        )
        .height(58.0)
        .fill_width(),
    )
    .overlay_opt(state.local_overlay_open.then(|| {
        feedback_overlay()
            .background(Rgba8::new(74, 178, 116, 48))
            .edge(
                Rgba8::new(74, 178, 116, 210),
                2.0,
                BorderSides {
                    top: true,
                    bottom: true,
                    left: true,
                    right: true,
                },
            )
            .view()
            .key("scene-local-overlay")
            .height(58.0)
            .fill_width()
    }))
    .into_view()
    .height(58.0)
    .fill_width()
}

fn toggle_button(
    label: &'static str,
    open: bool,
    message: SceneExampleMessage,
) -> ViewNode<SceneExampleMessage> {
    let state_label = if open { "visible" } else { "hidden" };
    button(format!("{label}: {state_label}"))
        .message(message)
        .height(30.0)
        .width(180.0)
}

fn floating_layer_slot() -> ViewNode<SceneExampleMessage> {
    floating_layer(
        Point::new(232.0, 42.0),
        Vector2::new(160.0, 58.0),
        panel("Floating", "Generic floating layer"),
    )
    .key("scene-floating")
}

fn popover_slot() -> ViewNode<SceneExampleMessage> {
    anchored_layer(
        panel("Popover", "Above generic floating layers"),
        Vector2::new(192.0, 64.0),
        LayerHorizontalAnchor::End,
        LayerVerticalAnchor::Start,
        18.0,
        18.0,
    )
    .key("scene-popover")
}

fn modal_slot() -> ViewNode<SceneExampleMessage> {
    centered_layer(
        panel("Modal", "Modals paint above popovers"),
        Vector2::new(220.0, 86.0),
    )
    .key("scene-modal")
}

fn context_menu_slot() -> ViewNode<SceneExampleMessage> {
    message_context_menu_overlay(
        Point::new(328.0, 226.0),
        Vector2::new(168.0, 116.0),
        "Context menu",
        [
            MenuCommand::new("Inspect", SceneExampleMessage::CloseContextMenu).primary(),
            MenuCommand::new("Duplicate", SceneExampleMessage::CloseContextMenu).subtle(),
            MenuCommand::new("Close", SceneExampleMessage::CloseContextMenu).subtle(),
        ],
    )
    .key("scene-context-menu")
}

fn tooltip_slot() -> ViewNode<SceneExampleMessage> {
    floating_layer(
        Point::new(246.0, 140.0),
        Vector2::new(150.0, 34.0),
        text("Tooltip").height(24.0).fill_width(),
    )
    .key("scene-tooltip")
}

fn drag_preview_slot() -> ViewNode<SceneExampleMessage> {
    drag_preview("Drag preview", Point::new(408.0, 80.0)).key("scene-drag-preview")
}

fn panel<Message: 'static>(title: &'static str, detail: &'static str) -> ViewNode<Message> {
    column([
        text(title).height(22.0).fill_width(),
        text(detail).height(24.0).fill_width().truncate(),
    ])
    .style(WidgetStyle {
        tone: WidgetTone::Accent,
        prominence: WidgetProminence::Strong,
    })
    .padding(8.0)
    .spacing(4.0)
    .fill_width()
    .fill_height()
}
