use super::{Command, RepaintScope, TaskPriority};

impl<Message> Command<Message> {
    /// Return whether this command performs no work.
    pub fn is_empty(&self) -> bool {
        match self {
            Self::None => true,
            Self::Message(_)
            | Self::RequestRepaint
            | Self::RequestPaintOnly
            | Self::UpdateGpuShaderPresentationUniform(_)
            | Self::RequestProjectionRefresh
            | Self::RequestLayoutRefresh
            | Self::Timer(..)
            | Self::PerformWorker(..)
            | Self::Focus(_)
            | Self::ClearFocus
            | Self::ScrollTo { .. }
            | Self::ScrollIntoView { .. }
            | Self::ScrollFixedRowIntoView { .. }
            | Self::BeginExternalDrag { .. }
            | Self::BeginDrag { .. }
            | Self::PlatformRequest { .. }
            | Self::PlatformEffect(..)
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
            Self::RequestPaintOnly | Self::UpdateGpuShaderPresentationUniform(_) => {
                Some(RepaintScope::PaintOnly)
            }
            Self::RequestProjectionRefresh => Some(RepaintScope::Projection),
            Self::RequestLayoutRefresh => Some(RepaintScope::Layout),
            Self::Batch(commands) => commands
                .iter()
                .filter_map(Self::repaint_scope)
                .reduce(RepaintScope::merge),
            Self::None
            | Self::Message(_)
            | Self::Timer(..)
            | Self::PerformWorker(..)
            | Self::Focus(_)
            | Self::ClearFocus
            | Self::ScrollTo { .. }
            | Self::ScrollIntoView { .. }
            | Self::ScrollFixedRowIntoView { .. }
            | Self::BeginExternalDrag { .. }
            | Self::BeginDrag { .. }
            | Self::PlatformRequest { .. }
            | Self::PlatformEffect(..)
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
            Self::Timer(effect) if effect.owner.is_some() || effect.lifecycle.is_some() => true,
            Self::PerformWorker(effect) if effect.owner.is_some() || effect.lifecycle.is_some() => {
                true
            }
            Self::PlatformEffect(..) => true,
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
            | Self::UpdateGpuShaderPresentationUniform(_)
            | Self::RequestProjectionRefresh
            | Self::RequestLayoutRefresh
            | Self::SetDpiScale(_)
            | Self::SetWindowLogicalSize(_)
            | Self::Timer(..)
            | Self::PerformWorker(..)
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
    /// This inspects one-shot and streaming worker effects and walks nested
    /// batches in dispatch order. It is primarily useful in tests and
    /// diagnostics that need to verify app work was routed to the intended
    /// runtime-managed business lane without pattern-matching hidden command
    /// internals.
    pub fn business_task_priority(&self, name: &'static str) -> Option<TaskPriority> {
        match self {
            Self::PerformWorker(effect) if effect.name == name => Some(effect.priority),
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
    fn business_task_priority_finds_worker_effect_in_batch() {
        let command = Command::batch([
            Command::Message(1),
            Command::perform_worker_effect_with_priority(
                "target",
                TaskPriority::Interactive,
                None,
                0,
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
    fn business_task_priority_finds_worker_stream_in_batch() {
        let command = Command::batch([
            Command::Message(1),
            Command::perform_worker_stream_with_priority(
                "target",
                TaskPriority::BlockingIo,
                crate::runtime::command::WorkerStreamOptions {
                    is_cancelled: None,
                    generation: 0,
                    latest: false,
                },
                |_| (),
                |_: ()| 2,
                |_: ()| 3,
            ),
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

    #[test]
    fn owner_latest_worker_commands_require_fresh_surface_before_dispatch() {
        let owner = crate::application::DeclarativeEffectOwner::new();
        let mut latest = crate::application::LatestTask::new();
        let transaction = latest.begin_replacement();
        let command = Command::<()>::perform_worker_effect_with_identity_and_transaction_and_receipt_for_owner(
            crate::runtime::command::EffectId(1),
            "owner-worker",
            TaskPriority::Interactive,
            None,
            transaction.generation(),
            Some(transaction),
            None,
            Some(owner),
            |_| (),
            |_| (),
        );

        assert!(command.requires_fresh_surface_before_dispatch());
    }

    #[test]
    fn owner_ordered_stream_commands_require_fresh_surface_before_dispatch() {
        let owner = crate::application::DeclarativeEffectOwner::new();
        let command = Command::<()>::perform_worker_stream_with_priority_and_receipt_for_owner(
            owner,
            "owner-stream",
            TaskPriority::Background,
            None,
            |_, _| 1_u8,
            |_: u8| (),
            |_: u8| (),
        );

        assert!(command.requires_fresh_surface_before_dispatch());
    }

    #[test]
    fn owner_coalesced_stream_commands_require_fresh_surface_before_dispatch() {
        let owner = crate::application::DeclarativeEffectOwner::new();
        let command =
            Command::<()>::perform_worker_stream_latest_with_priority_and_receipt_for_owner(
                owner,
                "owner-stream-latest",
                TaskPriority::Background,
                None,
                |_, _| 1_u8,
                |_: u8| (),
                |_: u8| (),
            );

        assert!(command.requires_fresh_surface_before_dispatch());
    }
}
