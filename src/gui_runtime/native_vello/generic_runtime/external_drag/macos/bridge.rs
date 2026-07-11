use std::ffi::{CStr, c_char, c_void};

pub(super) type Id = *mut c_void;
pub(super) type Sel = *mut c_void;
pub(super) type ObjcBool = i8;

pub(super) const YES: ObjcBool = 1;
pub(super) const NO: ObjcBool = 0;
const NS_LEFT_MOUSE_DOWN: usize = 1;
const NS_LEFT_MOUSE_DRAGGED: usize = 6;
const NS_UTF8_STRING_ENCODING: usize = 4;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct NSPoint {
    pub(super) x: f64,
    pub(super) y: f64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct NSSize {
    pub(super) width: f64,
    pub(super) height: f64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct NSRect {
    pub(super) origin: NSPoint,
    pub(super) size: NSSize,
}

#[link(name = "AppKit", kind = "framework")]
unsafe extern "C" {}

#[link(name = "Foundation", kind = "framework")]
unsafe extern "C" {}

#[link(name = "objc")]
unsafe extern "C" {
    pub(super) fn objc_allocateClassPair(
        superclass: Id,
        name: *const c_char,
        extra_bytes: usize,
    ) -> Id;
    pub(super) fn objc_getClass(name: *const c_char) -> Id;
    fn objc_msgSend();
    pub(super) fn objc_registerClassPair(class: Id);
    pub(super) fn class_addMethod(
        class: Id,
        name: Sel,
        imp: *const c_void,
        types: *const c_char,
    ) -> ObjcBool;
    fn sel_registerName(name: *const c_char) -> Sel;
}

pub(super) struct AutoreleasePool {
    pool: Id,
}

impl AutoreleasePool {
    pub(super) fn new() -> Result<Self, String> {
        let pool = unsafe {
            let class = class(c"NSAutoreleasePool")?;
            msg_id(class, selector(c"new"))
        };
        if pool.is_null() {
            Err(String::from("Failed to create NSAutoreleasePool"))
        } else {
            Ok(Self { pool })
        }
    }
}

impl Drop for AutoreleasePool {
    fn drop(&mut self) {
        unsafe { msg_void(self.pool, selector(c"drain")) };
    }
}

pub(super) unsafe fn shared_application() -> Result<Id, String> {
    let app = unsafe {
        let class = class(c"NSApplication")?;
        msg_id(class, selector(c"sharedApplication"))
    };
    if app.is_null() {
        Err(String::from("NSApplication sharedApplication returned nil"))
    } else {
        Ok(app)
    }
}

pub(super) unsafe fn external_drag_event(app: Id, window: Id) -> Result<Id, String> {
    let event = unsafe { msg_id(app, selector(c"currentEvent")) };
    if !event.is_null()
        && matches!(
            unsafe { msg_usize(event, selector(c"type")) },
            NS_LEFT_MOUSE_DOWN | NS_LEFT_MOUSE_DRAGGED
        )
    {
        return Ok(event);
    }
    unsafe { synthetic_left_drag_event(window) }
}

unsafe fn synthetic_left_drag_event(window: Id) -> Result<Id, String> {
    let point = unsafe { msg_point(window, selector(c"mouseLocationOutsideOfEventStream")) };
    let window_number = unsafe { msg_isize(window, selector(c"windowNumber")) };
    let event_class = unsafe { class(c"NSEvent")? };
    let event = unsafe {
        msg_id_usize_point_usize_f64_isize_id_isize_isize_f64(
            event_class,
            selector(
                c"mouseEventWithType:location:modifierFlags:timestamp:windowNumber:context:eventNumber:clickCount:pressure:",
            ),
            NS_LEFT_MOUSE_DRAGGED,
            point,
            0,
            0.0,
            window_number,
            std::ptr::null_mut(),
            0,
            1,
            1.0,
        )
    };
    if event.is_null() {
        Err(String::from(
            "Failed to synthesize NSEvent for external drag",
        ))
    } else {
        Ok(event)
    }
}

pub(super) unsafe fn key_window_and_content_view(app: Id) -> Result<(Id, Id), String> {
    let mut window = unsafe { msg_id(app, selector(c"keyWindow")) };
    if window.is_null() {
        window = unsafe { msg_id(app, selector(c"mainWindow")) };
    }
    if window.is_null() {
        return Err(String::from("NSApplication has no key or main window"));
    }
    let view = unsafe { msg_id(window, selector(c"contentView")) };
    if view.is_null() {
        Err(String::from("NSWindow contentView returned nil"))
    } else {
        Ok((window, view))
    }
}

pub(super) unsafe fn begin_dragging_session(
    view: Id,
    items: Id,
    event: Id,
    source: Id,
) -> Result<(), String> {
    let session = unsafe {
        msg_id_id_id_id(
            view,
            selector(c"beginDraggingSessionWithItems:event:source:"),
            items,
            event,
            source,
        )
    };
    if session.is_null() {
        Err(String::from(
            "NSView beginDraggingSessionWithItems returned nil",
        ))
    } else {
        Ok(())
    }
}

pub(super) unsafe fn ns_string(value: &str) -> Result<Id, String> {
    let allocated = unsafe {
        let class = class(c"NSString")?;
        msg_id(class, selector(c"alloc"))
    };
    if allocated.is_null() {
        return Err(String::from("Failed to allocate NSString"));
    }
    let string = unsafe {
        msg_id_ptr_usize_usize(
            allocated,
            selector(c"initWithBytes:length:encoding:"),
            value.as_ptr().cast(),
            value.len(),
            NS_UTF8_STRING_ENCODING,
        )
    };
    if string.is_null() {
        Err(String::from("Failed to create NSString"))
    } else {
        Ok(unsafe { msg_id(string, selector(c"autorelease")) })
    }
}

pub(super) unsafe fn class(name: &'static CStr) -> Result<Id, String> {
    let class = unsafe { objc_getClass(name.as_ptr()) };
    if class.is_null() {
        Err(format!(
            "Objective-C class {} not found",
            name.to_string_lossy()
        ))
    } else {
        Ok(class)
    }
}

pub(super) unsafe fn selector(name: &'static CStr) -> Sel {
    unsafe { sel_registerName(name.as_ptr()) }
}

pub(super) unsafe fn msg_id(receiver: Id, selector: Sel) -> Id {
    let msg: unsafe extern "C" fn(Id, Sel) -> Id =
        unsafe { std::mem::transmute(objc_msgSend as *const ()) };
    unsafe { msg(receiver, selector) }
}

pub(super) unsafe fn msg_id_id(receiver: Id, selector: Sel, arg: Id) -> Id {
    let msg: unsafe extern "C" fn(Id, Sel, Id) -> Id =
        unsafe { std::mem::transmute(objc_msgSend as *const ()) };
    unsafe { msg(receiver, selector, arg) }
}

unsafe fn msg_id_id_id_id(receiver: Id, selector: Sel, first: Id, second: Id, third: Id) -> Id {
    let msg: unsafe extern "C" fn(Id, Sel, Id, Id, Id) -> Id =
        unsafe { std::mem::transmute(objc_msgSend as *const ()) };
    unsafe { msg(receiver, selector, first, second, third) }
}

pub(super) unsafe fn msg_id_usize(receiver: Id, selector: Sel, arg: usize) -> Id {
    let msg: unsafe extern "C" fn(Id, Sel, usize) -> Id =
        unsafe { std::mem::transmute(objc_msgSend as *const ()) };
    unsafe { msg(receiver, selector, arg) }
}

unsafe fn msg_id_usize_point_usize_f64_isize_id_isize_isize_f64(
    receiver: Id,
    selector: Sel,
    event_type: usize,
    location: NSPoint,
    modifier_flags: usize,
    timestamp: f64,
    window_number: isize,
    context: Id,
    event_number: isize,
    click_count: isize,
    pressure: f64,
) -> Id {
    let msg: unsafe extern "C" fn(
        Id,
        Sel,
        usize,
        NSPoint,
        usize,
        f64,
        isize,
        Id,
        isize,
        isize,
        f64,
    ) -> Id = unsafe { std::mem::transmute(objc_msgSend as *const ()) };
    unsafe {
        msg(
            receiver,
            selector,
            event_type,
            location,
            modifier_flags,
            timestamp,
            window_number,
            context,
            event_number,
            click_count,
            pressure,
        )
    }
}

unsafe fn msg_id_ptr_usize_usize(
    receiver: Id,
    selector: Sel,
    bytes: *const c_void,
    length: usize,
    encoding: usize,
) -> Id {
    let msg: unsafe extern "C" fn(Id, Sel, *const c_void, usize, usize) -> Id =
        unsafe { std::mem::transmute(objc_msgSend as *const ()) };
    unsafe { msg(receiver, selector, bytes, length, encoding) }
}

pub(super) unsafe fn msg_void(receiver: Id, selector: Sel) {
    let msg: unsafe extern "C" fn(Id, Sel) =
        unsafe { std::mem::transmute(objc_msgSend as *const ()) };
    unsafe { msg(receiver, selector) }
}

pub(super) unsafe fn msg_void_id(receiver: Id, selector: Sel, arg: Id) {
    let msg: unsafe extern "C" fn(Id, Sel, Id) =
        unsafe { std::mem::transmute(objc_msgSend as *const ()) };
    unsafe { msg(receiver, selector, arg) }
}

pub(super) unsafe fn msg_void_rect_id(receiver: Id, selector: Sel, rect: NSRect, arg: Id) {
    let msg: unsafe extern "C" fn(Id, Sel, NSRect, Id) =
        unsafe { std::mem::transmute(objc_msgSend as *const ()) };
    unsafe { msg(receiver, selector, rect, arg) }
}

unsafe fn msg_isize(receiver: Id, selector: Sel) -> isize {
    let msg: unsafe extern "C" fn(Id, Sel) -> isize =
        unsafe { std::mem::transmute(objc_msgSend as *const ()) };
    unsafe { msg(receiver, selector) }
}

unsafe fn msg_point(receiver: Id, selector: Sel) -> NSPoint {
    let msg: unsafe extern "C" fn(Id, Sel) -> NSPoint =
        unsafe { std::mem::transmute(objc_msgSend as *const ()) };
    unsafe { msg(receiver, selector) }
}

unsafe fn msg_usize(receiver: Id, selector: Sel) -> usize {
    let msg: unsafe extern "C" fn(Id, Sel) -> usize =
        unsafe { std::mem::transmute(objc_msgSend as *const ()) };
    unsafe { msg(receiver, selector) }
}
