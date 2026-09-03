use super::{
    Command, TaskPriority, TimerEffect, WorkerCancellationProbe, WorkerEffectMapper,
    WorkerEffectSink, WorkerEffectWork,
};

pub(crate) struct WorkerStreamOptions {
    pub(crate) is_cancelled: Option<Box<dyn Fn() -> bool + Send + Sync + 'static>>,
    pub(crate) generation: u64,
    pub(crate) latest: bool,
}
use crate::{
    runtime::{
        DragRequest, ExternalDragOutcome, ExternalDragRequest, GpuShaderPresentationUniformUpdate,
        PlatformRequest, PlatformResult, RepaintScope,
    },
    theme::DpiScale,
    widgets::WidgetId,
};
use std::{
    any::Any,
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

static NEXT_EFFECT_ID: AtomicU64 = AtomicU64::new(1);

mod scroll;

impl<Message> Command<Message> {
    /// Build a command from the qualified runtime effect facade.
    ///
    /// This is an explicit bridge to the existing timer and worker command
    /// lanes. It does not add a third transport or bypass controller
    /// lifecycle checks.
    pub fn effect(effect: crate::runtime::Effect<Message>) -> Self {
        effect.into()
    }

    /// Return an empty command.
    pub const fn none() -> Self {
        Self::None
    }

    /// Build a command that dispatches one host-defined message.
    pub const fn message(message: Message) -> Self {
        Self::Message(message)
    }

    /// Build a command that dispatches multiple commands in order.
    pub fn batch(command_iter: impl IntoIterator<Item = Command<Message>>) -> Self {
        let mut command_iter = command_iter.into_iter();
        match command_iter.size_hint() {
            (_, Some(0)) => return Self::None,
            (1, Some(1)) => {
                let Some(command) = command_iter.next() else {
                    return Self::None;
                };
                debug_assert!(command_iter.next().is_none());
                return command.collapse_for_batch();
            }
            _ => {}
        }
        let mut commands = Vec::with_capacity(command_iter.size_hint().0);
        for command in command_iter {
            command.append_to_batch(&mut commands);
        }
        match commands.len() {
            0 => Self::None,
            1 => match commands.pop() {
                Some(command) => command,
                None => Self::None,
            },
            _ => Self::Batch(commands),
        }
    }

    /// Build a command that asks the active runtime adapter to repaint.
    pub const fn request_repaint() -> Self {
        Self::RequestRepaint
    }

    /// Build a command that repaints without refreshing the declarative surface.
    pub const fn request_paint_only() -> Self {
        Self::RequestPaintOnly
    }

    /// Build a command that updates volatile presentation uniforms for a custom
    /// GPU shader surface.
    pub const fn update_gpu_shader_presentation_uniform(
        update: GpuShaderPresentationUniformUpdate,
    ) -> Self {
        Self::UpdateGpuShaderPresentationUniform(update)
    }

    /// Build a command that overrides native DPI scale for the active runtime adapter.
    pub const fn set_dpi_scale(scale: DpiScale) -> Self {
        Self::SetDpiScale(scale)
    }

    /// Build a command that requests a native-window logical viewport size.
    pub const fn set_window_logical_size(size: crate::layout::Vector2) -> Self {
        Self::SetWindowLogicalSize(size)
    }

    /// Build a repaint command from a typed repaint scope.
    pub const fn repaint(scope: RepaintScope) -> Self {
        match scope {
            RepaintScope::Surface => Self::RequestRepaint,
            RepaintScope::Layout => Self::RequestLayoutRefresh,
            RepaintScope::Projection => Self::RequestProjectionRefresh,
            RepaintScope::PaintOnly => Self::RequestPaintOnly,
        }
    }

    /// Build a command that dispatches one message after the provided delay.
    ///
    /// The delay is registered as a UI-owned mapper. A host timer lane carries
    /// only an opaque wake; when the UI runtime drains that wake it invokes the
    /// mapper and dispatches `message` on the UI owner. The message does not
    /// cross the timer thread.
    pub fn after(delay: Duration, message: Message) -> Self
    where
        Message: 'static,
    {
        Self::Timer(TimerEffect {
            delay,
            transaction: None,
            owner: None,
            map: Box::new(move || message),
        })
    }

    pub(crate) fn after_for_owner(
        owner: crate::application::DeclarativeEffectOwner,
        delay: Duration,
        message: Message,
    ) -> Self
    where
        Message: 'static,
    {
        Self::Timer(TimerEffect {
            delay,
            transaction: None,
            owner: Some(owner),
            map: Box::new(move || message),
        })
    }

    pub(crate) fn after_latest(
        delay: Duration,
        ticket: crate::application::TaskTicket,
        transaction: crate::application::LatestTimerTransaction,
        map: impl FnOnce(crate::application::TaskTicket) -> Message + 'static,
    ) -> Self
    where
        Message: 'static,
    {
        Self::Timer(TimerEffect {
            delay,
            transaction: Some(transaction),
            owner: None,
            map: Box::new(move || map(ticket)),
        })
    }

    pub(crate) fn after_latest_for_owner(
        owner: crate::application::DeclarativeEffectOwner,
        delay: Duration,
        ticket: crate::application::TaskTicket,
        transaction: crate::application::LatestTimerTransaction,
        map: impl FnOnce(crate::application::TaskTicket) -> Message + 'static,
    ) -> Self
    where
        Message: 'static,
    {
        Self::Timer(TimerEffect {
            delay,
            transaction: Some(transaction),
            owner: Some(owner),
            map: Box::new(move || map(ticket)),
        })
    }

    /// Build a worker-only effect whose output is mapped on the UI owner.
    ///
    /// The worker closure never constructs or transports `Message`. The mapper
    /// remains UI-local.
    pub(crate) fn perform_worker_effect_with_priority<Output>(
        name: &'static str,
        priority: TaskPriority,
        is_cancelled: Option<Box<dyn Fn() -> bool + Send + Sync + 'static>>,
        generation: u64,
        work: impl FnOnce() -> Output + Send + 'static,
        map: impl FnOnce(Output) -> Message + 'static,
    ) -> Self
    where
        Output: Send + 'static,
    {
        let id = NEXT_EFFECT_ID.fetch_add(1, Ordering::Relaxed);
        Self::perform_worker_effect_with_identity(
            super::EffectId(id),
            name,
            priority,
            is_cancelled,
            generation,
            work,
            map,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn perform_worker_effect_with_priority_and_receipt<Output>(
        name: &'static str,
        priority: TaskPriority,
        is_cancelled: Option<Box<dyn Fn() -> bool + Send + Sync + 'static>>,
        generation: u64,
        admission_receipt: Option<
            crate::application::runtime::update_context::business::admission::AdmissionReceiptGuard,
        >,
        work: impl FnOnce() -> Output + Send + 'static,
        map: impl FnOnce(Output) -> Message + 'static,
    ) -> Self
    where
        Output: Send + 'static,
    {
        let id = NEXT_EFFECT_ID.fetch_add(1, Ordering::Relaxed);
        Self::perform_worker_effect_with_identity_and_transaction_and_receipt(
            super::EffectId(id),
            name,
            priority,
            is_cancelled,
            generation,
            None,
            admission_receipt,
            work,
            map,
        )
    }

    pub(crate) fn perform_worker_effect_with_priority_and_receipt_for_owner<Output>(
        owner: crate::application::DeclarativeEffectOwner,
        name: &'static str,
        priority: TaskPriority,
        admission_receipt: Option<
            crate::application::runtime::update_context::business::admission::AdmissionReceiptGuard,
        >,
        work: impl FnOnce(Option<WorkerCancellationProbe>) -> Output + Send + 'static,
        map: impl FnOnce(Output) -> Message + 'static,
    ) -> Self
    where
        Output: Send + 'static,
    {
        Self::perform_worker_effect_with_priority_and_receipt_for_owner_with_options(
            owner,
            name,
            priority,
            None,
            admission_receipt,
            work,
            map,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn perform_worker_effect_with_priority_and_receipt_for_owner_with_options<Output>(
        owner: crate::application::DeclarativeEffectOwner,
        name: &'static str,
        priority: TaskPriority,
        is_cancelled: Option<Box<dyn Fn() -> bool + Send + Sync + 'static>>,
        admission_receipt: Option<
            crate::application::runtime::update_context::business::admission::AdmissionReceiptGuard,
        >,
        work: impl FnOnce(Option<WorkerCancellationProbe>) -> Output + Send + 'static,
        map: impl FnOnce(Output) -> Message + 'static,
    ) -> Self
    where
        Output: Send + 'static,
    {
        let id = NEXT_EFFECT_ID.fetch_add(1, Ordering::Relaxed);
        Self::perform_worker_effect_with_identity_and_transaction_and_receipt_for_owner(
            super::EffectId(id),
            name,
            priority,
            is_cancelled,
            0,
            None,
            admission_receipt,
            Some(owner),
            work,
            map,
        )
    }

    pub(crate) fn perform_worker_effect_with_identity<Output>(
        id: super::EffectId,
        name: &'static str,
        priority: TaskPriority,
        is_cancelled: Option<Box<dyn Fn() -> bool + Send + Sync + 'static>>,
        generation: u64,
        work: impl FnOnce() -> Output + Send + 'static,
        map: impl FnOnce(Output) -> Message + 'static,
    ) -> Self
    where
        Output: Send + 'static,
    {
        Self::perform_worker_effect_with_identity_and_transaction(
            id,
            name,
            priority,
            is_cancelled,
            generation,
            None,
            work,
            map,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn perform_worker_effect_with_identity_and_transaction<Output>(
        id: super::EffectId,
        name: &'static str,
        priority: TaskPriority,
        is_cancelled: Option<Box<dyn Fn() -> bool + Send + Sync + 'static>>,
        generation: u64,
        transaction: Option<crate::application::LatestTaskTransaction>,
        work: impl FnOnce() -> Output + Send + 'static,
        map: impl FnOnce(Output) -> Message + 'static,
    ) -> Self
    where
        Output: Send + 'static,
    {
        Self::perform_worker_effect_with_identity_and_transaction_and_receipt(
            id,
            name,
            priority,
            is_cancelled,
            generation,
            transaction,
            None,
            work,
            map,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn perform_worker_effect_with_identity_and_transaction_and_receipt<Output>(
        id: super::EffectId,
        name: &'static str,
        priority: TaskPriority,
        is_cancelled: Option<Box<dyn Fn() -> bool + Send + Sync + 'static>>,
        generation: u64,
        transaction: Option<crate::application::LatestTaskTransaction>,
        admission_receipt: Option<
            crate::application::runtime::update_context::business::admission::AdmissionReceiptGuard,
        >,
        work: impl FnOnce() -> Output + Send + 'static,
        map: impl FnOnce(Output) -> Message + 'static,
    ) -> Self
    where
        Output: Send + 'static,
    {
        Self::perform_worker_effect_with_identity_and_transaction_and_receipt_for_owner(
            id,
            name,
            priority,
            is_cancelled,
            generation,
            transaction,
            admission_receipt,
            None,
            move |_| work(),
            map,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn perform_worker_effect_with_identity_and_transaction_and_receipt_for_owner<
        Output,
    >(
        id: super::EffectId,
        name: &'static str,
        priority: TaskPriority,
        is_cancelled: Option<Box<dyn Fn() -> bool + Send + Sync + 'static>>,
        generation: u64,
        transaction: Option<crate::application::LatestTaskTransaction>,
        admission_receipt: Option<
            crate::application::runtime::update_context::business::admission::AdmissionReceiptGuard,
        >,
        owner: Option<crate::application::DeclarativeEffectOwner>,
        work: impl FnOnce(Option<WorkerCancellationProbe>) -> Output + Send + 'static,
        map: impl FnOnce(Output) -> Message + 'static,
    ) -> Self
    where
        Output: Send + 'static,
    {
        Self::PerformWorker(super::WorkerEffect {
            name,
            priority,
            is_cancelled,
            owner,
            id,
            generation: super::EffectGeneration(generation),
            transaction,
            admission_receipt,
            work: WorkerEffectWork::Once(Box::new(move |cancellation_probe| {
                Box::new(work(cancellation_probe)) as Box<dyn Any + Send>
            })),
            mapper: WorkerEffectMapper::Once(Box::new(move |output| {
                match output.downcast::<Output>() {
                    Ok(output) => Some(map(*output)),
                    Err(_) => {
                        tracing::error!(
                            effect_name = name,
                            "Radiant worker effect output type did not match its mapper"
                        );
                        None
                    }
                }
            })),
        })
    }

    pub(crate) fn perform_worker_stream_with_priority<Event, Output>(
        name: &'static str,
        priority: TaskPriority,
        options: WorkerStreamOptions,
        work: impl FnOnce(WorkerEffectSink) -> Output + Send + 'static,
        map_event: impl Fn(Event) -> Message + 'static,
        map_final: impl FnOnce(Output) -> Message + 'static,
    ) -> Self
    where
        Event: Send + 'static,
        Output: Send + 'static,
    {
        let id = NEXT_EFFECT_ID.fetch_add(1, Ordering::Relaxed);
        Self::perform_worker_stream_with_identity(
            super::EffectId(id),
            name,
            priority,
            options,
            work,
            map_event,
            map_final,
        )
    }

    /// Compatibility constructor for existing internal owner-stream callers.
    #[allow(dead_code)]
    pub(crate) fn perform_worker_stream_with_priority_and_receipt_for_owner<Event, Output>(
        owner: crate::application::DeclarativeEffectOwner,
        name: &'static str,
        priority: TaskPriority,
        admission_receipt: Option<
            crate::application::runtime::update_context::business::admission::AdmissionReceiptGuard,
        >,
        work: impl FnOnce(WorkerEffectSink, Option<WorkerCancellationProbe>) -> Output + Send + 'static,
        map_event: impl Fn(Event) -> Message + 'static,
        map_final: impl FnOnce(Output) -> Message + 'static,
    ) -> Self
    where
        Event: Send + 'static,
        Output: Send + 'static,
    {
        Self::perform_worker_stream_with_priority_and_receipt_for_owner_with_options(
            owner,
            name,
            priority,
            admission_receipt,
            WorkerStreamOptions {
                is_cancelled: None,
                generation: 0,
                latest: false,
            },
            work,
            map_event,
            map_final,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn perform_worker_stream_with_priority_and_receipt_for_owner_with_options<
        Event,
        Output,
    >(
        owner: crate::application::DeclarativeEffectOwner,
        name: &'static str,
        priority: TaskPriority,
        admission_receipt: Option<
            crate::application::runtime::update_context::business::admission::AdmissionReceiptGuard,
        >,
        options: WorkerStreamOptions,
        work: impl FnOnce(WorkerEffectSink, Option<WorkerCancellationProbe>) -> Output + Send + 'static,
        map_event: impl Fn(Event) -> Message + 'static,
        map_final: impl FnOnce(Output) -> Message + 'static,
    ) -> Self
    where
        Event: Send + 'static,
        Output: Send + 'static,
    {
        let id = NEXT_EFFECT_ID.fetch_add(1, Ordering::Relaxed);
        Self::perform_worker_stream_with_identity_and_transaction_and_receipt_for_owner(
            super::EffectId(id),
            name,
            priority,
            options,
            None,
            admission_receipt,
            Some(owner),
            work,
            map_event,
            map_final,
        )
    }

    pub(crate) fn perform_worker_stream_latest_with_priority_and_receipt_for_owner<Event, Output>(
        owner: crate::application::DeclarativeEffectOwner,
        name: &'static str,
        priority: TaskPriority,
        admission_receipt: Option<
            crate::application::runtime::update_context::business::admission::AdmissionReceiptGuard,
        >,
        work: impl FnOnce(WorkerEffectSink, Option<WorkerCancellationProbe>) -> Output + Send + 'static,
        map_event: impl Fn(Event) -> Message + 'static,
        map_final: impl FnOnce(Output) -> Message + 'static,
    ) -> Self
    where
        Event: Send + 'static,
        Output: Send + 'static,
    {
        let id = NEXT_EFFECT_ID.fetch_add(1, Ordering::Relaxed);
        Self::perform_worker_stream_with_identity_and_transaction_and_receipt_for_owner(
            super::EffectId(id),
            name,
            priority,
            WorkerStreamOptions {
                is_cancelled: None,
                generation: 0,
                latest: true,
            },
            None,
            admission_receipt,
            Some(owner),
            work,
            map_event,
            map_final,
        )
    }

    pub(crate) fn perform_worker_stream_with_identity<Event, Output>(
        id: super::EffectId,
        name: &'static str,
        priority: TaskPriority,
        options: WorkerStreamOptions,
        work: impl FnOnce(WorkerEffectSink) -> Output + Send + 'static,
        map_event: impl Fn(Event) -> Message + 'static,
        map_final: impl FnOnce(Output) -> Message + 'static,
    ) -> Self
    where
        Event: Send + 'static,
        Output: Send + 'static,
    {
        Self::perform_worker_stream_with_identity_and_transaction_and_receipt_for_owner(
            id,
            name,
            priority,
            options,
            None,
            None,
            None,
            move |sink, _| work(sink),
            map_event,
            map_final,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn perform_worker_stream_with_identity_and_transaction<Event, Output>(
        id: super::EffectId,
        name: &'static str,
        priority: TaskPriority,
        options: WorkerStreamOptions,
        transaction: Option<crate::application::LatestTaskTransaction>,
        work: impl FnOnce(WorkerEffectSink) -> Output + Send + 'static,
        map_event: impl Fn(Event) -> Message + 'static,
        map_final: impl FnOnce(Output) -> Message + 'static,
    ) -> Self
    where
        Event: Send + 'static,
        Output: Send + 'static,
    {
        Self::perform_worker_stream_with_identity_and_transaction_and_receipt_for_owner(
            id,
            name,
            priority,
            options,
            transaction,
            None,
            None,
            move |sink, _| work(sink),
            map_event,
            map_final,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn perform_worker_stream_with_identity_and_transaction_and_receipt_for_owner<
        Event,
        Output,
    >(
        id: super::EffectId,
        name: &'static str,
        priority: TaskPriority,
        options: WorkerStreamOptions,
        transaction: Option<crate::application::LatestTaskTransaction>,
        admission_receipt: Option<
            crate::application::runtime::update_context::business::admission::AdmissionReceiptGuard,
        >,
        owner: Option<crate::application::DeclarativeEffectOwner>,
        work: impl FnOnce(WorkerEffectSink, Option<WorkerCancellationProbe>) -> Output + Send + 'static,
        map_event: impl Fn(Event) -> Message + 'static,
        map_final: impl FnOnce(Output) -> Message + 'static,
    ) -> Self
    where
        Event: Send + 'static,
        Output: Send + 'static,
    {
        Self::PerformWorker(super::WorkerEffect {
            name,
            priority,
            is_cancelled: options.is_cancelled,
            owner,
            id,
            generation: super::EffectGeneration(options.generation),
            transaction,
            admission_receipt,
            work: WorkerEffectWork::Stream(Box::new(move |sink, cancellation_probe| {
                Box::new(work(sink, cancellation_probe)) as Box<dyn Any + Send>
            })),
            mapper: WorkerEffectMapper::Stream {
                latest: options.latest,
                map_event: Box::new(move |event| {
                    event
                        .downcast::<Event>()
                        .ok()
                        .map(|event| map_event(*event))
                }),
                map_final: Box::new(move |output| {
                    output
                        .downcast::<Output>()
                        .ok()
                        .map(|output| map_final(*output))
                }),
            },
        })
    }

    /// Build a command that moves keyboard focus to one widget.
    pub const fn focus(widget_id: WidgetId) -> Self {
        Self::Focus(widget_id)
    }

    /// Build a command that clears keyboard focus.
    pub const fn clear_focus() -> Self {
        Self::ClearFocus
    }

    /// Build a command that arms a native external drag session.
    pub fn begin_external_drag(
        request: ExternalDragRequest,
        on_completed: impl FnOnce(Result<ExternalDragOutcome, String>) -> Message + 'static,
    ) -> Self {
        Self::BeginExternalDrag {
            request,
            on_completed: Some(Box::new(on_completed)),
        }
    }

    /// Build a command that begins an in-window drag preview and arms a native
    /// external drag payload as one drag session.
    pub fn begin_drag_with_external(
        drag: DragRequest,
        external: ExternalDragRequest,
        on_completed: impl FnOnce(Result<ExternalDragOutcome, String>) -> Message + 'static,
    ) -> Self {
        Self::batch([
            Self::begin_drag(drag),
            Self::begin_external_drag(external, on_completed),
        ])
    }

    /// Build the commands needed to begin any available drag-session surfaces.
    ///
    /// This is useful when a host gesture may have an in-window preview, a
    /// native external-drag payload, both, or neither. The returned command is
    /// empty when both requests are `None`.
    pub fn begin_drag_session(
        drag: Option<DragRequest>,
        external: Option<ExternalDragRequest>,
        on_completed: impl FnOnce(Result<ExternalDragOutcome, String>) -> Message + 'static,
    ) -> Self {
        match (drag, external) {
            (Some(drag), Some(external)) => {
                Self::begin_drag_with_external(drag, external, on_completed)
            }
            (Some(drag), None) => Self::begin_drag(drag),
            (None, Some(external)) => Self::begin_external_drag(external, on_completed),
            (None, None) => Self::none(),
        }
    }

    /// Build a command that arms a native external drag session without completion notification.
    pub fn begin_external_drag_without_completion(request: ExternalDragRequest) -> Self {
        Self::BeginExternalDrag {
            request,
            on_completed: None,
        }
    }

    /// Build a command that begins a runtime-owned pointer drag preview.
    pub const fn begin_drag(request: DragRequest) -> Self {
        Self::BeginDrag { request }
    }

    /// Build a command that clears any active runtime-owned pointer drag preview.
    pub const fn end_drag() -> Self {
        Self::EndDrag
    }

    /// Build a command that requests a platform service.
    pub fn platform_request(
        request: PlatformRequest,
        on_completed: impl FnOnce(PlatformResult) -> Message + 'static,
    ) -> Self {
        Self::PlatformRequest {
            request,
            on_completed: Box::new(on_completed),
        }
    }

    /// Build a command that clears any active native external drag session.
    pub const fn end_external_drag() -> Self {
        Self::EndExternalDrag
    }

    /// Build a command that asks the active runtime to exit.
    pub const fn exit() -> Self {
        Self::Exit
    }

    fn append_to_batch(self, commands: &mut Vec<Command<Message>>) {
        match self {
            Self::None => {}
            Self::Batch(nested) => {
                commands.reserve(nested.len());
                for command in nested {
                    command.append_to_batch(commands);
                }
            }
            command => commands.push(command),
        }
    }

    fn collapse_for_batch(self) -> Self {
        match self {
            Self::None => Self::None,
            Self::Batch(commands) => Self::batch(commands),
            command => command,
        }
    }
}
