use super::{DeviceLossRegistration, RuntimeUserEvent};
use std::sync::Arc;
use vello::wgpu;
use winit::event_loop::EventLoopProxy;

pub(super) const DEVICE_LOSS_MESSAGE_FALLBACK: &str = "WGPU device lost without backend details";

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
    reason: wgpu::DeviceLostReason,
    message: String,
) -> Option<RuntimeUserEvent> {
    classify_device_lost(reason, message).map(|message| RuntimeUserEvent::DeviceLost {
        registration,
        message,
    })
}

fn send_device_lost_event(
    registration: Arc<DeviceLossRegistration>,
    reason: wgpu::DeviceLostReason,
    message: String,
    send_event: impl FnOnce(RuntimeUserEvent) -> bool,
) -> bool {
    let Some(event) = device_lost_event(registration, reason, message) else {
        return false;
    };
    send_event(event)
}

pub(super) fn install_device_loss_callback(
    device: &wgpu::Device,
    proxy: EventLoopProxy<RuntimeUserEvent>,
) -> Arc<DeviceLossRegistration> {
    let registration = Arc::new(DeviceLossRegistration::new());
    let callback_registration = Arc::clone(&registration);
    device.set_device_lost_callback(move |reason, message| {
        let _ = send_device_lost_event(
            Arc::clone(&callback_registration),
            reason,
            message,
            |event| proxy.send_event(event).is_ok(),
        );
    });
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
            wgpu::DeviceLostReason::Unknown,
            String::from("driver reset"),
            |_| false,
        );

        assert!(!sent);
    }
}
