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
        self.pending.lock().expect("pending messages poisoned").push(message);
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
        self.commands
            .lock()
            .expect("pending commands poisoned")
            .push(command);
        self.request_repaint();
        true
    }

    fn take_pending(&self) -> Vec<Message> {
        let pending = std::mem::take(&mut *self.pending.lock().expect("pending messages poisoned"));
        self.frame_pending.store(false, Ordering::Release);
        pending
    }

    fn take_commands(&self) -> Vec<Command<Message>> {
        std::mem::take(&mut *self.commands.lock().expect("pending commands poisoned"))
    }

    fn install_repaint(&self, signal: Arc<dyn RepaintSignal>) {
        *self.repaint.lock().expect("repaint signal poisoned") = Some(signal);
    }

    fn request_repaint(&self) {
        if let Some(signal) = self
            .repaint
            .lock()
            .expect("repaint signal poisoned")
            .as_ref()
        {
            signal.request_repaint();
        }
    }

    fn shutdown(&self) {
        self.alive.store(false, Ordering::Release);
        self.frame_pending.store(false, Ordering::Release);
        self.pending
            .lock()
            .expect("pending messages poisoned")
            .clear();
        self.commands
            .lock()
            .expect("pending commands poisoned")
            .clear();
    }

    fn is_alive(&self) -> bool {
        self.alive.load(Ordering::Acquire)
    }
}
