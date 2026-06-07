//! Declarative root layer host example.

use radiant::prelude::*;
use radiant::{
    gui::types::Rect,
    layout::{Point, Vector2},
};

#[derive(Clone, Debug)]
struct LayerHostExampleState {
    floating_open: bool,
    popover_open: bool,
    modal_open: bool,
    context_menu_open: bool,
    tooltip_open: bool,
    drag_preview_open: bool,
}

impl Default for LayerHostExampleState {
    fn default() -> Self {
        Self {
            floating_open: true,
            popover_open: true,
            modal_open: true,
            context_menu_open: true,
            tooltip_open: true,
            drag_preview_open: true,
        }
    }
}

impl LayerHostExampleState {
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
    radiant::app(LayerHostExampleState::default())
        .title("Radiant Layer Host")
        .size(560, 360)
        .min_size(460, 280)
        .view(|state| {
            layer_host(base_layout(state))
                .floating_opt(state.floating_open.then(floating_layer_slot))
                .popover_opt(state.popover_open.then(popover_slot))
                .modal_opt(state.modal_open.then(modal_slot))
                .context_menu_opt(state.context_menu_open.then(context_menu_slot))
                .tooltip_opt(state.tooltip_open.then(tooltip_slot))
                .drag_preview_opt(state.drag_preview_open.then(drag_preview_slot))
                .into_view()
                .fill()
        })
        .run()
}

fn base_layout(state: &LayerHostExampleState) -> StateView<LayerHostExampleState> {
    column([
        text("Layer Host").height(28.0).fill_width(),
        text("Each toggle changes state; Radiant assembles the root layer order.")
            .height(24.0)
            .fill_width(),
        toggle_button(
            "Floating",
            state.floating_open,
            LayerHostExampleState::toggle_floating,
        ),
        toggle_button(
            "Popover",
            state.popover_open,
            LayerHostExampleState::toggle_popover,
        ),
        toggle_button(
            "Modal",
            state.modal_open,
            LayerHostExampleState::toggle_modal,
        ),
        toggle_button(
            "Context menu",
            state.context_menu_open,
            LayerHostExampleState::toggle_context_menu,
        ),
        toggle_button(
            "Tooltip",
            state.tooltip_open,
            LayerHostExampleState::toggle_tooltip,
        ),
        toggle_button(
            "Drag preview",
            state.drag_preview_open,
            LayerHostExampleState::toggle_drag_preview,
        ),
    ])
    .padding(16.0)
    .spacing(8.0)
    .fill_width()
    .fill_height()
}

fn toggle_button(
    label: &'static str,
    open: bool,
    toggle: fn(&mut LayerHostExampleState),
) -> StateView<LayerHostExampleState> {
    let state_label = if open { "visible" } else { "hidden" };
    button(format!("{label}: {state_label}"))
        .on_click(toggle)
        .height(30.0)
        .width(180.0)
}

fn floating_layer_slot() -> StateView<LayerHostExampleState> {
    floating_layer(
        Point::new(232.0, 42.0),
        Vector2::new(160.0, 58.0),
        panel("Floating", "Generic floating layer"),
    )
    .key("layer-host-floating")
}

fn popover_slot() -> StateView<LayerHostExampleState> {
    anchored_layer(
        panel("Popover", "Above generic floating layers"),
        Vector2::new(192.0, 64.0),
        LayerHorizontalAnchor::End,
        LayerVerticalAnchor::Start,
        18.0,
        18.0,
    )
    .key("layer-host-popover")
}

fn modal_slot() -> StateView<LayerHostExampleState> {
    centered_layer(
        panel("Modal", "Modals paint above popovers"),
        Vector2::new(220.0, 86.0),
    )
    .key("layer-host-modal")
}

fn context_menu_slot() -> StateView<LayerHostExampleState> {
    context_menu_overlay(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(560.0, 360.0)),
        Point::new(328.0, 226.0),
        Vector2::new(168.0, 116.0),
        "Context menu",
        [
            MenuItem::new("Inspect", LayerHostExampleState::close_context_menu).primary(),
            MenuItem::new("Duplicate", LayerHostExampleState::close_context_menu).subtle(),
            MenuItem::new("Close", LayerHostExampleState::close_context_menu).subtle(),
        ],
    )
    .key("layer-host-context-menu")
}

fn tooltip_slot() -> StateView<LayerHostExampleState> {
    floating_layer(
        Point::new(246.0, 140.0),
        Vector2::new(150.0, 34.0),
        text("Tooltip").height(24.0).fill_width(),
    )
    .key("layer-host-tooltip")
}

fn drag_preview_slot() -> StateView<LayerHostExampleState> {
    drag_preview("Drag preview", Point::new(408.0, 80.0)).key("layer-host-drag-preview")
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
