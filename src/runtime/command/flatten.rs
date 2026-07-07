use super::Command;

impl<Message> Command<Message> {
    /// Flatten immediate host-defined messages carried by this command.
    ///
    /// Runtime-visible effects such as delayed messages, background work, focus,
    /// repaint, and exit are intentionally not flattened. Execute the command
    /// through `SurfaceRuntime` when those effects must be preserved.
    pub fn into_messages(self) -> Vec<Message> {
        let mut messages = Vec::with_capacity(self.message_collection_hint());
        self.collect_messages_into(&mut messages);
        messages
    }

    /// Flatten immediate host-defined messages into caller-owned storage.
    ///
    /// This is the allocation-reusing counterpart to [`Command::into_messages`]
    /// for dispatch loops or tests that repeatedly inspect command batches.
    /// Existing contents are cleared before immediate messages are appended.
    pub fn into_messages_into(self, messages: &mut Vec<Message>) {
        messages.clear();
        let hint = self.message_collection_hint();
        let additional =
            additional_reserve_for_target_capacity(messages.len(), messages.capacity(), hint);
        if additional > 0 {
            messages.reserve(additional);
        }
        self.collect_messages_into(messages);
    }

    fn message_collection_hint(&self) -> usize {
        match self {
            Self::Message(_) => 1,
            Self::Batch(commands) => commands.iter().map(Self::message_collection_hint).sum(),
            Self::None
            | Self::RequestRepaint
            | Self::RequestPaintOnly
            | Self::After { .. }
            | Self::Perform { .. }
            | Self::PerformStream { .. }
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
            | Self::Exit => 0,
        }
    }

    fn collect_messages_into(self, messages: &mut Vec<Message>) {
        match self {
            Self::Message(message) => messages.push(message),
            Self::Batch(commands) => {
                for command in commands {
                    command.collect_messages_into(messages);
                }
            }
            Self::None
            | Self::RequestRepaint
            | Self::RequestPaintOnly
            | Self::After { .. }
            | Self::Perform { .. }
            | Self::PerformStream { .. }
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
            | Self::Exit => {}
        }
    }
}

pub(super) fn additional_reserve_for_target_capacity(
    current_len: usize,
    current_capacity: usize,
    target_capacity: usize,
) -> usize {
    if target_capacity > current_capacity {
        target_capacity.saturating_sub(current_len)
    } else {
        0
    }
}
