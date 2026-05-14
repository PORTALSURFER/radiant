use windows::Win32::Foundation::{COLORREF, POINT, RECT, SIZE};
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleBitmap, CreateCompatibleDC, CreateSolidBrush, DT_END_ELLIPSIS, DT_LEFT,
    DT_SINGLELINE, DT_VCENTER, DeleteDC, DeleteObject, DrawTextW, FillRect, GetDC, HBITMAP,
    HGDIOBJ, ReleaseDC, SelectObject, SetBkMode, SetTextColor, TRANSPARENT,
};
use windows::Win32::UI::Shell::SHDRAGIMAGE;

const PREVIEW_MIN_WIDTH: i32 = 132;
const PREVIEW_MAX_WIDTH: i32 = 280;
const PREVIEW_HEIGHT: i32 = 30;
const PREVIEW_COLOR_KEY: COLORREF = COLORREF(0x00ff00ff);

pub(super) struct DragImage {
    bitmap: HBITMAP,
    image: SHDRAGIMAGE,
}

impl DragImage {
    pub(super) fn new(label: &str) -> Result<Self, String> {
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

    pub(super) fn as_shell_image(&self) -> &SHDRAGIMAGE {
        &self.image
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_width_clamps_to_usable_native_drag_size() {
        assert_eq!(preview_width("a"), PREVIEW_MIN_WIDTH);
        assert_eq!(preview_width(&"a".repeat(200)), PREVIEW_MAX_WIDTH);
    }
}
