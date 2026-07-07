use super::{BusinessMessageSink, Command};

impl<Message> Command<Message> {
    /// Run message-producing command work synchronously for host-side tests.
    ///
    /// This helper is intended for deterministic unit tests around host update
    /// logic. It dispatches [`Command::Message`] values, recursively walks
    /// batches, runs [`Command::Perform`] work inline, and runs
    /// [`Command::PerformStream`] work inline while preserving emitted message
    /// order. Runtime, platform, window, focus, drag, scroll, repaint, timer,
    /// and exit commands are intentionally ignored because they require an
    /// installed runtime adapter.
    ///
    /// Production application code should submit business work through
    /// [`crate::application::UiUpdateContext::business`] so it runs on a
    /// runtime-managed worker lane.
    pub fn run_inline_for_tests(self, mut dispatch: impl FnMut(Message))
    where
        Message: Send + 'static,
    {
        self.run_inline_for_tests_inner(&mut dispatch);
    }

    fn run_inline_for_tests_inner(self, dispatch: &mut impl FnMut(Message))
    where
        Message: Send + 'static,
    {
        match self {
            Self::None
            | Self::RequestRepaint
            | Self::RequestPaintOnly
            | Self::SetDpiScale(_)
            | Self::SetWindowLogicalSize(_)
            | Self::Focus(_)
            | Self::ClearFocus
            | Self::ScrollTo { .. }
            | Self::ScrollIntoView { .. }
            | Self::ScrollFixedRowIntoView { .. }
            | Self::BeginExternalDrag { .. }
            | Self::BeginDrag { .. }
            | Self::EndDrag
            | Self::PlatformRequest { .. }
            | Self::EndExternalDrag
            | Self::After { .. }
            | Self::Exit => {}
            Self::Message(message) => dispatch(message),
            Self::Batch(commands) => {
                for command in commands {
                    command.run_inline_for_tests_inner(dispatch);
                }
            }
            Self::Perform { work, .. } => dispatch(work()),
            Self::PerformStream { work, .. } => {
                let (sender, receiver) = std::sync::mpsc::channel();
                let sink = BusinessMessageSink::new(move |message| sender.send(message).is_ok());
                work(sink);
                for message in receiver.try_iter() {
                    dispatch(message);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::runtime::{Command, TaskPriority};

    #[test]
    fn inline_runner_dispatches_perform_stream_messages_in_order() {
        let command = Command::perform_stream_with_priority(
            "inline-stream",
            TaskPriority::Background,
            None,
            |sink| {
                assert!(sink.emit(1));
                assert!(sink.emit(2));
            },
        );
        let mut messages = Vec::new();

        command.run_inline_for_tests(|message| messages.push(message));

        assert_eq!(messages, vec![1, 2]);
    }

    #[test]
    fn inline_runner_walks_batches_and_perform_work() {
        let command = Command::batch([
            Command::Message(1),
            Command::perform_with_priority(
                "inline-work",
                TaskPriority::Background,
                None,
                || 2,
                |n| n + 1,
            ),
            Command::batch([Command::Message(4)]),
        ]);
        let mut messages = Vec::new();

        command.run_inline_for_tests(|message| messages.push(message));

        assert_eq!(messages, vec![1, 3, 4]);
    }
}
