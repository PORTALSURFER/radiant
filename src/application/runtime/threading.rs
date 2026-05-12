use super::AppRuntime;
use std::sync::Weak;
use std::thread;
use std::time::Duration;

const RUNTIME_CANCEL_POLL: Duration = Duration::from_millis(50);
const BUSINESS_THREAD_PREFIX: &str = "radiant-business";

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
        AppRuntime, business_thread_name, sleep_while_runtime_alive, spawn_business_thread,
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
