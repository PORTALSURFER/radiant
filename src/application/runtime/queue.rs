use super::threading::BusinessThreadPool;
use super::timer::TimerLane;
use crate::runtime::{RuntimeDiagnostics, RuntimeDiagnosticsRecorder, TaskPriority};
use crate::{gui::repaint::RepaintSignal, runtime::Command};
use std::sync::{
    Arc, Mutex, OnceLock,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;

pub(in crate::application) struct AppRuntime<Message> {
    pending: Mutex<Vec<Message>>,
    pending_frame: Mutex<Option<Message>>,
    commands: Mutex<Vec<Command<Message>>>,
    repaint: Mutex<Option<Arc<dyn RepaintSignal>>>,
    business: BusinessThreadPool,
    diagnostics: Arc<RuntimeDiagnosticsRecorder>,
    timers: OnceLock<TimerLane<Message>>,
    alive: AtomicBool,
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
        }
    }
}

impl<Message> AppRuntime<Message> {
    pub(super) fn enqueue(&self, message: Message) -> bool {
        if !self.is_alive() {
            return false;
        }
        lock_runtime_state(&self.pending).push(message);
        self.request_repaint();
        true
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

    pub(super) fn can_spawn_business_tasks(&self) -> bool {
        self.business.is_available()
    }

    pub(super) fn diagnostics_snapshot(&self) -> RuntimeDiagnostics {
        self.diagnostics.snapshot()
    }

    pub(super) fn take_pending(&self) -> Vec<Message> {
        let frame = lock_runtime_state(&self.pending_frame).take();
        let pending = drain_runtime_vec(&self.pending);
        prepend_pending_frame(frame, pending)
    }

    pub(super) fn drain_pending_into(&self, pending: &mut Vec<Message>) {
        if let Some(frame) = lock_runtime_state(&self.pending_frame).take() {
            pending.insert(0, frame);
        }
        drain_runtime_vec_into(&self.pending, pending);
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
    }

    pub(super) fn is_alive(&self) -> bool {
        self.alive.load(Ordering::Acquire)
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
