use std::path::{Path, PathBuf};

use super::bridge::{
    Id, NSPoint, NSRect, NSSize, class, msg_id, msg_id_id, msg_id_usize, msg_void_id,
    msg_void_rect_id, ns_string, selector,
};

const DRAG_ICON_SIZE: f64 = 48.0;

pub(super) unsafe fn dragging_items(paths: &[PathBuf]) -> Result<Id, String> {
    let items = unsafe { mutable_array(paths.len())? };
    let contents = unsafe { drag_preview_contents(paths)? };
    for path in paths {
        let url = unsafe { file_url_for_path(path)? };
        let item = unsafe { dragging_item(url)? };
        unsafe {
            msg_void_rect_id(
                item,
                selector(c"setDraggingFrame:contents:"),
                dragging_frame(),
                contents,
            );
            msg_void_id(items, selector(c"addObject:"), item);
        }
    }
    Ok(items)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum DragPreviewKind {
    FileIcon,
    SharedFileTypeIcon,
}

pub(super) fn drag_preview_kind(path_count: usize) -> DragPreviewKind {
    if path_count <= 1 {
        DragPreviewKind::FileIcon
    } else {
        DragPreviewKind::SharedFileTypeIcon
    }
}

unsafe fn drag_preview_contents(paths: &[PathBuf]) -> Result<Id, String> {
    let first = paths
        .first()
        .ok_or_else(|| String::from("No files to drag"))?;
    match drag_preview_kind(paths.len()) {
        DragPreviewKind::FileIcon => unsafe { file_icon_for_path(first) },
        DragPreviewKind::SharedFileTypeIcon => unsafe { file_type_icon_for_path(first) },
    }
}

unsafe fn mutable_array(capacity: usize) -> Result<Id, String> {
    let array = unsafe {
        let class = class(c"NSMutableArray")?;
        msg_id_usize(class, selector(c"arrayWithCapacity:"), capacity)
    };
    if array.is_null() {
        Err(String::from("Failed to create NSMutableArray"))
    } else {
        Ok(array)
    }
}

unsafe fn file_url_for_path(path: &Path) -> Result<Id, String> {
    let path = path_to_string(path)?;
    let ns_path = unsafe { ns_string(&path)? };
    let url = unsafe {
        let class = class(c"NSURL")?;
        msg_id_id(class, selector(c"fileURLWithPath:"), ns_path)
    };
    if url.is_null() {
        Err(format!("Failed to create file URL for {path}"))
    } else {
        Ok(url)
    }
}

unsafe fn dragging_item(url: Id) -> Result<Id, String> {
    let allocated = unsafe {
        let class = class(c"NSDraggingItem")?;
        msg_id(class, selector(c"alloc"))
    };
    if allocated.is_null() {
        return Err(String::from("Failed to allocate NSDraggingItem"));
    }
    let item = unsafe { msg_id_id(allocated, selector(c"initWithPasteboardWriter:"), url) };
    if item.is_null() {
        Err(String::from("Failed to create NSDraggingItem"))
    } else {
        Ok(unsafe { msg_id(item, selector(c"autorelease")) })
    }
}

unsafe fn file_icon_for_path(path: &Path) -> Result<Id, String> {
    let path = path_to_string(path)?;
    let ns_path = unsafe { ns_string(&path)? };
    let workspace = unsafe {
        let class = class(c"NSWorkspace")?;
        msg_id(class, selector(c"sharedWorkspace"))
    };
    if workspace.is_null() {
        return Err(String::from("NSWorkspace sharedWorkspace returned nil"));
    }
    let icon = unsafe { msg_id_id(workspace, selector(c"iconForFile:"), ns_path) };
    if icon.is_null() {
        Err(format!("NSWorkspace iconForFile returned nil for {path}"))
    } else {
        Ok(icon)
    }
}

unsafe fn file_type_icon_for_path(path: &Path) -> Result<Id, String> {
    let file_type = path
        .extension()
        .and_then(|extension| extension.to_str())
        .filter(|extension| !extension.is_empty())
        .unwrap_or("public.data");
    let ns_file_type = unsafe { ns_string(file_type)? };
    let workspace = unsafe {
        let class = class(c"NSWorkspace")?;
        msg_id(class, selector(c"sharedWorkspace"))
    };
    if workspace.is_null() {
        return Err(String::from("NSWorkspace sharedWorkspace returned nil"));
    }
    let icon = unsafe { msg_id_id(workspace, selector(c"iconForFileType:"), ns_file_type) };
    if icon.is_null() {
        Err(format!(
            "NSWorkspace iconForFileType returned nil for {file_type}"
        ))
    } else {
        Ok(icon)
    }
}

fn dragging_frame() -> NSRect {
    NSRect {
        origin: NSPoint { x: 0.0, y: 0.0 },
        size: NSSize {
            width: DRAG_ICON_SIZE,
            height: DRAG_ICON_SIZE,
        },
    }
}

fn path_to_string(path: &Path) -> Result<String, String> {
    path.to_str().map(ToOwned::to_owned).ok_or_else(|| {
        format!(
            "Cannot drag non-UTF-8 path to external application: {}",
            path.display()
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dragging_frame_uses_stable_icon_geometry() {
        let frame = dragging_frame();

        assert_eq!(frame.origin, NSPoint { x: 0.0, y: 0.0 });
        assert_eq!(
            frame.size,
            NSSize {
                width: 48.0,
                height: 48.0,
            }
        );
    }

    #[test]
    fn utf8_drag_path_round_trips_without_normalization() {
        let path = Path::new("/tmp/Radiant Drag/kick.wav");

        assert_eq!(path_to_string(path).unwrap(), path.to_str().unwrap());
    }

    #[test]
    fn single_file_drag_keeps_file_specific_preview() {
        assert_eq!(drag_preview_kind(1), DragPreviewKind::FileIcon);
    }

    #[test]
    fn multi_file_drag_uses_one_shared_file_type_preview_at_every_scale() {
        for path_count in [2, 10, 100, 1_000] {
            assert_eq!(
                drag_preview_kind(path_count),
                DragPreviewKind::SharedFileTypeIcon,
                "selection size {path_count} should not trigger per-file icon lookup"
            );
        }
    }
}
