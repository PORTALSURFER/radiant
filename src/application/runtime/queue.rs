use super::threading::BusinessThreadPool;
use super::timer::TimerLane;
use crate::runtime::{RuntimeDiagnostics, RuntimeDiagnosticsRecorder, TaskPriority};
use crate::{gui::repaint::RepaintSignal, runtime::Command};
use std::sync::{
    Arc, Mutex, OnceLock,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::application) struct RuntimeStreamSlot(u64);

enum PendingMessage<Message> {
    Ordinary(Message),
    StreamLatest {
        slot: RuntimeStreamSlot,
        message: Message,
    },
}

pub(in crate::application) struct AppRuntime<Message> {
    pending: Mutex<Vec<PendingMessage<Message>>>,
    pending_frame: Mutex<Option<Message>>,
    commands: Mutex<Vec<Command<Message>>>,
    repaint: Mutex<Option<Arc<dyn RepaintSignal>>>,
    business: BusinessThreadPool,
    diagnostics: Arc<RuntimeDiagnosticsRecorder>,
    timers: OnceLock<TimerLane<Message>>,
    alive: AtomicBool,
    next_stream_slot: AtomicU64,
}

impl<Message> Default for AppRuntime<Message> {
    fn default() -> Self {
        let diagnostics = Arc::new(RuntimeDiagnosticsRecorder::default());
        Self {
            pending: Mutex::new(Vec::new()),
            pending_frame: Mutex::new(None),
            commands: Mutex::new(Vec::new()),
            repaint: Mutex::new(None),
            business: BusinessThreadPool::new_with_diagnostics(Arc::clone(&diagnostics)),
            diagnostics,
            timers: OnceLock::new(),
            alive: AtomicBool::new(true),
            next_stream_slot: AtomicU64::new(1),
        }
    }
}

impl<Message> AppRuntime<Message> {
    pub(super) fn enqueue(&self, message: Message) -> bool {
        if !self.is_alive() {
            return false;
        }
        {
            let mut pending = lock_runtime_state(&self.pending);
            pending.push(PendingMessage::Ordinary(message));
            self.record_pending_depth(&pending);
        }
        self.request_repaint();
        true
    }

    pub(super) fn begin_stream_slot(&self) -> RuntimeStreamSlot {
        RuntimeStreamSlot(self.next_stream_slot.fetch_add(1, Ordering::Relaxed))
    }

    pub(super) fn enqueue_stream_latest(&self, slot: RuntimeStreamSlot, message: Message) -> bool {
        if !self.is_alive() {
            self.diagnostics.record_stream_message_dropped();
            return false;
        }
        {
            let mut pending = lock_runtime_state(&self.pending);
            if let Some(existing) = pending.iter_mut().find_map(|pending| match pending {
                PendingMessage::StreamLatest {
                    slot: pending_slot,
                    message,
                } if *pending_slot == slot => Some(message),
                PendingMessage::Ordinary(_) | PendingMessage::StreamLatest { .. } => None,
            }) {
                *existing = message;
                self.diagnostics.record_stream_message_coalesced();
                self.record_pending_depth(&pending);
            } else {
                pending.push(PendingMessage::StreamLatest { slot, message });
                self.record_pending_depth(&pending);
            }
        }
        self.request_repaint();
        true
    }

    pub(super) fn record_stale_stream_event(&self) {
        self.diagnostics.record_stream_message_stale();
    }

    pub(super) fn enqueue_frame(&self, message: Message) -> bool {
        if !self.is_alive() {
            return false;
        }
        {
            let mut pending_frame = lock_runtime_state(&self.pending_frame);
            if pending_frame.is_some() {
                return false;
            }
            *pending_frame = Some(message);
        }
        self.record_current_pending_depth();
        self.request_repaint();
        true
    }

    pub(super) fn enqueue_command(&self, command: Command<Message>) -> bool {
        if !self.is_alive() || command.is_empty() {
            return false;
        }
        lock_runtime_state(&self.commands).push(command);
        self.request_repaint();
        true
    }

    pub(super) fn spawn_business_task(
        &self,
        name: &'static str,
        priority: TaskPriority,
        is_cancelled: Option<Box<dyn Fn() -> bool + Send + Sync + 'static>>,
        work: impl FnOnce() + Send + 'static,
    ) -> bool {
        if !self.is_alive() {
            return false;
        }
        self.business.spawn(name, priority, is_cancelled, work)
    }

    pub(super) fn spawn_business_task_with_payload<Payload>(
        &self,
        name: &'static str,
        priority: TaskPriority,
        payload: Payload,
        work: impl FnOnce(Payload) + Send + 'static,
    ) -> Result<(), Payload>
    where
        Payload: Send + 'static,
    {
        if !self.is_alive() {
            return Err(payload);
        }
        self.business
            .spawn_with_payload(name, priority, payload, work)
    }

    pub(super) fn can_spawn_business_tasks(&self, priority: TaskPriority) -> bool {
        self.business.is_available(priority)
    }

    pub(super) fn diagnostics_snapshot(&self) -> RuntimeDiagnostics {
        self.diagnostics.snapshot()
    }

    pub(super) fn take_pending(&self) -> Vec<Message> {
        let frame = lock_runtime_state(&self.pending_frame).take();
        let pending = drain_runtime_vec(&self.pending)
            .into_iter()
            .map(PendingMessage::into_message)
            .collect();
        self.record_current_pending_depth();
        prepend_pending_frame(frame, pending)
    }

    pub(super) fn drain_pending_into(&self, pending: &mut Vec<Message>) {
        if let Some(frame) = lock_runtime_state(&self.pending_frame).take() {
            pending.insert(0, frame);
        }
        let mut queued = lock_runtime_state(&self.pending);
        pending.extend(queued.drain(..).map(PendingMessage::into_message));
        self.record_pending_depth(&queued);
    }

    pub(super) fn drain_pending_batch_into(
        &self,
        pending: &mut Vec<Message>,
        max_messages: usize,
    ) -> bool {
        let max_messages = max_messages.max(1);
        if let Some(frame) = lock_runtime_state(&self.pending_frame).take() {
            pending.insert(0, frame);
        }
        let available = max_messages.saturating_sub(pending.len());
        let mut queued = lock_runtime_state(&self.pending);
        if available > 0 {
            let drain_count = queued.len().min(available);
            pending.extend(
                queued
                    .drain(..drain_count)
                    .map(PendingMessage::into_message),
            );
        }
        let remaining = !queued.is_empty();
        self.record_pending_depth(&queued);
        remaining
    }

    pub(super) fn take_commands(&self) -> Vec<Command<Message>> {
        drain_runtime_vec(&self.commands)
    }

    pub(super) fn drain_commands_into(&self, commands: &mut Vec<Command<Message>>) {
        drain_runtime_vec_into(&self.commands, commands);
    }

    pub(super) fn install_repaint(&self, signal: Arc<dyn RepaintSignal>) {
        *lock_runtime_state(&self.repaint) = Some(signal);
    }

    fn request_repaint(&self) {
        let signal = lock_runtime_state(&self.repaint).as_ref().map(Arc::clone);
        if let Some(signal) = signal {
            signal.request_repaint();
        }
    }

    pub(super) fn shutdown(&self) {
        self.alive.store(false, Ordering::Release);
        *lock_runtime_state(&self.pending_frame) = None;
        lock_runtime_state(&self.pending).clear();
        lock_runtime_state(&self.commands).clear();
        self.record_current_pending_depth();
    }

    pub(super) fn is_alive(&self) -> bool {
        self.alive.load(Ordering::Acquire)
    }

    fn record_current_pending_depth(&self) {
        let pending = lock_runtime_state(&self.pending);
        self.record_pending_depth(&pending);
    }

    fn record_pending_depth(&self, pending: &[PendingMessage<Message>]) {
        let pending_frame = lock_runtime_state(&self.pending_frame).is_some() as usize;
        self.diagnostics.record_message_queue_depth(
            pending.len() + pending_frame,
            pending.iter().filter(|message| message.is_stream()).count(),
        );
    }
}

impl<Message> AppRuntime<Message>
where
    Message: Send + 'static,
{
    pub(super) fn schedule_message(self: &Arc<Self>, delay: Duration, message: Message) -> bool {
        if !self.is_alive() {
            return false;
        }
        self.timers
            .get_or_init(TimerLane::new)
            .schedule(Arc::downgrade(self), delay, message)
    }

    pub(super) fn schedule_interval(
        self: &Arc<Self>,
        every: Duration,
        message: Arc<dyn Fn() -> Message + Send + Sync>,
    ) -> bool {
        if !self.is_alive() {
            return false;
        }
        self.timers.get_or_init(TimerLane::new).schedule_interval(
            Arc::downgrade(self),
            every,
            message,
        )
    }
}

fn lock_runtime_state<T>(state: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    state
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn drain_runtime_vec<T>(state: &Mutex<Vec<T>>) -> Vec<T> {
    let mut queued = lock_runtime_state(state);
    let retained_capacity = queued.capacity();
    std::mem::replace(&mut *queued, Vec::with_capacity(retained_capacity))
}

impl<Message> PendingMessage<Message> {
    fn into_message(self) -> Message {
        match self {
            Self::Ordinary(message) | Self::StreamLatest { message, .. } => message,
        }
    }

    fn is_stream(&self) -> bool {
        matches!(self, Self::StreamLatest { .. })
    }
}

fn prepend_pending_frame<T>(frame: Option<T>, mut pending: Vec<T>) -> Vec<T> {
    if let Some(frame) = frame {
        pending.insert(0, frame);
    }
    pending
}

fn drain_runtime_vec_into<T>(state: &Mutex<Vec<T>>, out: &mut Vec<T>) {
    let mut queued = lock_runtime_state(state);
    out.extend(queued.drain(..));
}

#[cfg(test)]
mod tests;
