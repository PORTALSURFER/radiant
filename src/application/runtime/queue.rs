use super::threading::BusinessThreadPool;
use super::timer::TimerLane;
use crate::runtime::TaskPriority;
use crate::{gui::repaint::RepaintSignal, runtime::Command};
use std::sync::{
    Arc, Mutex, OnceLock,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;

pub(in crate::application) struct AppRuntime<Message> {
    pending: Mutex<Vec<Message>>,
    commands: Mutex<Vec<Command<Message>>>,
    repaint: Mutex<Option<Arc<dyn RepaintSignal>>>,
    business: BusinessThreadPool,
    timers: OnceLock<TimerLane<Message>>,
    alive: AtomicBool,
    frame_pending: AtomicBool,
}

impl<Message> Default for AppRuntime<Message> {
    fn default() -> Self {
        Self {
            pending: Mutex::new(Vec::new()),
            commands: Mutex::new(Vec::new()),
            repaint: Mutex::new(None),
            business: BusinessThreadPool::default(),
            timers: OnceLock::new(),
            alive: AtomicBool::new(true),
            frame_pending: AtomicBool::new(false),
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
        if self.frame_pending.swap(true, Ordering::AcqRel) {
            return false;
        }
        self.enqueue(message)
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
        work: impl FnOnce() + Send + 'static,
    ) -> bool {
        if !self.is_alive() {
            return false;
        }
        self.business.spawn(name, priority, work)
    }

    pub(super) fn can_spawn_business_tasks(&self) -> bool {
        self.business.is_available()
    }

    pub(super) fn take_pending(&self) -> Vec<Message> {
        let pending = drain_runtime_vec(&self.pending);
        self.frame_pending.store(false, Ordering::Release);
        pending
    }

    pub(super) fn drain_pending_into(&self, pending: &mut Vec<Message>) {
        drain_runtime_vec_into(&self.pending, pending);
        self.frame_pending.store(false, Ordering::Release);
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
        self.frame_pending.store(false, Ordering::Release);
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

fn drain_runtime_vec_into<T>(state: &Mutex<Vec<T>>, out: &mut Vec<T>) {
    let mut queued = lock_runtime_state(state);
    out.extend(queued.drain(..));
}

#[cfg(test)]
mod tests;
