//! Native external drag launching for the generic Vello runtime.

use super::*;
use crate::runtime::{
    ExternalDragOutcome, ExternalDragPayload, ExternalDragRequest, RuntimeBridge,
};

impl<Bridge, Message> GenericNativeVelloRunner<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(super) fn launch_external_drag_if_armed(&mut self) -> GenericRouteOutcome {
        let Some(session) = self.core.runtime.take_external_drag_session() else {
            return GenericRouteOutcome::default();
        };
        let path_count = match &session.request.payload {
            ExternalDragPayload::Files(paths) => paths.len(),
        };
        info!(
            path_count,
            preview = %session.request.preview.label,
            "radiant generic native vello: launching external drag"
        );
        let result = platform_start_external_drag(&session.request);
        let outcome = self
            .core
            .runtime
            .dispatch_external_drag_result(session, result);
        self.core.route_command_outcome(outcome)
    }
}

#[cfg(target_os = "windows")]
fn platform_start_external_drag(
    request: &ExternalDragRequest,
) -> Result<ExternalDragOutcome, String> {
    windows_external_drag::start_external_drag(request)
}

#[cfg(not(target_os = "windows"))]
fn platform_start_external_drag(
    _request: &ExternalDragRequest,
) -> Result<ExternalDragOutcome, String> {
    Err(String::from(
        "External drag-out is only supported on Windows in this backend",
    ))
}

#[cfg(target_os = "windows")]
mod windows_external_drag {
    use crate::runtime::{
        ExternalDragEffect, ExternalDragOutcome, ExternalDragPayload, ExternalDragRequest,
    };
    use std::cell::Cell;
    use std::mem::ManuallyDrop;
    use std::os::windows::ffi::OsStrExt;
    use std::path::{Path, PathBuf};
    use std::sync::OnceLock;
    use tracing::warn;
    use windows::Win32::Foundation::{
        COLORREF, DRAGDROP_S_CANCEL, DRAGDROP_S_DROP, DRAGDROP_S_USEDEFAULTCURSORS, DV_E_FORMATETC,
        E_INVALIDARG, HGLOBAL, POINT, RECT, SIZE,
    };
    use windows::Win32::Graphics::Gdi::{
        CreateCompatibleBitmap, CreateCompatibleDC, CreateSolidBrush, DT_END_ELLIPSIS, DT_LEFT,
        DT_SINGLELINE, DT_VCENTER, DeleteDC, DeleteObject, DrawTextW, FillRect, GetDC, HBITMAP,
        HGDIOBJ, ReleaseDC, SelectObject, SetBkMode, SetTextColor, TRANSPARENT,
    };
    use windows::Win32::System::Com::{
        CLSCTX_INPROC_SERVER, CoCreateInstance, DATADIR_GET, DVASPECT_CONTENT, FORMATETC,
        IAdviseSink, IDataObject, IEnumFORMATETC, STGMEDIUM, STGMEDIUM_0, TYMED_HGLOBAL,
    };
    use windows::Win32::System::DataExchange::RegisterClipboardFormatW;
    use windows::Win32::System::Memory::{
        GMEM_MOVEABLE, GMEM_ZEROINIT, GlobalAlloc, GlobalLock, GlobalUnlock,
    };
    use windows::Win32::System::Ole::{
        CF_HDROP, DROPEFFECT, DROPEFFECT_COPY, DROPEFFECT_LINK, DROPEFFECT_MOVE, DROPEFFECT_NONE,
        DoDragDrop, IDropSource, OleInitialize, OleUninitialize,
    };
    use windows::Win32::System::SystemServices::{MK_LBUTTON, MODIFIERKEYS_FLAGS};
    use windows::Win32::UI::Shell::{
        CLSID_DragDropHelper, DROPFILES, IDragSourceHelper, SHCreateStdEnumFmtEtc, SHDRAGIMAGE,
    };
    use windows::core::{BOOL, HRESULT, Ref, implement, w};

    const PREVIEW_MIN_WIDTH: i32 = 132;
    const PREVIEW_MAX_WIDTH: i32 = 280;
    const PREVIEW_HEIGHT: i32 = 30;
    const PREVIEW_COLOR_KEY: COLORREF = COLORREF(0x00ff00ff);

    struct ComApartment;

    impl ComApartment {
        fn new() -> Result<Self, String> {
            unsafe { OleInitialize(None) }.map_err(|err| format!("COM init failed: {err}"))?;
            Ok(Self)
        }
    }

    impl Drop for ComApartment {
        fn drop(&mut self) {
            unsafe { OleUninitialize() };
        }
    }

    #[implement(IDataObject)]
    #[derive(Clone)]
    struct FileDropDataObject {
        paths: Vec<PathBuf>,
        format: FORMATETC,
        preferred_drop_effect: u16,
        performed_drop_effect: u16,
        performed_effect: Cell<DROPEFFECT>,
    }

    impl FileDropDataObject {
        fn new(paths: Vec<PathBuf>) -> Result<Self, String> {
            if paths.is_empty() {
                return Err(String::from("No files to drag"));
            }
            let (preferred_drop_effect, performed_drop_effect) = drop_effect_formats()?;
            Ok(Self {
                paths,
                format: build_file_format(),
                preferred_drop_effect,
                performed_drop_effect,
                performed_effect: Cell::new(DROPEFFECT_NONE),
            })
        }

        fn matches_format(&self, fmt: &FORMATETC) -> bool {
            (fmt.cfFormat == CF_HDROP.0
                && fmt.dwAspect == DVASPECT_CONTENT.0
                && (fmt.tymed & TYMED_HGLOBAL.0 as u32) != 0
                && (fmt.lindex == -1 || fmt.lindex == 0))
                || ((fmt.cfFormat == self.preferred_drop_effect
                    || fmt.cfFormat == self.performed_drop_effect)
                    && (fmt.tymed & TYMED_HGLOBAL.0 as u32) != 0)
        }

        fn fill_medium(&self, fmt: &FORMATETC) -> windows::core::Result<STGMEDIUM> {
            if fmt.cfFormat == self.preferred_drop_effect {
                return drop_effect_medium(DROPEFFECT_COPY);
            }
            if fmt.cfFormat == self.performed_drop_effect {
                return drop_effect_medium(self.performed_effect.get());
            }
            let hglobal = create_hglobal_for_paths(&self.paths)
                .map_err(|_| windows::core::Error::from_thread())?;
            Ok(STGMEDIUM {
                tymed: TYMED_HGLOBAL.0 as u32,
                u: STGMEDIUM_0 { hGlobal: hglobal },
                pUnkForRelease: ManuallyDrop::new(None),
            })
        }
    }

    #[allow(non_snake_case)]
    impl windows::Win32::System::Com::IDataObject_Impl for FileDropDataObject_Impl {
        fn GetData(&self, formatetcin: *const FORMATETC) -> windows::core::Result<STGMEDIUM> {
            if formatetcin.is_null() {
                return Err(windows::core::Error::from(E_INVALIDARG));
            }
            let fmt = unsafe { &*formatetcin };
            if !self.matches_format(fmt) {
                return Err(windows::core::Error::from(DV_E_FORMATETC));
            }
            self.fill_medium(fmt)
        }

        fn GetDataHere(
            &self,
            _pformatetc: *const FORMATETC,
            _pmedium: *mut STGMEDIUM,
        ) -> windows::core::Result<()> {
            Err(windows::core::Error::from(DV_E_FORMATETC))
        }

        fn QueryGetData(&self, pformatetc: *const FORMATETC) -> HRESULT {
            if pformatetc.is_null() {
                return E_INVALIDARG;
            }
            let fmt = unsafe { &*pformatetc };
            if self.matches_format(fmt) {
                HRESULT(0)
            } else {
                DV_E_FORMATETC
            }
        }

        fn GetCanonicalFormatEtc(
            &self,
            pformatectin: *const FORMATETC,
            pformatetcout: *mut FORMATETC,
        ) -> HRESULT {
            if pformatectin.is_null() || pformatetcout.is_null() {
                return E_INVALIDARG;
            }
            unsafe {
                *pformatetcout = *pformatectin;
            }
            HRESULT(0)
        }

        fn SetData(
            &self,
            pformatetc: *const FORMATETC,
            pmedium: *const STGMEDIUM,
            _frelease: BOOL,
        ) -> windows::core::Result<()> {
            if pformatetc.is_null() || pmedium.is_null() {
                return Err(windows::core::Error::from(E_INVALIDARG));
            }
            let fmt = unsafe { &*pformatetc };
            if fmt.cfFormat != self.performed_drop_effect
                || (fmt.tymed & TYMED_HGLOBAL.0 as u32) == 0
            {
                return Err(windows::core::Error::from(
                    windows::Win32::Foundation::E_NOTIMPL,
                ));
            }
            let medium = unsafe { &*pmedium };
            if medium.tymed != TYMED_HGLOBAL.0 as u32 {
                return Err(windows::core::Error::from(E_INVALIDARG));
            }
            let handle = unsafe { medium.u.hGlobal };
            let ptr = unsafe { GlobalLock(handle) } as *const u32;
            if ptr.is_null() {
                unsafe {
                    let _ = GlobalUnlock(handle);
                }
                return Err(windows::core::Error::from_thread());
            }
            let effect = unsafe { *ptr };
            self.performed_effect.set(DROPEFFECT(effect));
            unsafe {
                let _ = GlobalUnlock(handle);
            }
            Ok(())
        }

        fn EnumFormatEtc(&self, dwdirection: u32) -> windows::core::Result<IEnumFORMATETC> {
            if dwdirection != DATADIR_GET.0 as u32 {
                return Err(windows::core::Error::from(
                    windows::Win32::Foundation::E_NOTIMPL,
                ));
            }
            let formats = [
                self.format,
                build_drop_effect_format(self.preferred_drop_effect),
                build_drop_effect_format(self.performed_drop_effect),
            ];
            unsafe { SHCreateStdEnumFmtEtc(&formats) }
        }

        fn DAdvise(
            &self,
            _pformatetc: *const FORMATETC,
            _advf: u32,
            _padvsink: Ref<'_, IAdviseSink>,
        ) -> windows::core::Result<u32> {
            Err(windows::core::Error::from(
                windows::Win32::Foundation::E_NOTIMPL,
            ))
        }

        fn DUnadvise(&self, _dwconnection: u32) -> windows::core::Result<()> {
            Err(windows::core::Error::from(
                windows::Win32::Foundation::E_NOTIMPL,
            ))
        }

        fn EnumDAdvise(&self) -> windows::core::Result<windows::Win32::System::Com::IEnumSTATDATA> {
            Err(windows::core::Error::from(
                windows::Win32::Foundation::E_NOTIMPL,
            ))
        }
    }

    #[implement(IDropSource)]
    #[derive(Clone)]
    struct SimpleDropSource;

    #[allow(non_snake_case)]
    impl windows::Win32::System::Ole::IDropSource_Impl for SimpleDropSource_Impl {
        fn QueryContinueDrag(
            &self,
            escape_pressed: BOOL,
            key_state: MODIFIERKEYS_FLAGS,
        ) -> HRESULT {
            if escape_pressed.as_bool() {
                return DRAGDROP_S_CANCEL;
            }
            if key_state.0 & MK_LBUTTON.0 == 0 {
                return DRAGDROP_S_DROP;
            }
            HRESULT(0)
        }

        fn GiveFeedback(&self, _dweffect: DROPEFFECT) -> HRESULT {
            DRAGDROP_S_USEDEFAULTCURSORS
        }
    }

    pub(super) fn start_external_drag(
        request: &ExternalDragRequest,
    ) -> Result<ExternalDragOutcome, String> {
        let ExternalDragPayload::Files(paths) = &request.payload;
        if paths.is_empty() {
            return Err(String::from("No files to drag"));
        }
        let _com = ComApartment::new()?;
        let absolute = paths
            .iter()
            .map(|path| normalize_path(path.as_path()))
            .collect::<Vec<_>>();
        let data_object_impl = FileDropDataObject::new(absolute)?;
        let data_object: IDataObject = data_object_impl.into();
        let drop_source: IDropSource = SimpleDropSource.into();
        let drag_image = match DragImage::new(&request.preview.label) {
            Ok(image) => {
                if let Err(err) = image.initialize(&data_object) {
                    warn!(
                        error = %err,
                        "radiant generic native vello: external drag preview image failed"
                    );
                }
                Some(image)
            }
            Err(err) => {
                warn!(
                    error = %err,
                    "radiant generic native vello: external drag preview image could not be built"
                );
                None
            }
        };
        let mut effect = DROPEFFECT_NONE;
        let drag_result = unsafe {
            DoDragDrop(
                &data_object,
                &drop_source,
                DROPEFFECT_COPY | DROPEFFECT_LINK | DROPEFFECT_MOVE,
                &mut effect,
            )
        }
        .ok();
        drop(drag_image);
        drag_result.map_err(|err| format!("Drag failed: {err}"))?;
        Ok(ExternalDragOutcome {
            effect: external_drag_effect(effect),
        })
    }

    struct DragImage {
        bitmap: HBITMAP,
        image: SHDRAGIMAGE,
    }

    impl DragImage {
        fn new(label: &str) -> Result<Self, String> {
            let label = preview_label(label);
            let width = preview_width(&label);
            let height = PREVIEW_HEIGHT;
            let screen_dc = unsafe { GetDC(None) };
            if screen_dc.0.is_null() {
                return Err(String::from("GetDC failed"));
            }
            let memory_dc = unsafe { CreateCompatibleDC(Some(screen_dc)) };
            if memory_dc.0.is_null() {
                unsafe {
                    ReleaseDC(None, screen_dc);
                }
                return Err(String::from("CreateCompatibleDC failed"));
            }
            let bitmap = unsafe { CreateCompatibleBitmap(screen_dc, width, height) };
            if bitmap.0.is_null() {
                unsafe {
                    let _ = DeleteDC(memory_dc);
                    ReleaseDC(None, screen_dc);
                }
                return Err(String::from("CreateCompatibleBitmap failed"));
            }
            let old = unsafe { SelectObject(memory_dc, HGDIOBJ(bitmap.0)) };
            paint_drag_preview(memory_dc, width, height, &label);
            unsafe {
                if !old.0.is_null() {
                    SelectObject(memory_dc, old);
                }
                let _ = DeleteDC(memory_dc);
                ReleaseDC(None, screen_dc);
            }
            Ok(Self {
                bitmap,
                image: SHDRAGIMAGE {
                    sizeDragImage: SIZE {
                        cx: width,
                        cy: height,
                    },
                    ptOffset: POINT { x: 18, y: 16 },
                    hbmpDragImage: bitmap,
                    crColorKey: PREVIEW_COLOR_KEY,
                },
            })
        }

        fn initialize(&self, data_object: &IDataObject) -> windows::core::Result<()> {
            let helper: IDragSourceHelper =
                unsafe { CoCreateInstance(&CLSID_DragDropHelper, None, CLSCTX_INPROC_SERVER) }?;
            unsafe { helper.InitializeFromBitmap(&self.image, data_object) }
        }
    }

    impl Drop for DragImage {
        fn drop(&mut self) {
            unsafe {
                let _ = DeleteObject(HGDIOBJ(self.bitmap.0));
            }
        }
    }

    fn paint_drag_preview(
        dc: windows::Win32::Graphics::Gdi::HDC,
        width: i32,
        height: i32,
        label: &str,
    ) {
        fill_rect(dc, 0, 0, width, height, PREVIEW_COLOR_KEY);
        fill_rect(dc, 1, 1, width - 1, height - 1, rgb(34, 34, 34));
        fill_rect(dc, 1, 1, 8, height - 1, rgb(255, 92, 70));
        fill_rect(dc, 8, 1, width - 1, 2, rgb(87, 87, 87));
        fill_rect(dc, 8, height - 2, width - 1, height - 1, rgb(87, 87, 87));
        fill_rect(dc, width - 2, 1, width - 1, height - 1, rgb(87, 87, 87));
        unsafe {
            SetBkMode(dc, TRANSPARENT);
            SetTextColor(dc, rgb(238, 238, 238));
        }
        let mut wide = label.encode_utf16().collect::<Vec<_>>();
        let mut rect = RECT {
            left: 17,
            top: 0,
            right: width - 10,
            bottom: height,
        };
        unsafe {
            DrawTextW(
                dc,
                &mut wide,
                &mut rect,
                DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS,
            );
        }
    }

    fn fill_rect(
        dc: windows::Win32::Graphics::Gdi::HDC,
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
        color: COLORREF,
    ) {
        let rect = RECT {
            left,
            top,
            right,
            bottom,
        };
        let brush = unsafe { CreateSolidBrush(color) };
        if brush.0.is_null() {
            return;
        }
        unsafe {
            FillRect(dc, &rect, brush);
            let _ = DeleteObject(HGDIOBJ(brush.0));
        }
    }

    fn preview_label(label: &str) -> String {
        let label = label.trim();
        if label.is_empty() {
            String::from("Dragging")
        } else {
            label.chars().take(80).collect()
        }
    }

    fn preview_width(label: &str) -> i32 {
        (label.chars().count() as i32 * 7 + 72).clamp(PREVIEW_MIN_WIDTH, PREVIEW_MAX_WIDTH)
    }

    fn rgb(red: u8, green: u8, blue: u8) -> COLORREF {
        COLORREF((red as u32) | ((green as u32) << 8) | ((blue as u32) << 16))
    }

    fn build_file_format() -> FORMATETC {
        FORMATETC {
            cfFormat: CF_HDROP.0,
            ptd: std::ptr::null_mut(),
            dwAspect: DVASPECT_CONTENT.0,
            lindex: -1,
            tymed: TYMED_HGLOBAL.0 as u32,
        }
    }

    fn build_drop_effect_format(format: u16) -> FORMATETC {
        FORMATETC {
            cfFormat: format,
            ptd: std::ptr::null_mut(),
            dwAspect: DVASPECT_CONTENT.0,
            lindex: -1,
            tymed: TYMED_HGLOBAL.0 as u32,
        }
    }

    fn drop_effect_formats() -> Result<(u16, u16), String> {
        static FORMATS: OnceLock<Result<(u16, u16), String>> = OnceLock::new();
        FORMATS
            .get_or_init(|| {
                let preferred = unsafe { RegisterClipboardFormatW(w!("Preferred DropEffect")) };
                let performed = unsafe { RegisterClipboardFormatW(w!("Performed DropEffect")) };
                if preferred == 0 || performed == 0 {
                    Err(String::from("RegisterClipboardFormatW failed"))
                } else {
                    Ok((preferred as u16, performed as u16))
                }
            })
            .clone()
    }

    fn drop_effect_medium(effect: DROPEFFECT) -> windows::core::Result<STGMEDIUM> {
        let handle =
            unsafe { GlobalAlloc(GMEM_MOVEABLE | GMEM_ZEROINIT, std::mem::size_of::<u32>()) }
                .map_err(|_| windows::core::Error::from_thread())?;
        let ptr = unsafe { GlobalLock(handle) } as *mut u32;
        if ptr.is_null() {
            unsafe {
                let _ = GlobalUnlock(handle);
            }
            return Err(windows::core::Error::from_thread());
        }
        unsafe {
            *ptr = effect.0;
            let _ = GlobalUnlock(handle);
        }
        Ok(STGMEDIUM {
            tymed: TYMED_HGLOBAL.0 as u32,
            u: STGMEDIUM_0 { hGlobal: handle },
            pUnkForRelease: ManuallyDrop::new(None),
        })
    }

    fn create_hglobal_for_paths(paths: &[PathBuf]) -> Result<HGLOBAL, std::io::Error> {
        let payload = build_dropfiles_payload(paths);
        let handle = unsafe { GlobalAlloc(GMEM_MOVEABLE | GMEM_ZEROINIT, payload.len()) }
            .map_err(last_error_from_win32)?;
        let ptr = unsafe { GlobalLock(handle) };
        if ptr.is_null() {
            unsafe {
                let _ = GlobalUnlock(handle);
            }
            return Err(std::io::Error::last_os_error());
        }
        unsafe {
            std::ptr::copy_nonoverlapping(payload.as_ptr(), ptr.cast::<u8>(), payload.len());
            let _ = GlobalUnlock(handle);
        }
        Ok(handle)
    }

    fn build_dropfiles_payload(paths: &[PathBuf]) -> Vec<u8> {
        let path_bytes = encode_drag_paths(paths);
        let mut payload = Vec::with_capacity(std::mem::size_of::<DROPFILES>() + path_bytes.len());
        payload.extend_from_slice(&dropfiles_header_bytes());
        payload.extend_from_slice(&path_bytes);
        payload
    }

    fn encode_drag_paths(paths: &[PathBuf]) -> Vec<u8> {
        let mut utf16_paths = Vec::new();
        for path in paths {
            utf16_paths.extend(
                path.as_os_str()
                    .encode_wide()
                    .chain(std::iter::once(0))
                    .flat_map(u16::to_le_bytes),
            );
        }
        utf16_paths.extend_from_slice(&0u16.to_le_bytes());
        utf16_paths
    }

    fn dropfiles_header_bytes() -> [u8; std::mem::size_of::<DROPFILES>()] {
        let header = DROPFILES {
            pFiles: std::mem::size_of::<DROPFILES>() as u32,
            pt: POINT { x: 0, y: 0 },
            fNC: false.into(),
            fWide: true.into(),
        };
        let mut bytes = [0u8; std::mem::size_of::<DROPFILES>()];
        unsafe {
            std::ptr::copy_nonoverlapping(
                (&header as *const DROPFILES).cast::<u8>(),
                bytes.as_mut_ptr(),
                bytes.len(),
            );
        }
        bytes
    }

    fn external_drag_effect(effect: DROPEFFECT) -> ExternalDragEffect {
        if effect.0 & DROPEFFECT_MOVE.0 != 0 {
            ExternalDragEffect::Move
        } else if effect.0 & DROPEFFECT_COPY.0 != 0 {
            ExternalDragEffect::Copy
        } else if effect.0 & DROPEFFECT_LINK.0 != 0 {
            ExternalDragEffect::Link
        } else {
            ExternalDragEffect::None
        }
    }

    fn normalize_path(path: &Path) -> PathBuf {
        let absolute = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let text = absolute.as_os_str().to_string_lossy();
        let verbatim_prefix = "\\\\?\\";
        if text.starts_with(verbatim_prefix) {
            PathBuf::from(text.trim_start_matches(verbatim_prefix))
        } else {
            absolute
        }
    }

    fn last_error_from_win32(err: windows::core::Error) -> std::io::Error {
        std::io::Error::from_raw_os_error(err.code().0)
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::os::windows::ffi::OsStringExt;

        fn decode_drag_paths(bytes: &[u8]) -> Vec<String> {
            let utf16 = bytes
                .chunks_exact(std::mem::size_of::<u16>())
                .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
                .collect::<Vec<_>>();
            let mut paths = Vec::new();
            let mut start = 0usize;
            while start < utf16.len() {
                let Some(end) = utf16[start..].iter().position(|value| *value == 0) else {
                    panic!("drag path payload must be null terminated");
                };
                if end == 0 {
                    break;
                }
                paths.push(
                    std::ffi::OsString::from_wide(&utf16[start..start + end])
                        .to_string_lossy()
                        .into_owned(),
                );
                start += end + 1;
            }
            paths
        }

        #[test]
        fn normalize_path_strips_windows_verbatim_prefix() {
            let normalized = normalize_path(Path::new(r"\\?\C:\samples\kick.wav"));
            assert_eq!(normalized, PathBuf::from(r"C:\samples\kick.wav"));
        }

        #[test]
        fn encode_drag_paths_double_null_terminates_multi_path_payload() {
            let encoded = encode_drag_paths(&[
                PathBuf::from(r"C:\samples\kick.wav"),
                PathBuf::from(r"D:\packs\snare.wav"),
            ]);

            assert!(encoded.ends_with(&[0, 0, 0, 0]));
            assert_eq!(
                decode_drag_paths(&encoded),
                vec![
                    String::from(r"C:\samples\kick.wav"),
                    String::from(r"D:\packs\snare.wav"),
                ]
            );
        }

        #[test]
        fn build_dropfiles_payload_prepends_wide_dropfiles_header() {
            let payload = build_dropfiles_payload(&[PathBuf::from(r"C:\samples\hat.wav")]);
            let header_len = std::mem::size_of::<DROPFILES>();
            let header = unsafe { std::ptr::read_unaligned(payload.as_ptr().cast::<DROPFILES>()) };

            assert_eq!(header.pFiles as usize, header_len);
            assert!(header.fWide.as_bool());
            assert!(!header.fNC.as_bool());
            assert_eq!(
                decode_drag_paths(&payload[header_len..]),
                vec![String::from(r"C:\samples\hat.wav")]
            );
        }

        #[test]
        fn preview_width_clamps_to_usable_native_drag_size() {
            assert_eq!(preview_width("a"), PREVIEW_MIN_WIDTH);
            assert_eq!(preview_width(&"a".repeat(200)), PREVIEW_MAX_WIDTH);
        }
    }
}
