use super::{CommandOutcome, SurfaceRuntime};
use crate::{
    gui::types::Vector2,
    runtime::{Command, DragSession, ExternalDragSession, RuntimeBridge},
};

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(in crate::runtime::controller) fn dispatch_message_inner(
        &mut self,
        message: Message,
        outcome: &mut CommandOutcome,
    ) {
        let refresh_before = outcome.surface_refresh_requested;
        outcome.messages_dispatched += 1;
        let command = self.bridge.update(message);
        let paint_only = command
            .repaint_scope()
            .is_some_and(|scope| scope.is_paint_only());
        let messages_before_command = outcome.messages_dispatched;
        self.execute_command_inner(command, outcome);
        let command_dispatched_messages = outcome.messages_dispatched > messages_before_command;
        if !paint_only || command_dispatched_messages {
            outcome.surface_refresh_requested = true;
        } else {
            outcome.surface_refresh_requested = refresh_before;
        }
    }

    pub(super) fn execute_command_inner(
        &mut self,
        command: Command<Message>,
        outcome: &mut CommandOutcome,
    ) {
        match command {
            Command::None => {}
            Command::Message(message) => self.dispatch_message_inner(message, outcome),
            Command::Batch(commands) => {
                for command in commands {
                    self.execute_command_inner(command, outcome);
                }
            }
            Command::RequestRepaint => {
                self.repaint_requested = true;
                outcome.repaint_requested = true;
                outcome.surface_repaint_requested = true;
            }
            Command::RequestPaintOnly => {
                self.repaint_requested = true;
                outcome.repaint_requested = true;
                outcome.paint_only_requested = true;
            }
            Command::SetDpiScale(scale) => {
                self.repaint_requested = true;
                outcome.repaint_requested = true;
                outcome.surface_repaint_requested = true;
                outcome.surface_refresh_requested = true;
                outcome.dpi_scale_override = Some(scale);
            }
            Command::SetWindowLogicalSize(size) => {
                self.repaint_requested = true;
                outcome.repaint_requested = true;
                outcome.surface_repaint_requested = true;
                outcome.surface_refresh_requested = true;
                outcome.window_logical_size = Some(size);
            }
            Command::After { delay, message } => {
                if self.bridge.schedule_message(delay, message) {
                    outcome.repaint_requested = true;
                }
            }
            Command::Perform {
                name,
                priority,
                work,
            } => {
                if self.bridge.spawn_message_task(name, priority, work) {
                    outcome.repaint_requested = true;
                }
            }
            Command::Focus(widget_id) => {
                let focused = self.focus_widget(widget_id);
                outcome.repaint_requested |= focused;
                outcome.surface_repaint_requested |= focused;
            }
            Command::ScrollTo { node_id, offset } => {
                let offset = Vector2::new(offset.x.max(0.0), offset.y.max(0.0));
                self.scroll_to_offset(node_id, offset);
                self.repaint_requested = true;
                outcome.repaint_requested = true;
                outcome.surface_repaint_requested = true;
            }
            Command::ScrollIntoView {
                node_id,
                target_y,
                target_height,
                margin_top,
                margin_bottom,
                snap_y,
            } => {
                if let Some(offset) = self.scroll_into_view_offset(
                    node_id,
                    target_y,
                    target_height,
                    margin_top,
                    margin_bottom,
                    snap_y,
                ) {
                    self.scroll_to_offset(node_id, offset);
                }
                self.repaint_requested = true;
                outcome.repaint_requested = true;
                outcome.surface_repaint_requested = true;
            }
            Command::ScrollFixedRowIntoView {
                node_id,
                row_index,
                row_stride,
                leading_context_rows,
                trailing_context_rows,
                direction,
            } => {
                if let Some(offset) = self.scroll_fixed_row_into_view_offset(
                    node_id,
                    row_index,
                    row_stride,
                    leading_context_rows,
                    trailing_context_rows,
                    direction,
                ) {
                    self.scroll_to_offset(node_id, offset);
                }
                self.repaint_requested = true;
                outcome.repaint_requested = true;
                outcome.surface_repaint_requested = true;
            }
            Command::BeginExternalDrag {
                request,
                on_completed,
            } => {
                self.interaction.drag.external_session =
                    Some(ExternalDragSession::new(request, on_completed));
            }
            Command::BeginDrag { request } => {
                self.interaction.drag.session = Some(DragSession::new(request));
                self.repaint_requested = true;
                outcome.repaint_requested = true;
                outcome.surface_repaint_requested = true;
            }
            Command::PlatformRequest {
                request,
                on_completed,
            } => {
                if let Err(fallback) = self.bridge.request_platform_service(request, on_completed) {
                    let (_request, on_completed) = *fallback;
                    let message = on_completed(Err(String::from(
                        "platform service requests are not supported by this runtime bridge",
                    )));
                    self.dispatch_message_inner(message, outcome);
                }
            }
            Command::EndExternalDrag => {
                self.interaction.drag.external_session = None;
            }
            Command::EndDrag => {
                self.interaction.drag.session = None;
                self.repaint_requested = true;
                outcome.repaint_requested = true;
                outcome.surface_repaint_requested = true;
            }
            Command::Exit => {
                outcome.exit_requested = true;
                self.exit_requested = true;
            }
        }
    }
}
