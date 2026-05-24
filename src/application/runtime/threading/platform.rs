#[cfg(target_os = "windows")]
pub(super) fn configure_business_worker_thread() {
    use windows_sys::Win32::System::Threading::{
        GetCurrentThread, SetThreadPriority, THREAD_PRIORITY_BELOW_NORMAL,
    };

    let ok = unsafe { SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_BELOW_NORMAL) };
    if ok == 0 {
        tracing::debug!("Radiant app runtime could not lower business worker thread priority");
    }
}

#[cfg(not(target_os = "windows"))]
pub(super) fn configure_business_worker_thread() {}
