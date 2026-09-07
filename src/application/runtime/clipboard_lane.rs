use super::SharedRuntimeIngress;
use super::queue::DeliveryReservation;
use crate::runtime::{
    PlatformFailure, PlatformRequest, PlatformResponse, PlatformResult, RuntimePlatformResultSink,
};
use std::sync::{
    Arc, Weak,
    atomic::{AtomicBool, Ordering},
    mpsc::{Receiver, SyncSender, TrySendError, sync_channel},
};

pub(super) const CLIPBOARD_LANE_CAPACITY: usize = 64;

/// A serial lane for requests whose platform backends retain clipboard ownership.
///
/// The sender is owned by the runtime ingress. Dropping it lets the worker exit after
/// the current operation; shutdown also makes queued work inert without joining it.
pub(super) struct ClipboardLane {
    sender: SyncSender<LaneMessage>,
    accepting: Arc<AtomicBool>,
}

impl ClipboardLane {
    pub(super) fn start() -> Result<Self, ()> {
        let (sender, receiver) = sync_channel(CLIPBOARD_LANE_CAPACITY);
        let accepting = Arc::new(AtomicBool::new(true));
        let worker_accepting = Arc::clone(&accepting);
        std::thread::Builder::new()
            .name("radiant-clipboard-service".to_owned())
            .spawn(move || run_lane(receiver, worker_accepting))
            .map_err(|_| ())?;
        Ok(Self { sender, accepting })
    }

    pub(super) fn submit(&self, job: ClipboardJob) -> Result<(), ClipboardJob> {
        if !self.accepting.load(Ordering::Acquire) {
            return Err(job);
        }
        match self.sender.try_send(LaneMessage::Job(job)) {
            Ok(()) => Ok(()),
            Err(
                TrySendError::Full(LaneMessage::Job(job))
                | TrySendError::Disconnected(LaneMessage::Job(job)),
            ) => Err(job),
            Err(
                TrySendError::Full(LaneMessage::Shutdown)
                | TrySendError::Disconnected(LaneMessage::Shutdown),
            ) => unreachable!("shutdown is internal"),
        }
    }

    pub(super) fn shutdown(&self) {
        self.accepting.store(false, Ordering::Release);
        // This wakes an idle worker. If the bounded queue is full, the first queued
        // job observes `accepting == false` and exits, dropping the rest.
        let _ = self.sender.try_send(LaneMessage::Shutdown);
    }
}

enum LaneMessage {
    Job(ClipboardJob),
    Shutdown,
}

pub(super) struct ClipboardJob {
    request: PlatformRequest,
    sink: RuntimePlatformResultSink,
    reservation: DeliveryReservation,
    runtime: Weak<SharedRuntimeIngress>,
}

impl ClipboardJob {
    pub(in crate::application::runtime) fn new(
        request: PlatformRequest,
        sink: RuntimePlatformResultSink,
        reservation: DeliveryReservation,
        runtime: Weak<SharedRuntimeIngress>,
    ) -> Self {
        Self {
            request,
            sink,
            reservation,
            runtime,
        }
    }

    pub(in crate::application::runtime) fn into_fallback(
        self,
    ) -> (PlatformRequest, RuntimePlatformResultSink) {
        (self.request, self.sink)
    }

    fn run(self, backend: &mut dyn ClipboardBackend) {
        // Cancellation is cooperative: do not begin an OS clipboard operation after the
        // owner has revoked it. A native call already in progress cannot be interrupted.
        let response = {
            let is_active = || {
                !self.sink.is_cancelled()
                    && self
                        .runtime
                        .upgrade()
                        .is_some_and(|runtime| runtime.is_alive())
            };
            if is_active() {
                backend.perform(&self.request, &is_active)
            } else {
                None
            }
        };
        let Some(response) = response else {
            self.finish(Err(PlatformFailure::transport(
                "Clipboard operation canceled",
            )));
            return;
        };
        self.finish(response);
    }

    fn finish(self, response: PlatformResult) {
        if let Some(runtime) = self.runtime.upgrade()
            && runtime.enqueue_platform_completion_reserved(
                self.reservation,
                self.sink.into_delivery(response),
            )
        {
            runtime.request_repaint();
        }
    }
}

trait ClipboardBackend {
    /// `None` means the owner was revoked after the lane dequeued work.
    fn perform(
        &mut self,
        request: &PlatformRequest,
        is_active: &dyn Fn() -> bool,
    ) -> Option<PlatformResult>;
}

struct ArboardClipboardBackend {
    clipboard: Option<arboard::Clipboard>,
}

impl ArboardClipboardBackend {
    fn clipboard(&mut self) -> Result<&mut arboard::Clipboard, PlatformFailure> {
        if self.clipboard.is_none() {
            self.clipboard = Some(
                arboard::Clipboard::new()
                    .map_err(|_| PlatformFailure::transport("Clipboard unavailable"))?,
            );
        }
        self.clipboard
            .as_mut()
            .ok_or_else(|| PlatformFailure::transport("Clipboard unavailable"))
    }
}

impl ClipboardBackend for ArboardClipboardBackend {
    fn perform(
        &mut self,
        request: &PlatformRequest,
        is_active: &dyn Fn() -> bool,
    ) -> Option<PlatformResult> {
        match request {
            PlatformRequest::CopyText(text) => {
                let clipboard = match self.clipboard() {
                    Ok(clipboard) => clipboard,
                    Err(error) => return Some(Err(error)),
                };
                if !is_active() {
                    return None;
                }
                Some(
                    clipboard
                        .set_text(text.clone())
                        .map_err(|_| PlatformFailure::transport("Clipboard write failed"))
                        .map(|()| PlatformResponse::Completed),
                )
            }
            PlatformRequest::CopyFilePaths(paths) => {
                if paths.is_empty() {
                    return Some(Err(PlatformFailure::transport("Clipboard write failed")));
                }
                let clipboard = match self.clipboard() {
                    Ok(clipboard) => clipboard,
                    Err(error) => return Some(Err(error)),
                };
                if !is_active() {
                    return None;
                }
                let result = clipboard
                    .set()
                    .file_list(paths)
                    .map_err(|_| PlatformFailure::transport("Clipboard write failed"))
                    .map(|()| PlatformResponse::Completed);
                Some(result)
            }
            PlatformRequest::ReadText => {
                let clipboard = match self.clipboard() {
                    Ok(clipboard) => clipboard,
                    Err(error) => return Some(Err(error)),
                };
                if !is_active() {
                    return None;
                }
                Some(
                    clipboard
                        .get_text()
                        .map(PlatformResponse::Text)
                        .map_err(|_| PlatformFailure::transport("Clipboard read failed")),
                )
            }
            PlatformRequest::ReadFilePaths => {
                let clipboard = match self.clipboard() {
                    Ok(clipboard) => clipboard,
                    Err(error) => return Some(Err(error)),
                };
                if !is_active() {
                    return None;
                }
                Some(
                    clipboard
                        .get()
                        .file_list()
                        .map(PlatformResponse::FilePaths)
                        .map_err(|_| PlatformFailure::transport("Clipboard read failed")),
                )
            }
            _ => Some(Err(PlatformFailure::InvalidRequest)),
        }
    }
}

fn run_lane(receiver: Receiver<LaneMessage>, accepting: Arc<AtomicBool>) {
    let mut backend = ArboardClipboardBackend { clipboard: None };
    run_lane_with_backend(receiver, accepting, &mut backend);
}

fn run_lane_with_backend(
    receiver: Receiver<LaneMessage>,
    accepting: Arc<AtomicBool>,
    backend: &mut dyn ClipboardBackend,
) {
    while let Ok(message) = receiver.recv() {
        let LaneMessage::Job(job) = message else {
            break;
        };
        if !accepting.load(Ordering::Acquire) {
            break;
        }
        job.run(backend);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::PlatformCompletionIdentity;
    use std::sync::{Mutex, mpsc::sync_channel};

    #[derive(Default)]
    struct FakeClipboard {
        calls: Arc<Mutex<Vec<&'static str>>>,
    }

    impl ClipboardBackend for FakeClipboard {
        fn perform(
            &mut self,
            request: &PlatformRequest,
            _is_active: &dyn Fn() -> bool,
        ) -> Option<PlatformResult> {
            let call = match request {
                PlatformRequest::CopyText(_) => "copy-text",
                PlatformRequest::ReadText => "read-text",
                _ => "other",
            };
            self.calls.lock().expect("fake calls poisoned").push(call);
            Some(Ok(PlatformResponse::Completed))
        }
    }

    fn job(
        runtime: &Arc<SharedRuntimeIngress>,
        request: PlatformRequest,
        cancelled: bool,
    ) -> ClipboardJob {
        let reservation = runtime.reserve_delivery().expect("reservation");
        let cancellation = Arc::new(move || cancelled);
        ClipboardJob::new(
            request,
            RuntimePlatformResultSink::new(PlatformCompletionIdentity { id: 1, epoch: 1 }, |_| {})
                .with_cancellation(cancellation),
            reservation,
            Arc::downgrade(runtime),
        )
    }

    #[test]
    fn lane_preserves_order_and_skips_cancelled_work() {
        let runtime = Arc::new(SharedRuntimeIngress::default());
        let (sender, receiver) = sync_channel(3);
        let accepting = Arc::new(AtomicBool::new(true));
        sender
            .send(LaneMessage::Job(job(
                &runtime,
                PlatformRequest::CopyText("first".into()),
                false,
            )))
            .expect("first job");
        sender
            .send(LaneMessage::Job(job(
                &runtime,
                PlatformRequest::ReadText,
                true,
            )))
            .expect("cancelled job");
        sender
            .send(LaneMessage::Job(job(
                &runtime,
                PlatformRequest::CopyText("last".into()),
                false,
            )))
            .expect("last job");
        drop(sender);
        let mut backend = FakeClipboard::default();
        let calls = Arc::clone(&backend.calls);
        run_lane_with_backend(receiver, accepting, &mut backend);
        assert_eq!(
            calls.lock().expect("fake calls poisoned").as_slice(),
            ["copy-text", "copy-text"]
        );
    }

    #[test]
    fn lane_shutdown_skips_queued_work() {
        let runtime = Arc::new(SharedRuntimeIngress::default());
        let (sender, receiver) = sync_channel(1);
        let accepting = Arc::new(AtomicBool::new(false));
        sender
            .send(LaneMessage::Job(job(
                &runtime,
                PlatformRequest::CopyText("queued".into()),
                false,
            )))
            .expect("queued job");
        drop(sender);
        let mut backend = FakeClipboard::default();
        let calls = Arc::clone(&backend.calls);
        run_lane_with_backend(receiver, accepting, &mut backend);
        assert!(calls.lock().expect("fake calls poisoned").is_empty());
    }

    #[test]
    fn lane_admission_is_bounded_without_starting_a_clipboard_backend() {
        let runtime = Arc::new(SharedRuntimeIngress::default());
        let (sender, _receiver) = sync_channel(1);
        let lane = ClipboardLane {
            sender,
            accepting: Arc::new(AtomicBool::new(true)),
        };
        assert!(
            lane.submit(job(&runtime, PlatformRequest::ReadText, false))
                .is_ok()
        );
        assert!(
            lane.submit(job(&runtime, PlatformRequest::ReadText, false))
                .is_err()
        );
    }
}
