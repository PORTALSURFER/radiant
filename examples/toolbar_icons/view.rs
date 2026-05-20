use super::*;
use crate::icon_button::IconToggleButton;
use crate::model::{ToolId, ToolMessage, ToolbarState};

pub(super) fn project_surface(state: &mut ToolbarState) -> View<ToolMessage> {
    column([
        text("Icon Toolbar").height(28.0).fill_width(),
        toolbar(state),
        text(state.summary()).height(28.0).fill_width(),
    ])
    .padding(16.0)
    .spacing(12.0)
    .fill()
}

fn toolbar(state: &ToolbarState) -> View<ToolMessage> {
    row([
        toolbar_button(state, ToolId::Select).id(10),
        toolbar_button(state, ToolId::Brush).id(11),
        toolbar_button(state, ToolId::Erase).id(12),
        toolbar_button(state, ToolId::Snap).id(13),
    ])
    .spacing(8.0)
    .height(42.0)
    .fill_width()
}

fn toolbar_button(state: &ToolbarState, tool: ToolId) -> View<ToolMessage> {
    custom_widget_mapped(
        IconToggleButton::new(tool, state.icons.icon(tool), state.active(tool)),
        |message: ToolMessage| message,
    )
}
