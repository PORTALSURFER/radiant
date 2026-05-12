use super::AppRuntime;
use std::sync::{
    Arc, Mutex, Weak,
    mpsc::{self, Sender},
};
use std::thread;
use std::time::Duration;

const RUNTIME_CANCEL_POLL: Duration = Duration::from_millis(50);
const BUSINESS_THREAD_PREFIX: &str = "radiant-business";
const DEFAULT_BUSINESS_WORKERS: usize = 2;

type BusinessJob = Box<dyn FnOnce() + Send + 'static>;

pub(super) struct BusinessThreadPool {
    sender: Sender<BusinessJob>,
    _workers: Vec<thread::JoinHandle<()>>,
}

impl Default for BusinessThreadPool {
    fn default() -> Self {
        Self::new(default_business_worker_count())
    }
}

impl BusinessThreadPool {
    fn new(worker_count: usize) -> Self {
        let worker_count = worker_count.max(1);
        let (sender, receiver) = mpsc::channel::<BusinessJob>();
        let receiver = Arc::new(Mutex::new(receiver));
        let mut workers = Vec::with_capacity(worker_count);
        for worker_index in 0..worker_count {
            let receiver = Arc::clone(&receiver);
            let name = business_thread_name(format!("worker-{worker_index}"));
            let worker = thread::Builder::new()
                .name(name.clone())
                .spawn(move || worker_loop(receiver))
                .unwrap_or_else(|error| panic!("failed to spawn {name}: {error}"));
            workers.push(worker);
        }
        Self {
            sender,
            _workers: workers,
        }
    }

    pub(super) fn spawn(&self, name: &'static str, work: impl FnOnce() + Send + 'static) -> bool {
        match self.sender.send(Box::new(work)) {
            Ok(()) => true,
            Err(_) => {
                eprintln!("Radiant app runtime: failed to queue {name} on business workers");
                false
            }
        }
    }
}

fn worker_loop(receiver: Arc<Mutex<mpsc::Receiver<BusinessJob>>>) {
    loop {
        let Ok(job) = lock_business_receiver(&receiver).recv() else {
            break;
        };
        job();
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
            eprintln!("Radiant app runtime: failed to spawn {name}: {error}");
            false
        }
    }
}

fn business_thread_name(name: impl Into<String>) -> String {
    let name = name.into();
    format!("{BUSINESS_THREAD_PREFIX}-{name}")
}

pub(super) fn sleep_while_runtime_alive<Message>(
    runtime: &Weak<AppRuntime<Message>>,
    duration: Duration,
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
mod tests {
    use super::{
        AppRuntime, BusinessThreadPool, business_thread_name, default_business_worker_count,
        sleep_while_runtime_alive, spawn_business_thread,
    };
    use std::{
        sync::{Arc, mpsc},
        thread,
        time::{Duration, Instant},
    };

    #[test]
    fn spawn_business_thread_reports_success_for_started_work() {
        let (sender, receiver) = mpsc::channel();

        assert!(spawn_business_thread("test-thread", move || {
            let _ = sender.send(());
        }));
        assert!(receiver.recv().is_ok());
    }

    #[test]
    fn business_thread_names_make_offloaded_work_explicit() {
        assert_eq!(
            business_thread_name("asset-load"),
            "radiant-business-asset-load"
        );
    }

    #[test]
    fn default_business_worker_count_keeps_ui_capacity_available() {
        assert!((1..=2).contains(&default_business_worker_count()));
    }

    #[test]
    fn business_thread_pool_runs_queued_work_on_named_workers() {
        let pool = BusinessThreadPool::new(2);
        let (sender, receiver) = mpsc::channel();
        for index in 0..4 {
            let sender = sender.clone();
            assert!(pool.spawn("test-job", move || {
                let thread_name = thread::current().name().unwrap_or_default().to_string();
                sender.send((index, thread_name)).expect("send work result");
            }));
        }
        drop(sender);

        let mut completed = receiver.iter().collect::<Vec<_>>();
        completed.sort_by_key(|(index, _)| *index);

        assert_eq!(completed.len(), 4);
        assert!(
            completed
                .iter()
                .all(|(_, name)| name.starts_with("radiant-business-worker-"))
        );
    }

    #[test]
    fn runtime_sleep_stops_promptly_after_shutdown() {
        let runtime = Arc::new(AppRuntime::<u32>::default());
        let weak = Arc::downgrade(&runtime);
        let stopper = Arc::clone(&runtime);
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(10));
            stopper.shutdown();
        });

        let started = Instant::now();
        assert!(!sleep_while_runtime_alive(&weak, Duration::from_secs(60)));

        assert!(started.elapsed() < Duration::from_secs(1));
    }
}
