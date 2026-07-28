use super::RuntimeUserEvent;
use super::window_environment::AccessibilityDisplaySnapshot;
use winit::event_loop::EventLoopProxy;

#[cfg(target_os = "macos")]
mod platform {
    use super::*;
    use block2::RcBlock;
    use objc2::{rc::Retained, runtime::NSObject};
    use objc2_app_kit::{NSWorkspace, NSWorkspaceAccessibilityDisplayOptionsDidChangeNotification};
    use objc2_foundation::{NSNotification, NSNotificationCenter, NSOperationQueue};
    use std::ptr::NonNull;

    pub struct AccessibilityDisplayObserver {
        center: Retained<NSNotificationCenter>,
        token: Retained<NSObject>,
    }

    impl Drop for AccessibilityDisplayObserver {
        fn drop(&mut self) {
            // The observer token is retained by the notification center. Remove
            // it explicitly so teardown never leaves a callback targeting a
            // dropped event-loop proxy.
            unsafe { self.center.removeObserver(&self.token) };
        }
    }

    pub fn install(
        proxy: EventLoopProxy<RuntimeUserEvent>,
    ) -> Option<AccessibilityDisplayObserver> {
        let workspace = unsafe { NSWorkspace::sharedWorkspace() };
        let center = unsafe { workspace.notificationCenter() };
        let queue = unsafe { NSOperationQueue::mainQueue() };
        let callback = RcBlock::new(move |_notification: NonNull<NSNotification>| {
            let _ = proxy.send_event(RuntimeUserEvent::AccessibilityDisplayChanged);
        });
        let token = unsafe {
            center.addObserverForName_object_queue_usingBlock(
                Some(NSWorkspaceAccessibilityDisplayOptionsDidChangeNotification),
                None,
                Some(&queue),
                &callback,
            )
        };
        Some(AccessibilityDisplayObserver { center, token })
    }

    pub fn current_snapshot() -> AccessibilityDisplaySnapshot {
        let workspace = unsafe { NSWorkspace::sharedWorkspace() };
        AccessibilityDisplaySnapshot {
            increase_contrast: unsafe { workspace.accessibilityDisplayShouldIncreaseContrast() },
            reduce_motion: unsafe { workspace.accessibilityDisplayShouldReduceMotion() },
        }
    }
}

#[cfg(target_os = "macos")]
pub(super) use platform::{current_snapshot, install};

#[cfg(not(target_os = "macos"))]
pub(super) struct AccessibilityDisplayObserver;

#[cfg(not(target_os = "macos"))]
pub(super) fn install(
    _proxy: EventLoopProxy<RuntimeUserEvent>,
) -> Option<AccessibilityDisplayObserver> {
    None
}

#[cfg(not(target_os = "macos"))]
pub(super) const fn current_snapshot() -> AccessibilityDisplaySnapshot {
    AccessibilityDisplaySnapshot {
        increase_contrast: false,
        reduce_motion: false,
    }
}
