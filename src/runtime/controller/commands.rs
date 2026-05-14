use super::*;

mod batching;

use batching::{take_runtime_command_batch_into, take_runtime_message_batch_into};

/// Summary of one command-dispatch pass through a [`SurfaceRuntime`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CommandOutcome {
    /// Number of host-defined messages reduced during this pass.
    pub messages_dispatched: usize,
    /// Whether any command requested a repaint.
    pub repaint_requested: bool,
    /// Whether any command requested a redraw without surface reprojection.
    pub paint_only_requested: bool,
    /// Whether any command requested a repaint of the projected surface.
    pub surface_repaint_requested: bool,
    /// Whether this pass requires declarative surface reprojection and layout.
    pub surface_refresh_requested: bool,
    /// Whether any command requested runtime exit.
    pub exit_requested: bool,
    /// Whether runtime-owned background work still has queued commands/messages.
    ///
    /// Native backends use this to keep the UI/event/render owner responsive:
    /// one drain pass handles a bounded slice of background commands/messages,
    /// then schedules another wakeup instead of monopolizing the UI path.
    pub runtime_work_remaining: bool,
}

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    /// Reduce one host-defined message and execute its runtime-visible command.
    pub fn dispatch_message(&mut self, message: Message) -> CommandOutcome {
        let mut outcome = CommandOutcome::default();
        self.dispatch_message_inner(message, &mut outcome);
        self.finish_command_outcome(outcome)
    }

    /// Execute a command without an initial widget message.
    ///
    /// This is useful for backend adapters or host shells that need to replay a
    /// queued command through the same message/repaint handling path used by
    /// widget dispatch.
    pub fn execute_command(&mut self, command: Command<Message>) -> CommandOutcome {
        let mut outcome = CommandOutcome::default();
        self.execute_command_inner(command, &mut outcome);
        self.finish_command_outcome(outcome)
    }

    /// Dispatch any messages queued by bridge-owned runtime work.
    pub fn drain_runtime_messages(&mut self) -> CommandOutcome {
        let mut outcome = CommandOutcome::default();
        self.bridge
            .drain_runtime_commands_into(&mut self.runtime_commands);
        take_runtime_command_batch_into(
            &mut self.runtime_commands,
            &mut self.runtime_command_batch,
        );
        let mut command_batch = std::mem::take(&mut self.runtime_command_batch);
        while let Some(command) = command_batch.pop() {
            self.execute_command_inner(command, &mut outcome);
        }
        self.runtime_command_batch = command_batch;

        self.bridge
            .drain_runtime_messages_into(&mut self.runtime_messages);
        take_runtime_message_batch_into(
            &mut self.runtime_messages,
            &mut self.runtime_message_batch,
        );
        while let Some(message) = self.runtime_message_batch.pop() {
            self.dispatch_message_inner(message, &mut outcome);
        }

        if !self.runtime_commands.is_empty() || !self.runtime_messages.is_empty() {
            outcome.runtime_work_remaining = true;
            outcome.repaint_requested = true;
            self.repaint_requested = true;
        }

        self.finish_command_outcome(outcome)
    }

    fn finish_command_outcome(&mut self, outcome: CommandOutcome) -> CommandOutcome {
        self.refresh_if_requested(outcome.surface_refresh_requested);
        if outcome.surface_refresh_requested {
            self.repaint_requested = true;
        }
        if outcome.exit_requested {
            self.exit_requested = true;
        }
        outcome
    }

    fn refresh_if_requested(&mut self, requested: bool) {
        if requested {
            self.refresh();
        }
    }

    pub(super) fn dispatch_message_inner(
        &mut self,
        message: Message,
        outcome: &mut CommandOutcome,
    ) {
        let refresh_before = outcome.surface_refresh_requested;
        outcome.messages_dispatched += 1;
        let command = self.bridge.update(message);
        let paint_only = command_requests_paint_only(&command);
        let messages_before_command = outcome.messages_dispatched;
        self.execute_command_inner(command, outcome);
        let command_dispatched_messages = outcome.messages_dispatched > messages_before_command;
        if !paint_only || command_dispatched_messages {
            outcome.surface_refresh_requested = true;
        } else {
            outcome.surface_refresh_requested = refresh_before;
        }
    }

    fn execute_command_inner(&mut self, command: Command<Message>, outcome: &mut CommandOutcome) {
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
            Command::After { delay, message } => {
                if self.bridge.schedule_message(delay, message) {
                    outcome.repaint_requested = true;
                }
            }
            Command::Perform { name, work } => {
                if self.bridge.spawn_message_task(name, work) {
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
                self.external_drag_session = Some(ExternalDragSession::new(request, on_completed));
            }
            Command::EndExternalDrag => {
                self.external_drag_session = None;
            }
            Command::Exit => {
                outcome.exit_requested = true;
                self.exit_requested = true;
            }
        }
    }

    /// Return whether a native external drag session is currently armed.
    pub fn external_drag_armed(&self) -> bool {
        self.external_drag_session.is_some()
    }

    pub(crate) fn take_external_drag_session(&mut self) -> Option<ExternalDragSession<Message>> {
        self.external_drag_session.take()
    }

    pub(crate) fn dispatch_external_drag_result(
        &mut self,
        session: ExternalDragSession<Message>,
        result: Result<ExternalDragOutcome, String>,
    ) -> CommandOutcome {
        let Some(map) = session.on_completed else {
            return CommandOutcome::default();
        };
        self.dispatch_message(map(result))
    }
}

fn command_requests_paint_only<Message>(command: &Command<Message>) -> bool {
    match command {
        Command::RequestPaintOnly => true,
        Command::Batch(commands) => commands.iter().any(command_requests_paint_only),
        Command::None
        | Command::Message(_)
        | Command::RequestRepaint
        | Command::After { .. }
        | Command::Perform { .. }
        | Command::Focus(_)
        | Command::ScrollTo { .. }
        | Command::ScrollIntoView { .. }
        | Command::ScrollFixedRowIntoView { .. }
        | Command::BeginExternalDrag { .. }
        | Command::EndExternalDrag
        | Command::Exit => false,
    }
}

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    fn scroll_to_offset(&mut self, node_id: NodeId, offset: Vector2) {
        let previous_offset = self.layout_state.scroll_offset(node_id);
        self.layout_state.scroll_offsets.insert(node_id, offset);
        self.relayout_current_surface();
        let offset = self.layout_state.scroll_offset(node_id);
        if offset == previous_offset {
            return;
        }
        let viewport = self
            .layout
            .rects
            .get(&node_id)
            .map(|rect| Vector2::new(rect.width(), rect.height()))
            .unwrap_or_default();
        self.report_scroll_update(ScrollUpdate {
            node_id,
            position: Point::new(0.0, 0.0),
            delta: Vector2::new(offset.x - previous_offset.x, offset.y - previous_offset.y),
            previous_offset,
            offset,
            viewport,
        });
    }

    fn scroll_into_view_offset(
        &self,
        node_id: NodeId,
        target_y: f32,
        target_height: f32,
        margin_top: f32,
        margin_bottom: f32,
        snap_y: Option<f32>,
    ) -> Option<Vector2> {
        let viewport = self.layout.rects.get(&node_id)?;
        let current = self.layout_state.scroll_offset(node_id);
        let viewport_height = viewport.height().max(0.0);
        if viewport_height <= 0.0 {
            return None;
        }
        let target_top = target_y.max(0.0);
        let target_bottom = target_top + target_height.max(0.0);
        let margin_top = margin_top.max(0.0);
        let margin_bottom = margin_bottom.max(0.0);
        let top_limit = target_top.saturating_sub_f32(margin_top);
        let bottom_limit = (target_bottom + margin_bottom - viewport_height).max(0.0);
        let target_offset_y = if current.y > top_limit {
            top_limit
        } else if current.y < bottom_limit {
            bottom_limit
        } else {
            current.y
        };
        let target_offset_y = snap_scroll_offset_y(current.y, target_offset_y, snap_y);
        Some(Vector2::new(current.x, target_offset_y))
    }

    fn scroll_fixed_row_into_view_offset(
        &self,
        node_id: NodeId,
        row_index: usize,
        row_stride: f32,
        leading_context_rows: usize,
        trailing_context_rows: usize,
        direction: i32,
    ) -> Option<Vector2> {
        let viewport = self.layout.rects.get(&node_id)?;
        let current = self.layout_state.scroll_offset(node_id);
        if !row_stride.is_finite() || row_stride <= f32::EPSILON {
            return None;
        }
        let visible_rows = (viewport.height().max(0.0) / row_stride).floor().max(1.0) as usize;
        let row_index = row_index;
        let target_offset_y = if direction < 0 {
            let top_limit = row_index.saturating_sub(leading_context_rows);
            let top_limit_y = top_limit as f32 * row_stride;
            if current.y > top_limit_y {
                top_limit_y
            } else {
                current.y
            }
        } else if direction > 0 {
            let bottom_limit = row_index
                .saturating_add(trailing_context_rows)
                .saturating_add(1)
                .saturating_sub(visible_rows);
            let bottom_limit_y = bottom_limit as f32 * row_stride;
            if current.y < bottom_limit_y {
                bottom_limit_y
            } else {
                current.y
            }
        } else {
            current.y
        };
        Some(Vector2::new(current.x, target_offset_y))
    }
}

fn snap_scroll_offset_y(current_y: f32, target_y: f32, snap_y: Option<f32>) -> f32 {
    let Some(snap_y) = snap_y.filter(|snap_y| snap_y.is_finite() && *snap_y > 0.0) else {
        return target_y;
    };
    if target_y == current_y {
        target_y
    } else {
        ((target_y / snap_y).round() * snap_y).max(0.0)
    }
}

trait SaturatingSubF32 {
    fn saturating_sub_f32(self, rhs: f32) -> f32;
}

impl SaturatingSubF32 for f32 {
    fn saturating_sub_f32(self, rhs: f32) -> f32 {
        (self - rhs).max(0.0)
    }
}

#[cfg(test)]
mod tests;
