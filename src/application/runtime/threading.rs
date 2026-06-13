use super::AppRuntime;
use crate::runtime::{
    BusinessTaskDiagnosticState, RuntimeDiagnosticsRecorder, TaskPriority, elapsed_since,
};
use std::sync::{
    Arc, Mutex, Weak,
    mpsc::{self, Sender},
};
use std::thread;

mod platform;

#[cfg(test)]
const RUNTIME_CANCEL_POLL: std::time::Duration = std::time::Duration::from_millis(50);
const BUSINESS_THREAD_PREFIX: &str = "radiant-business";
const DEFAULT_BUSINESS_WORKERS: usize = 2;

struct BusinessJob {
    priority: TaskPriority,
    name: &'static str,
    queued_at: std::time::Instant,
    is_cancelled: Option<Box<dyn Fn() -> bool + Send + Sync + 'static>>,
    work: Box<dyn FnOnce() + Send + 'static>,
}

/// Runtime-owned lane for application business work.
///
/// The UI/event/render path must be able to start even when background worker
/// capacity is unavailable. Work submission reports failure instead of falling
/// back to synchronous execution on the UI owner.
pub(super) struct BusinessThreadPool {
    sender: Option<Sender<BusinessJob>>,
    diagnostics: Arc<RuntimeDiagnosticsRecorder>,
    _workers: Vec<thread::JoinHandle<()>>,
}

impl Default for BusinessThreadPool {
    fn default() -> Self {
        Self::new(default_business_worker_count())
    }
}

impl BusinessThreadPool {
    fn new(worker_count: usize) -> Self {
        Self::with_diagnostics_and_worker_count(
            Arc::new(RuntimeDiagnosticsRecorder::default()),
            worker_count,
        )
    }

    pub(super) fn new_with_diagnostics(diagnostics: Arc<RuntimeDiagnosticsRecorder>) -> Self {
        Self::with_diagnostics_and_worker_count(diagnostics, default_business_worker_count())
    }

    fn with_diagnostics_and_worker_count(
        diagnostics: Arc<RuntimeDiagnosticsRecorder>,
        worker_count: usize,
    ) -> Self {
        let worker_count = worker_count.max(1);
        let (sender, receiver) = mpsc::channel::<BusinessJob>();
        let receiver = Arc::new(Mutex::new(receiver));
        let mut workers = Vec::with_capacity(worker_count);
        for worker_index in 0..worker_count {
            let receiver = Arc::clone(&receiver);
            let diagnostics = Arc::clone(&diagnostics);
            let name = business_thread_name(format!("worker-{worker_index}"));
            match thread::Builder::new()
                .name(name.clone())
                .spawn(move || worker_loop(receiver, diagnostics))
            {
                Ok(worker) => workers.push(worker),
                Err(error) => {
                    tracing::warn!(
                        thread.name = %name,
                        error = %error,
                        "Radiant app runtime failed to spawn business worker"
                    );
                }
            }
        }
        let sender = (!workers.is_empty()).then_some(sender);
        Self {
            sender,
            diagnostics,
            _workers: workers,
        }
    }

    pub(super) fn spawn(
        &self,
        name: &'static str,
        priority: TaskPriority,
        is_cancelled: Option<Box<dyn Fn() -> bool + Send + Sync + 'static>>,
        work: impl FnOnce() + Send + 'static,
    ) -> bool {
        let Some(sender) = &self.sender else {
            self.diagnostics.record_business_rejected(name, priority);
            tracing::warn!(
                work.name = name,
                "Radiant app runtime has no business workers available; refusing to block the UI path"
            );
            return false;
        };
        match sender.send(BusinessJob {
            priority,
            name,
            queued_at: std::time::Instant::now(),
            is_cancelled,
            work: Box::new(work),
        }) {
            Ok(()) => {
                self.diagnostics.record_business_queued(name, priority);
                true
            }
            Err(_) => {
                self.diagnostics.record_business_rejected(name, priority);
                tracing::warn!(
                    work.name = name,
                    "Radiant app runtime failed to queue work on business workers"
                );
                false
            }
        }
    }

    pub(super) const fn is_available(&self) -> bool {
        self.sender.is_some()
    }

    #[cfg(test)]
    fn without_workers_for_test() -> Self {
        Self {
            sender: None,
            diagnostics: Arc::new(RuntimeDiagnosticsRecorder::default()),
            _workers: Vec::new(),
        }
    }
}

fn worker_loop(
    receiver: Arc<Mutex<mpsc::Receiver<BusinessJob>>>,
    diagnostics: Arc<RuntimeDiagnosticsRecorder>,
) {
    platform::configure_business_worker_thread(TaskPriority::Background);
    loop {
        let Ok(job) = lock_business_receiver(&receiver).recv() else {
            break;
        };
        platform::configure_business_worker_thread(job.priority);
        let queue_delay = elapsed_since(job.queued_at);
        diagnostics.record_business_started(job.name, job.priority, queue_delay);
        let started = std::time::Instant::now();
        (job.work)();
        let state = if job
            .is_cancelled
            .as_ref()
            .is_some_and(|is_cancelled| is_cancelled())
        {
            BusinessTaskDiagnosticState::Cancelled
        } else {
            BusinessTaskDiagnosticState::Completed
        };
        diagnostics.record_business_finished(job.name, job.priority, state, elapsed_since(started));
        platform::configure_business_worker_thread(TaskPriority::Background);
    }
}

fn lock_business_receiver(
    receiver: &Mutex<mpsc::Receiver<BusinessJob>>,
) -> std::sync::MutexGuard<'_, mpsc::Receiver<BusinessJob>> {
    receiver
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn default_business_worker_count() -> usize {
    thread::available_parallelism()
        .map(|parallelism| parallelism.get().saturating_sub(1))
        .unwrap_or(DEFAULT_BUSINESS_WORKERS)
        .clamp(1, DEFAULT_BUSINESS_WORKERS)
}

pub(super) fn spawn_business_thread(
    name: impl Into<String>,
    work: impl FnOnce() + Send + 'static,
) -> bool {
    spawn_named_thread(business_thread_name(name), work)
}

fn spawn_named_thread(name: String, work: impl FnOnce() + Send + 'static) -> bool {
    match thread::Builder::new().name(name.clone()).spawn(work) {
        Ok(_) => true,
        Err(error) => {
            tracing::warn!(
                thread.name = %name,
                error = %error,
                "Radiant app runtime failed to spawn business thread"
            );
            false
        }
    }
}

fn business_thread_name(name: impl Into<String>) -> String {
    let name = name.into();
    format!("{BUSINESS_THREAD_PREFIX}-{name}")
}

#[cfg(test)]
pub(super) fn sleep_while_runtime_alive<Message>(
    runtime: &Weak<AppRuntime<Message>>,
    duration: std::time::Duration,
) -> bool {
    let mut remaining = duration;
    while !remaining.is_zero() {
        if !runtime_alive(runtime) {
            return false;
        }
        let sleep_for = remaining.min(RUNTIME_CANCEL_POLL);
        thread::sleep(sleep_for);
        remaining = remaining.saturating_sub(sleep_for);
    }
    runtime_alive(runtime)
}

pub(super) fn runtime_alive<Message>(runtime: &Weak<AppRuntime<Message>>) -> bool {
    runtime.upgrade().is_some_and(|runtime| runtime.is_alive())
}

#[cfg(test)]
mod tests;
