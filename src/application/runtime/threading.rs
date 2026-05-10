use std::thread;

pub(super) fn spawn_runtime_thread(
    name: impl Into<String>,
    work: impl FnOnce() + Send + 'static,
) -> bool {
    let name = name.into();
    match thread::Builder::new().name(name.clone()).spawn(work) {
        Ok(_) => true,
        Err(error) => {
            eprintln!("Radiant app runtime: failed to spawn {name}: {error}");
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::spawn_runtime_thread;
    use std::sync::mpsc;

    #[test]
    fn spawn_runtime_thread_reports_success_for_started_work() {
        let (sender, receiver) = mpsc::channel();

        assert!(spawn_runtime_thread("radiant-test-thread", move || {
            let _ = sender.send(());
        }));
        assert!(receiver.recv().is_ok());
    }
}
