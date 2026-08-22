use super::{
    DeviceLossRegistration, NativeAdapterGeneration, NativeRenderDeviceErrorKind, RuntimeUserEvent,
};
use std::sync::Arc;
use vello::wgpu;
use winit::event_loop::EventLoopProxy;

pub(super) const DEVICE_LOSS_MESSAGE_FALLBACK: &str = "WGPU device lost without backend details";
pub(super) const RENDER_DEVICE_ERROR_MESSAGE_FALLBACK: &str =
    "WGPU render device error without backend details";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct DeviceFeatureSelection {
    baseline: wgpu::Features,
    initial_request: wgpu::Features,
}

impl DeviceFeatureSelection {
    pub(super) fn for_adapter(advertised: wgpu::Features) -> Self {
        let baseline =
            advertised & (wgpu::Features::CLEAR_TEXTURE | wgpu::Features::PIPELINE_CACHE);
        let initial_request = baseline | (advertised & wgpu::Features::TIMESTAMP_QUERY);
        Self {
            baseline,
            initial_request,
        }
    }

    pub(super) const fn initial_request(self) -> wgpu::Features {
        self.initial_request
    }

    /// Return the one permitted fallback after a timestamp-enabled request
    /// fails. A baseline request never retries, so device creation cannot
    /// become an unbounded feature negotiation loop.
    pub(super) fn retry_after_failure(self) -> Option<wgpu::Features> {
        if self.initial_request != self.baseline {
            Some(self.baseline)
        } else {
            None
        }
    }
}

pub(super) fn classify_device_lost(
    reason: wgpu::DeviceLostReason,
    message: String,
) -> Option<String> {
    match reason {
        wgpu::DeviceLostReason::Unknown => Some(if message.is_empty() {
            DEVICE_LOSS_MESSAGE_FALLBACK.to_owned()
        } else {
            message
        }),
        wgpu::DeviceLostReason::Destroyed => None,
    }
}

fn device_lost_event(
    registration: Arc<DeviceLossRegistration>,
    generation: NativeAdapterGeneration,
    reason: wgpu::DeviceLostReason,
    message: String,
) -> Option<RuntimeUserEvent> {
    classify_device_lost(reason, message).map(|message| RuntimeUserEvent::DeviceLost {
        registration,
        generation,
        message,
    })
}

fn send_device_lost_event(
    registration: Arc<DeviceLossRegistration>,
    generation: NativeAdapterGeneration,
    reason: wgpu::DeviceLostReason,
    message: String,
    send_event: impl FnOnce(RuntimeUserEvent) -> bool,
) -> bool {
    let Some(event) = device_lost_event(registration, generation, reason, message) else {
        return false;
    };
    send_event(event)
}

pub(super) fn classify_uncaptured_error(
    error: wgpu::Error,
) -> (NativeRenderDeviceErrorKind, String) {
    match error {
        wgpu::Error::OutOfMemory { .. } => (
            NativeRenderDeviceErrorKind::OutOfMemory,
            String::from(RENDER_DEVICE_ERROR_MESSAGE_FALLBACK),
        ),
        wgpu::Error::Validation { description, .. } => (
            NativeRenderDeviceErrorKind::Validation,
            owned_error_message(description),
        ),
        wgpu::Error::Internal { description, .. } => (
            NativeRenderDeviceErrorKind::Internal,
            owned_error_message(description),
        ),
    }
}

fn owned_error_message(description: String) -> String {
    if description.is_empty() {
        String::from(RENDER_DEVICE_ERROR_MESSAGE_FALLBACK)
    } else {
        description
    }
}

fn render_device_error_event(
    registration: Arc<DeviceLossRegistration>,
    generation: NativeAdapterGeneration,
    error: wgpu::Error,
) -> RuntimeUserEvent {
    let (kind, message) = classify_uncaptured_error(error);
    registration.observe_uncaptured_error(kind);
    RuntimeUserEvent::RenderDeviceError {
        registration,
        generation,
        kind,
        message,
    }
}

fn send_render_device_error_event(
    registration: Arc<DeviceLossRegistration>,
    generation: NativeAdapterGeneration,
    error: wgpu::Error,
    send_event: impl FnOnce(RuntimeUserEvent) -> bool,
) -> bool {
    send_event(render_device_error_event(registration, generation, error))
}

pub(super) fn install_device_loss_callback(
    device: &wgpu::Device,
    proxy: EventLoopProxy<RuntimeUserEvent>,
    generation: NativeAdapterGeneration,
) -> Arc<DeviceLossRegistration> {
    let registration = Arc::new(DeviceLossRegistration::new());
    let callback_registration = Arc::clone(&registration);
    let device_loss_proxy = proxy.clone();
    device.set_device_lost_callback(move |reason, message| {
        let _ = send_device_lost_event(
            Arc::clone(&callback_registration),
            generation,
            reason,
            message,
            |event| device_loss_proxy.send_event(event).is_ok(),
        );
    });
    let callback_registration = Arc::clone(&registration);
    device.on_uncaptured_error(Arc::new(move |error| {
        let _ = send_render_device_error_event(
            Arc::clone(&callback_registration),
            generation,
            error,
            |event| proxy.send_event(event).is_ok(),
        );
    }));
    registration
}

pub(super) fn wgpu_device_id(device: &wgpu::Device) -> usize {
    device as *const wgpu::Device as usize
}

pub(super) fn wgpu_target_matches(
    cached_device: usize,
    cached_format: wgpu::TextureFormat,
    target_device: &wgpu::Device,
    target_format: wgpu::TextureFormat,
) -> bool {
    target_key_matches(
        cached_device,
        cached_format,
        wgpu_device_id(target_device),
        target_format,
    )
}

fn target_key_matches(
    cached_device: usize,
    cached_format: wgpu::TextureFormat,
    target_device: usize,
    target_format: wgpu::TextureFormat,
) -> bool {
    cached_device == target_device && cached_format == target_format
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsupported_timestamps_use_vello_baseline_without_retry() {
        let advertised = wgpu::Features::CLEAR_TEXTURE | wgpu::Features::PIPELINE_CACHE;
        let selection = DeviceFeatureSelection::for_adapter(advertised);

        assert_eq!(selection.initial_request(), advertised);
        assert_eq!(selection.retry_after_failure(), None);
    }

    #[test]
    fn timestamp_request_has_exactly_one_baseline_retry() {
        let baseline = wgpu::Features::CLEAR_TEXTURE | wgpu::Features::PIPELINE_CACHE;
        let advertised = baseline
            | wgpu::Features::TIMESTAMP_QUERY
            | wgpu::Features::TIMESTAMP_QUERY_INSIDE_PASSES;
        let selection = DeviceFeatureSelection::for_adapter(advertised);

        assert_eq!(
            selection.initial_request(),
            baseline | wgpu::Features::TIMESTAMP_QUERY
        );
        assert!(
            !selection
                .initial_request()
                .contains(wgpu::Features::TIMESTAMP_QUERY_INSIDE_PASSES)
        );
        assert_eq!(selection.retry_after_failure(), Some(baseline));
    }

    #[test]
    fn feature_selection_never_requests_unrelated_advertised_features() {
        let advertised = wgpu::Features::TEXTURE_COMPRESSION_BC
            | wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS;
        let selection = DeviceFeatureSelection::for_adapter(advertised);

        assert_eq!(selection.initial_request(), wgpu::Features::empty());
        assert_eq!(selection.retry_after_failure(), None);
    }

    #[test]
    fn target_key_tracks_device_and_format() {
        let format = wgpu::TextureFormat::Bgra8UnormSrgb;
        let target_device = 7;
        assert!(target_key_matches(7, format, target_device, format));
        assert!(!target_key_matches(7, format, 8, format));
        assert!(!target_key_matches(
            7,
            format,
            target_device,
            wgpu::TextureFormat::Rgba8UnormSrgb
        ));
    }

    #[test]
    fn unknown_device_loss_owns_non_empty_backend_text() {
        let message = String::from("driver reset");
        let classified = classify_device_lost(wgpu::DeviceLostReason::Unknown, message);

        assert_eq!(classified, Some(String::from("driver reset")));
    }

    #[test]
    fn destroyed_device_loss_is_vetoed() {
        assert_eq!(
            classify_device_lost(wgpu::DeviceLostReason::Destroyed, String::from("disposed")),
            None
        );
    }

    #[test]
    fn empty_unknown_device_loss_uses_stable_fallback() {
        assert_eq!(
            classify_device_lost(wgpu::DeviceLostReason::Unknown, String::new()),
            Some(String::from(DEVICE_LOSS_MESSAGE_FALLBACK))
        );
        assert!(!DEVICE_LOSS_MESSAGE_FALLBACK.is_empty());
    }

    #[test]
    fn device_loss_event_preserves_owned_text_after_source_is_dropped() {
        let event = device_lost_event(
            Arc::new(DeviceLossRegistration::new()),
            NativeAdapterGeneration::from_test_serial(1),
            wgpu::DeviceLostReason::Unknown,
            String::from("backend-owned diagnostic"),
        )
        .expect("unknown device loss should produce an event");

        match event {
            RuntimeUserEvent::DeviceLost { message, .. } => {
                assert_eq!(message, "backend-owned diagnostic")
            }
            _ => panic!("unknown device loss should use the device-loss event"),
        }
    }

    #[test]
    fn device_loss_proxy_send_failure_is_harmless() {
        let sent = send_device_lost_event(
            Arc::new(DeviceLossRegistration::new()),
            NativeAdapterGeneration::from_test_serial(1),
            wgpu::DeviceLostReason::Unknown,
            String::from("driver reset"),
            |_| false,
        );

        assert!(!sent);
    }

    fn backend_error_source() -> Box<dyn std::error::Error + Send + Sync> {
        Box::new(std::io::Error::other("backend source"))
    }

    fn validation_error(description: &str) -> wgpu::Error {
        wgpu::Error::Validation {
            source: backend_error_source(),
            description: description.to_owned(),
        }
    }

    fn internal_error(description: &str) -> wgpu::Error {
        wgpu::Error::Internal {
            source: backend_error_source(),
            description: description.to_owned(),
        }
    }

    #[test]
    fn uncaptured_errors_convert_all_backend_categories_to_owned_evidence() {
        assert_eq!(
            classify_uncaptured_error(wgpu::Error::OutOfMemory {
                source: backend_error_source(),
            }),
            (
                NativeRenderDeviceErrorKind::OutOfMemory,
                String::from(RENDER_DEVICE_ERROR_MESSAGE_FALLBACK)
            )
        );
        assert_eq!(
            classify_uncaptured_error(validation_error("shader rejected")),
            (
                NativeRenderDeviceErrorKind::Validation,
                String::from("shader rejected")
            )
        );
        assert_eq!(
            classify_uncaptured_error(internal_error("driver fault")),
            (
                NativeRenderDeviceErrorKind::Internal,
                String::from("driver fault")
            )
        );
    }

    #[test]
    fn empty_uncaptured_error_descriptions_use_stable_non_empty_fallback() {
        for error in [validation_error(""), internal_error("")] {
            let (_, message) = classify_uncaptured_error(error);
            assert_eq!(message, RENDER_DEVICE_ERROR_MESSAGE_FALLBACK);
            assert!(!message.is_empty());
        }
    }

    #[test]
    fn uncaptured_error_event_keeps_only_owned_backend_neutral_evidence() {
        let registration = Arc::new(DeviceLossRegistration::new());
        let generation = NativeAdapterGeneration::from_test_serial(1);
        let event = render_device_error_event(
            Arc::clone(&registration),
            generation,
            validation_error("bad"),
        );

        match event {
            RuntimeUserEvent::RenderDeviceError {
                registration: event_registration,
                generation: event_generation,
                kind,
                message,
            } => {
                assert!(Arc::ptr_eq(&event_registration, &registration));
                assert_eq!(event_generation, generation);
                assert_eq!(kind, NativeRenderDeviceErrorKind::Validation);
                assert_eq!(message, "bad");
            }
            _ => panic!("uncaptured WGPU errors should use the render-device event"),
        }
    }

    #[test]
    fn uncaptured_error_proxy_send_failure_is_harmless() {
        let sent = send_render_device_error_event(
            Arc::new(DeviceLossRegistration::new()),
            NativeAdapterGeneration::from_test_serial(1),
            internal_error("driver fault"),
            |_| false,
        );

        assert!(!sent);
    }

    #[test]
    fn surface_acquire_correlation_is_active_only_for_the_current_call() {
        let registration = Arc::new(DeviceLossRegistration::new());

        registration.observe_uncaptured_error(NativeRenderDeviceErrorKind::OutOfMemory);
        registration.begin_surface_acquire();
        assert_eq!(registration.finish_surface_acquire(), None);

        registration.begin_surface_acquire();
        let _ = render_device_error_event(
            Arc::clone(&registration),
            NativeAdapterGeneration::from_test_serial(1),
            wgpu::Error::OutOfMemory {
                source: backend_error_source(),
            },
        );
        assert_eq!(
            registration.finish_surface_acquire(),
            Some(NativeRenderDeviceErrorKind::OutOfMemory)
        );

        registration.begin_surface_acquire();
        let _ = render_device_error_event(
            Arc::clone(&registration),
            NativeAdapterGeneration::from_test_serial(1),
            validation_error("surface validation"),
        );
        assert_eq!(
            registration.finish_surface_acquire(),
            Some(NativeRenderDeviceErrorKind::Validation)
        );
    }
}
