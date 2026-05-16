use super::Command;

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
            | Self::Focus(_)
            | Self::ScrollTo { .. }
            | Self::ScrollIntoView { .. }
            | Self::ScrollFixedRowIntoView { .. }
            | Self::BeginExternalDrag { .. }
            | Self::PlatformRequest { .. }
            | Self::EndExternalDrag
            | Self::Exit => false,
            Self::Batch(commands) => commands.iter().all(Self::is_empty),
        }
    }

    /// Return whether this command or any nested command requests repaint.
    pub fn requests_repaint(&self) -> bool {
        match self {
            Self::RequestRepaint | Self::RequestPaintOnly => true,
            Self::Batch(commands) => commands.iter().any(Self::requests_repaint),
            Self::None
            | Self::Message(_)
            | Self::After { .. }
            | Self::Perform { .. }
            | Self::Focus(_)
            | Self::ScrollTo { .. }
            | Self::ScrollIntoView { .. }
            | Self::ScrollFixedRowIntoView { .. }
            | Self::BeginExternalDrag { .. }
            | Self::PlatformRequest { .. }
            | Self::EndExternalDrag
            | Self::Exit => false,
        }
    }

    /// Return whether this command or any nested command requests paint-only redraw.
    pub fn requests_paint_only(&self) -> bool {
        match self {
            Self::RequestPaintOnly => true,
            Self::Batch(commands) => commands.iter().any(Self::requests_paint_only),
            Self::None
            | Self::Message(_)
            | Self::RequestRepaint
            | Self::After { .. }
            | Self::Perform { .. }
            | Self::Focus(_)
            | Self::ScrollTo { .. }
            | Self::ScrollIntoView { .. }
            | Self::ScrollFixedRowIntoView { .. }
            | Self::BeginExternalDrag { .. }
            | Self::PlatformRequest { .. }
            | Self::EndExternalDrag
            | Self::Exit => false,
        }
    }
}
