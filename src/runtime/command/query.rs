use super::{Command, RepaintScope, TaskPriority};

impl<Message> Command<Message> {
    /// Return whether this command performs no work.
    pub fn is_empty(&self) -> bool {
        match self {
            Self::None => true,
            Self::Message(_)
            | Self::RequestRepaint
            | Self::RequestPaintOnly
            | Self::After { .. }
            | Self::Perform { .. }
            | Self::PerformStream { .. }
            | Self::PerformStreamLatest { .. }
            | Self::Focus(_)
            | Self::ClearFocus
            | Self::ScrollTo { .. }
            | Self::ScrollIntoView { .. }
            | Self::ScrollFixedRowIntoView { .. }
            | Self::BeginExternalDrag { .. }
            | Self::BeginDrag { .. }
            | Self::PlatformRequest { .. }
            | Self::SetDpiScale(_)
            | Self::SetWindowLogicalSize(_)
            | Self::EndExternalDrag
            | Self::EndDrag
            | Self::Exit => false,
            Self::Batch(commands) => commands.iter().all(Self::is_empty),
        }
    }

    /// Return whether this command or any nested command requests repaint.
    pub fn requests_repaint(&self) -> bool {
        self.repaint_scope().is_some()
    }

    /// Return the effective repaint scope for this command or nested batch.
    ///
    /// `RepaintScope::Surface` wins over `RepaintScope::PaintOnly` because a
    /// surface refresh also covers paint-only overlay work. This makes mixed
    /// batches explicit and avoids accidentally skipping surface reprojection.
    pub fn repaint_scope(&self) -> Option<RepaintScope> {
        match self {
            Self::RequestRepaint | Self::SetDpiScale(_) | Self::SetWindowLogicalSize(_) => {
                Some(RepaintScope::Surface)
            }
            Self::RequestPaintOnly => Some(RepaintScope::PaintOnly),
            Self::Batch(commands) => commands
                .iter()
                .filter_map(Self::repaint_scope)
                .reduce(RepaintScope::merge),
            Self::None
            | Self::Message(_)
            | Self::After { .. }
            | Self::Perform { .. }
            | Self::PerformStream { .. }
            | Self::PerformStreamLatest { .. }
            | Self::Focus(_)
            | Self::ClearFocus
            | Self::ScrollTo { .. }
            | Self::ScrollIntoView { .. }
            | Self::ScrollFixedRowIntoView { .. }
            | Self::BeginExternalDrag { .. }
            | Self::BeginDrag { .. }
            | Self::PlatformRequest { .. }
            | Self::EndExternalDrag
            | Self::EndDrag
            | Self::Exit => None,
        }
    }

    /// Return whether this command or any nested command requests paint-only redraw.
    pub fn requests_paint_only(&self) -> bool {
        matches!(self.repaint_scope(), Some(RepaintScope::PaintOnly))
    }

    pub(in crate::runtime) fn requires_fresh_surface_before_dispatch(&self) -> bool {
        match self {
            Self::Focus(_)
            | Self::ScrollTo { .. }
            | Self::ScrollIntoView { .. }
            | Self::ScrollFixedRowIntoView { .. } => true,
            Self::Batch(commands) => commands
                .iter()
                .any(Self::requires_fresh_surface_before_dispatch),
            Self::Message(_)
            | Self::None
            | Self::RequestRepaint
            | Self::RequestPaintOnly
            | Self::SetDpiScale(_)
            | Self::SetWindowLogicalSize(_)
            | Self::After { .. }
            | Self::Perform { .. }
            | Self::PerformStream { .. }
            | Self::PerformStreamLatest { .. }
            | Self::ClearFocus
            | Self::BeginExternalDrag { .. }
            | Self::BeginDrag { .. }
            | Self::PlatformRequest { .. }
            | Self::EndExternalDrag
            | Self::EndDrag
            | Self::Exit => false,
        }
    }

    /// Return the priority for the first queued business command with `name`.
    ///
    /// This inspects both one-shot and streaming business commands and walks
    /// nested batches in dispatch order. It is primarily useful in tests and
    /// diagnostics that need to verify app work was routed to the intended
    /// runtime-managed business lane without pattern-matching hidden command
    /// internals.
    pub fn business_task_priority(&self, name: &'static str) -> Option<TaskPriority> {
        match self {
            Self::Perform {
                name: command_name,
                priority,
                ..
            }
            | Self::PerformStream {
                name: command_name,
                priority,
                ..
            }
            | Self::PerformStreamLatest {
                name: command_name,
                priority,
                ..
            } if *command_name == name => Some(*priority),
            Self::Batch(commands) => commands
                .iter()
                .find_map(|command| command.business_task_priority(name)),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::layout::Vector2;
    use crate::runtime::{Command, TaskPriority};

    #[test]
    fn business_task_priority_finds_perform_command_in_batch() {
        let command = Command::batch([
            Command::Message(1),
            Command::perform_with_priority(
                "target",
                TaskPriority::Interactive,
                None,
                || 2,
                |message| message,
            ),
        ]);

        assert_eq!(
            command.business_task_priority("target"),
            Some(TaskPriority::Interactive)
        );
    }

    #[test]
    fn business_task_priority_finds_stream_command_in_batch() {
        let command = Command::batch([
            Command::Message(1),
            Command::perform_stream_with_priority("target", TaskPriority::BlockingIo, None, |_| {}),
        ]);

        assert_eq!(
            command.business_task_priority("target"),
            Some(TaskPriority::BlockingIo)
        );
    }

    #[test]
    fn layout_dependent_commands_require_fresh_surface_before_dispatch() {
        assert!(Command::<()>::focus(42).requires_fresh_surface_before_dispatch());
        assert!(
            Command::<()>::scroll_to(10, Vector2::new(0.0, 24.0))
                .requires_fresh_surface_before_dispatch()
        );
        assert!(
            Command::<()>::scroll_into_view(10, 40.0, 20.0, 0.0, 0.0)
                .requires_fresh_surface_before_dispatch()
        );
        assert!(
            Command::<()>::scroll_fixed_row_into_view(10, 5, 20.0, 1, 1, 1)
                .requires_fresh_surface_before_dispatch()
        );
        assert!(
            Command::<()>::batch([
                Command::request_repaint(),
                Command::scroll_to(10, Vector2::new(0.0, 24.0)),
            ])
            .requires_fresh_surface_before_dispatch()
        );
        assert!(!Command::<()>::request_repaint().requires_fresh_surface_before_dispatch());
        assert!(!Command::<()>::clear_focus().requires_fresh_surface_before_dispatch());
    }
}
