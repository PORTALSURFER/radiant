//! Generic command values returned or queued by host-side runtime code.

use super::drag::DragRequest;
use super::external_drag::{ExternalDragCompletion, ExternalDragRequest};
use super::platform::{PlatformCompletion, PlatformRequest};
use crate::application::{DeclarativeEffectOwner, LatestTaskTransaction};
use crate::{gui::types::Vector2, layout::NodeId, theme::DpiScale, widgets::WidgetId};
use std::time::Duration;
use std::{any::Any, sync::Arc};

mod constructors;
mod debug;
mod flatten;
mod query;
mod repaint;
mod scroll;

pub(crate) use constructors::WorkerStreamOptions;

pub use repaint::RepaintScope;
pub use repaint::{SurfaceInvalidation, SurfaceRevisions};
pub use scroll::{ScrollFixedRowIntoViewParts, ScrollIntoViewParts};

/// Runtime hint for host-owned background work scheduled through Radiant.
///
/// Radiant treats this as a best-effort scheduling hint. Platforms that cannot
/// adjust worker priority keep the same queueing behavior without changing the
/// public command contract.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TaskPriority {
    /// User-visible work that should complete promptly without running on the
    /// UI/event/render path.
    Interactive,
    /// Ordinary background work. This is the default and preserves Radiant's
    /// existing business-worker behavior.
    #[default]
    Background,
    /// Blocking filesystem, database, process, or other IO work that should be
    /// explicit and limited separately from ordinary CPU/background work.
    BlockingIo,
    /// Opportunistic work that should yield to interaction and rendering.
    Idle,
}

/// Runtime-facing command produced by host application logic.
///
/// Radiant commands are intentionally small and domain-neutral. Hosts keep
/// ownership of IO, background work, and other side effects; this type only
/// represents values the generic runtime can understand directly.
///
/// UI reducers should stay short and non-blocking. Expensive host work should
/// be submitted through [`crate::application::UiUpdateContext::business`],
/// which offloads it to a runtime-managed worker before delivering the
/// resulting message back through the normal UI update path.
#[derive(Default)]
pub enum Command<Message> {
    /// No follow-up work is required.
    #[default]
    None,
    /// Dispatch a host-defined message.
    Message(Message),
    /// Dispatch multiple commands in order.
    Batch(Vec<Command<Message>>),
    /// Request another redraw from the active runtime adapter.
    RequestRepaint,
    /// Request redraw without forcing declarative surface reprojection.
    RequestPaintOnly,
    /// Request fresh projection/traversal while reusing revision-proven layout.
    RequestProjectionRefresh,
    /// Request fresh projection/traversal and a layout pass.
    RequestLayoutRefresh,
    /// Override the active native DPI scale for runtime adapters that own native windows.
    SetDpiScale(DpiScale),
    /// Request a native-window logical viewport size from runtime adapters that own windows.
    SetWindowLogicalSize(Vector2),
    /// Schedule a UI-local mapper after a delay.
    #[doc(hidden)]
    Timer(TimerEffect<Message>),
    /// Run worker-only work and deliver its owned output to a UI-local mapper.
    #[doc(hidden)]
    PerformWorker(WorkerEffect<Message>),
    /// Move keyboard focus to one widget.
    Focus(WidgetId),
    /// Clear keyboard focus from any focused widget.
    ClearFocus,
    /// Move one scroll container to a logical offset.
    ScrollTo {
        /// Scroll container node to move.
        node_id: NodeId,
        /// Requested logical scroll offset.
        offset: Vector2,
    },
    /// Reveal one vertical content span inside a scroll container.
    ScrollIntoView {
        /// Scroll container node to move.
        node_id: NodeId,
        /// Logical top edge of the target span inside the scroll content.
        target_y: f32,
        /// Logical height of the target span.
        target_height: f32,
        /// Preferred space to keep above the target.
        margin_top: f32,
        /// Preferred space to keep below the target.
        margin_bottom: f32,
        /// Optional vertical snap interval for fixed-row lists.
        snap_y: Option<f32>,
    },
    /// Reveal one fixed-stride row with directional context rows.
    ScrollFixedRowIntoView {
        /// Scroll container node to move.
        node_id: NodeId,
        /// Zero-based row index inside the scroll content.
        row_index: usize,
        /// Fixed distance between adjacent row starts in logical pixels.
        row_stride: f32,
        /// Rows to keep above the target while navigating upward.
        leading_context_rows: usize,
        /// Rows to keep below the target while navigating downward.
        trailing_context_rows: usize,
        /// Negative for upward navigation, positive for downward navigation.
        direction: i32,
    },
    /// Arm a native external drag session.
    ///
    /// Native backends launch the session when the active pointer drag leaves
    /// the application window, allowing external targets such as file managers
    /// to accept the payload.
    BeginExternalDrag {
        /// Payload and preview metadata for the native drag session.
        request: ExternalDragRequest,
        /// Optional host callback mapped into a message when the native drag loop ends.
        on_completed: Option<ExternalDragCompletion<Message>>,
    },
    /// Begin a runtime-owned pointer drag preview session.
    BeginDrag {
        /// Preview and initial pointer metadata.
        request: DragRequest,
    },
    /// End any active runtime-owned pointer drag preview session.
    EndDrag,
    /// Request a platform service such as a file picker or confirmation dialog.
    PlatformRequest {
        /// Platform service request.
        request: PlatformRequest,
        /// Host callback mapped into a message when the request completes.
        on_completed: PlatformCompletion<Message>,
    },
    /// Clear any active native external drag session.
    EndExternalDrag,
    /// Request that the active runtime exits.
    Exit,
}

/// UI-owned delayed work. Only its opaque identity crosses the host boundary.
#[doc(hidden)]
pub struct TimerEffect<Message> {
    pub(crate) delay: Duration,
    pub(crate) transaction: Option<LatestTaskTransaction>,
    pub(crate) owner: Option<DeclarativeEffectOwner>,
    pub(crate) map: Box<dyn FnOnce() -> Message + 'static>,
}

/// Opaque worker-effect command payload.
///
/// This is intentionally hidden from the normal application vocabulary. The
/// worker closure only returns an owned, type-erased `Send` value; the mapper
/// is retained by the UI runtime and is never moved to a worker.
#[doc(hidden)]
pub struct WorkerEffect<Message> {
    pub(crate) name: &'static str,
    pub(crate) priority: TaskPriority,
    pub(crate) is_cancelled: Option<Box<dyn Fn() -> bool + Send + Sync + 'static>>,
    pub(crate) id: EffectId,
    pub(crate) generation: EffectGeneration,
    pub(crate) transaction: Option<LatestTaskTransaction>,
    pub(crate) admission_receipt: Option<
        crate::application::runtime::update_context::business::admission::AdmissionReceiptGuard,
    >,
    pub(crate) work: WorkerEffectWork,
    pub(crate) mapper: WorkerEffectMapper<Message>,
}

pub(crate) enum WorkerEffectWork {
    Once(Box<dyn FnOnce() -> Box<dyn Any + Send> + Send + 'static>),
    Stream(Box<dyn FnOnce(WorkerEffectSink) -> Box<dyn Any + Send> + Send + 'static>),
}

pub(crate) enum WorkerEffectMapper<Message> {
    Once(Box<dyn FnOnce(Box<dyn Any + Send>) -> Option<Message> + 'static>),
    Stream {
        latest: bool,
        map_event: Box<dyn Fn(Box<dyn Any + Send>) -> Option<Message> + 'static>,
        map_final: Box<dyn FnOnce(Box<dyn Any + Send>) -> Option<Message> + 'static>,
    },
}

type WorkerPayloadSink = Arc<dyn Fn(Box<dyn Any + Send>) -> bool + Send + Sync + 'static>;

/// Worker-side payload sink used by UI-owned streaming worker effects.
///
/// The sink carries only opaque `Send` payloads. Event and final mappers stay
/// in the UI-owned effect registry and are never moved onto the worker lane.
#[derive(Clone)]
pub(crate) struct WorkerEffectSink {
    emit: WorkerPayloadSink,
    emit_latest: Option<WorkerPayloadSink>,
    close_latest: Option<Arc<dyn Fn() + Send + Sync + 'static>>,
}

impl WorkerEffectSink {
    pub(crate) fn new_ordered(
        emit: impl Fn(Box<dyn Any + Send>) -> bool + Send + Sync + 'static,
    ) -> Self {
        Self {
            emit: Arc::new(emit),
            emit_latest: None,
            close_latest: None,
        }
    }

    pub(crate) fn new_latest(
        emit: impl Fn(Box<dyn Any + Send>) -> bool + Send + Sync + 'static,
        emit_latest: impl Fn(Box<dyn Any + Send>) -> bool + Send + Sync + 'static,
        close_latest: impl Fn() + Send + Sync + 'static,
    ) -> Self {
        Self {
            emit: Arc::new(emit),
            emit_latest: Some(Arc::new(emit_latest)),
            close_latest: Some(Arc::new(close_latest)),
        }
    }

    pub(crate) fn emit(&self, payload: Box<dyn Any + Send>) -> bool {
        (self.emit)(payload)
    }

    pub(crate) fn emit_latest(&self, payload: Box<dyn Any + Send>) -> bool {
        match &self.emit_latest {
            Some(emit) => emit(payload),
            None => self.emit(payload),
        }
    }

    pub(crate) fn close_latest(&self) {
        if let Some(close) = &self.close_latest {
            close();
        }
    }
}

/// Opaque identity for one worker effect slot.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct EffectId(pub(crate) u64);

/// Opaque generation for replacement/latest effect slots.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct EffectGeneration(pub(crate) u64);

#[cfg(test)]
mod tests;
