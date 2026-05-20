use windows::Win32::System::Com::STGMEDIUM;
use windows::Win32::System::Memory::{GlobalLock, GlobalUnlock};
use windows::Win32::System::Ole::DROPEFFECT;

pub(super) fn drop_effect_from_medium(medium: &STGMEDIUM) -> windows::core::Result<DROPEFFECT> {
    let handle = unsafe { medium.u.hGlobal };
    let ptr = unsafe { GlobalLock(handle) } as *const u32;
    if ptr.is_null() {
        return Err(windows::core::Error::from_thread());
    }
    let effect = unsafe { *ptr };
    unsafe {
        let _ = GlobalUnlock(handle);
    }
    Ok(DROPEFFECT(effect))
}
