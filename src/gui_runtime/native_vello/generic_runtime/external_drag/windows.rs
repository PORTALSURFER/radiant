//! Windows OLE external drag launching.

#[path = "data_object.rs"]
mod data_object;
#[path = "drop_source.rs"]
mod drop_source;
#[path = "payload.rs"]
mod payload;
#[path = "preview.rs"]
mod preview;

use crate::runtime::{ExternalDragOutcome, ExternalDragPayload, ExternalDragRequest};
use data_object::FileDropDataObject;
use drop_source::SimpleDropSource;
use payload::{external_drag_effect, normalize_path};
use preview::DragImage;
use tracing::warn;
use windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, CoCreateInstance, IDataObject};
use windows::Win32::System::Ole::{
    DROPEFFECT_COPY, DROPEFFECT_LINK, DROPEFFECT_MOVE, DROPEFFECT_NONE, DoDragDrop, IDropSource,
    OleInitialize, OleUninitialize,
};
use windows::Win32::UI::Shell::{CLSID_DragDropHelper, IDragSourceHelper};

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
            if let Err(err) = initialize_drag_image(&image, &data_object) {
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

fn initialize_drag_image(
    image: &DragImage,
    data_object: &IDataObject,
) -> windows::core::Result<()> {
    let helper: IDragSourceHelper =
        unsafe { CoCreateInstance(&CLSID_DragDropHelper, None, CLSCTX_INPROC_SERVER) }?;
    unsafe { helper.InitializeFromBitmap(image.as_shell_image(), data_object) }
}
