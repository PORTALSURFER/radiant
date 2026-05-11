use crate::{gui::repaint::RepaintSignal, runtime::Command};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};

pub(in crate::application) struct AppRuntime<Message> {
    pending: Mutex<Vec<Message>>,
    commands: Mutex<Vec<Command<Message>>>,
    repaint: Mutex<Option<Arc<dyn RepaintSignal>>>,
    alive: AtomicBool,
    frame_pending: AtomicBool,
}

impl<Message> Default for AppRuntime<Message> {
    fn default() -> Self {
        Self {
            pending: Mutex::new(Vec::new()),
            commands: Mutex::new(Vec::new()),
            repaint: Mutex::new(None),
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
mod tests {
    use super::*;
    use std::{
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
        thread,
    };

    struct TestRepaintSignal {
        called: Arc<AtomicBool>,
    }

    impl RepaintSignal for TestRepaintSignal {
        fn request_repaint(&self) {
            self.called.store(true, Ordering::Release);
        }
    }

    #[test]
    fn pending_messages_recover_after_poisoned_queue_lock() {
        let runtime = Arc::new(AppRuntime::<u32>::default());
        let poisoned = Arc::clone(&runtime);
        let _ = thread::spawn(move || {
            let mut pending = poisoned.pending.lock().expect("pending messages lock");
            pending.push(1);
            panic!("poison pending message queue");
        })
        .join();

        assert!(runtime.enqueue(2));

        assert_eq!(runtime.take_pending(), vec![1, 2]);
    }

    #[test]
    fn commands_recover_after_poisoned_queue_lock() {
        let runtime = Arc::new(AppRuntime::<u32>::default());
        let poisoned = Arc::clone(&runtime);
        let _ = thread::spawn(move || {
            let mut commands = poisoned.commands.lock().expect("pending commands lock");
            commands.push(Command::message(1));
            panic!("poison pending command queue");
        })
        .join();

        assert!(runtime.enqueue_command(Command::message(2)));

        assert_eq!(runtime.take_commands().len(), 2);
    }

    #[test]
    fn repaint_requests_recover_after_poisoned_signal_lock() {
        let runtime = Arc::new(AppRuntime::<u32>::default());
        let called = Arc::new(AtomicBool::new(false));
        let poisoned = Arc::clone(&runtime);
        let signal = Arc::clone(&called);
        let _ = thread::spawn(move || {
            let mut repaint = poisoned.repaint.lock().expect("repaint signal lock");
            *repaint = Some(Arc::new(TestRepaintSignal { called: signal }));
            panic!("poison repaint signal lock");
        })
        .join();

        assert!(runtime.enqueue(1));

        assert!(called.load(Ordering::Acquire));
    }

    #[test]
    fn pending_message_queue_retains_capacity_after_drain() {
        let runtime = AppRuntime::<u32>::default();
        for message in 0..32 {
            assert!(runtime.enqueue(message));
        }
        let capacity = runtime.pending.lock().expect("pending lock").capacity();

        let pending = runtime.take_pending();
        assert_eq!(pending.len(), 32);
        assert_eq!(pending.capacity(), capacity);

        let retained_capacity = runtime.pending.lock().expect("pending lock").capacity();
        assert_eq!(retained_capacity, capacity);
    }

    #[test]
    fn command_queue_retains_capacity_after_drain() {
        let runtime = AppRuntime::<u32>::default();
        for message in 0..32 {
            assert!(runtime.enqueue_command(Command::message(message)));
        }
        let capacity = runtime.commands.lock().expect("commands lock").capacity();

        let commands = runtime.take_commands();
        assert_eq!(commands.len(), 32);
        assert_eq!(commands.capacity(), capacity);

        let retained_capacity = runtime.commands.lock().expect("commands lock").capacity();
        assert_eq!(retained_capacity, capacity);
    }

    #[test]
    fn pending_message_queue_drains_into_reused_output_without_replacing_queue_storage() {
        let runtime = AppRuntime::<u32>::default();
        for message in 0..32 {
            assert!(runtime.enqueue(message));
        }
        let queue_capacity = runtime.pending.lock().expect("pending lock").capacity();
        let mut pending = Vec::with_capacity(64);
        let output_capacity = pending.capacity();

        runtime.drain_pending_into(&mut pending);

        assert_eq!(pending, (0..32).collect::<Vec<_>>());
        assert_eq!(pending.capacity(), output_capacity);
        let queue = runtime.pending.lock().expect("pending lock");
        assert!(queue.is_empty());
        assert_eq!(queue.capacity(), queue_capacity);
    }

    #[test]
    fn command_queue_drains_into_reused_output_without_replacing_queue_storage() {
        let runtime = AppRuntime::<u32>::default();
        for message in 0..32 {
            assert!(runtime.enqueue_command(Command::message(message)));
        }
        let queue_capacity = runtime.commands.lock().expect("commands lock").capacity();
        let mut commands = Vec::with_capacity(64);
        let output_capacity = commands.capacity();

        runtime.drain_commands_into(&mut commands);

        assert_eq!(commands.len(), 32);
        assert_eq!(commands.capacity(), output_capacity);
        let queue = runtime.commands.lock().expect("commands lock");
        assert!(queue.is_empty());
        assert_eq!(queue.capacity(), queue_capacity);
    }
}
