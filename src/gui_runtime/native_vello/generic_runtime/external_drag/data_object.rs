use super::payload::{
    build_drop_effect_format, build_file_format, create_hglobal_for_paths, drop_effect_formats,
    drop_effect_medium,
};
use std::cell::Cell;
use std::mem::ManuallyDrop;
use std::path::PathBuf;
use windows::Win32::Foundation::{DV_E_FORMATETC, E_INVALIDARG};
use windows::Win32::System::Com::{
    DATADIR_GET, DVASPECT_CONTENT, FORMATETC, IAdviseSink, IDataObject, IEnumFORMATETC, STGMEDIUM,
    STGMEDIUM_0, TYMED_HGLOBAL,
};
use windows::Win32::System::Memory::{GlobalLock, GlobalUnlock};
use windows::Win32::System::Ole::{CF_HDROP, DROPEFFECT, DROPEFFECT_COPY, DROPEFFECT_NONE};
use windows::Win32::UI::Shell::SHCreateStdEnumFmtEtc;
use windows::core::{BOOL, HRESULT, Ref, implement};

#[implement(IDataObject)]
#[derive(Clone)]
pub(super) struct FileDropDataObject {
    paths: Vec<PathBuf>,
    format: FORMATETC,
    preferred_drop_effect: u16,
    performed_drop_effect: u16,
    performed_effect: Cell<DROPEFFECT>,
}

impl FileDropDataObject {
    pub(super) fn new(paths: Vec<PathBuf>) -> Result<Self, String> {
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
        if fmt.cfFormat != self.performed_drop_effect || (fmt.tymed & TYMED_HGLOBAL.0 as u32) == 0 {
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
