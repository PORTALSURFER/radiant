use super::Command;
use std::fmt;

impl<Message> fmt::Debug for Command<Message>
where
    Message: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::None => f.write_str("None"),
            Self::Message(message) => f.debug_tuple("Message").field(message).finish(),
            Self::Batch(commands) => f.debug_tuple("Batch").field(commands).finish(),
            Self::RequestRepaint => f.write_str("RequestRepaint"),
            Self::RequestPaintOnly => f.write_str("RequestPaintOnly"),
            Self::SetDpiScale(scale) => f.debug_tuple("SetDpiScale").field(scale).finish(),
            Self::SetWindowLogicalSize(size) => {
                f.debug_tuple("SetWindowLogicalSize").field(size).finish()
            }
            Self::After { delay, message } => f
                .debug_struct("After")
                .field("delay", delay)
                .field("message", message)
                .finish(),
            Self::Perform { name, priority, .. } => f
                .debug_struct("Perform")
                .field("name", name)
                .field("priority", priority)
                .finish(),
            Self::PerformStream { name, priority, .. } => f
                .debug_struct("PerformStream")
                .field("name", name)
                .field("priority", priority)
                .finish(),
            Self::PerformStreamLatest { name, priority, .. } => f
                .debug_struct("PerformStreamLatest")
                .field("name", name)
                .field("priority", priority)
                .finish(),
            Self::Focus(widget_id) => f.debug_tuple("Focus").field(widget_id).finish(),
            Self::ClearFocus => f.write_str("ClearFocus"),
            Self::ScrollTo { node_id, offset } => f
                .debug_struct("ScrollTo")
                .field("node_id", node_id)
                .field("offset", offset)
                .finish(),
            Self::ScrollIntoView {
                node_id,
                target_y,
                target_height,
                margin_top,
                margin_bottom,
                snap_y,
            } => f
                .debug_struct("ScrollIntoView")
                .field("node_id", node_id)
                .field("target_y", target_y)
                .field("target_height", target_height)
                .field("margin_top", margin_top)
                .field("margin_bottom", margin_bottom)
                .field("snap_y", snap_y)
                .finish(),
            Self::ScrollFixedRowIntoView {
                node_id,
                row_index,
                row_stride,
                leading_context_rows,
                trailing_context_rows,
                direction,
            } => f
                .debug_struct("ScrollFixedRowIntoView")
                .field("node_id", node_id)
                .field("row_index", row_index)
                .field("row_stride", row_stride)
                .field("leading_context_rows", leading_context_rows)
                .field("trailing_context_rows", trailing_context_rows)
                .field("direction", direction)
                .finish(),
            Self::BeginExternalDrag { request, .. } => f
                .debug_struct("BeginExternalDrag")
                .field("request", request)
                .finish_non_exhaustive(),
            Self::BeginDrag { request } => f
                .debug_struct("BeginDrag")
                .field("request", request)
                .finish(),
            Self::PlatformRequest { request, .. } => f
                .debug_struct("PlatformRequest")
                .field("request", request)
                .finish_non_exhaustive(),
            Self::EndExternalDrag => f.write_str("EndExternalDrag"),
            Self::EndDrag => f.write_str("EndDrag"),
            Self::Exit => f.write_str("Exit"),
        }
    }
}
