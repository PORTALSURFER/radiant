use super::{Command, RepaintScope};

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
            | Self::Focus(_)
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
            | Self::Focus(_)
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
}
