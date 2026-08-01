//! Bounded, private WGPU device-loss recovery coordination.

use super::{
    RuntimeUserEvent,
    adapter::{GenericNativeAdapterOwner, NativeAdapterGeneration},
    device::install_device_loss_callback,
    runner_state::NativeWindowResourceBundle,
};
use crate::gui_runtime::native_vello::{select_present_mode, startup_renderer_options};
use std::{
    future::Future,
    pin::pin,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{Receiver, TryRecvError, sync_channel},
    },
    task::{Context, Wake, Waker},
    thread,
};
use vello::{Renderer, util::RenderContext, wgpu};
use winit::event_loop::EventLoopProxy;

/// Opaque identity for one accepted recovery episode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::gui_runtime::native_vello) struct NativeRecoveryEpisodeToken(u64);

impl NativeRecoveryEpisodeToken {
    fn next(serial: &mut u64) -> Option<Self> {
        let next = serial.checked_add(1)?;
        *serial = next;
        Some(Self(next))
    }

    #[cfg(test)]
    const fn from_test_serial(serial: u64) -> Self {
        Self(serial)
    }
}

pub(super) struct NativeRecoveryRequest {
    pub(super) instance: wgpu::Instance,
    pub(super) surface: wgpu::Surface<'static>,
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) target_fps: u32,
    pub(super) generation: NativeAdapterGeneration,
    pub(super) previous_device_identity: usize,
    pub(super) event_proxy: EventLoopProxy<RuntimeUserEvent>,
}

pub(super) struct NativeRecoveryCandidate {
    pub(super) adapter: GenericNativeAdapterOwner,
    pub(super) primary: NativeWindowResourceBundle,
}

struct RecoveryWorkerWake {
    thread: thread::Thread,
    notified: AtomicBool,
}

impl Wake for RecoveryWorkerWake {
    fn wake(self: Arc<Self>) {
        self.notified.store(true, Ordering::Release);
        self.thread.unpark();
    }
}

/// Drive one WGPU future on the recovery worker while allowing WGPU's
/// non-blocking instance poll to make progress. This is deliberately a
/// one-shot future driver, not an event-loop executor or a retry policy.
fn drive_wgpu_future<F>(instance: &wgpu::Instance, future: F) -> F::Output
where
    F: Future + Send,
{
    let wake = Arc::new(RecoveryWorkerWake {
        thread: thread::current(),
        notified: AtomicBool::new(false),
    });
    let waker = Waker::from(Arc::clone(&wake));
    let mut context = Context::from_waker(&waker);
    let mut future = pin!(future);
    loop {
        match future.as_mut().poll(&mut context) {
            std::task::Poll::Ready(output) => return output,
            std::task::Poll::Pending => {
                let _ = instance.poll_all(false);
                if !wake.notified.swap(false, Ordering::Acquire) {
                    thread::park();
                }
            }
        }
    }
}

fn prepare_recovery_candidate(
    request: NativeRecoveryRequest,
) -> Result<NativeRecoveryCandidate, String> {
    let NativeRecoveryRequest {
        instance,
        surface,
        width,
        height,
        target_fps,
        generation,
        previous_device_identity,
        event_proxy,
    } = request;
    let mut context = RenderContext {
        instance,
        devices: Vec::new(),
    };
    let polling_instance = context.instance.clone();
    let device_id = drive_wgpu_future(&polling_instance, context.device(Some(&surface)))
        .ok_or_else(|| String::from("no compatible fresh render device found"))?;
    let present_mode = {
        let device_handle = context
            .devices
            .get(device_id)
            .ok_or_else(|| String::from("fresh context did not retain its selected device"))?;
        let capabilities = surface.get_capabilities(device_handle.adapter());
        select_present_mode(target_fps, &capabilities.present_modes)
    };
    let render_surface = drive_wgpu_future(
        &polling_instance,
        context.create_render_surface(surface, width, height, present_mode),
    )
    .map_err(|error| error.to_string())?;
    let device_handle = context
        .devices
        .get(device_id)
        .ok_or_else(|| String::from("fresh context did not retain its selected device"))?;
    let candidate_device_identity = super::device::wgpu_device_id(&device_handle.device);
    if candidate_device_identity == previous_device_identity {
        return Err(String::from(
            "fresh recovery candidate reused the lost device handle",
        ));
    }
    let device_loss_registration =
        install_device_loss_callback(&device_handle.device, event_proxy.clone(), generation);
    let renderer = Renderer::new(&device_handle.device, startup_renderer_options())
        .map_err(|error| format!("fresh recovery renderer creation failed: {error}"))?;
    let primary = NativeWindowResourceBundle::new(
        generation,
        render_surface,
        renderer,
        &device_handle.device,
        &device_handle.queue,
        event_proxy,
    )
    .ok_or_else(|| String::from("fresh recovery primary bundle was not generation-bound"))?;
    let adapter = GenericNativeAdapterOwner::from_fresh_recovery_context(
        context,
        device_id,
        generation,
        device_loss_registration,
    )
    .map_err(String::from)?;
    Ok(NativeRecoveryCandidate { adapter, primary })
}

#[derive(Default)]
struct RecoveryAttemptTracker {
    in_flight: bool,
    candidate_starts: u64,
    candidate_completions: u64,
    max_in_flight: u8,
}

impl RecoveryAttemptTracker {
    fn admit(&mut self) -> bool {
        if self.in_flight {
            return false;
        }
        self.in_flight = true;
        self.candidate_starts = self.candidate_starts.saturating_add(1);
        self.max_in_flight = self.max_in_flight.max(1);
        true
    }

    fn complete(&mut self) {
        if self.in_flight {
            self.in_flight = false;
            self.candidate_completions = self.candidate_completions.saturating_add(1);
        }
    }
}

struct NativeRecoveryEpisode {
    token: NativeRecoveryEpisodeToken,
    result: Receiver<Result<NativeRecoveryCandidate, String>>,
}

#[derive(Default)]
pub(super) struct NativeRecoveryCoordinator {
    next_serial: u64,
    episode: Option<NativeRecoveryEpisode>,
    tracker: RecoveryAttemptTracker,
}

impl NativeRecoveryCoordinator {
    pub(super) fn start(
        &mut self,
        request: NativeRecoveryRequest,
    ) -> Result<NativeRecoveryEpisodeToken, String> {
        if self.episode.is_some() || !self.tracker.admit() {
            return Err(String::from(
                "native recovery candidate is already in flight",
            ));
        }
        let Some(token) = NativeRecoveryEpisodeToken::next(&mut self.next_serial) else {
            self.tracker.complete();
            return Err(String::from("native recovery episode token is exhausted"));
        };
        let (sender, result) = sync_channel(1);
        let event_proxy = request.event_proxy.clone();
        let worker = thread::Builder::new()
            .name(String::from("radiant-native-recovery"))
            .spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    prepare_recovery_candidate(request)
                }))
                .map_err(|_| String::from("fresh recovery candidate panicked"))
                .and_then(|result| result);
                let _ = sender.send(result);
                let _ = event_proxy
                    .send_event(RuntimeUserEvent::DeviceRecoveryReady { episode: token });
            })
            .map_err(|error| error.to_string());
        if let Err(error) = worker {
            self.tracker.complete();
            return Err(error);
        }
        self.episode = Some(NativeRecoveryEpisode { token, result });
        Ok(token)
    }

    pub(super) fn take_ready(
        &mut self,
        token: NativeRecoveryEpisodeToken,
    ) -> Option<Result<NativeRecoveryCandidate, String>> {
        let episode = self.episode.as_ref()?;
        if episode.token != token {
            return None;
        }
        let result = match episode.result.try_recv() {
            Ok(result) => result,
            Err(TryRecvError::Empty) => return None,
            Err(TryRecvError::Disconnected) => Err(String::from(
                "native recovery candidate result was disconnected",
            )),
        };
        self.episode.take();
        self.tracker.complete();
        Some(result)
    }

    #[cfg(test)]
    pub(super) fn has_in_flight_candidate(&self) -> bool {
        self.episode.is_some()
    }

    pub(super) fn cancel(&mut self) {
        self.episode.take();
        self.tracker.complete();
    }

    #[cfg(test)]
    fn tracker(&self) -> &RecoveryAttemptTracker {
        &self.tracker
    }
}

#[cfg(test)]
mod tests {
    use super::{NativeRecoveryCoordinator, NativeRecoveryEpisodeToken, RecoveryAttemptTracker};

    #[test]
    fn recovery_episode_token_is_private_and_monotonic() {
        let mut serial = 0;
        assert_eq!(
            NativeRecoveryEpisodeToken::next(&mut serial),
            Some(NativeRecoveryEpisodeToken::from_test_serial(1))
        );
        assert_eq!(
            NativeRecoveryEpisodeToken::next(&mut serial),
            Some(NativeRecoveryEpisodeToken::from_test_serial(2))
        );
    }

    #[test]
    fn candidate_tracker_allows_one_in_flight_candidate() {
        let mut tracker = RecoveryAttemptTracker::default();

        assert!(tracker.admit());
        assert!(!tracker.admit());
        assert_eq!(tracker.candidate_starts, 1);
        assert_eq!(tracker.max_in_flight, 1);
        tracker.complete();
        assert_eq!(tracker.candidate_completions, 1);
        assert!(tracker.admit());
        assert_eq!(tracker.candidate_starts, 2);
        assert_eq!(tracker.max_in_flight, 1);
    }

    #[test]
    fn coordinator_starts_without_an_event_loop_or_global_state() {
        let coordinator = NativeRecoveryCoordinator::default();

        assert!(!coordinator.has_in_flight_candidate());
        assert_eq!(coordinator.tracker().max_in_flight, 0);
    }
}
