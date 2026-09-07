use windows::Win32::System::Com::{DVASPECT_CONTENT, FORMATETC, TYMED_HGLOBAL};
use windows::Win32::System::Ole::CF_HDROP;

pub(super) fn data_object_format_matches(
    fmt: &FORMATETC,
    payload_format: &FORMATETC,
    preferred_drop_effect: u16,
    performed_drop_effect: u16,
) -> bool {
    is_payload_format(fmt, payload_format)
        || (is_drop_effect_format(fmt, preferred_drop_effect)
            || is_drop_effect_format(fmt, performed_drop_effect))
}

fn is_payload_format(fmt: &FORMATETC, payload_format: &FORMATETC) -> bool {
    fmt.cfFormat == payload_format.cfFormat
        && fmt.dwAspect == DVASPECT_CONTENT.0
        && (fmt.tymed & payload_format.tymed) != 0
        && (fmt.lindex == -1 || (payload_format.cfFormat == CF_HDROP.0 && fmt.lindex == 0))
}

fn is_drop_effect_format(fmt: &FORMATETC, drop_effect_format: u16) -> bool {
    fmt.cfFormat == drop_effect_format && uses_hglobal_storage(fmt)
}

fn uses_hglobal_storage(fmt: &FORMATETC) -> bool {
    (fmt.tymed & TYMED_HGLOBAL.0 as u32) != 0
}
