#[cfg(target_os = "windows")]
pub(super) fn configure_business_worker_thread(priority: crate::runtime::TaskPriority) {
    use windows_sys::Win32::System::Threading::{
        GetCurrentThread, SetThreadPriority, THREAD_PRIORITY_BELOW_NORMAL, THREAD_PRIORITY_LOWEST,
        THREAD_PRIORITY_NORMAL,
    };

    let native_priority = match priority {
        crate::runtime::TaskPriority::Interactive => THREAD_PRIORITY_NORMAL,
        crate::runtime::TaskPriority::Background => THREAD_PRIORITY_BELOW_NORMAL,
        crate::runtime::TaskPriority::Idle => THREAD_PRIORITY_LOWEST,
    };
    let ok = unsafe { SetThreadPriority(GetCurrentThread(), native_priority) };
    if ok == 0 {
        tracing::debug!("Radiant app runtime could not lower business worker thread priority");
    }
}

#[cfg(not(target_os = "windows"))]
pub(super) fn configure_business_worker_thread(_priority: crate::runtime::TaskPriority) {}
