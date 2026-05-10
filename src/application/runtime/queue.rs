struct AppRuntime<Message> {
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
    fn enqueue(&self, message: Message) -> bool {
        if !self.is_alive() {
            return false;
        }
        lock_runtime_state(&self.pending).push(message);
        self.request_repaint();
        true
    }

    fn enqueue_frame(&self, message: Message) -> bool {
        if self.frame_pending.swap(true, Ordering::AcqRel) {
            return false;
        }
        self.enqueue(message)
    }

    fn enqueue_command(&self, command: Command<Message>) -> bool {
        if !self.is_alive() || command.is_empty() {
            return false;
        }
        lock_runtime_state(&self.commands).push(command);
        self.request_repaint();
        true
    }

    fn take_pending(&self) -> Vec<Message> {
        let pending = std::mem::take(&mut *lock_runtime_state(&self.pending));
        self.frame_pending.store(false, Ordering::Release);
        pending
    }

    fn take_commands(&self) -> Vec<Command<Message>> {
        std::mem::take(&mut *lock_runtime_state(&self.commands))
    }

    fn install_repaint(&self, signal: Arc<dyn RepaintSignal>) {
        *lock_runtime_state(&self.repaint) = Some(signal);
    }

    fn request_repaint(&self) {
        let signal = lock_runtime_state(&self.repaint).as_ref().map(Arc::clone);
        if let Some(signal) = signal {
            signal.request_repaint();
        }
    }

    fn shutdown(&self) {
        self.alive.store(false, Ordering::Release);
        self.frame_pending.store(false, Ordering::Release);
        lock_runtime_state(&self.pending).clear();
        lock_runtime_state(&self.commands).clear();
    }

    fn is_alive(&self) -> bool {
        self.alive.load(Ordering::Acquire)
    }
}

fn lock_runtime_state<T>(state: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    state.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
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
}
