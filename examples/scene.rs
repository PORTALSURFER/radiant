//! Declarative scene root example.

use radiant::Layer;
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
    playback_overlay_running: bool,
    frame: u32,
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
            playback_overlay_running: true,
            frame: 0,
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
    TogglePlaybackOverlay,
    Frame,
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

    fn toggle_playback_overlay(&mut self) {
        self.playback_overlay_running = !self.playback_overlay_running;
    }

    fn advance_frame(&mut self) {
        self.frame = self.frame.wrapping_add(1);
    }
}

fn main() -> radiant::Result {
    radiant::app(SceneExampleState::default())
        .title("Radiant Scene")
        .size(560, 360)
        .min_size(460, 280)
        .view(|state| {
            scene(base_layout(state))
                .shortcuts(scene_shortcuts(state))
                .frame_clock(
                    FrameClock::message(SceneExampleMessage::Frame)
                        .when(|state: &mut SceneExampleState| state.playback_overlay_running)
                        .fps(60),
                )
                .overlay(
                    TransientOverlay::new(1_u64)
                        .paint_only()
                        .when(|state: &mut SceneExampleState| state.playback_overlay_running)
                        .fps(60)
                        .paint(paint_playback_cursor),
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
            SceneExampleMessage::TogglePlaybackOverlay => state.toggle_playback_overlay(),
            SceneExampleMessage::Frame => state.advance_frame(),
        })
        .run()
}

fn base_layout(state: &SceneExampleState) -> ViewNode<SceneExampleMessage> {
    column([
        status_bar(state),
        browser_area(state),
        modal_owner(state),
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
        toggle_button(
            "Playback overlay",
            state.playback_overlay_running,
            SceneExampleMessage::TogglePlaybackOverlay,
        ),
    ])
    .padding(16.0)
    .spacing(8.0)
    .fill_width()
    .fill_height()
}

fn status_bar(state: &SceneExampleState) -> ViewNode<SceneExampleMessage> {
    row([
        text("Scene").height(28.0).fill_width(),
        toggle_button(
            "Popover",
            state.popover_open,
            SceneExampleMessage::TogglePopover,
        ),
        toggle_button(
            "Tooltip",
            state.tooltip_open,
            SceneExampleMessage::ToggleTooltip,
        ),
    ])
    .spacing(8.0)
    .fill_width()
    .overlays(
        overlays()
            .popover_opt(state.popover_open.then(popover_slot))
            .tooltip_opt(state.tooltip_open.then(tooltip_slot)),
    )
}

fn browser_area(state: &SceneExampleState) -> ViewNode<SceneExampleMessage> {
    panel(
        "Browser",
        "Context menu and drag preview are declared by this component.",
    )
    .height(58.0)
    .fill_width()
    .overlays(
        overlays()
            .floating_opt(state.floating_open.then(floating_layer_slot))
            .drag_preview_opt(state.drag_preview_open.then(drag_preview_slot)),
    )
    .overlays(overlays().layer_opt(state.context_menu_open.then(|| {
        Layer::context_menu(context_menu_slot())
            .dismiss_on_outside_click(SceneExampleMessage::CloseContextMenu)
    })))
}

fn modal_owner(state: &SceneExampleState) -> ViewNode<SceneExampleMessage> {
    panel(
        "Workspace",
        "The modal is owned by this workspace component.",
    )
    .height(58.0)
    .fill_width()
    .overlays(
        overlays().layer_opt(
            state
                .modal_open
                .then(|| Layer::modal(modal_slot()).block_input()),
        ),
    )
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

fn scene_shortcuts(state: &SceneExampleState) -> ShortcutCatalog<SceneExampleMessage> {
    ShortcutCatalog::new()
        .layer_when(
            state.context_menu_open,
            ShortcutLayer::modal_escape(SceneExampleMessage::CloseContextMenu),
        )
        .layer_when(
            state.modal_open,
            ShortcutLayer::modal_escape(SceneExampleMessage::ToggleModal),
        )
        .layer(ShortcutLayer::new().bind(
            KeyPress::new(KeyCode::Space),
            SceneExampleMessage::TogglePlaybackOverlay,
        ))
        .fallback(|press| {
            if press == KeyPress::new(KeyCode::ArrowDown) {
                ShortcutResolution::action(SceneExampleMessage::ToggleFloating)
            } else {
                ShortcutResolution::unhandled()
            }
        })
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
    context_menu(
        "Context menu",
        [
            MenuCommand::new("Inspect", SceneExampleMessage::CloseContextMenu).primary(),
            MenuCommand::new("Duplicate", SceneExampleMessage::CloseContextMenu).subtle(),
            MenuCommand::new("Close", SceneExampleMessage::CloseContextMenu).subtle(),
        ],
    )
    .anchor(Point::new(328.0, 226.0))
    .size(Vector2::new(168.0, 116.0))
    .view()
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

fn paint_playback_cursor(
    state: &mut SceneExampleState,
    _context: TransientOverlayContext<'_>,
    primitives: &mut Vec<PaintPrimitive>,
) {
    let x = 24.0 + (state.frame % 480) as f32;
    primitives.push(PaintPrimitive::FillRect(PaintFillRect {
        widget_id: 0,
        rect: Rect::from_min_size(Point::new(x, 28.0), Vector2::new(2.0, 292.0)),
        color: Rgba8::new(255, 126, 64, 220),
    }));
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
