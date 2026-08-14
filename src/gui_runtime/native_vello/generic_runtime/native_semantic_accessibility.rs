//! Private primary-window macOS semantic accessibility consumer.
//!
//! The adapter is deliberately a native boundary, not a public runtime
//! capability.  AppKit calls below are made on the main thread and are
//! wrapped in Rust `catch_unwind` and narrow Objective-C exception boundaries
//! so no Rust panic or Objective-C exception can escape through the callback.
//! Callback state is callback-local and bounded; it never borrows or enters a
//! `SurfaceRuntime`.  Runtime/provider work happens only after the event-loop
//! turn owns `SurfaceRuntime`.

#[cfg(target_os = "macos")]
mod macos {
    use crate::{
        gui::automation::{
            AUTOMATION_ACTION_DECREMENT, AUTOMATION_ACTION_INCREMENT, AUTOMATION_ACTION_SET_TEXT,
            AutomationBounds, AutomationNodeId, AutomationNodeSemantics, AutomationNodeSnapshot,
            AutomationRole, AutomationTarget, GuiAutomationSnapshot, GuiAutomationTargetSnapshot,
        },
        gui_runtime::native_vello::runtime_event::{
            NativeNumericAccessibilityAction, NativeSemanticAccessibilityQuery, RuntimeUserEvent,
        },
        layout::{VIRTUAL_LAYOUT_MAX_QUERY_ENTRIES, VirtualLayoutItemKey},
        runtime::{
            NativeSemanticContainerSnapshot, NativeSemanticCoordinateAuthority,
            NumericAccessibilityRequest, RuntimeBridge, SemanticAutomationContainerHandle,
            SemanticAutomationDemand, SemanticAutomationFallbackReason,
            SemanticAutomationRefreshStatus, SemanticAutomationSessionError,
            SemanticAutomationSessionHandle, SurfaceRuntime,
        },
        widgets::NumericAccessibilityAction,
    };
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use std::{
        cell::RefCell,
        ffi::{CStr, c_char, c_void},
        mem::{align_of, size_of, transmute},
        panic::{AssertUnwindSafe, catch_unwind},
        ptr::null_mut,
        sync::OnceLock,
    };
    use winit::{
        event_loop::EventLoopProxy,
        window::{Window, WindowId},
    };

    #[cfg(target_arch = "x86_64")]
    use std::mem::MaybeUninit;

    const MAX_NATIVE_REGISTRATIONS: usize = 64;
    const MAX_NATIVE_ITEMS: usize = VIRTUAL_LAYOUT_MAX_QUERY_ENTRIES;
    const MAX_NATIVE_NUMERIC_ACTIONS: usize = 64;
    const NATIVE_STATE_IVAR_NAME: &CStr = c"radiantNativeSemanticState";
    const NATIVE_STATE_IVAR_TYPE: &CStr = c"^v";
    const NATIVE_CLASS_NAME: &CStr = c"RadiantNativeSemanticAccessibilityElement";
    const NS_UTF8_STRING_ENCODING: usize = 4;
    const NS_ACCESSIBILITY_INCREMENT_ACTION: &CStr = c"AXIncrement";
    const NS_ACCESSIBILITY_DECREMENT_ACTION: &CStr = c"AXDecrement";
    const NS_ACCESSIBILITY_VALUE_ATTRIBUTE: &CStr = c"AXValue";
    const NS_NOT_FOUND: usize = isize::MAX as usize;
    #[cfg(test)]
    const MAX_NATIVE_VALUE_UTF16_UNITS: usize = 1_024;
    const MAX_NATIVE_VALUE_UTF8_BYTES: usize = 4_096;
    const MODERN_ACTION_METHOD_TYPE: &CStr = c"c@:";
    const DEPRECATED_ACTION_METHOD_TYPE: &CStr = c"v@:@";
    const MODERN_VALUE_METHOD_TYPE: &CStr = c"@@:";
    const VALUE_SETTER_METHOD_TYPE: &CStr = c"v@:@";
    const LEGACY_VALUE_SETTER_METHOD_TYPE: &CStr = c"v@:@@";
    const INDEX_OF_CHILD_METHOD_TYPE: &CStr = c"Q@:@";
    const ACCESSIBILITY_NOTIFIES_WHEN_DESTROYED_METHOD_TYPE: &CStr = c"c@:";
    const YES: ObjcBool = 1;
    const NO: ObjcBool = 0;

    type Id = *mut c_void;
    type Ivar = *mut c_void;
    type Sel = *mut c_void;
    type ObjcBool = i8;

    #[repr(C)]
    #[derive(Clone, Copy, Debug, PartialEq)]
    struct NSPoint {
        x: f64,
        y: f64,
    }

    #[repr(C)]
    #[derive(Clone, Copy, Debug, PartialEq)]
    struct NSSize {
        width: f64,
        height: f64,
    }

    #[repr(C)]
    #[derive(Clone, Copy, Debug, PartialEq)]
    struct NSRect {
        origin: NSPoint,
        size: NSSize,
    }

    #[repr(C)]
    struct ObjcSuper {
        receiver: Id,
        superclass: Id,
    }

    #[link(name = "AppKit", kind = "framework")]
    unsafe extern "C" {}

    #[link(name = "Foundation", kind = "framework")]
    unsafe extern "C" {}

    #[link(name = "objc")]
    unsafe extern "C" {
        fn objc_allocateClassPair(superclass: Id, name: *const c_char, extra_bytes: usize) -> Id;
        fn objc_getClass(name: *const c_char) -> Id;
        fn objc_registerClassPair(class: Id);
        fn objc_disposeClassPair(class: Id);
        fn class_addIvar(
            class: Id,
            name: *const c_char,
            size: usize,
            alignment: u8,
            types: *const c_char,
        ) -> ObjcBool;
        fn class_getInstanceVariable(class: Id, name: *const c_char) -> Ivar;
        #[cfg(test)]
        fn class_getInstanceMethod(class: Id, name: Sel) -> Id;
        #[cfg(test)]
        fn method_getTypeEncoding(method: Id) -> *const c_char;
        fn class_addMethod(
            class: Id,
            name: Sel,
            imp: *const c_void,
            types: *const c_char,
        ) -> ObjcBool;
        #[cfg(test)]
        fn method_getImplementation(method: Id) -> *const c_void;
        fn object_getClass(object: Id) -> Id;
        fn object_getIvar(object: Id, ivar: Ivar) -> Id;
        fn object_setIvar(object: Id, ivar: Ivar, value: Id);
        fn sel_registerName(name: *const c_char) -> Sel;
        fn objc_msgSend();
        #[cfg(target_arch = "x86_64")]
        fn objc_msgSend_stret();
        fn objc_msgSendSuper();
        fn radiant_native_bounded_ns_string_to_utf8(
            value: Id,
            out: *mut u8,
            cap: usize,
            len: *mut usize,
        ) -> ObjcBool;
        fn radiant_native_convert_view_rect_to_screen(
            view: Id,
            window: Id,
            source: *const NSRect,
            out: *mut NSRect,
        ) -> ObjcBool;
        #[cfg(test)]
        fn radiant_native_test_convert_view_rect_to_screen(
            source: *const NSRect,
            out: *mut NSRect,
        ) -> ObjcBool;
        fn radiant_native_attribute_is(
            attribute: Id,
            expected: *const u8,
            expected_len: usize,
        ) -> ObjcBool;
        fn radiant_native_configure_accessibility_element(
            element: Id,
            role: Id,
            parent: Id,
            children: Id,
            frame: *const NSRect,
            label: Id,
            title: Id,
            help: Id,
            has_enabled: ObjcBool,
            enabled: ObjcBool,
        ) -> ObjcBool;
        fn radiant_native_set_accessibility_children(host: Id, root: Id) -> ObjcBool;
        fn radiant_native_clear_accessibility_children(host: Id) -> ObjcBool;
        #[cfg(test)]
        fn radiant_native_test_make_accessibility_element(kind: u8) -> Id;
        #[cfg(test)]
        fn radiant_native_test_make_accessibility_children_host(kind: u8) -> Id;
    }

    #[link(name = "AppKit", kind = "framework")]
    unsafe extern "C" {
        fn NSAccessibilityPostNotification(element: Id, notification: Id);
    }

    #[derive(Clone, Copy, Debug)]
    struct NativeClass {
        class: usize,
        state_ivar: usize,
    }

    impl NativeClass {
        fn class(self) -> Id {
            self.class as Id
        }

        fn state_ivar(self) -> Ivar {
            self.state_ivar as Ivar
        }
    }

    fn native_class() -> Result<NativeClass, String> {
        static CLASS: OnceLock<Result<NativeClass, String>> = OnceLock::new();
        CLASS
            .get_or_init(|| unsafe { create_native_class() })
            .clone()
    }

    unsafe fn create_native_class() -> Result<NativeClass, String> {
        let superclass = unsafe { objc_getClass(c"NSAccessibilityElement".as_ptr()) };
        if superclass.is_null() {
            return Err(String::from("NSAccessibilityElement class was unavailable"));
        }
        let mut class = unsafe { objc_getClass(NATIVE_CLASS_NAME.as_ptr()) };
        if class.is_null() {
            class = unsafe { objc_allocateClassPair(superclass, NATIVE_CLASS_NAME.as_ptr(), 0) };
            if class.is_null() {
                return Err(String::from("objc_allocateClassPair failed"));
            }
            let added_ivar = unsafe {
                class_addIvar(
                    class,
                    NATIVE_STATE_IVAR_NAME.as_ptr(),
                    size_of::<Id>(),
                    align_of::<Id>().trailing_zeros() as u8,
                    NATIVE_STATE_IVAR_TYPE.as_ptr(),
                )
            };
            if added_ivar == NO {
                unsafe { objc_disposeClassPair(class) };
                return Err(String::from("class_addIvar failed for semantic state"));
            }
            let methods = [
                (c"dealloc", native_dealloc as *const c_void, c"v@:"),
                (
                    c"accessibilityNotifiesWhenDestroyed",
                    native_accessibility_notifies_when_destroyed as *const c_void,
                    ACCESSIBILITY_NOTIFIES_WHEN_DESTROYED_METHOD_TYPE,
                ),
                (
                    c"isAccessibilityElement",
                    native_is_accessibility_element as *const c_void,
                    c"c@:",
                ),
                (
                    c"accessibilityAttributeValue:",
                    native_attribute_value as *const c_void,
                    c"@@:@",
                ),
                (
                    c"accessibilityValue",
                    native_accessibility_value as *const c_void,
                    MODERN_VALUE_METHOD_TYPE,
                ),
                (
                    c"accessibilityArrayAttributeCount:",
                    native_array_attribute_count as *const c_void,
                    c"Q@:@",
                ),
                (
                    c"accessibilityIndexOfChild:",
                    native_index_of_child as *const c_void,
                    INDEX_OF_CHILD_METHOD_TYPE,
                ),
                (
                    c"accessibilityArrayAttributeValues:index:maxCount:",
                    native_array_attribute_values as *const c_void,
                    c"@@:@QQ",
                ),
                (
                    c"accessibilityIsAttributeSettable:",
                    native_attribute_settable as *const c_void,
                    c"c@:@",
                ),
                (
                    c"setAccessibilityValue:",
                    native_set_accessibility_value as *const c_void,
                    VALUE_SETTER_METHOD_TYPE,
                ),
                (
                    c"accessibilitySetValue:forAttribute:",
                    native_set_accessibility_value_for_attribute as *const c_void,
                    LEGACY_VALUE_SETTER_METHOD_TYPE,
                ),
                (
                    c"accessibilityActionNames",
                    native_action_names as *const c_void,
                    c"@@:",
                ),
                (
                    c"accessibilityPerformAction:",
                    native_perform_action as *const c_void,
                    DEPRECATED_ACTION_METHOD_TYPE,
                ),
                (
                    c"accessibilityPerformIncrement",
                    native_perform_increment as *const c_void,
                    MODERN_ACTION_METHOD_TYPE,
                ),
                (
                    c"accessibilityPerformDecrement",
                    native_perform_decrement as *const c_void,
                    MODERN_ACTION_METHOD_TYPE,
                ),
            ];
            for (name, implementation, types) in methods {
                let added =
                    unsafe { class_addMethod(class, sel(name), implementation, types.as_ptr()) };
                if added == NO {
                    unsafe { objc_disposeClassPair(class) };
                    return Err(format!(
                        "class_addMethod failed for {}",
                        name.to_string_lossy()
                    ));
                }
            }
            unsafe { objc_registerClassPair(class) };
        }
        let state_ivar =
            unsafe { class_getInstanceVariable(class, NATIVE_STATE_IVAR_NAME.as_ptr()) };
        if state_ivar.is_null() {
            return Err(String::from("semantic state ivar was unavailable"));
        }
        Ok(NativeClass {
            class: class as usize,
            state_ivar: state_ivar as usize,
        })
    }

    unsafe fn class_named(name: &'static CStr) -> Result<Id, String> {
        let class = unsafe { objc_getClass(name.as_ptr()) };
        if class.is_null() {
            Err(format!(
                "Objective-C class {} was unavailable",
                name.to_string_lossy()
            ))
        } else {
            Ok(class)
        }
    }

    unsafe fn sel(name: &'static CStr) -> Sel {
        unsafe { sel_registerName(name.as_ptr()) }
    }

    unsafe fn msg_id(receiver: Id, selector: Sel) -> Id {
        let message: unsafe extern "C" fn(Id, Sel) -> Id =
            unsafe { transmute(objc_msgSend as *const ()) };
        unsafe { message(receiver, selector) }
    }

    #[cfg(test)]
    unsafe fn msg_usize_id(receiver: Id, selector: Sel, argument: Id) -> usize {
        let message: unsafe extern "C" fn(Id, Sel, Id) -> usize =
            unsafe { transmute(objc_msgSend as *const ()) };
        unsafe { message(receiver, selector, argument) }
    }

    #[cfg(test)]
    unsafe fn msg_usize(receiver: Id, selector: Sel) -> usize {
        let message: unsafe extern "C" fn(Id, Sel) -> usize =
            unsafe { transmute(objc_msgSend as *const ()) };
        unsafe { message(receiver, selector) }
    }

    #[cfg(test)]
    unsafe fn msg_id_usize(receiver: Id, selector: Sel, argument: usize) -> Id {
        let message: unsafe extern "C" fn(Id, Sel, usize) -> Id =
            unsafe { transmute(objc_msgSend as *const ()) };
        unsafe { message(receiver, selector, argument) }
    }

    unsafe fn msg_id_ptr(receiver: Id, selector: Sel) -> *const c_char {
        let message: unsafe extern "C" fn(Id, Sel) -> *const c_char =
            unsafe { transmute(objc_msgSend as *const ()) };
        unsafe { message(receiver, selector) }
    }

    unsafe fn msg_bool(receiver: Id, selector: Sel) -> ObjcBool {
        let message: unsafe extern "C" fn(Id, Sel) -> ObjcBool =
            unsafe { transmute(objc_msgSend as *const ()) };
        unsafe { message(receiver, selector) }
    }

    unsafe fn msg_f64(receiver: Id, selector: Sel) -> f64 {
        let message: unsafe extern "C" fn(Id, Sel) -> f64 =
            unsafe { transmute(objc_msgSend as *const ()) };
        unsafe { message(receiver, selector) }
    }

    unsafe fn msg_void(receiver: Id, selector: Sel) {
        let message: unsafe extern "C" fn(Id, Sel) =
            unsafe { transmute(objc_msgSend as *const ()) };
        unsafe { message(receiver, selector) };
    }

    unsafe fn msg_id_usize_usize(receiver: Id, selector: Sel, first: usize, second: usize) -> Id {
        let message: unsafe extern "C" fn(Id, Sel, usize, usize) -> Id =
            unsafe { transmute(objc_msgSend as *const ()) };
        unsafe { message(receiver, selector, first, second) }
    }

    unsafe fn msg_id_rect(receiver: Id, selector: Sel, rect: NSRect) -> Id {
        let message: unsafe extern "C" fn(Id, Sel, NSRect) -> Id =
            unsafe { transmute(objc_msgSend as *const ()) };
        unsafe { message(receiver, selector, rect) }
    }

    unsafe fn msg_rect(receiver: Id, selector: Sel) -> NSRect {
        #[cfg(target_arch = "x86_64")]
        {
            let message: unsafe extern "C" fn(*mut NSRect, Id, Sel) =
                unsafe { transmute(objc_msgSend_stret as *const ()) };
            let mut result = MaybeUninit::<NSRect>::uninit();
            unsafe { message(result.as_mut_ptr(), receiver, selector) };
            unsafe { result.assume_init() }
        }

        #[cfg(not(target_arch = "x86_64"))]
        {
            let message: unsafe extern "C" fn(Id, Sel) -> NSRect =
                unsafe { transmute(objc_msgSend as *const ()) };
            unsafe { message(receiver, selector) }
        }
    }

    unsafe fn msg_super_void(receiver: Id, superclass: Id, selector: Sel) {
        let message: unsafe extern "C" fn(*mut ObjcSuper, Sel) =
            unsafe { transmute(objc_msgSendSuper as *const ()) };
        let mut call = ObjcSuper {
            receiver,
            superclass,
        };
        unsafe { message(&mut call, selector) };
    }

    unsafe fn ns_string(value: &str) -> Id {
        let class = match unsafe { class_named(c"NSString") } {
            Ok(class) => class,
            Err(_) => return null_mut(),
        };
        let allocated = unsafe { msg_id(class, sel(c"alloc")) };
        if allocated.is_null() {
            return null_mut();
        }
        let message: unsafe extern "C" fn(Id, Sel, *const c_void, usize, usize) -> Id =
            unsafe { transmute(objc_msgSend as *const ()) };
        let initialized = unsafe {
            message(
                allocated,
                sel(c"initWithBytes:length:encoding:"),
                value.as_ptr().cast(),
                value.len(),
                NS_UTF8_STRING_ENCODING,
            )
        };
        if initialized.is_null() {
            return null_mut();
        }
        unsafe { msg_id(initialized, sel(c"autorelease")) }
    }

    /// Copy one AppKit NSString into Rust only after both bounded Foundation
    /// length checks have passed.  `NSData` is used instead of `UTF8String` so
    /// embedded NULs remain part of the exact payload.
    unsafe fn bounded_ns_string_to_rust(value: Id) -> Option<String> {
        let mut bytes = [0_u8; MAX_NATIVE_VALUE_UTF8_BYTES];
        let mut len = 0;
        let accepted = unsafe {
            radiant_native_bounded_ns_string_to_utf8(
                value,
                bytes.as_mut_ptr(),
                bytes.len(),
                &mut len,
            )
        };
        if accepted == NO || len > bytes.len() {
            return None;
        }
        std::str::from_utf8(&bytes[..len])
            .ok()
            .map(|text| text.to_owned())
    }

    unsafe fn ns_array(values: &[Id]) -> Id {
        let Ok(class) = (unsafe { class_named(c"NSArray") }) else {
            return null_mut();
        };
        if values.is_empty() {
            return unsafe { msg_id(class, sel(c"array")) };
        }
        unsafe {
            msg_id_usize_usize(
                class,
                sel(c"arrayWithObjects:count:"),
                values.as_ptr() as usize,
                values.len(),
            )
        }
    }

    unsafe fn ns_number_bool(value: bool) -> Id {
        let Ok(class) = (unsafe { class_named(c"NSNumber") }) else {
            return null_mut();
        };
        unsafe {
            let message: unsafe extern "C" fn(Id, Sel, ObjcBool) -> Id =
                transmute(objc_msgSend as *const ());
            message(class, sel(c"numberWithBool:"), if value { YES } else { NO })
        }
    }

    unsafe fn ns_number_usize(value: usize) -> Id {
        let Ok(class) = (unsafe { class_named(c"NSNumber") }) else {
            return null_mut();
        };
        let message: unsafe extern "C" fn(Id, Sel, u64) -> Id =
            unsafe { transmute(objc_msgSend as *const ()) };
        unsafe { message(class, sel(c"numberWithUnsignedLongLong:"), value as u64) }
    }

    unsafe fn ns_value_rect(value: NSRect) -> Id {
        let Ok(class) = (unsafe { class_named(c"NSValue") }) else {
            return null_mut();
        };
        unsafe { msg_id_rect(class, sel(c"valueWithRect:"), value) }
    }

    unsafe fn ns_value_point(value: NSPoint) -> Id {
        let Ok(class) = (unsafe { class_named(c"NSValue") }) else {
            return null_mut();
        };
        let message: unsafe extern "C" fn(Id, Sel, NSPoint) -> Id =
            unsafe { transmute(objc_msgSend as *const ()) };
        unsafe { message(class, sel(c"valueWithPoint:"), value) }
    }

    unsafe fn ns_value_size(value: NSSize) -> Id {
        let Ok(class) = (unsafe { class_named(c"NSValue") }) else {
            return null_mut();
        };
        let message: unsafe extern "C" fn(Id, Sel, NSSize) -> Id =
            unsafe { transmute(objc_msgSend as *const ()) };
        unsafe { message(class, sel(c"valueWithSize:"), value) }
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum NativeNodeKind {
        Root,
        Ordinary,
        Container,
        Item,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum NativeSemanticUnavailableReason {
        SessionContended,
        RuntimeRejected,
        Stale,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct NativeQueryKey {
        token: u64,
        start_index: usize,
        max_count: usize,
    }

    fn query_retry_mode(
        key: NativeQueryKey,
        in_flight: &[NativeQueryKey],
        deferred: &[NativeQueryKey],
        logical_count: usize,
        published_logical_children: &[(usize, u64)],
    ) -> Option<bool> {
        if key.max_count == 0
            || key.start_index >= logical_count
            || logical_child_range_is_retained(
                published_logical_children,
                logical_count,
                key.start_index,
                key.max_count,
            )
            || in_flight.contains(&key)
        {
            return None;
        }
        Some(deferred.contains(&key))
    }

    fn logical_child_range_is_retained(
        children: &[(usize, u64)],
        logical_count: usize,
        start_index: usize,
        max_count: usize,
    ) -> bool {
        let Some(remaining) = logical_count.checked_sub(start_index) else {
            return false;
        };
        let length = max_count.min(remaining);
        if length == 0 {
            return false;
        }
        let Ok(start_position) = children.binary_search_by_key(&start_index, |(index, _)| *index)
        else {
            return false;
        };
        let Some(end_position) = start_position.checked_add(length) else {
            return false;
        };
        let Some(retained) = children.get(start_position..end_position) else {
            return false;
        };
        retained
            .iter()
            .enumerate()
            .all(|(offset, (index, _))| start_index.checked_add(offset) == Some(*index))
    }

    fn retained_logical_child_tokens(
        children: &[(usize, u64)],
        logical_count: usize,
        start_index: usize,
        max_count: usize,
    ) -> Option<Vec<u64>> {
        if !logical_child_range_is_retained(children, logical_count, start_index, max_count) {
            return None;
        }
        let length = max_count.min(logical_count.checked_sub(start_index)?);
        let start_position = children
            .binary_search_by_key(&start_index, |(index, _)| *index)
            .ok()?;
        let end_position = start_position.checked_add(length)?;
        Some(
            children
                .get(start_position..end_position)?
                .iter()
                .map(|(_, token)| *token)
                .collect(),
        )
    }

    fn accessibility_children_count(node: &NativeCallbackNode) -> usize {
        node.logical_count.unwrap_or(node.children.len())
    }

    fn unique_projection_node_for_object(
        projection: &NativeCallbackProjection,
        object: Id,
    ) -> Option<&NativeCallbackNode> {
        if object.is_null() {
            return None;
        }
        let mut found = None;
        for node in &projection.nodes {
            if node.object == object {
                if found.is_some() {
                    return None;
                }
                found = Some(node);
            }
        }
        found
    }

    fn unique_projection_node_for_token(
        projection: &NativeCallbackProjection,
        token: u64,
    ) -> Option<&NativeCallbackNode> {
        if token == 0 {
            return None;
        }
        let mut found = None;
        for node in &projection.nodes {
            if node.token == token {
                if found.is_some() {
                    return None;
                }
                found = Some(node);
            }
        }
        found
    }

    fn native_index_child_kind_is_valid(receiver: NativeNodeKind, child: NativeNodeKind) -> bool {
        match receiver {
            NativeNodeKind::Container => child == NativeNodeKind::Item,
            NativeNodeKind::Root | NativeNodeKind::Ordinary | NativeNodeKind::Item => {
                matches!(child, NativeNodeKind::Ordinary | NativeNodeKind::Container)
            }
        }
    }

    fn native_index_of_child_in_projection(
        projection: &NativeCallbackProjection,
        receiver: Id,
        child: Id,
    ) -> usize {
        if receiver.is_null() || child.is_null() || receiver == child {
            return NS_NOT_FOUND;
        }
        let Some(receiver_node) = unique_projection_node_for_object(projection, receiver) else {
            return NS_NOT_FOUND;
        };
        let Some(child_node) = unique_projection_node_for_object(projection, child) else {
            return NS_NOT_FOUND;
        };
        if receiver_node.token == 0
            || child_node.token == 0
            || unique_projection_node_for_token(projection, receiver_node.token).is_none()
            || unique_projection_node_for_token(projection, child_node.token).is_none()
            || child_node.parent != Some(receiver_node.token)
        {
            return NS_NOT_FOUND;
        }
        if receiver_node.kind == NativeNodeKind::Root
            && (receiver_node.parent.is_some()
                || projection.root_token != Some(receiver_node.token))
        {
            return NS_NOT_FOUND;
        }

        match receiver_node.kind {
            NativeNodeKind::Container => {
                if receiver_node.logical_count.is_none()
                    || receiver_node.logical_children.len() > MAX_NATIVE_ITEMS
                {
                    return NS_NOT_FOUND;
                }
                let logical_count = receiver_node.logical_count.unwrap_or(0);
                let mut previous_index = None;
                let mut found = None;
                for (position, (logical_index, token)) in
                    receiver_node.logical_children.iter().enumerate()
                {
                    if *logical_index == NS_NOT_FOUND
                        || *logical_index >= logical_count
                        || *token == 0
                        || previous_index.is_some_and(|previous| *logical_index <= previous)
                        || receiver_node
                            .logical_children
                            .iter()
                            .take(position)
                            .any(|(_, previous_token)| previous_token == token)
                    {
                        return NS_NOT_FOUND;
                    }
                    let Some(mapped_child) = unique_projection_node_for_token(projection, *token)
                    else {
                        return NS_NOT_FOUND;
                    };
                    if mapped_child.parent != Some(receiver_node.token)
                        || !native_index_child_kind_is_valid(receiver_node.kind, mapped_child.kind)
                    {
                        return NS_NOT_FOUND;
                    }
                    if mapped_child.object == child && found.replace(*logical_index).is_some() {
                        return NS_NOT_FOUND;
                    }
                    previous_index = Some(*logical_index);
                }
                found
                    .filter(|index| *index != NS_NOT_FOUND)
                    .unwrap_or(NS_NOT_FOUND)
            }
            NativeNodeKind::Root | NativeNodeKind::Ordinary | NativeNodeKind::Item => {
                if receiver_node.logical_count.is_some()
                    || !receiver_node.logical_children.is_empty()
                {
                    return NS_NOT_FOUND;
                }
                let mut found = None;
                for (position, token) in receiver_node.children.iter().enumerate() {
                    if *token == 0
                        || position == NS_NOT_FOUND
                        || receiver_node
                            .children
                            .iter()
                            .take(position)
                            .any(|previous_token| previous_token == token)
                    {
                        return NS_NOT_FOUND;
                    }
                    let Some(mapped_child) = unique_projection_node_for_token(projection, *token)
                    else {
                        return NS_NOT_FOUND;
                    };
                    if mapped_child.parent != Some(receiver_node.token)
                        || !native_index_child_kind_is_valid(receiver_node.kind, mapped_child.kind)
                    {
                        return NS_NOT_FOUND;
                    }
                    if mapped_child.object == child && found.replace(position).is_some() {
                        return NS_NOT_FOUND;
                    }
                }
                found
                    .filter(|index| *index != NS_NOT_FOUND)
                    .unwrap_or(NS_NOT_FOUND)
            }
        }
    }

    fn complete_virtual_child_tokens(node: &NativeCallbackNode) -> Option<Vec<u64>> {
        let count = node.logical_count?;
        if count > MAX_NATIVE_ITEMS {
            return None;
        }
        if count == 0 {
            return (node.children.is_empty() && node.logical_children.is_empty()).then(Vec::new);
        }
        if node.logical_children.len() != count {
            return None;
        }
        if !node
            .logical_children
            .iter()
            .enumerate()
            .all(|(offset, (index, _))| *index == offset)
        {
            return None;
        }
        let tokens = node
            .logical_children
            .iter()
            .map(|(_, token)| *token)
            .collect::<Vec<_>>();
        (node.children == tokens).then_some(tokens)
    }

    fn native_range_query_is_admitted(
        cardinality: crate::application::virtual_layout::VirtualLayoutSemanticCardinality,
        has_range_provider: bool,
        normalized_length: Option<usize>,
    ) -> bool {
        normalized_length.is_some() && (cardinality.logical_item_count == 0 || has_range_provider)
    }

    fn container_token_fence_matches(
        previous: &NativeContainerToken,
        current: &NativeSemanticContainerSnapshot,
        lease: Option<SemanticAutomationSessionHandle>,
        window_generation: u64,
    ) -> bool {
        previous.container_id == current.container_id
            && previous.mount_generation == current.mount_generation
            && previous.registration_generation == current.registration_generation
            && previous.provider_generation == current.provider_generation
            && previous.coordinate_authority == current.coordinate_authority
            && previous.cardinality == current.cardinality
            && previous.lease == lease
            && previous.window_generation == window_generation
    }

    #[derive(Clone, Debug, PartialEq)]
    struct NativeCallbackNode {
        object: Id,
        token: u64,
        kind: NativeNodeKind,
        parent: Option<u64>,
        children: Vec<u64>,
        logical_children: Vec<(usize, u64)>,
        role: &'static str,
        frame: NSRect,
        label: Option<String>,
        description: Option<String>,
        value: Option<String>,
        action_target: Option<AutomationTarget>,
        logical_count: Option<usize>,
    }

    #[derive(Clone, Debug, Default, PartialEq)]
    struct NativeCallbackProjection {
        nodes: Vec<NativeCallbackNode>,
        root_token: Option<u64>,
    }

    struct NativeSemanticCallbackState {
        projection: NativeCallbackProjection,
        #[cfg(not(test))]
        proxy: EventLoopProxy<RuntimeUserEvent>,
        #[cfg(test)]
        proxy: Option<EventLoopProxy<RuntimeUserEvent>>,
        window_id: WindowId,
        generation: u64,
        view: Id,
        in_flight: Vec<NativeQueryKey>,
        deferred: Vec<NativeQueryKey>,
        pending_numeric_actions: usize,
        last_unavailable: Option<NativeSemanticUnavailableReason>,
    }

    impl NativeSemanticCallbackState {
        fn new(
            proxy: EventLoopProxy<RuntimeUserEvent>,
            window_id: WindowId,
            generation: u64,
            view: Id,
        ) -> Self {
            Self {
                projection: NativeCallbackProjection::default(),
                #[cfg(not(test))]
                proxy,
                #[cfg(test)]
                proxy: Some(proxy),
                window_id,
                generation,
                view,
                in_flight: Vec::new(),
                deferred: Vec::new(),
                pending_numeric_actions: 0,
                last_unavailable: None,
            }
        }

        #[cfg(test)]
        fn new_for_test(window_id: WindowId, generation: u64, view: Id) -> Self {
            Self {
                projection: NativeCallbackProjection::default(),
                proxy: None,
                window_id,
                generation,
                view,
                in_flight: Vec::new(),
                deferred: Vec::new(),
                pending_numeric_actions: 0,
                last_unavailable: None,
            }
        }

        fn node_for_object(&self, object: Id) -> Option<&NativeCallbackNode> {
            self.projection
                .nodes
                .iter()
                .find(|node| node.object == object)
        }

        fn node_for_token(&self, token: u64) -> Option<&NativeCallbackNode> {
            self.projection
                .nodes
                .iter()
                .find(|node| node.token == token)
        }

        fn request_range(&mut self, token: u64, start_index: usize, max_count: usize) {
            let max_count = max_count.min(MAX_NATIVE_ITEMS);
            if self.in_flight.len() >= MAX_NATIVE_ITEMS {
                return;
            }
            let key = NativeQueryKey {
                token,
                start_index,
                max_count,
            };
            let Some((logical_count, published_logical_children)) =
                self.published_item_children(token)
            else {
                return;
            };
            let Some(explicit_retry) = query_retry_mode(
                key,
                &self.in_flight,
                &self.deferred,
                logical_count,
                &published_logical_children,
            ) else {
                return;
            };
            if explicit_retry {
                let Some(index) = self.deferred.iter().position(|pending| *pending == key) else {
                    return;
                };
                self.deferred.swap_remove(index);
            }
            self.in_flight.push(key);
            let event = RuntimeUserEvent::NativeSemanticAccessibilityQuery {
                window_id: self.window_id,
                generation: self.generation,
                query: NativeSemanticAccessibilityQuery::ChildrenRange {
                    token,
                    start_index,
                    max_count,
                    explicit_retry,
                },
            };
            #[cfg(not(test))]
            let sent = self.proxy.send_event(event).is_ok();
            #[cfg(test)]
            let sent = self
                .proxy
                .as_ref()
                .is_some_and(|proxy| proxy.send_event(event).is_ok());
            if !sent {
                self.in_flight.retain(|pending| *pending != key);
            }
        }

        fn published_item_children(&self, token: u64) -> Option<(usize, Vec<(usize, u64)>)> {
            let node = self.node_for_token(token)?;
            if node.kind != NativeNodeKind::Container {
                return None;
            }
            Some((
                node.logical_count.unwrap_or(0),
                node.logical_children.clone(),
            ))
        }

        fn finish_query(&mut self, key: NativeQueryKey, deferred: bool) {
            self.in_flight.retain(|pending| *pending != key);
            if deferred && !self.deferred.contains(&key) && self.deferred.len() < MAX_NATIVE_ITEMS {
                self.deferred.push(key);
            }
        }

        fn enqueue_numeric_action(
            &mut self,
            token: u64,
            target: AutomationTarget,
            action: NativeNumericAccessibilityAction,
        ) -> bool {
            if self.pending_numeric_actions >= MAX_NATIVE_NUMERIC_ACTIONS {
                return false;
            }
            self.pending_numeric_actions = self.pending_numeric_actions.saturating_add(1);
            let event = RuntimeUserEvent::NativeNumericAccessibilityAction {
                window_id: self.window_id,
                generation: self.generation,
                token,
                target: Box::new(target),
                action,
            };
            #[cfg(not(test))]
            let sent = self.proxy.send_event(event).is_ok();
            #[cfg(test)]
            let sent = self
                .proxy
                .as_ref()
                .is_some_and(|proxy| proxy.send_event(event).is_ok());
            if !sent {
                self.pending_numeric_actions = self.pending_numeric_actions.saturating_sub(1);
            }
            sent
        }

        fn finish_numeric_action(&mut self) {
            self.pending_numeric_actions = self.pending_numeric_actions.saturating_sub(1);
        }
    }

    #[derive(Clone, Copy, Debug)]
    struct NativeCoordinateTransform {
        view: Id,
        window: Id,
        screen: Id,
        view_bounds: NSRect,
        scale_factor: f64,
        backing_scale: f64,
        flipped: bool,
        #[cfg(test)]
        deterministic_test: bool,
    }

    impl NativeCoordinateTransform {
        fn read(window: &Window) -> Option<Self> {
            let (native_window, view) = native_window_and_view(window)?;
            let view_bounds = unsafe { msg_rect(view, sel(c"bounds")) };
            let scale_factor = window.scale_factor();
            let screen = unsafe { msg_id(native_window, sel(c"screen")) };
            let backing_scale = if screen.is_null() {
                f64::NAN
            } else {
                unsafe { msg_f64(screen, sel(c"backingScaleFactor")) }
            };
            let flipped = unsafe { msg_bool(view, sel(c"isFlipped")) != NO };
            if screen.is_null()
                || !rect_is_finite(view_bounds)
                || view_bounds.size.width <= 0.0
                || view_bounds.size.height <= 0.0
                || !backing_scale.is_finite()
                || backing_scale <= 0.0
                || !scale_factor.is_finite()
                || scale_factor <= 0.0
            {
                return None;
            }
            Some(Self {
                view,
                window: native_window,
                screen,
                view_bounds,
                scale_factor,
                backing_scale,
                flipped,
                #[cfg(test)]
                deterministic_test: false,
            })
        }

        #[cfg(test)]
        fn for_test(view_bounds: NSRect) -> Self {
            Self {
                view: null_mut(),
                window: null_mut(),
                screen: null_mut(),
                view_bounds,
                scale_factor: 1.0,
                backing_scale: 1.0,
                flipped: true,
                deterministic_test: true,
            }
        }

        fn convert(&self, bounds: AutomationBounds) -> Option<NSRect> {
            #[cfg(test)]
            if self.deterministic_test {
                if !bounds_are_finite(bounds)
                    || bounds.width < 0.0
                    || bounds.height < 0.0
                    || bounds.x < 0.0
                    || bounds.y < 0.0
                    || f64::from(bounds.x) + f64::from(bounds.width) > self.view_bounds.size.width
                    || f64::from(bounds.y) + f64::from(bounds.height) > self.view_bounds.size.height
                {
                    return None;
                }
                return Some(NSRect {
                    origin: NSPoint {
                        x: self.view_bounds.origin.x + f64::from(bounds.x),
                        y: self.view_bounds.origin.y + f64::from(bounds.y),
                    },
                    size: NSSize {
                        width: f64::from(bounds.width),
                        height: f64::from(bounds.height),
                    },
                });
            }

            let current_window = unsafe { msg_id(self.view, sel(c"window")) };
            let current_screen = if current_window.is_null() {
                null_mut()
            } else {
                unsafe { msg_id(current_window, sel(c"screen")) }
            };
            let current_backing_scale = if current_screen.is_null() {
                f64::NAN
            } else {
                unsafe { msg_f64(current_screen, sel(c"backingScaleFactor")) }
            };
            let current_flipped = unsafe { msg_bool(self.view, sel(c"isFlipped")) != NO };
            if current_window != self.window
                || current_screen != self.screen
                || current_flipped != self.flipped
                || !current_backing_scale.is_finite()
                || current_backing_scale != self.backing_scale
                || !self.scale_factor.is_finite()
                || self.scale_factor <= 0.0
                || self.scale_factor != self.backing_scale
            {
                return None;
            }
            if !bounds_are_finite(bounds)
                || bounds.width < 0.0
                || bounds.height < 0.0
                || bounds.x < 0.0
                || bounds.y < 0.0
                || f64::from(bounds.x) + f64::from(bounds.width) > self.view_bounds.size.width
                || f64::from(bounds.y) + f64::from(bounds.height) > self.view_bounds.size.height
            {
                return None;
            }
            let logical_y = if self.flipped {
                f64::from(bounds.y)
            } else {
                self.view_bounds.size.height - f64::from(bounds.y) - f64::from(bounds.height)
            };
            let source = NSRect {
                origin: NSPoint {
                    x: self.view_bounds.origin.x + f64::from(bounds.x),
                    y: self.view_bounds.origin.y + logical_y,
                },
                size: NSSize {
                    width: f64::from(bounds.width),
                    height: f64::from(bounds.height),
                },
            };
            let mut screen = NSRect {
                origin: NSPoint {
                    x: f64::NAN,
                    y: f64::NAN,
                },
                size: NSSize {
                    width: f64::NAN,
                    height: f64::NAN,
                },
            };
            let converted = unsafe {
                radiant_native_convert_view_rect_to_screen(
                    self.view,
                    self.window,
                    &source,
                    &mut screen,
                )
            };
            validated_screen_rect(converted, screen, self.scale_factor)
        }
    }

    fn validated_screen_rect(
        converted: ObjcBool,
        screen: NSRect,
        scale_factor: f64,
    ) -> Option<NSRect> {
        if converted != YES
            || !rect_is_finite(screen)
            || screen.size.width < 0.0
            || screen.size.height < 0.0
            || !scale_factor.is_finite()
            || scale_factor <= 0.0
        {
            None
        } else {
            Some(screen)
        }
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct NativeContainerToken {
        token: u64,
        container_id: u64,
        mount_generation: u64,
        registration_generation: u64,
        provider_generation: u64,
        coordinate_authority: NativeSemanticCoordinateAuthority,
        cardinality: crate::application::virtual_layout::VirtualLayoutSemanticCardinality,
        lease: Option<SemanticAutomationSessionHandle>,
        window_generation: u64,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct NativeActiveRange {
        key: NativeQueryKey,
        container: NativeContainerToken,
        length: usize,
    }

    fn stage_active_range(
        existing: &[NativeActiveRange],
        candidate: NativeActiveRange,
    ) -> Option<Vec<NativeActiveRange>> {
        if candidate.length == 0 {
            return None;
        }
        let mut staged = existing.to_vec();
        if let Some(index) = staged
            .iter()
            .position(|active| active.container.container_id == candidate.container.container_id)
        {
            staged[index] = candidate;
        } else {
            staged.push(candidate);
        }

        let mut total = 0_usize;
        for active in &staged {
            total = total.checked_add(active.length)?;
            if total > MAX_NATIVE_ITEMS {
                return None;
            }
        }
        Some(staged)
    }

    fn active_range_matches_retry(
        active: &[NativeActiveRange],
        key: NativeQueryKey,
        current: &NativeContainerToken,
        normalized_length: usize,
    ) -> bool {
        active.iter().any(|range| {
            range.key == key && range.length == normalized_length && range.container.eq(current)
        })
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum NativeSemanticQueryAction {
        ExactRetry,
        RejectRetry,
        CompleteRefresh,
    }

    fn native_semantic_query_action(
        explicit_retry: bool,
        exact_active_retry: bool,
    ) -> NativeSemanticQueryAction {
        match (explicit_retry, exact_active_retry) {
            (true, true) => NativeSemanticQueryAction::ExactRetry,
            (true, false) => NativeSemanticQueryAction::RejectRetry,
            (false, _) => NativeSemanticQueryAction::CompleteRefresh,
        }
    }

    fn semantic_demands_for_active_ranges(
        active_ranges: &[NativeActiveRange],
        handles: &[SemanticAutomationContainerHandle],
    ) -> Option<Vec<SemanticAutomationDemand>> {
        active_ranges
            .iter()
            .map(|range| {
                let handle = handles.iter().find(|handle| {
                    handle.container_id == range.container.container_id
                        && handle.mount_generation == range.container.mount_generation
                })?;
                Some(SemanticAutomationDemand::range(
                    *handle,
                    range.key.start_index,
                    range.length,
                ))
            })
            .collect()
    }

    fn semantic_projection_is_eligible(
        refresh_status: SemanticAutomationRefreshStatus,
        selected_status: Option<SemanticAutomationRefreshStatus>,
    ) -> bool {
        let Some(selected_status) = selected_status else {
            return false;
        };
        match (refresh_status, selected_status) {
            (
                SemanticAutomationRefreshStatus::Published,
                SemanticAutomationRefreshStatus::Published,
            ) => true,
            (
                SemanticAutomationRefreshStatus::Retained { reason },
                SemanticAutomationRefreshStatus::Retained {
                    reason: selected_reason,
                },
            ) => reason == selected_reason,
            _ => false,
        }
    }

    #[derive(Clone, Debug, PartialEq)]
    struct NativeItemToken {
        token: u64,
        container_id: u64,
        mount_generation: u64,
        logical_index: usize,
        key: VirtualLayoutItemKey,
        coordinate_authority: NativeSemanticCoordinateAuthority,
        fences: crate::runtime::NormalizedSemanticPublicationFenceSet,
        lease: Option<SemanticAutomationSessionHandle>,
        window_generation: u64,
    }

    #[derive(Clone, Debug, PartialEq)]
    struct NativeOrdinaryToken {
        token: u64,
        id: AutomationNodeId,
        parent: Option<u64>,
        lease: Option<SemanticAutomationSessionHandle>,
        window_generation: u64,
    }

    #[derive(Clone, Debug, Default, PartialEq)]
    struct NativeTokenLedger {
        next: u64,
        root: Option<(u64, Option<SemanticAutomationSessionHandle>, u64)>,
        ordinary: Vec<NativeOrdinaryToken>,
        containers: Vec<NativeContainerToken>,
        items: Vec<NativeItemToken>,
    }

    impl NativeTokenLedger {
        fn issue(&mut self) -> Option<u64> {
            let token = self.next.checked_add(1)?;
            self.next = token;
            Some(token)
        }

        fn retire_all(&mut self) {
            self.root = None;
            self.ordinary.clear();
            self.containers.clear();
            self.items.clear();
        }
    }

    #[derive(Clone, Debug)]
    struct NativeNodeSpec {
        token: u64,
        kind: NativeNodeKind,
        parent: Option<u64>,
        children: Vec<u64>,
        logical_children: Vec<(usize, u64)>,
        bounds: AutomationBounds,
        semantics: AutomationNodeSemantics,
        action_target: Option<AutomationTarget>,
        logical_count: Option<usize>,
    }

    fn numeric_action_target_fence_matches(
        captured: &AutomationTarget,
        current: &AutomationTarget,
    ) -> bool {
        captured.id == current.id
            && captured.path == current.path
            && captured.role == current.role
            && captured.authority == current.authority
    }

    fn numeric_action_target_continuity_matches(
        previous: &AutomationTarget,
        current: &AutomationTarget,
    ) -> bool {
        previous.id == current.id
            && previous.path == current.path
            && previous.role == current.role
            && previous.enabled == current.enabled
            && previous.focusable == current.focusable
            && previous.available_actions == current.available_actions
            && previous
                .authority
                .zip(current.authority)
                .is_some_and(|(previous, current)| previous.materialized && current.materialized)
    }

    fn qualified_numeric_action_target(
        targets: &GuiAutomationTargetSnapshot,
        node: &AutomationNodeSnapshot,
        semantic_path: &[AutomationNodeId],
        kind: NativeNodeKind,
    ) -> Option<AutomationTarget> {
        if kind != NativeNodeKind::Ordinary
            || node.role != AutomationRole::TextInput
            || !node.enabled
            || node.semantics.disabled
            || node.semantics.read_only
            || !node.semantics.focusable
            || node.semantics.value_text.is_none()
        {
            return None;
        }

        let mut matches = targets.targets.iter().filter(|target| {
            target.id == node.id
                && target.path == semantic_path
                && target.role == AutomationRole::TextInput
                && target.enabled
                && target.focusable
                && target
                    .authority
                    .is_some_and(|authority| authority.materialized)
                && target.value.as_deref() == node.semantics.value_text.as_deref()
                && target
                    .available_actions
                    .iter()
                    .any(|action| action == AUTOMATION_ACTION_INCREMENT)
                && target
                    .available_actions
                    .iter()
                    .any(|action| action == AUTOMATION_ACTION_DECREMENT)
                && target
                    .available_actions
                    .iter()
                    .any(|action| action == AUTOMATION_ACTION_SET_TEXT)
        });
        let target = matches.next()?.clone();
        matches.next().is_none().then_some(target)
    }

    fn native_numeric_action_target(
        targets: &GuiAutomationTargetSnapshot,
        node: &AutomationNodeSnapshot,
        semantic_path: &[AutomationNodeId],
        kind: NativeNodeKind,
        provider_descendant: bool,
    ) -> Option<AutomationTarget> {
        (!provider_descendant)
            .then(|| qualified_numeric_action_target(targets, node, semantic_path, kind))
            .flatten()
    }

    fn native_numeric_action_name(
        action: &NativeNumericAccessibilityAction,
    ) -> Option<&'static CStr> {
        match action {
            NativeNumericAccessibilityAction::Increment => Some(NS_ACCESSIBILITY_INCREMENT_ACTION),
            NativeNumericAccessibilityAction::Decrement => Some(NS_ACCESSIBILITY_DECREMENT_ACTION),
            NativeNumericAccessibilityAction::SetValueText(_) => None,
        }
    }

    fn native_numeric_action_text(
        action: &NativeNumericAccessibilityAction,
    ) -> Option<&'static str> {
        native_numeric_action_name(action).and_then(|name| name.to_str().ok())
    }

    fn native_numeric_action_from_name(name: &CStr) -> Option<NativeNumericAccessibilityAction> {
        if name == NS_ACCESSIBILITY_INCREMENT_ACTION {
            Some(NativeNumericAccessibilityAction::Increment)
        } else if name == NS_ACCESSIBILITY_DECREMENT_ACTION {
            Some(NativeNumericAccessibilityAction::Decrement)
        } else {
            None
        }
    }

    fn numeric_action_names() -> [NativeNumericAccessibilityAction; 2] {
        [
            NativeNumericAccessibilityAction::Increment,
            NativeNumericAccessibilityAction::Decrement,
        ]
    }

    #[derive(Clone, Debug, PartialEq)]
    struct StableValueProjectionUpdate {
        index: usize,
        value: Option<String>,
        action_target: Option<AutomationTarget>,
        value_changed: bool,
    }

    fn collect_stable_value_projection_updates(
        projection: &NativeCallbackProjection,
        specs: &[NativeNodeSpec],
        frames: &[NSRect],
    ) -> Option<Vec<StableValueProjectionUpdate>> {
        if specs.len() != projection.nodes.len() || specs.len() != frames.len() {
            return None;
        }
        let mut updates = Vec::new();
        for (index, ((spec, node), frame)) in
            specs.iter().zip(&projection.nodes).zip(frames).enumerate()
        {
            let stable_native_evidence = node.token == spec.token
                && node.kind == spec.kind
                && node.parent == spec.parent
                && node.children == spec.children
                && node.logical_children == spec.logical_children
                && node.frame == *frame
                && node.role
                    == native_role(spec.kind, spec.semantics.role, spec.action_target.is_some())
                && node.label == spec.semantics.label
                && node.description == spec.semantics.description
                && node.logical_count == spec.logical_count;
            if !stable_native_evidence {
                return None;
            }

            let target_continuity_matches = match (&node.action_target, &spec.action_target) {
                (None, None) => true,
                (Some(previous), Some(current)) => {
                    numeric_action_target_continuity_matches(previous, current)
                }
                _ => false,
            };
            if !target_continuity_matches {
                return None;
            }
            let value_changed = node.value != spec.semantics.value_text;
            if value_changed && spec.action_target.is_none() {
                return None;
            }
            if value_changed || node.action_target != spec.action_target {
                updates.push(StableValueProjectionUpdate {
                    index,
                    value: spec.semantics.value_text.clone(),
                    action_target: spec.action_target.clone(),
                    value_changed,
                });
            }
        }
        Some(updates)
    }

    fn apply_stable_value_projection_updates(
        projection: &mut NativeCallbackProjection,
        updates: &[StableValueProjectionUpdate],
    ) -> Option<Vec<Id>> {
        if updates
            .iter()
            .any(|update| update.index >= projection.nodes.len())
        {
            return None;
        }
        let mut value_notifications = Vec::new();
        for update in updates {
            let node = projection.nodes.get_mut(update.index)?;
            node.value = update.value.clone();
            node.action_target = update.action_target.clone();
            if update.value_changed {
                value_notifications.push(node.object);
            }
        }
        Some(value_notifications)
    }

    /// One owned primary-window native semantic adapter.  It is intentionally
    /// not `Send`/`Sync`; its Objective-C objects, lease, and callback state
    /// are owned by the primary event-loop thread.
    pub struct NativeSemanticAccessibilityAdapter {
        view: Id,
        callback_state: Box<RefCell<NativeSemanticCallbackState>>,
        objects: Vec<Id>,
        // Only objects in this list have crossed the exact host-attachment
        // commit point.  Allocated objects outside it are pre-commit and
        // must never receive an AXUIElementDestroyed notification.
        committed_objects: Vec<Id>,
        tokens: NativeTokenLedger,
        lease: Option<SemanticAutomationSessionHandle>,
        generation: u64,
        window_generation: u64,
        transform: Option<NativeCoordinateTransform>,
        current_containers: Vec<NativeSemanticContainerSnapshot>,
        active_ranges: Vec<NativeActiveRange>,
        attached: bool,
        #[cfg(test)]
        layout_notifications: usize,
        #[cfg(test)]
        value_notifications: usize,
        #[cfg(test)]
        destroyed_notifications: usize,
    }

    impl NativeSemanticAccessibilityAdapter {
        pub(crate) fn attach(
            window: &Window,
            proxy: EventLoopProxy<RuntimeUserEvent>,
        ) -> Result<Self, String> {
            let (_, view) = native_window_and_view(window)
                .ok_or_else(|| String::from("primary AppKit window/content view unavailable"))?;
            native_class()?;
            let generation = 1;
            let callback_state = Box::new(RefCell::new(NativeSemanticCallbackState::new(
                proxy,
                window.id(),
                generation,
                view,
            )));
            Ok(Self {
                view,
                callback_state,
                objects: Vec::new(),
                committed_objects: Vec::new(),
                tokens: NativeTokenLedger::default(),
                lease: None,
                generation,
                window_generation: 1,
                transform: NativeCoordinateTransform::read(window),
                current_containers: Vec::new(),
                active_ranges: Vec::new(),
                attached: false,
                #[cfg(test)]
                layout_notifications: 0,
                #[cfg(test)]
                value_notifications: 0,
                #[cfg(test)]
                destroyed_notifications: 0,
            })
        }

        pub(crate) fn publish_passive<Bridge, Message>(
            &mut self,
            runtime: &SurfaceRuntime<Bridge, Message>,
        ) -> Result<(), String>
        where
            Bridge: RuntimeBridge<Message>,
        {
            let ordinary = runtime.automation_snapshot();
            let targets = runtime.automation_target_snapshot();
            let containers = runtime.native_semantic_containers();
            let selected = self.lease.and_then(|session| {
                match runtime.native_semantic_automation_composition(session) {
                    Ok(selected) => selected,
                    Err(error) => {
                        self.handle_session_error(error);
                        None
                    }
                }
            });
            self.publish_projection(&ordinary, &targets, &containers, selected)
        }

        fn publish_ordinary_projection<Bridge, Message>(
            &mut self,
            runtime: &SurfaceRuntime<Bridge, Message>,
        ) -> Result<(), String>
        where
            Bridge: RuntimeBridge<Message>,
        {
            let ordinary = runtime.automation_snapshot();
            let targets = runtime.automation_target_snapshot();
            let containers = runtime.native_semantic_containers();
            self.publish_projection(&ordinary, &targets, &containers, None)
        }

        pub(crate) fn accepts_generation(&self, generation: u64) -> bool {
            self.generation == generation
        }

        pub(crate) const fn is_attached(&self) -> bool {
            self.attached
        }

        pub(crate) fn numeric_accessibility_request(
            &self,
            token: u64,
            target: AutomationTarget,
            action: NativeNumericAccessibilityAction,
        ) -> Option<NumericAccessibilityRequest> {
            let state = self.callback_state.try_borrow().ok()?;
            let node = state.node_for_token(token)?;
            let current = node.action_target.as_ref()?;
            if !target
                .authority
                .as_ref()
                .is_some_and(|authority| authority.materialized)
                || !numeric_action_target_fence_matches(&target, current)
            {
                return None;
            }
            let action = match action {
                NativeNumericAccessibilityAction::Increment => {
                    NumericAccessibilityAction::Increment
                }
                NativeNumericAccessibilityAction::Decrement => {
                    NumericAccessibilityAction::Decrement
                }
                NativeNumericAccessibilityAction::SetValueText(text) => {
                    NumericAccessibilityAction::SetValueText(text)
                }
            };
            Some(NumericAccessibilityRequest::new(target, action))
        }

        pub(crate) fn finish_numeric_action(&mut self) {
            if let Ok(mut state) = self.callback_state.try_borrow_mut() {
                state.finish_numeric_action();
            }
        }

        pub(crate) fn handle_query<Bridge, Message>(
            &mut self,
            runtime: &mut SurfaceRuntime<Bridge, Message>,
            query: NativeSemanticAccessibilityQuery,
        ) where
            Bridge: RuntimeBridge<Message>,
        {
            let (key, explicit_retry) = match query {
                NativeSemanticAccessibilityQuery::ChildrenRange {
                    token,
                    start_index,
                    max_count,
                    explicit_retry,
                } => (
                    NativeQueryKey {
                        token,
                        start_index,
                        max_count: max_count.min(MAX_NATIVE_ITEMS),
                    },
                    explicit_retry,
                ),
            };
            if key.max_count == 0 {
                return;
            }

            let Some(container) = self.container_for_token(key.token) else {
                self.finish_query(key, false);
                return;
            };
            let Some(current_token) = self.current_container_token(key.token) else {
                self.finish_query(key, false);
                return;
            };
            let Some(length) = normalize_range(
                container.cardinality.logical_item_count,
                container.max_entries,
                key.start_index,
                key.max_count,
                0,
            ) else {
                self.finish_query(key, false);
                return;
            };
            if !native_range_query_is_admitted(
                container.cardinality,
                container.has_range_provider,
                Some(length),
            ) {
                self.finish_query(key, false);
                return;
            }

            let mut prior_ranges = self.active_ranges.clone();
            prior_ranges.retain(|active| {
                self.tokens
                    .containers
                    .iter()
                    .any(|current| current == &active.container)
            });
            let action = native_semantic_query_action(
                explicit_retry,
                active_range_matches_retry(&prior_ranges, key, &current_token, length),
            );
            if action == NativeSemanticQueryAction::RejectRetry {
                self.finish_query(key, false);
                return;
            }

            let session = match self.lease {
                Some(session) => session,
                None => match runtime.open_semantic_automation_session() {
                    Ok(session) => {
                        self.lease = Some(session);
                        session
                    }
                    Err(SemanticAutomationSessionError::SessionAlreadyActive) => {
                        self.callback_state.borrow_mut().last_unavailable =
                            Some(NativeSemanticUnavailableReason::SessionContended);
                        self.finish_query(key, false);
                        return;
                    }
                    Err(_) => {
                        self.callback_state.borrow_mut().last_unavailable =
                            Some(NativeSemanticUnavailableReason::RuntimeRejected);
                        self.finish_query(key, false);
                        return;
                    }
                },
            };
            let handles = match runtime.semantic_automation_containers(session) {
                Ok(handles) => handles,
                Err(error) => {
                    self.handle_session_error(error);
                    let _ = self.publish_ordinary_projection(runtime);
                    self.finish_query(key, false);
                    return;
                }
            };
            let (staged_ranges, refresh) = match action {
                NativeSemanticQueryAction::ExactRetry => {
                    let Some(handle) = handles
                        .iter()
                        .find(|handle| {
                            handle.container_id == current_token.container_id
                                && handle.mount_generation == current_token.mount_generation
                        })
                        .copied()
                    else {
                        self.handle_session_error(
                            SemanticAutomationSessionError::StaleContainerHandle,
                        );
                        let _ = self.publish_ordinary_projection(runtime);
                        self.finish_query(key, false);
                        return;
                    };
                    (
                        prior_ranges.clone(),
                        runtime.retry_semantic_automation_range(
                            session,
                            handle,
                            key.start_index,
                            length,
                        ),
                    )
                }
                NativeSemanticQueryAction::CompleteRefresh => {
                    let candidate = NativeActiveRange {
                        key,
                        container: current_token.clone(),
                        length,
                    };
                    let Some(staged_ranges) = stage_active_range(&prior_ranges, candidate) else {
                        self.finish_query(key, false);
                        return;
                    };
                    let Some(demands) =
                        semantic_demands_for_active_ranges(&staged_ranges, &handles)
                    else {
                        self.finish_query(key, false);
                        return;
                    };
                    (
                        staged_ranges,
                        runtime.refresh_semantic_automation_session(session, &demands),
                    )
                }
                NativeSemanticQueryAction::RejectRetry => unreachable!(),
            };
            let refresh = match refresh {
                Ok(refresh) => {
                    self.active_ranges = staged_ranges;
                    refresh
                }
                Err(error) => {
                    self.handle_session_error(error);
                    let _ = self.publish_ordinary_projection(runtime);
                    self.finish_query(key, false);
                    return;
                }
            };
            let status = refresh.status;
            let projection = match runtime.native_semantic_automation_composition(session) {
                Ok(projection) => projection,
                Err(error) => {
                    self.handle_session_error(error);
                    None
                }
            };
            let projection = projection.filter(|(_, selected_status)| {
                semantic_projection_is_eligible(status, Some(*selected_status))
            });
            let ordinary = runtime.automation_snapshot();
            let targets = runtime.automation_target_snapshot();
            let containers = runtime.native_semantic_containers();
            let _ = self.publish_projection(&ordinary, &targets, &containers, projection);
            let deferred = matches!(
                status,
                SemanticAutomationRefreshStatus::Baseline {
                    reason: SemanticAutomationFallbackReason::Deferred
                }
            ) || matches!(
                status,
                SemanticAutomationRefreshStatus::Retained {
                    reason: SemanticAutomationFallbackReason::Deferred
                }
            );
            self.finish_query(key, deferred);
        }

        pub(crate) fn invalidate_window_generation(&mut self, window: &Window) {
            self.window_generation = self.window_generation.saturating_add(1);
            self.transform = NativeCoordinateTransform::read(window);
            self.retire_published_objects();
            self.tokens.retire_all();
            self.active_ranges.clear();
        }

        pub(crate) fn close_lease<Bridge, Message>(
            &mut self,
            runtime: &mut SurfaceRuntime<Bridge, Message>,
        ) where
            Bridge: RuntimeBridge<Message>,
        {
            if let Some(session) = self.lease.take() {
                let _ = runtime.close_semantic_automation_session(session);
            }
            self.retire_published_objects();
            self.tokens.retire_all();
            self.active_ranges.clear();
        }

        pub(crate) fn retire(&mut self) {
            self.retire_published_objects();
            self.tokens.retire_all();
            self.active_ranges.clear();
            self.attached = false;
        }

        fn finish_query(&mut self, key: NativeQueryKey, deferred: bool) {
            if let Ok(mut state) = self.callback_state.try_borrow_mut() {
                state.finish_query(key, deferred);
            }
        }

        fn handle_session_error(&mut self, error: SemanticAutomationSessionError) {
            let reason = match error {
                SemanticAutomationSessionError::SessionAlreadyActive => {
                    NativeSemanticUnavailableReason::SessionContended
                }
                SemanticAutomationSessionError::UnknownSession
                | SemanticAutomationSessionError::StaleContainerHandle => {
                    self.lease = None;
                    self.current_containers.clear();
                    self.tokens.retire_all();
                    self.active_ranges.clear();
                    self.retire_published_objects();
                    NativeSemanticUnavailableReason::Stale
                }
                _ => NativeSemanticUnavailableReason::RuntimeRejected,
            };
            self.callback_state.borrow_mut().last_unavailable = Some(reason);
        }

        fn container_for_token(&self, token: u64) -> Option<NativeContainerTokenView> {
            let state = self.callback_state.try_borrow().ok()?;
            let node = state.node_for_token(token)?;
            if node.kind != NativeNodeKind::Container {
                return None;
            }
            self.tokens
                .containers
                .iter()
                .find(|container| container.token == token)
                .map(|container| NativeContainerTokenView {
                    cardinality: container.cardinality,
                    max_entries: self
                        .current_containers
                        .iter()
                        .find(|current| current.container_id == container.container_id)
                        .map_or(MAX_NATIVE_ITEMS, |current| current.max_entries),
                    has_range_provider: self
                        .current_containers
                        .iter()
                        .find(|current| current.container_id == container.container_id)
                        .is_some_and(|current| current.has_range_provider),
                })
        }

        fn current_container_token(&self, token: u64) -> Option<NativeContainerToken> {
            let state = self.callback_state.try_borrow().ok()?;
            let node = state.node_for_token(token)?;
            if node.kind != NativeNodeKind::Container {
                return None;
            }
            self.tokens
                .containers
                .iter()
                .find(|container| container.token == token)
                .cloned()
        }

        fn publish_projection(
            &mut self,
            ordinary: &GuiAutomationSnapshot,
            targets: &GuiAutomationTargetSnapshot,
            containers: &[NativeSemanticContainerSnapshot],
            selected: Option<(
                crate::runtime::VirtualLayoutAutomationComposition,
                SemanticAutomationRefreshStatus,
            )>,
        ) -> Result<(), String> {
            let composition = selected.as_ref().map(|(composition, _)| composition);
            let snapshot = composition.map_or(ordinary, |composition| composition.snapshot());
            let sidecar = composition.map(|composition| composition.normalized_sidecar());
            let specs = match self.build_specs(snapshot, targets, containers, sidecar) {
                Ok(specs) => specs,
                Err(error) => {
                    self.tokens.retire_all();
                    self.retire_published_objects();
                    self.current_containers.clear();
                    self.active_ranges.clear();
                    self.attached = false;
                    return Err(error);
                }
            };
            self.prune_token_ledger(&specs);
            self.reconcile_active_ranges_with_tokens();
            if self.attached && self.specs_match_projection(&specs) {
                self.current_containers = containers.to_vec();
                if let Ok(mut state) = self.callback_state.try_borrow_mut() {
                    state.last_unavailable = None;
                }
                return Ok(());
            }
            if self.attached
                && let Some(value_notifications) = self.update_stable_value_projection(&specs)
            {
                self.current_containers = containers.to_vec();
                for object in value_notifications {
                    self.post_value_changed(object);
                }
                return Ok(());
            }
            let had_objects = !self.objects.is_empty();
            if !self.retire_published_objects() && had_objects {
                self.tokens.retire_all();
                self.current_containers.clear();
                self.active_ranges.clear();
                self.attached = false;
                return Err(String::from(
                    "native semantic accessibility host children clear could not be verified",
                ));
            }
            let object_start = self.objects.len();
            let projection = match self.instantiate_specs(specs) {
                Ok(projection) => projection,
                Err(error) => {
                    self.tokens.retire_all();
                    self.retire_published_objects();
                    self.current_containers.clear();
                    self.active_ranges.clear();
                    self.attached = false;
                    return Err(error);
                }
            };
            let changed = match self.replace_callback_projection(projection) {
                Ok(changed) => changed,
                Err(error) => {
                    self.tokens.retire_all();
                    self.current_containers.clear();
                    self.active_ranges.clear();
                    self.attached = false;
                    return Err(error);
                }
            };
            self.committed_objects
                .extend(self.objects[object_start..].iter().copied());
            self.current_containers = containers.to_vec();
            self.attached = true;
            if changed {
                self.post_layout_changed();
            }
            Ok(())
        }

        fn build_specs(
            &mut self,
            snapshot: &GuiAutomationSnapshot,
            targets: &GuiAutomationTargetSnapshot,
            containers: &[NativeSemanticContainerSnapshot],
            sidecar: Option<&crate::runtime::VirtualLayoutNormalizedSemanticSidecar>,
        ) -> Result<Vec<NativeNodeSpec>, String> {
            if containers.len() > MAX_NATIVE_REGISTRATIONS {
                return Err(String::from("native semantic registration cap exceeded"));
            }
            let root_token = self.retain_or_issue_root_token()?;
            let root_bounds = self.transform.map_or(
                AutomationBounds {
                    x: 0.0,
                    y: 0.0,
                    width: snapshot.viewport_width as f32,
                    height: snapshot.viewport_height as f32,
                },
                |transform| AutomationBounds {
                    x: 0.0,
                    y: 0.0,
                    width: transform.view_bounds.size.width as f32,
                    height: transform.view_bounds.size.height as f32,
                },
            );
            let mut specs = Vec::new();
            specs.push(NativeNodeSpec {
                token: root_token,
                kind: NativeNodeKind::Root,
                parent: None,
                children: Vec::new(),
                logical_children: Vec::new(),
                bounds: root_bounds,
                semantics: AutomationNodeSemantics::new(AutomationRole::Root),
                action_target: None,
                logical_count: None,
            });
            let mut accepted = containers.to_vec();
            accepted.sort_by_key(|container| container.container_id);
            if let Some(sidecar) = sidecar {
                for entry in sidecar.entries() {
                    let Some(container) = accepted
                        .iter()
                        .find(|container| container.container_id == entry.container_id())
                    else {
                        return Err(String::from(
                            "native semantic sidecar referenced an unadmitted container",
                        ));
                    };
                    if !entry.matches_native_coordinate_authority(&container.coordinate_authority) {
                        return Err(String::from(
                            "native semantic sidecar coordinate authority mismatch",
                        ));
                    }
                }
            }
            let mut anchor_ids = Vec::with_capacity(accepted.len());
            for container in &accepted {
                anchor_ids.push(AutomationNodeId::new(container.container_id.to_string()));
            }
            let mut item_paths = Vec::new();
            if let Some(sidecar) = sidecar {
                for entry in sidecar.entries() {
                    item_paths.push((entry.container_id(), entry.normalized_path().to_vec()));
                }
            }
            let mut child_tokens = Vec::new();
            for (index, child) in snapshot.root.children.iter().enumerate() {
                let mut path = vec![index];
                let mut semantic_path = vec![snapshot.root.id.clone(), child.id.clone()];
                let token = self.build_snapshot_node(
                    snapshot,
                    child,
                    Some(root_token),
                    &mut path,
                    &mut semantic_path,
                    false,
                    targets,
                    &accepted,
                    &anchor_ids,
                    sidecar,
                    &item_paths,
                    &mut specs,
                )?;
                child_tokens.push(token);
            }
            specs[0].children = child_tokens;
            Ok(specs)
        }

        #[allow(clippy::too_many_arguments)]
        fn build_snapshot_node(
            &mut self,
            snapshot: &GuiAutomationSnapshot,
            node: &AutomationNodeSnapshot,
            parent: Option<u64>,
            path: &mut Vec<usize>,
            semantic_path: &mut Vec<AutomationNodeId>,
            provider_descendant: bool,
            targets: &GuiAutomationTargetSnapshot,
            containers: &[NativeSemanticContainerSnapshot],
            anchor_ids: &[AutomationNodeId],
            sidecar: Option<&crate::runtime::VirtualLayoutNormalizedSemanticSidecar>,
            item_paths: &[(u64, Vec<usize>)],
            specs: &mut Vec<NativeNodeSpec>,
        ) -> Result<u64, String> {
            let container = anchor_ids
                .iter()
                .position(|anchor| anchor == &node.id)
                .and_then(|index| containers.get(index));
            let token = if let Some(container) = container {
                self.retain_or_issue_container_token(container)?
            } else {
                self.retain_or_issue_ordinary_token(&node.id, parent, specs)?
            };
            let kind = container.map_or(NativeNodeKind::Ordinary, |_| NativeNodeKind::Container);
            let action_target = native_numeric_action_target(
                targets,
                node,
                semantic_path,
                kind,
                provider_descendant,
            );
            let spec_index = specs.len();
            specs.push(NativeNodeSpec {
                token,
                kind,
                parent,
                children: Vec::new(),
                logical_children: Vec::new(),
                bounds: node.bounds,
                semantics: node.semantics.clone(),
                action_target,
                logical_count: container.map(|container| container.cardinality.logical_item_count),
            });
            let mut children = Vec::new();
            let mut logical_children = Vec::new();
            if let Some(container) = container {
                let mut virtual_children = Vec::new();
                if let Some(sidecar) = sidecar {
                    let mut entries = sidecar
                        .entries()
                        .iter()
                        .filter(|entry| entry.container_id() == container.container_id)
                        .collect::<Vec<_>>();
                    entries.sort_by_key(|entry| entry.logical_index());
                    for entry in entries {
                        if specs
                            .iter()
                            .filter(|spec| spec.kind == NativeNodeKind::Item)
                            .count()
                            >= MAX_NATIVE_ITEMS
                        {
                            return Err(String::from("native semantic item cap exceeded"));
                        }
                        let Some(item_node) = node_at_path(&snapshot.root, entry.normalized_path())
                        else {
                            return Err(String::from("native sidecar path was not present"));
                        };
                        let item_token =
                            self.retain_or_issue_item_token(container, entry, specs)?;
                        let item_index = specs.len();
                        specs.push(NativeNodeSpec {
                            token: item_token,
                            kind: NativeNodeKind::Item,
                            parent: Some(token),
                            children: Vec::new(),
                            logical_children: Vec::new(),
                            bounds: item_node.bounds,
                            semantics: item_node.semantics.clone(),
                            action_target: None,
                            logical_count: None,
                        });
                        let mut nested = Vec::new();
                        let mut item_path = entry.normalized_path().to_vec();
                        let mut item_semantic_path = semantic_path.clone();
                        item_semantic_path.push(item_node.id.clone());
                        for (child_index, child) in item_node.children.iter().enumerate() {
                            item_path.push(child_index);
                            item_semantic_path.push(child.id.clone());
                            let child_token = self.build_snapshot_node(
                                snapshot,
                                child,
                                Some(item_token),
                                &mut item_path,
                                &mut item_semantic_path,
                                true,
                                targets,
                                containers,
                                anchor_ids,
                                Some(sidecar),
                                item_paths,
                                specs,
                            )?;
                            item_semantic_path.pop();
                            item_path.pop();
                            nested.push(child_token);
                        }
                        specs[item_index].children = nested;
                        virtual_children.push(item_token);
                        logical_children.push((entry.logical_index(), item_token));
                    }
                }
                children.extend(virtual_children);
                for (child_index, child) in node.children.iter().enumerate() {
                    path.push(child_index);
                    semantic_path.push(child.id.clone());
                    let is_sidecar_path = item_paths.iter().any(|(_, item_path)| {
                        item_path.len() >= path.len() && item_path.starts_with(path)
                    });
                    if !is_sidecar_path {
                        let child_token = self.build_snapshot_node(
                            snapshot,
                            child,
                            Some(token),
                            path,
                            semantic_path,
                            provider_descendant,
                            targets,
                            containers,
                            anchor_ids,
                            sidecar,
                            item_paths,
                            specs,
                        )?;
                        children.push(child_token);
                    }
                    semantic_path.pop();
                    path.pop();
                }
            } else {
                for (child_index, child) in node.children.iter().enumerate() {
                    path.push(child_index);
                    semantic_path.push(child.id.clone());
                    let child_token = self.build_snapshot_node(
                        snapshot,
                        child,
                        Some(token),
                        path,
                        semantic_path,
                        provider_descendant,
                        targets,
                        containers,
                        anchor_ids,
                        sidecar,
                        item_paths,
                        specs,
                    )?;
                    semantic_path.pop();
                    path.pop();
                    children.push(child_token);
                }
            }
            specs[spec_index].children = children;
            specs[spec_index].logical_children = logical_children;
            Ok(token)
        }

        fn update_stable_value_projection(&mut self, specs: &[NativeNodeSpec]) -> Option<Vec<Id>> {
            let transform = self.transform?;
            let state = self.callback_state.try_borrow().ok()?;
            let frames = specs
                .iter()
                .map(|spec| transform.convert(spec.bounds))
                .collect::<Option<Vec<_>>>()?;
            let updates =
                collect_stable_value_projection_updates(&state.projection, specs, &frames)?;
            drop(state);

            if updates.is_empty() {
                return None;
            }
            let mut state = self.callback_state.try_borrow_mut().ok()?;
            apply_stable_value_projection_updates(&mut state.projection, &updates)
        }

        fn retain_or_issue_ordinary_token(
            &mut self,
            id: &AutomationNodeId,
            parent: Option<u64>,
            specs: &[NativeNodeSpec],
        ) -> Result<u64, String> {
            if let Some(previous) = self.tokens.ordinary.iter().find(|previous| {
                previous.id == *id
                    && previous.parent == parent
                    && previous.lease == self.lease
                    && previous.window_generation == self.window_generation
                    && !specs.iter().any(|spec| spec.token == previous.token)
            }) {
                return Ok(previous.token);
            }
            let token = self
                .tokens
                .issue()
                .ok_or_else(|| String::from("native semantic token space exhausted"))?;
            self.tokens.ordinary.push(NativeOrdinaryToken {
                token,
                id: id.clone(),
                parent,
                lease: self.lease,
                window_generation: self.window_generation,
            });
            Ok(token)
        }

        fn specs_match_projection(&self, specs: &[NativeNodeSpec]) -> bool {
            let Some(transform) = self.transform else {
                return false;
            };
            let Ok(state) = self.callback_state.try_borrow() else {
                return false;
            };
            specs.len() == state.projection.nodes.len()
                && specs
                    .iter()
                    .zip(&state.projection.nodes)
                    .all(|(spec, node)| {
                        transform.convert(spec.bounds).is_some_and(|frame| {
                            node.token == spec.token
                                && node.kind == spec.kind
                                && node.parent == spec.parent
                                && node.children == spec.children
                                && node.logical_children == spec.logical_children
                                && node.frame == frame
                                && node.role
                                    == native_role(
                                        spec.kind,
                                        spec.semantics.role,
                                        spec.action_target.is_some(),
                                    )
                                && node.label == spec.semantics.label
                                && node.description == spec.semantics.description
                                && node.value == spec.semantics.value_text
                                && node.action_target == spec.action_target
                                && node.logical_count == spec.logical_count
                        })
                    })
        }

        fn instantiate_specs(
            &mut self,
            specs: Vec<NativeNodeSpec>,
        ) -> Result<NativeCallbackProjection, String> {
            let class = native_class()?.class();
            let state_ptr =
                (&*self.callback_state as *const RefCell<NativeSemanticCallbackState>) as Id;
            let mut object_by_token = Vec::with_capacity(specs.len());
            for spec in &specs {
                let allocated = unsafe { msg_id(class, sel(c"alloc")) };
                if allocated.is_null() {
                    return Err(String::from(
                        "native accessibility element allocation failed",
                    ));
                }
                let object = unsafe { msg_id(allocated, sel(c"init")) };
                if object.is_null() {
                    return Err(String::from(
                        "native accessibility element initialization failed",
                    ));
                }
                unsafe { object_setIvar(object, native_class()?.state_ivar(), state_ptr) };
                object_by_token.push((spec.token, object));
                self.objects.push(object);
            }
            let mut nodes = Vec::with_capacity(specs.len());
            for spec in specs {
                let object = object_by_token
                    .iter()
                    .find(|(token, _)| *token == spec.token)
                    .map(|(_, object)| *object)
                    .ok_or_else(|| String::from("native token/object mapping failed"))?;
                let frame = self
                    .transform
                    .and_then(|transform| transform.convert(spec.bounds))
                    .ok_or_else(|| {
                        String::from("logical-to-AppKit frame conversion unavailable")
                    })?;
                nodes.push(NativeCallbackNode {
                    object,
                    token: spec.token,
                    kind: spec.kind,
                    parent: spec.parent,
                    children: spec.children,
                    logical_children: spec.logical_children,
                    role: native_role(spec.kind, spec.semantics.role, spec.action_target.is_some()),
                    frame,
                    label: spec.semantics.label,
                    description: spec.semantics.description,
                    value: spec.semantics.value_text,
                    action_target: spec.action_target,
                    logical_count: spec.logical_count,
                });
            }
            let root_token = nodes.first().map(|node| node.token);
            let projection = NativeCallbackProjection { nodes, root_token };
            self.configure_modern_projection(&projection)?;
            Ok(projection)
        }

        fn configure_modern_projection(
            &self,
            projection: &NativeCallbackProjection,
        ) -> Result<(), String> {
            for node in &projection.nodes {
                let parent = match node.parent {
                    Some(parent_token) => projection
                        .nodes
                        .iter()
                        .find(|candidate| candidate.token == parent_token)
                        .map_or(null_mut(), |candidate| candidate.object),
                    None if projection.root_token == Some(node.token) => self.view,
                    None => null_mut(),
                };
                if parent.is_null() {
                    return Err(String::from(
                        "native semantic modern projection parent was unavailable",
                    ));
                }

                let mut child_objects = Vec::with_capacity(node.children.len());
                for child_token in &node.children {
                    let Some(child) = projection
                        .nodes
                        .iter()
                        .find(|candidate| candidate.token == *child_token)
                        .map(|candidate| candidate.object)
                    else {
                        return Err(String::from(
                            "native semantic modern projection child was unavailable",
                        ));
                    };
                    if child.is_null() {
                        return Err(String::from(
                            "native semantic modern projection child object was nil",
                        ));
                    }
                    child_objects.push(child);
                }

                let role = unsafe { ns_string(node.role) };
                let children = unsafe { ns_array(&child_objects) };
                let label = node
                    .label
                    .as_deref()
                    .map_or(null_mut(), |value| unsafe { ns_string(value) });
                let title = node
                    .label
                    .as_deref()
                    .map_or(null_mut(), |value| unsafe { ns_string(value) });
                let help = node
                    .description
                    .as_deref()
                    .map_or(null_mut(), |value| unsafe { ns_string(value) });
                if role.is_null() || children.is_null() {
                    return Err(String::from(
                        "native semantic modern projection property allocation failed",
                    ));
                }
                let has_enabled = if node.action_target.is_some() {
                    YES
                } else {
                    NO
                };
                let enabled = if node
                    .action_target
                    .as_ref()
                    .is_some_and(|target| target.enabled)
                {
                    YES
                } else {
                    NO
                };
                let configured = unsafe {
                    radiant_native_configure_accessibility_element(
                        node.object,
                        role,
                        parent,
                        children,
                        &node.frame,
                        label,
                        title,
                        help,
                        has_enabled,
                        enabled,
                    )
                };
                if configured != YES {
                    return Err(String::from(
                        "native semantic modern accessibility element configuration failed",
                    ));
                }
            }
            Ok(())
        }

        fn replace_callback_projection(
            &mut self,
            projection: NativeCallbackProjection,
        ) -> Result<bool, String> {
            let changed = self
                .callback_state
                .try_borrow()
                .ok()
                .map(|state| callback_projection_changed(&state.projection, &projection));
            let Some(changed) = changed else {
                self.retire_published_objects();
                return Err(String::from(
                    "native semantic callback state is borrowed during publication",
                ));
            };
            let Ok(mut state) = self.callback_state.try_borrow_mut() else {
                self.retire_published_objects();
                return Err(String::from(
                    "native semantic callback state cannot be published",
                ));
            };
            state.projection = projection.clone();
            state.last_unavailable = None;
            drop(state);
            let Some(root) = projection
                .root_token
                .and_then(|token| projection.nodes.iter().find(|node| node.token == token))
                .map(|node| node.object)
            else {
                self.retire_published_objects();
                return Err(String::from(
                    "native semantic projection has no root object",
                ));
            };
            if root.is_null() {
                self.retire_published_objects();
                return Err(String::from(
                    "native semantic projection root object is nil",
                ));
            }
            let installed =
                unsafe { radiant_native_set_accessibility_children(self.view, root) == YES };
            if !installed {
                self.retire_published_objects();
                return Err(String::from(
                    "native semantic accessibility host rejected accessibilityChildren",
                ));
            }
            Ok(changed)
        }

        fn retire_published_objects(&mut self) -> bool {
            self.advance_generation();
            self.attached = false;
            self.clear_callback_state();
            let class = native_class().ok();
            if let Some(class) = class {
                // Clear the non-retained callback pointer before any AppKit
                // operation can synchronously observe a retired object.
                for object in &self.objects {
                    unsafe { object_setIvar(*object, class.state_ivar(), null_mut()) };
                }
            }
            let Some(class) = class else {
                // The state has already been emptied where possible. Retain
                // objects if Objective-C cannot expose their ivar, so no
                // object is released while it may still point at live state.
                return false;
            };
            let host_clear_verified = self.view.is_null()
                || unsafe { radiant_native_clear_accessibility_children(self.view) == YES };
            if !host_clear_verified {
                // Keep every object quarantined when the host may still retain
                // a stale root, including allocations from a failed
                // pre-commit publication.  Their callback ivars are already
                // inert, so dropping this adapter remains safe.
                return false;
            };
            let committed_objects = std::mem::take(&mut self.committed_objects);
            for object in self.objects.drain(..) {
                unsafe {
                    // Keep this immediately before notification/release as a
                    // final per-object lifecycle fence, including reentrant
                    // or partially borrowed callback-state cases.
                    object_setIvar(object, class.state_ivar(), null_mut());
                    if committed_objects.contains(&object) {
                        let notification = ns_string("AXUIElementDestroyed");
                        if !notification.is_null() {
                            NSAccessibilityPostNotification(object, notification);
                            #[cfg(test)]
                            {
                                self.destroyed_notifications =
                                    self.destroyed_notifications.saturating_add(1);
                            }
                        }
                    }
                    msg_void(object, sel(c"release"));
                }
            }
            true
        }

        fn clear_callback_state(&mut self) {
            if let Ok(mut state) = self.callback_state.try_borrow_mut() {
                state.projection = NativeCallbackProjection::default();
                state.in_flight.clear();
                state.deferred.clear();
                state.pending_numeric_actions = 0;
                state.last_unavailable = None;
            }
        }

        fn advance_generation(&mut self) {
            self.generation = self.generation.saturating_add(1);
            if let Ok(mut state) = self.callback_state.try_borrow_mut() {
                state.generation = self.generation;
            }
        }

        fn post_layout_changed(&mut self) {
            unsafe {
                let notification = ns_string("AXLayoutChanged");
                if !notification.is_null() {
                    NSAccessibilityPostNotification(self.view, notification);
                }
            }
            #[cfg(test)]
            {
                self.layout_notifications = self.layout_notifications.saturating_add(1);
            }
        }

        fn post_value_changed(&mut self, object: Id) {
            #[cfg(not(test))]
            unsafe {
                let notification = ns_string("AXValueChanged");
                if !notification.is_null() {
                    NSAccessibilityPostNotification(object, notification);
                }
            }
            #[cfg(test)]
            {
                let _ = object;
                self.value_notifications = self.value_notifications.saturating_add(1);
            }
        }

        fn retain_or_issue_root_token(&mut self) -> Result<u64, String> {
            if let Some((token, lease, generation)) = self.tokens.root
                && lease == self.lease
                && generation == self.window_generation
            {
                return Ok(token);
            }
            let token = self
                .tokens
                .issue()
                .ok_or_else(|| String::from("native semantic token space exhausted"))?;
            self.tokens.root = Some((token, self.lease, self.window_generation));
            Ok(token)
        }

        fn retain_or_issue_container_token(
            &mut self,
            current: &NativeSemanticContainerSnapshot,
        ) -> Result<u64, String> {
            if let Some(previous) = self.tokens.containers.iter().find(|previous| {
                container_token_fence_matches(previous, current, self.lease, self.window_generation)
            }) {
                return Ok(previous.token);
            }
            let token = self
                .tokens
                .issue()
                .ok_or_else(|| String::from("native semantic token space exhausted"))?;
            self.tokens.containers.push(NativeContainerToken {
                token,
                container_id: current.container_id,
                mount_generation: current.mount_generation,
                registration_generation: current.registration_generation,
                provider_generation: current.provider_generation,
                coordinate_authority: current.coordinate_authority.clone(),
                cardinality: current.cardinality,
                lease: self.lease,
                window_generation: self.window_generation,
            });
            Ok(token)
        }

        fn retain_or_issue_item_token(
            &mut self,
            container: &NativeSemanticContainerSnapshot,
            entry: &crate::runtime::VirtualLayoutNormalizedSemanticSidecarEntry,
            specs: &[NativeNodeSpec],
        ) -> Result<u64, String> {
            if let Some(previous) = self.tokens.items.iter().find(|previous| {
                previous.container_id == container.container_id
                    && previous.mount_generation == container.mount_generation
                    && previous.logical_index == entry.logical_index()
                    && previous.key.stable_equals(entry.key()) == Some(true)
                    && previous.coordinate_authority == container.coordinate_authority
                    && previous.fences.same_exact(entry.publication_fences())
                    && previous.lease == self.lease
                    && previous.window_generation == self.window_generation
                    && !specs.iter().any(|spec| spec.token == previous.token)
            }) {
                return Ok(previous.token);
            }
            let token = self
                .tokens
                .issue()
                .ok_or_else(|| String::from("native semantic token space exhausted"))?;
            self.tokens.items.push(NativeItemToken {
                token,
                container_id: container.container_id,
                mount_generation: container.mount_generation,
                logical_index: entry.logical_index(),
                key: entry.key().clone(),
                coordinate_authority: container.coordinate_authority.clone(),
                fences: entry.publication_fences().clone(),
                lease: self.lease,
                window_generation: self.window_generation,
            });
            Ok(token)
        }

        fn prune_token_ledger(&mut self, specs: &[NativeNodeSpec]) {
            let active_tokens = specs.iter().map(|spec| spec.token).collect::<Vec<_>>();
            self.tokens
                .ordinary
                .retain(|entry| active_tokens.contains(&entry.token));
            self.tokens
                .containers
                .retain(|entry| active_tokens.contains(&entry.token));
            self.tokens
                .items
                .retain(|entry| active_tokens.contains(&entry.token));
            if self
                .tokens
                .root
                .is_some_and(|(token, _, _)| !active_tokens.contains(&token))
            {
                self.tokens.root = None;
            }
        }

        fn reconcile_active_ranges_with_tokens(&mut self) {
            let containers = &self.tokens.containers;
            self.active_ranges.retain_mut(|active| {
                let Some(current) = containers
                    .iter()
                    .find(|container| {
                        container.container_id == active.container.container_id
                            && container.mount_generation == active.container.mount_generation
                            && container.registration_generation
                                == active.container.registration_generation
                            && container.provider_generation == active.container.provider_generation
                            && container.cardinality == active.container.cardinality
                            && container.coordinate_authority
                                == active.container.coordinate_authority
                            && container.lease == active.container.lease
                            && container.window_generation == active.container.window_generation
                    })
                    .cloned()
                else {
                    return false;
                };
                active.container = current;
                true
            });
        }
    }

    impl Drop for NativeSemanticAccessibilityAdapter {
        fn drop(&mut self) {
            let _ = self.retire_published_objects();
            self.tokens.retire_all();
        }
    }

    #[derive(Clone, Copy)]
    struct NativeContainerTokenView {
        cardinality: crate::application::virtual_layout::VirtualLayoutSemanticCardinality,
        max_entries: usize,
        has_range_provider: bool,
    }

    fn native_window_and_view(window: &Window) -> Option<(Id, Id)> {
        let handle = window.window_handle().ok()?;
        let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
            return None;
        };
        let view = handle.ns_view.as_ptr();
        if view.is_null() {
            return None;
        }
        let native_window = unsafe { msg_id(view, sel(c"window")) };
        (!native_window.is_null()).then_some((native_window, view))
    }

    fn native_role(
        _kind: NativeNodeKind,
        role: AutomationRole,
        numeric_action_target: bool,
    ) -> &'static str {
        if numeric_action_target {
            "AXIncrementor"
        } else if matches!(role, AutomationRole::Text | AutomationRole::Readout) {
            "AXStaticText"
        } else {
            "AXGroup"
        }
    }

    fn bounds_are_finite(bounds: AutomationBounds) -> bool {
        bounds.x.is_finite()
            && bounds.y.is_finite()
            && bounds.width.is_finite()
            && bounds.height.is_finite()
    }

    fn rect_is_finite(rect: NSRect) -> bool {
        rect.origin.x.is_finite()
            && rect.origin.y.is_finite()
            && rect.size.width.is_finite()
            && rect.size.height.is_finite()
    }

    fn callback_projection_changed(
        previous: &NativeCallbackProjection,
        next: &NativeCallbackProjection,
    ) -> bool {
        previous.root_token != next.root_token
            || previous.nodes.len() != next.nodes.len()
            || previous
                .nodes
                .iter()
                .zip(&next.nodes)
                .any(|(previous, next)| {
                    previous.token != next.token
                        || previous.kind != next.kind
                        || previous.parent != next.parent
                        || previous.children != next.children
                        || previous.logical_children != next.logical_children
                        || previous.frame != next.frame
                        || previous.role != next.role
                        || previous.label != next.label
                        || previous.description != next.description
                        || previous.value != next.value
                        || previous.action_target != next.action_target
                        || previous.logical_count != next.logical_count
                })
    }

    fn node_at_path<'a>(
        root: &'a AutomationNodeSnapshot,
        path: &[usize],
    ) -> Option<&'a AutomationNodeSnapshot> {
        let mut node = root;
        for index in path {
            node = node.children.get(*index)?;
        }
        Some(node)
    }

    fn normalize_range(
        count: usize,
        declaration_budget: usize,
        start_index: usize,
        max_count: usize,
        aggregate_count: usize,
    ) -> Option<usize> {
        if count == 0 || max_count == 0 || start_index >= count {
            return None;
        }
        let _end = start_index.checked_add(max_count)?;
        let remaining = count.checked_sub(start_index)?;
        let aggregate_remaining = MAX_NATIVE_ITEMS.checked_sub(aggregate_count)?;
        let length = remaining
            .min(max_count)
            .min(declaration_budget)
            .min(MAX_NATIVE_ITEMS)
            .min(aggregate_remaining);
        (length > 0).then_some(length)
    }

    fn callback_state(receiver: Id) -> Option<&'static RefCell<NativeSemanticCallbackState>> {
        if receiver.is_null() {
            return None;
        }
        let class = native_class().ok()?;
        if unsafe { object_getClass(receiver) } != class.class() {
            return None;
        }
        let state = unsafe { object_getIvar(receiver, class.state_ivar()) };
        if state.is_null() {
            return None;
        }
        // The adapter owns this box until all custom elements are detached and
        // released.  AppKit callbacks are main-thread-only during that window.
        Some(unsafe { &*(state.cast::<RefCell<NativeSemanticCallbackState>>()) })
    }

    fn native_value_settable_target(node: &NativeCallbackNode) -> Option<(u64, AutomationTarget)> {
        if node.kind != NativeNodeKind::Ordinary
            || node.role != "AXIncrementor"
            || node.value.is_none()
        {
            return None;
        }
        let target = node.action_target.as_ref()?;
        if target.role != AutomationRole::TextInput
            || !target.enabled
            || !target.focusable
            || target.value.as_deref() != node.value.as_deref()
            || !target
                .authority
                .is_some_and(|authority| authority.materialized)
            || !target
                .available_actions
                .iter()
                .any(|action| action == AUTOMATION_ACTION_INCREMENT)
            || !target
                .available_actions
                .iter()
                .any(|action| action == AUTOMATION_ACTION_DECREMENT)
            || !target
                .available_actions
                .iter()
                .any(|action| action == AUTOMATION_ACTION_SET_TEXT)
        {
            return None;
        }
        Some((node.token, target.clone()))
    }

    unsafe fn objc_attribute_is(attribute: Id, expected: &CStr) -> bool {
        let expected = expected.to_bytes();
        unsafe { radiant_native_attribute_is(attribute, expected.as_ptr(), expected.len()) == YES }
    }

    fn native_value_from_node(node: &NativeCallbackNode) -> Id {
        node.value
            .as_deref()
            .map_or(null_mut(), |value| unsafe { ns_string(value) })
    }

    fn ffi_boundary<T>(fallback: T, callback: impl FnOnce() -> T) -> T {
        catch_unwind(AssertUnwindSafe(callback)).unwrap_or(fallback)
    }

    extern "C" fn native_dealloc(receiver: Id, _: Sel) {
        ffi_boundary((), || {
            if let Ok(class) = native_class() {
                unsafe { object_setIvar(receiver, class.state_ivar(), null_mut()) };
            }
            let superclass = unsafe { objc_getClass(c"NSAccessibilityElement".as_ptr()) };
            if !superclass.is_null() {
                unsafe { msg_super_void(receiver, superclass, sel(c"dealloc")) };
            }
        });
    }

    extern "C" fn native_is_accessibility_element(_: Id, _: Sel) -> ObjcBool {
        ffi_boundary(NO, || YES)
    }

    extern "C" fn native_accessibility_notifies_when_destroyed(_: Id, _: Sel) -> ObjcBool {
        ffi_boundary(YES, || YES)
    }

    extern "C" fn native_attribute_value(receiver: Id, _: Sel, attribute: Id) -> Id {
        ffi_boundary(null_mut(), || {
            let Some(state) = callback_state(receiver) else {
                return null_mut();
            };
            let Ok(state) = state.try_borrow() else {
                return null_mut();
            };
            let Some(node) = state.node_for_object(receiver) else {
                return null_mut();
            };
            let name = unsafe { msg_id_ptr(attribute, sel(c"UTF8String")) };
            if name.is_null() {
                return null_mut();
            }
            let name = unsafe { CStr::from_ptr(name) };
            if name == c"AXRole" {
                unsafe { ns_string(node.role) }
            } else if name == c"AXParent" {
                if let Some(parent) = node.parent {
                    state
                        .node_for_token(parent)
                        .map_or(null_mut(), |node| node.object)
                } else {
                    state.view
                }
            } else if name == c"AXChildren" {
                let children = if node.kind == NativeNodeKind::Container {
                    let Some(children) = complete_virtual_child_tokens(node) else {
                        return null_mut();
                    };
                    children
                } else {
                    node.children.clone()
                };
                let objects = children
                    .iter()
                    .filter_map(|token| state.node_for_token(*token).map(|node| node.object))
                    .collect::<Vec<_>>();
                unsafe { ns_array(&objects) }
            } else if name == c"AXFrame" {
                unsafe { ns_value_rect(node.frame) }
            } else if name == c"AXPosition" {
                unsafe { ns_value_point(node.frame.origin) }
            } else if name == c"AXSize" {
                unsafe { ns_value_size(node.frame.size) }
            } else if name == c"AXTitle" {
                node.label
                    .as_deref()
                    .map_or(null_mut(), |value| unsafe { ns_string(value) })
            } else if name == c"AXDescription" || name == c"AXHelp" {
                node.description
                    .as_deref()
                    .map_or(null_mut(), |value| unsafe { ns_string(value) })
            } else if name == c"AXValue" {
                native_value_from_node(node)
            } else if name == c"AXEnabled" {
                node.action_target
                    .as_ref()
                    .map_or(null_mut(), |target| unsafe {
                        ns_number_bool(target.enabled)
                    })
            } else if name == c"AXRowCount" || name == c"AXCount" {
                node.logical_count
                    .map_or(null_mut(), |count| unsafe { ns_number_usize(count) })
            } else if name == c"AXFocused" {
                unsafe { ns_number_bool(false) }
            } else {
                null_mut()
            }
        })
    }

    extern "C" fn native_accessibility_value(receiver: Id, _: Sel) -> Id {
        ffi_boundary(null_mut(), || {
            let Some(state) = callback_state(receiver) else {
                return null_mut();
            };
            let Ok(state) = state.try_borrow() else {
                return null_mut();
            };
            let Some(node) = state.node_for_object(receiver) else {
                return null_mut();
            };
            native_value_from_node(node)
        })
    }

    extern "C" fn native_array_attribute_count(receiver: Id, _: Sel, attribute: Id) -> usize {
        ffi_boundary(0, || {
            let Some(state) = callback_state(receiver) else {
                return 0;
            };
            let Ok(state) = state.try_borrow() else {
                return 0;
            };
            let Some(node) = state.node_for_object(receiver) else {
                return 0;
            };
            let name = unsafe { msg_id_ptr(attribute, sel(c"UTF8String")) };
            if name.is_null() || unsafe { CStr::from_ptr(name) } != c"AXChildren" {
                return 0;
            }
            accessibility_children_count(node)
        })
    }

    extern "C" fn native_index_of_child(receiver: Id, _: Sel, child: Id) -> usize {
        ffi_boundary(NS_NOT_FOUND, || {
            if receiver.is_null() || child.is_null() {
                return NS_NOT_FOUND;
            }
            let Some(state) = callback_state(receiver) else {
                return NS_NOT_FOUND;
            };
            let Ok(state) = state.try_borrow() else {
                return NS_NOT_FOUND;
            };
            native_index_of_child_in_projection(&state.projection, receiver, child)
        })
    }

    extern "C" fn native_array_attribute_values(
        receiver: Id,
        _: Sel,
        attribute: Id,
        index: usize,
        max_count: usize,
    ) -> Id {
        ffi_boundary(null_mut(), || {
            let Some(state) = callback_state(receiver) else {
                return null_mut();
            };
            let Ok(state) = state.try_borrow() else {
                return null_mut();
            };
            let Some(node) = state.node_for_object(receiver) else {
                return null_mut();
            };
            let name = unsafe { msg_id_ptr(attribute, sel(c"UTF8String")) };
            if name.is_null() || unsafe { CStr::from_ptr(name) } != c"AXChildren" {
                return null_mut();
            }
            let token = node.token;
            let kind = node.kind;
            let children = node.children.clone();
            let logical_count = node.logical_count;
            let logical_children = node.logical_children.clone();
            drop(state);
            if kind == NativeNodeKind::Container
                && let Some(state) = callback_state(receiver)
                && let Ok(mut state) = state.try_borrow_mut()
            {
                state.request_range(token, index, max_count);
            }
            let Some(state) = callback_state(receiver).and_then(|state| state.try_borrow().ok())
            else {
                return null_mut();
            };
            let tokens = if kind == NativeNodeKind::Container {
                retained_logical_child_tokens(
                    &logical_children,
                    logical_count.unwrap_or(0),
                    index,
                    max_count.min(MAX_NATIVE_ITEMS),
                )
                .unwrap_or_default()
            } else {
                children
                    .get(index..)
                    .unwrap_or(&[])
                    .iter()
                    .take(max_count.min(MAX_NATIVE_ITEMS))
                    .copied()
                    .collect()
            };
            let values = tokens
                .iter()
                .map(|token| state.node_for_token(*token).map(|node| node.object))
                .collect::<Option<Vec<_>>>()
                .unwrap_or_default();
            unsafe { ns_array(&values) }
        })
    }

    extern "C" fn native_attribute_settable(receiver: Id, _: Sel, attribute: Id) -> ObjcBool {
        ffi_boundary(NO, || {
            if !unsafe { objc_attribute_is(attribute, NS_ACCESSIBILITY_VALUE_ATTRIBUTE) } {
                return NO;
            }
            let Some(state) = callback_state(receiver) else {
                return NO;
            };
            let Ok(state) = state.try_borrow() else {
                return NO;
            };
            if state
                .node_for_object(receiver)
                .and_then(native_value_settable_target)
                .is_some()
            {
                YES
            } else {
                NO
            }
        })
    }

    fn enqueue_native_value(receiver: Id, value: Id) -> bool {
        let Some(state) = callback_state(receiver) else {
            return false;
        };
        let Ok(state) = state.try_borrow() else {
            return false;
        };
        let Some((token, target)) = state
            .node_for_object(receiver)
            .and_then(native_value_settable_target)
        else {
            return false;
        };
        let Some(value) = (unsafe { bounded_ns_string_to_rust(value) }) else {
            return false;
        };
        drop(state);

        let Some(state) = callback_state(receiver) else {
            return false;
        };
        let Ok(mut state) = state.try_borrow_mut() else {
            return false;
        };
        let Some(current) = state
            .node_for_token(token)
            .and_then(native_value_settable_target)
        else {
            return false;
        };
        if !numeric_action_target_fence_matches(&target, &current.1) {
            return false;
        }
        state.enqueue_numeric_action(
            token,
            current.1,
            NativeNumericAccessibilityAction::SetValueText(value),
        )
    }

    extern "C" fn native_set_accessibility_value(receiver: Id, _: Sel, value: Id) {
        ffi_boundary((), || {
            let _ = enqueue_native_value(receiver, value);
        });
    }

    extern "C" fn native_set_accessibility_value_for_attribute(
        receiver: Id,
        _: Sel,
        value: Id,
        attribute: Id,
    ) {
        ffi_boundary((), || {
            if unsafe { objc_attribute_is(attribute, NS_ACCESSIBILITY_VALUE_ATTRIBUTE) } {
                let _ = enqueue_native_value(receiver, value);
            }
        });
    }

    extern "C" fn native_action_names(receiver: Id, _: Sel) -> Id {
        ffi_boundary(null_mut(), || {
            let Some(state) = callback_state(receiver) else {
                return null_mut();
            };
            let Ok(state) = state.try_borrow() else {
                return null_mut();
            };
            let Some(node) = state.node_for_object(receiver) else {
                return null_mut();
            };
            if node.action_target.is_none() {
                return unsafe { ns_array(&[]) };
            }
            let values = numeric_action_names()
                .into_iter()
                .filter_map(|action| {
                    native_numeric_action_text(&action).map(|text| unsafe { ns_string(text) })
                })
                .collect::<Vec<_>>();
            if values.iter().any(|value| value.is_null()) {
                null_mut()
            } else {
                unsafe { ns_array(&values) }
            }
        })
    }

    fn perform_numeric_action(receiver: Id, action: NativeNumericAccessibilityAction) -> ObjcBool {
        let Some(state) = callback_state(receiver) else {
            return NO;
        };
        let Ok(mut state) = state.try_borrow_mut() else {
            return NO;
        };
        let Some((token, target)) = state.node_for_object(receiver).and_then(|node| {
            node.action_target
                .clone()
                .map(|target| (node.token, target))
        }) else {
            return NO;
        };
        if state.enqueue_numeric_action(token, target, action) {
            YES
        } else {
            NO
        }
    }

    extern "C" fn native_perform_increment(receiver: Id, _: Sel) -> ObjcBool {
        ffi_boundary(NO, || {
            perform_numeric_action(receiver, NativeNumericAccessibilityAction::Increment)
        })
    }

    extern "C" fn native_perform_decrement(receiver: Id, _: Sel) -> ObjcBool {
        ffi_boundary(NO, || {
            perform_numeric_action(receiver, NativeNumericAccessibilityAction::Decrement)
        })
    }

    extern "C" fn native_perform_action(receiver: Id, _: Sel, action: Id) {
        ffi_boundary((), || {
            let name = unsafe { msg_id_ptr(action, sel(c"UTF8String")) };
            if name.is_null() {
                return;
            }
            let Some(action) = native_numeric_action_from_name(unsafe { CStr::from_ptr(name) })
            else {
                return;
            };
            let _ = perform_numeric_action(receiver, action);
        });
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::widgets::{
            NumericAccessibilityOutcome, NumericAdjustment, NumericCodec, NumericInputWidget,
            NumericParseResult, NumericStep, NumericStepDirection, WidgetSizing,
        };
        use crate::{
            application::IntoView,
            application::{
                VirtualLayoutParts, row, scroll, spacer, text, virtual_layout_from_parts,
            },
            gui::{
                automation::AutomationTargetAuthority,
                types::{Rect, Vector2},
            },
            layout::{
                VirtualLayoutBoundsConfidence, VirtualLayoutBudget, VirtualLayoutExtentCandidate,
                VirtualLayoutItemCandidate, VirtualLayoutItemKey, VirtualLayoutOverscan,
                VirtualLayoutPolicy, VirtualLayoutPolicyDecision, VirtualLayoutPolicyIdentity,
                VirtualLayoutQueryInput, VirtualLayoutQuerySink, VirtualLayoutVisibility,
            },
            runtime::{
                Command, RuntimeBridge, SurfaceNode, SurfaceRuntime, UiSurface,
                VirtualLayoutRevisions, VirtualLayoutSemanticEntry,
                VirtualLayoutSemanticProviderOutcome, VirtualLayoutSemanticRangeProvider,
                VirtualLayoutSemanticRangeRequest, WidgetMessageMapper,
                virtual_layout::{
                    VirtualLayoutSemanticCoordinateTransform,
                    VirtualLayoutSemanticCoordinateTransformOutcome,
                },
            },
        };
        use std::{cell::Cell, rc::Rc, sync::Arc};

        struct TestVirtualLayoutPolicy;

        impl VirtualLayoutPolicy for TestVirtualLayoutPolicy {
            fn query(
                &self,
                _input: &VirtualLayoutQueryInput,
                sink: &mut VirtualLayoutQuerySink,
            ) -> VirtualLayoutPolicyDecision {
                sink.visit(VirtualLayoutItemCandidate::new(
                    VirtualLayoutItemKey::new(1_u32),
                    0,
                    Rect::from_xy_size(0.0, 0.0, 100.0, 20.0),
                    VirtualLayoutVisibility::Visible,
                    VirtualLayoutBoundsConfidence::Exact,
                ))
                .expect("test virtual-layout policy should accept its item");
                sink.set_extent(VirtualLayoutExtentCandidate::exact(Vector2::new(
                    100.0, 20.0,
                )))
                .expect("test virtual-layout policy should accept its extent");
                VirtualLayoutPolicyDecision::Ready
            }
        }

        struct CountingRangeProvider {
            calls: Cell<usize>,
        }

        impl VirtualLayoutSemanticRangeProvider for CountingRangeProvider {
            fn lookup_range(
                &self,
                _request: &VirtualLayoutSemanticRangeRequest,
            ) -> VirtualLayoutSemanticProviderOutcome<Vec<crate::runtime::VirtualLayoutSemanticEntry>>
            {
                self.calls.set(self.calls.get().saturating_add(1));
                VirtualLayoutSemanticProviderOutcome::NotFound
            }
        }

        struct TestBridge {
            surface: UiSurface<()>,
        }

        impl RuntimeBridge<()> for TestBridge {
            fn project_surface(&mut self) -> Arc<UiSurface<()>> {
                crate::runtime::test_arc_surface(self.surface.clone())
            }

            fn pull_surface(&mut self) -> UiSurface<()> {
                self.surface.clone()
            }
        }

        #[derive(Clone, Debug)]
        enum MappedNumericMessage {
            Accessibility(NumericAccessibilityOutcome<f32, (), ()>),
        }

        #[derive(Clone, Copy)]
        struct NativeNumericTestCodec;

        impl NumericCodec<f32> for NativeNumericTestCodec {
            type Error = ();

            fn parse(&self, text: &str) -> NumericParseResult<f32> {
                if text.is_empty() || text == "-" {
                    return NumericParseResult::Incomplete;
                }
                text.parse::<f32>()
                    .map(NumericParseResult::Valid)
                    .unwrap_or(NumericParseResult::Invalid)
            }

            fn format_editable(
                &self,
                value: &f32,
                output: &mut dyn std::fmt::Write,
            ) -> Result<(), Self::Error> {
                write!(output, "{value}").map_err(|_| ())
            }
        }

        #[derive(Clone, Copy)]
        struct NativeNumericTestAdjustment;

        impl NumericAdjustment<f32> for NativeNumericTestAdjustment {
            type Error = ();

            fn normalized_to_value(&self, normalized: f32) -> Result<f32, Self::Error> {
                Ok(normalized)
            }

            fn value_to_normalized(&self, value: &f32) -> Result<f32, Self::Error> {
                Ok(*value)
            }

            fn step(
                &self,
                value: &f32,
                direction: NumericStepDirection,
                _step: NumericStep,
            ) -> Result<f32, Self::Error> {
                Ok(*value
                    + match direction {
                        NumericStepDirection::Increase => 1.0,
                        NumericStepDirection::Decrease => -1.0,
                    })
            }

            fn scrub(
                &self,
                value: &f32,
                _normalized_delta: f32,
                _step: NumericStep,
            ) -> Result<f32, Self::Error> {
                Ok(*value)
            }

            fn wheel(
                &self,
                value: &f32,
                _delta: f32,
                _step: NumericStep,
            ) -> Result<f32, Self::Error> {
                Ok(*value)
            }
        }

        struct MappedNumericBridge {
            value: Rc<Cell<f32>>,
            mapped_actions: Rc<Cell<usize>>,
        }

        impl MappedNumericBridge {
            fn new(value: f32) -> Self {
                Self {
                    value: Rc::new(Cell::new(value)),
                    mapped_actions: Rc::new(Cell::new(0)),
                }
            }
        }

        impl RuntimeBridge<MappedNumericMessage> for MappedNumericBridge {
            fn project_surface(&mut self) -> Arc<UiSurface<MappedNumericMessage>> {
                let mut input = NumericInputWidget::try_new(
                    self.value.get(),
                    NativeNumericTestCodec,
                    NativeNumericTestAdjustment,
                    WidgetSizing::fixed(Vector2::new(120.0, 28.0)),
                )
                .expect("native numeric test input should construct");
                input.set_accessibility_action_mode();
                let mapped_actions = Rc::clone(&self.mapped_actions);
                let mapper = WidgetMessageMapper::none().with_accessibility_action(
                    move |outcome: NumericAccessibilityOutcome<f32, (), ()>| {
                        mapped_actions.set(mapped_actions.get().saturating_add(1));
                        MappedNumericMessage::Accessibility(outcome)
                    },
                );
                let numeric = SurfaceNode::widget(input, mapper).with_id(42);
                crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::row(
                    1,
                    0.0,
                    vec![crate::runtime::SurfaceChild::fill(numeric)],
                )))
            }

            fn update(&mut self, message: MappedNumericMessage) -> Command<MappedNumericMessage> {
                let MappedNumericMessage::Accessibility(outcome) = message;
                if let NumericAccessibilityOutcome::Edit(edit) = outcome
                    && let Some(event) = edit.events().last()
                {
                    self.value.set(event.value);
                }
                Command::none()
            }
        }

        fn test_container(container_id: u64) -> NativeContainerToken {
            NativeContainerToken {
                token: container_id,
                container_id,
                mount_generation: 1,
                registration_generation: 2,
                provider_generation: 3,
                coordinate_authority: NativeSemanticCoordinateAuthority::Logical,
                cardinality:
                    crate::application::virtual_layout::VirtualLayoutSemanticCardinality::new(8, 4),
                lease: None,
                window_generation: 5,
            }
        }

        fn test_handle(container_id: u64) -> SemanticAutomationContainerHandle {
            SemanticAutomationContainerHandle {
                runtime_id: 11,
                session_generation: 12,
                container_id,
                mount_generation: 1,
            }
        }

        fn test_active_range(
            container_id: u64,
            start_index: usize,
            length: usize,
        ) -> NativeActiveRange {
            let container = test_container(container_id);
            NativeActiveRange {
                key: NativeQueryKey {
                    token: container.token,
                    start_index,
                    max_count: length,
                },
                container,
                length,
            }
        }

        fn numeric_target_fixture() -> (AutomationNodeSnapshot, AutomationTarget) {
            let root_id = AutomationNodeId::new("root");
            let node_id = AutomationNodeId::new("gain");
            let mut semantics = AutomationNodeSemantics::new(AutomationRole::TextInput)
                .with_label("Gain")
                .with_value_text("7");
            semantics.focusable = true;
            semantics.focused = true;
            let mut node = AutomationNodeSnapshot::from_semantics(
                node_id.clone(),
                AutomationBounds {
                    x: 8.0,
                    y: 12.0,
                    width: 120.0,
                    height: 24.0,
                },
                semantics,
            );
            node.available_actions = vec![
                String::from("focus"),
                String::from(AUTOMATION_ACTION_INCREMENT),
                String::from(AUTOMATION_ACTION_DECREMENT),
                String::from("set_text"),
            ];
            let path = vec![root_id, node_id];
            let mut target = AutomationTarget::from_node(&node, 1, 1, path);
            target.authority = Some(AutomationTargetAuthority::materialized(9));
            (node, target)
        }

        fn numeric_callback_node(target: AutomationTarget, value: &str) -> NativeCallbackNode {
            NativeCallbackNode {
                object: 91_usize as Id,
                token: 91,
                kind: NativeNodeKind::Ordinary,
                parent: Some(1),
                children: Vec::new(),
                logical_children: Vec::new(),
                role: "AXIncrementor",
                frame: NSRect {
                    origin: NSPoint { x: 8.0, y: 12.0 },
                    size: NSSize {
                        width: 120.0,
                        height: 24.0,
                    },
                },
                label: Some(String::from("Gain")),
                description: None,
                value: Some(value.to_owned()),
                action_target: Some(target),
                logical_count: None,
            }
        }

        fn native_numeric_callback_receiver() -> (Id, Box<RefCell<NativeSemanticCallbackState>>) {
            let class = native_class().expect("the native accessibility class should exist");
            let allocated = unsafe { msg_id(class.class(), sel(c"alloc")) };
            let receiver = unsafe { msg_id(allocated, sel(c"init")) };
            assert!(!receiver.is_null());

            let (_, target) = numeric_target_fixture();
            let mut node = numeric_callback_node(target, "7");
            node.object = receiver;
            let callback_state = Box::new(RefCell::new(NativeSemanticCallbackState::new_for_test(
                WindowId::dummy(),
                3,
                null_mut(),
            )));
            callback_state.borrow_mut().projection = NativeCallbackProjection {
                nodes: vec![node],
                root_token: Some(91),
            };
            let state_ptr = (&*callback_state as *const RefCell<NativeSemanticCallbackState>) as Id;
            unsafe { object_setIvar(receiver, class.state_ivar(), state_ptr) };
            (receiver, callback_state)
        }

        fn index_test_node(
            object: usize,
            token: u64,
            kind: NativeNodeKind,
            parent: Option<u64>,
            children: Vec<u64>,
            logical_children: Vec<(usize, u64)>,
            logical_count: Option<usize>,
        ) -> NativeCallbackNode {
            NativeCallbackNode {
                object: object as Id,
                token,
                kind,
                parent,
                children,
                logical_children,
                role: "AXGroup",
                frame: NSRect {
                    origin: NSPoint { x: 0.0, y: 0.0 },
                    size: NSSize {
                        width: 1.0,
                        height: 1.0,
                    },
                },
                label: None,
                description: None,
                value: None,
                action_target: None,
                logical_count,
            }
        }

        fn index_topology_projection() -> NativeCallbackProjection {
            NativeCallbackProjection {
                nodes: vec![
                    index_test_node(
                        101,
                        1,
                        NativeNodeKind::Root,
                        None,
                        vec![2, 3],
                        Vec::new(),
                        None,
                    ),
                    index_test_node(
                        102,
                        2,
                        NativeNodeKind::Ordinary,
                        Some(1),
                        vec![4],
                        Vec::new(),
                        None,
                    ),
                    index_test_node(
                        103,
                        3,
                        NativeNodeKind::Container,
                        Some(1),
                        vec![6, 8],
                        vec![(100, 6)],
                        Some(110),
                    ),
                    index_test_node(
                        104,
                        4,
                        NativeNodeKind::Ordinary,
                        Some(2),
                        Vec::new(),
                        Vec::new(),
                        None,
                    ),
                    index_test_node(
                        106,
                        6,
                        NativeNodeKind::Item,
                        Some(3),
                        vec![7],
                        Vec::new(),
                        None,
                    ),
                    index_test_node(
                        107,
                        7,
                        NativeNodeKind::Ordinary,
                        Some(6),
                        Vec::new(),
                        Vec::new(),
                        None,
                    ),
                    index_test_node(
                        108,
                        8,
                        NativeNodeKind::Ordinary,
                        Some(3),
                        Vec::new(),
                        Vec::new(),
                        None,
                    ),
                ],
                root_token: Some(1),
            }
        }

        fn native_index_projection_fixture(
            mut projection: NativeCallbackProjection,
        ) -> (Vec<Id>, Box<RefCell<NativeSemanticCallbackState>>) {
            let class = native_class().expect("the native accessibility class should exist");
            let callback_state = Box::new(RefCell::new(NativeSemanticCallbackState::new_for_test(
                WindowId::dummy(),
                3,
                null_mut(),
            )));
            let state_ptr = (&*callback_state as *const RefCell<NativeSemanticCallbackState>) as Id;
            let mut objects = Vec::with_capacity(projection.nodes.len());
            for node in &mut projection.nodes {
                let allocated = unsafe { msg_id(class.class(), sel(c"alloc")) };
                let object = unsafe { msg_id(allocated, sel(c"init")) };
                assert!(!object.is_null());
                node.object = object;
                unsafe { object_setIvar(object, class.state_ivar(), state_ptr) };
                objects.push(object);
            }
            callback_state.borrow_mut().projection = projection;
            (objects, callback_state)
        }

        fn release_native_index_objects(objects: &[Id]) {
            for object in objects {
                unsafe { msg_void(*object, sel(c"release")) };
            }
        }

        fn native_destroyed_callback_adapter_fixture() -> NativeSemanticAccessibilityAdapter {
            let class = native_class().expect("the native accessibility class should exist");
            let allocated = unsafe { msg_id(class.class(), sel(c"alloc")) };
            let receiver = unsafe { msg_id(allocated, sel(c"init")) };
            assert!(!receiver.is_null());

            let session = SemanticAutomationSessionHandle {
                runtime_id: 7,
                generation: 8,
            };
            let mut container = test_container(3);
            container.lease = Some(session);
            let projection = NativeCallbackProjection {
                nodes: vec![index_test_node(
                    receiver as usize,
                    1,
                    NativeNodeKind::Root,
                    None,
                    Vec::new(),
                    Vec::new(),
                    None,
                )],
                root_token: Some(1),
            };
            let pending_key = NativeQueryKey {
                token: container.token,
                start_index: 2,
                max_count: 1,
            };
            let deferred_key = NativeQueryKey {
                token: container.token,
                start_index: 4,
                max_count: 2,
            };
            let mut callback_state =
                NativeSemanticCallbackState::new_for_test(WindowId::dummy(), 17, 1_usize as Id);
            callback_state.projection = projection;
            callback_state.in_flight = vec![pending_key];
            callback_state.deferred = vec![deferred_key];
            callback_state.pending_numeric_actions = 3;
            callback_state.last_unavailable =
                Some(NativeSemanticUnavailableReason::SessionContended);
            let callback_state = Box::new(RefCell::new(callback_state));
            let state_ptr = (&*callback_state as *const RefCell<NativeSemanticCallbackState>) as Id;
            unsafe { object_setIvar(receiver, class.state_ivar(), state_ptr) };

            NativeSemanticAccessibilityAdapter {
                view: null_mut(),
                callback_state,
                objects: vec![receiver],
                committed_objects: vec![receiver],
                tokens: NativeTokenLedger {
                    next: 8,
                    root: Some((1, Some(session), 5)),
                    ordinary: vec![NativeOrdinaryToken {
                        token: 2,
                        id: AutomationNodeId::new("ordinary"),
                        parent: Some(1),
                        lease: Some(session),
                        window_generation: 5,
                    }],
                    containers: vec![container.clone()],
                    items: vec![NativeItemToken {
                        token: 6,
                        container_id: container.container_id,
                        mount_generation: container.mount_generation,
                        logical_index: 2,
                        key: VirtualLayoutItemKey::new(6_u32),
                        coordinate_authority: container.coordinate_authority.clone(),
                        fences: crate::runtime::NormalizedSemanticPublicationFenceSet::default(),
                        lease: Some(session),
                        window_generation: 5,
                    }],
                },
                lease: Some(session),
                generation: 17,
                window_generation: 5,
                transform: None,
                current_containers: Vec::new(),
                active_ranges: vec![NativeActiveRange {
                    key: pending_key,
                    container,
                    length: 1,
                }],
                attached: true,
                layout_notifications: 11,
                value_notifications: 13,
                destroyed_notifications: 0,
            }
        }

        #[derive(Debug, PartialEq)]
        struct NativeDestroyedCallbackSnapshot {
            projection: NativeCallbackProjection,
            in_flight: Vec<NativeQueryKey>,
            deferred: Vec<NativeQueryKey>,
            pending_numeric_actions: usize,
            state_generation: u64,
            last_unavailable: Option<NativeSemanticUnavailableReason>,
            active_ranges: Vec<NativeActiveRange>,
            lease: Option<SemanticAutomationSessionHandle>,
            generation: u64,
            window_generation: u64,
            tokens: NativeTokenLedger,
            objects: Vec<Id>,
            current_containers: Vec<NativeSemanticContainerSnapshot>,
            attached: bool,
            layout_notifications: usize,
            value_notifications: usize,
            destroyed_notifications: usize,
        }

        fn native_destroyed_callback_snapshot(
            adapter: &NativeSemanticAccessibilityAdapter,
        ) -> NativeDestroyedCallbackSnapshot {
            let state = adapter.callback_state.borrow();
            NativeDestroyedCallbackSnapshot {
                projection: state.projection.clone(),
                in_flight: state.in_flight.clone(),
                deferred: state.deferred.clone(),
                pending_numeric_actions: state.pending_numeric_actions,
                state_generation: state.generation,
                last_unavailable: state.last_unavailable,
                active_ranges: adapter.active_ranges.clone(),
                lease: adapter.lease,
                generation: adapter.generation,
                window_generation: adapter.window_generation,
                tokens: adapter.tokens.clone(),
                objects: adapter.objects.clone(),
                current_containers: adapter.current_containers.clone(),
                attached: adapter.attached,
                layout_notifications: adapter.layout_notifications,
                value_notifications: adapter.value_notifications,
                destroyed_notifications: adapter.destroyed_notifications,
            }
        }

        fn throwing_foundation_object() -> Id {
            static CLASS: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
            let class = *CLASS.get_or_init(|| unsafe {
                let class = objc_getClass(c"RadiantNativeAXValueThrowingObject".as_ptr());
                if !class.is_null() {
                    return class as usize;
                }
                let superclass = objc_getClass(c"NSObject".as_ptr());
                assert!(!superclass.is_null());
                let class = objc_allocateClassPair(
                    superclass,
                    c"RadiantNativeAXValueThrowingObject".as_ptr(),
                    0,
                );
                assert!(!class.is_null());
                let method = class_getInstanceMethod(superclass, sel(c"doesNotRecognizeSelector:"));
                assert!(!method.is_null());
                let implementation = method_getImplementation(method);
                assert!(!implementation.is_null());
                assert_ne!(
                    class_addMethod(
                        class,
                        sel(c"isKindOfClass:"),
                        implementation,
                        c"c@:@".as_ptr(),
                    ),
                    NO
                );
                objc_registerClassPair(class);
                class as usize
            }) as Id;
            let allocated = unsafe { msg_id(class, sel(c"alloc")) };
            let object = unsafe { msg_id(allocated, sel(c"init")) };
            assert!(!object.is_null());
            object
        }

        const ACCESSIBILITY_CHILDREN_HOST_STANDARD: u8 = 0;
        const ACCESSIBILITY_CHILDREN_HOST_UNSUPPORTED: u8 = 1;
        const ACCESSIBILITY_CHILDREN_HOST_THROWING_SETTER: u8 = 2;
        const ACCESSIBILITY_CHILDREN_HOST_THROWING_GETTER: u8 = 3;
        const ACCESSIBILITY_CHILDREN_HOST_MISMATCHED: u8 = 4;
        const ACCESSIBILITY_CHILDREN_HOST_IGNORING_CLEAR: u8 = 5;
        const ACCESSIBILITY_CHILDREN_HOST_THROWING_CLEAR: u8 = 6;
        const MODERN_ACCESSIBILITY_ELEMENT_STANDARD: u8 = 0;
        const MODERN_ACCESSIBILITY_ELEMENT_THROWING_SETTER: u8 = 1;
        const MODERN_ACCESSIBILITY_ELEMENT_THROWING_GETTER: u8 = 2;
        const MODERN_ACCESSIBILITY_ELEMENT_MISMATCHED_PARENT: u8 = 3;
        const MODERN_ACCESSIBILITY_ELEMENT_MISMATCHED_CHILDREN: u8 = 4;
        const MODERN_ACCESSIBILITY_ELEMENT_FILTERED: u8 = 5;
        const MODERN_ACCESSIBILITY_ELEMENT_UNSUPPORTED: u8 = 6;

        fn accessibility_children_test_host(kind: u8) -> Id {
            unsafe { radiant_native_test_make_accessibility_children_host(kind) }
        }

        fn release_accessibility_children_test_host(host: Id) {
            if !host.is_null() {
                unsafe { msg_void(host, sel(c"release")) };
            }
        }

        fn modern_accessibility_element(kind: u8) -> Id {
            unsafe { radiant_native_test_make_accessibility_element(kind) }
        }

        fn release_modern_accessibility_element(element: Id) {
            if !element.is_null() {
                unsafe { msg_void(element, sel(c"release")) };
            }
        }

        fn modern_accessibility_children(element: Id) -> Vec<Id> {
            if element.is_null() {
                return Vec::new();
            }
            let children = unsafe { msg_id(element, sel(c"accessibilityChildren")) };
            if children.is_null() {
                return Vec::new();
            }
            let count = unsafe { msg_usize(children, sel(c"count")) };
            (0..count)
                .map(|index| unsafe { msg_id_usize(children, sel(c"objectAtIndex:"), index) })
                .collect()
        }

        fn modern_accessibility_string(element: Id, selector: Sel) -> Option<String> {
            let value = unsafe { msg_id(element, selector) };
            if value.is_null() {
                None
            } else {
                unsafe { bounded_ns_string_to_rust(value) }
            }
        }

        #[allow(clippy::too_many_arguments)]
        fn configure_modern_accessibility_element_fixture(
            element: Id,
            role: &str,
            parent: Id,
            children: &[Id],
            frame: NSRect,
            label: Option<&str>,
            title: Option<&str>,
            help: Option<&str>,
            enabled: Option<bool>,
        ) -> ObjcBool {
            let role = unsafe { ns_string(role) };
            let children = unsafe { ns_array(children) };
            let label = label.map_or(null_mut(), |value| unsafe { ns_string(value) });
            let title = title.map_or(null_mut(), |value| unsafe { ns_string(value) });
            let help = help.map_or(null_mut(), |value| unsafe { ns_string(value) });
            unsafe {
                radiant_native_configure_accessibility_element(
                    element,
                    role,
                    parent,
                    children,
                    &frame,
                    label,
                    title,
                    help,
                    enabled.map_or(NO, |_| YES),
                    enabled.map_or(NO, |value| if value { YES } else { NO }),
                )
            }
        }

        fn accessibility_children_readback(host: Id) -> (Id, usize) {
            if host.is_null() {
                return (null_mut(), 0);
            }
            let children = unsafe { msg_id(host, sel(c"accessibilityChildren")) };
            if children.is_null() {
                return (null_mut(), 0);
            }
            let count = unsafe { msg_usize(children, sel(c"count")) };
            let first = if count == 1 {
                unsafe { msg_id_usize(children, sel(c"objectAtIndex:"), 0) }
            } else {
                null_mut()
            };
            (first, count)
        }

        fn native_accessibility_root_fixture(
            view: Id,
        ) -> (Id, Box<RefCell<NativeSemanticCallbackState>>) {
            let class = native_class().expect("the native accessibility class should exist");
            let allocated = unsafe { msg_id(class.class(), sel(c"alloc")) };
            let root = unsafe { msg_id(allocated, sel(c"init")) };
            assert!(!root.is_null());
            let callback_state = Box::new(RefCell::new(NativeSemanticCallbackState::new_for_test(
                WindowId::dummy(),
                1,
                view,
            )));
            callback_state.borrow_mut().projection = NativeCallbackProjection {
                nodes: vec![index_test_node(
                    root as usize,
                    TEST_ROOT_TOKEN,
                    NativeNodeKind::Root,
                    None,
                    Vec::new(),
                    Vec::new(),
                    None,
                )],
                root_token: Some(TEST_ROOT_TOKEN),
            };
            let state_ptr = (&*callback_state as *const RefCell<NativeSemanticCallbackState>) as Id;
            unsafe { object_setIvar(root, class.state_ivar(), state_ptr) };
            (root, callback_state)
        }

        fn native_publication_adapter_fixture(view: Id) -> NativeSemanticAccessibilityAdapter {
            let callback_state =
                NativeSemanticCallbackState::new_for_test(WindowId::dummy(), 1, view);
            NativeSemanticAccessibilityAdapter {
                view,
                callback_state: Box::new(RefCell::new(callback_state)),
                objects: Vec::new(),
                committed_objects: Vec::new(),
                tokens: NativeTokenLedger::default(),
                lease: None,
                generation: 1,
                window_generation: 1,
                transform: Some(NativeCoordinateTransform::for_test(NSRect {
                    origin: NSPoint { x: 0.0, y: 0.0 },
                    size: NSSize {
                        width: 240.0,
                        height: 120.0,
                    },
                })),
                current_containers: Vec::new(),
                active_ranges: Vec::new(),
                attached: false,
                layout_notifications: 0,
                value_notifications: 0,
                destroyed_notifications: 0,
            }
        }

        #[test]
        fn native_coordinate_conversion_helper_contains_objective_c_exceptions() {
            let source = NSRect {
                origin: NSPoint { x: 4.0, y: 8.0 },
                size: NSSize {
                    width: 12.0,
                    height: 16.0,
                },
            };
            let mut screen = source;
            let converted =
                unsafe { radiant_native_test_convert_view_rect_to_screen(&source, &mut screen) };

            assert_eq!(converted, NO);
            assert!(!rect_is_finite(screen));
        }

        #[test]
        fn native_coordinate_conversion_failures_withhold_projection() {
            let valid = NSRect {
                origin: NSPoint { x: 10.0, y: 20.0 },
                size: NSSize {
                    width: 30.0,
                    height: 40.0,
                },
            };
            let invalid = NSRect {
                size: NSSize {
                    width: f64::NAN,
                    ..valid.size
                },
                ..valid
            };

            assert!(validated_screen_rect(NO, valid, 1.0).is_none());
            assert!(validated_screen_rect(YES, invalid, 1.0).is_none());
            assert_eq!(validated_screen_rect(YES, valid, 1.0), Some(valid));
        }

        #[test]
        fn accessibility_children_boundary_requires_exact_supported_root_attachment() {
            let host_kinds = [
                (ACCESSIBILITY_CHILDREN_HOST_STANDARD, YES),
                (ACCESSIBILITY_CHILDREN_HOST_UNSUPPORTED, NO),
                (ACCESSIBILITY_CHILDREN_HOST_THROWING_SETTER, NO),
                (ACCESSIBILITY_CHILDREN_HOST_THROWING_GETTER, NO),
                (ACCESSIBILITY_CHILDREN_HOST_MISMATCHED, NO),
            ];
            for (kind, expected_installation) in host_kinds {
                let host = accessibility_children_test_host(kind);
                assert!(!host.is_null());
                let (root, callback_state) = native_accessibility_root_fixture(host);
                assert_eq!(
                    unsafe { radiant_native_set_accessibility_children(host, root) },
                    expected_installation,
                    "unexpected accessibilityChildren installation result for fixture {kind}"
                );
                if kind == ACCESSIBILITY_CHILDREN_HOST_STANDARD {
                    let (readback, count) = accessibility_children_readback(host);
                    assert_eq!(count, 1);
                    assert_eq!(readback, root);
                    assert_eq!(
                        unsafe { radiant_native_set_accessibility_children(host, null_mut()) },
                        NO
                    );
                    assert_eq!(
                        unsafe { radiant_native_clear_accessibility_children(host) },
                        YES
                    );
                    let (readback, count) = accessibility_children_readback(host);
                    assert!(readback.is_null());
                    assert_eq!(count, 0);
                    assert_eq!(
                        unsafe { radiant_native_clear_accessibility_children(host) },
                        YES,
                        "clearing an already empty host must be idempotent"
                    );
                }
                let class = native_class().expect("the native accessibility class should exist");
                unsafe { object_setIvar(root, class.state_ivar(), null_mut()) };
                unsafe { msg_void(root, sel(c"release")) };
                drop(callback_state);
                release_accessibility_children_test_host(host);
            }

            let wrong_class =
                unsafe { class_named(c"NSObject") }.expect("the Foundation class should exist");
            let allocated = unsafe { msg_id(wrong_class, sel(c"alloc")) };
            let wrong_host = unsafe { msg_id(allocated, sel(c"init")) };
            let host = accessibility_children_test_host(ACCESSIBILITY_CHILDREN_HOST_STANDARD);
            let (root, callback_state) = native_accessibility_root_fixture(host);
            assert_eq!(
                unsafe { radiant_native_set_accessibility_children(wrong_host, root) },
                NO
            );
            assert_eq!(
                unsafe { radiant_native_clear_accessibility_children(wrong_host) },
                NO
            );
            assert_eq!(
                unsafe { radiant_native_set_accessibility_children(host, null_mut()) },
                NO
            );
            assert_eq!(
                unsafe { radiant_native_set_accessibility_children(null_mut(), root) },
                NO
            );
            assert_eq!(
                unsafe { radiant_native_clear_accessibility_children(null_mut()) },
                NO
            );
            let class = native_class().expect("the native accessibility class should exist");
            unsafe { object_setIvar(root, class.state_ivar(), null_mut()) };
            unsafe { msg_void(root, sel(c"release")) };
            drop(callback_state);
            release_accessibility_children_test_host(host);
            unsafe { msg_void(wrong_host, sel(c"release")) };
        }

        #[test]
        fn accessibility_children_boundary_preserves_content_view_parent_symmetry() {
            let host = accessibility_children_test_host(ACCESSIBILITY_CHILDREN_HOST_STANDARD);
            let (root, callback_state) = native_accessibility_root_fixture(host);
            assert_eq!(
                unsafe { radiant_native_set_accessibility_children(host, root) },
                YES
            );
            let (readback, count) = accessibility_children_readback(host);
            assert_eq!(count, 1);
            assert_eq!(readback, root);
            let parent_attribute = unsafe { ns_string("AXParent") };
            assert_eq!(
                native_attribute_value(root, null_mut(), parent_attribute),
                host,
                "the published root must report the same content view as its AXParent"
            );
            assert_eq!(
                unsafe { radiant_native_clear_accessibility_children(host) },
                YES
            );
            let (readback, count) = accessibility_children_readback(host);
            assert!(readback.is_null());
            assert_eq!(count, 0);
            let class = native_class().expect("the native accessibility class should exist");
            unsafe { object_setIvar(root, class.state_ivar(), null_mut()) };
            unsafe { msg_void(root, sel(c"release")) };
            drop(callback_state);
            release_accessibility_children_test_host(host);
        }

        #[test]
        fn modern_accessibility_configuration_is_bidirectional_and_reachable() {
            let host = accessibility_children_test_host(ACCESSIBILITY_CHILDREN_HOST_STANDARD);
            let root = modern_accessibility_element(MODERN_ACCESSIBILITY_ELEMENT_STANDARD);
            let numeric = modern_accessibility_element(MODERN_ACCESSIBILITY_ELEMENT_STANDARD);
            assert!(!host.is_null());
            assert!(!root.is_null());
            assert!(!numeric.is_null());
            let root_frame = NSRect {
                origin: NSPoint { x: 0.0, y: 0.0 },
                size: NSSize {
                    width: 240.0,
                    height: 120.0,
                },
            };
            let numeric_frame = NSRect {
                origin: NSPoint { x: 8.0, y: 12.0 },
                size: NSSize {
                    width: 120.0,
                    height: 24.0,
                },
            };
            assert_eq!(
                configure_modern_accessibility_element_fixture(
                    root,
                    "AXGroup",
                    host,
                    &[numeric],
                    root_frame,
                    None,
                    None,
                    None,
                    None,
                ),
                YES
            );
            assert_eq!(
                configure_modern_accessibility_element_fixture(
                    numeric,
                    "AXIncrementor",
                    root,
                    &[],
                    numeric_frame,
                    Some("Gain"),
                    Some("Gain"),
                    Some("Numeric gain"),
                    Some(true),
                ),
                YES
            );

            assert_eq!(
                modern_accessibility_string(root, unsafe { sel(c"accessibilityRole") }).as_deref(),
                Some("AXGroup")
            );
            assert_eq!(
                modern_accessibility_string(numeric, unsafe { sel(c"accessibilityRole") })
                    .as_deref(),
                Some("AXIncrementor")
            );
            assert_eq!(
                unsafe { msg_id(root, sel(c"accessibilityParent")) },
                host,
                "the modern root parent must be the exact content-view host"
            );
            assert_eq!(
                unsafe { msg_id(numeric, sel(c"accessibilityParent")) },
                root
            );
            assert_eq!(modern_accessibility_children(root), vec![numeric]);
            assert!(modern_accessibility_children(numeric).is_empty());
            assert_eq!(
                unsafe { msg_rect(root, sel(c"accessibilityFrame")) },
                root_frame
            );
            assert_eq!(
                unsafe { msg_rect(numeric, sel(c"accessibilityFrame")) },
                numeric_frame
            );
            assert!(rect_is_finite(numeric_frame));
            assert!(numeric_frame.size.width > 0.0);
            assert!(numeric_frame.size.height > 0.0);
            assert_eq!(
                modern_accessibility_string(numeric, unsafe { sel(c"accessibilityLabel") })
                    .as_deref(),
                Some("Gain")
            );
            assert_eq!(
                modern_accessibility_string(numeric, unsafe { sel(c"accessibilityTitle") })
                    .as_deref(),
                Some("Gain")
            );
            assert_eq!(
                modern_accessibility_string(numeric, unsafe { sel(c"accessibilityHelp") })
                    .as_deref(),
                Some("Numeric gain")
            );
            assert_eq!(
                unsafe { msg_bool(numeric, sel(c"isAccessibilityElement")) },
                YES
            );
            assert_eq!(
                unsafe { msg_bool(numeric, sel(c"isAccessibilityEnabled")) },
                YES
            );

            assert_eq!(
                unsafe { radiant_native_set_accessibility_children(host, root) },
                YES
            );
            assert_eq!(modern_accessibility_children(host), vec![root]);
            let (readback, count) = accessibility_children_readback(host);
            assert_eq!((readback, count), (root, 1));
            assert_eq!(
                unsafe { radiant_native_clear_accessibility_children(host) },
                YES
            );
            release_modern_accessibility_element(root);
            release_modern_accessibility_element(numeric);
            release_accessibility_children_test_host(host);
        }

        #[test]
        fn modern_accessibility_configuration_failures_are_inert_before_host_attachment() {
            let host = accessibility_children_test_host(ACCESSIBILITY_CHILDREN_HOST_STANDARD);
            let frame = NSRect {
                origin: NSPoint { x: 0.0, y: 0.0 },
                size: NSSize {
                    width: 120.0,
                    height: 24.0,
                },
            };
            for kind in [
                MODERN_ACCESSIBILITY_ELEMENT_THROWING_SETTER,
                MODERN_ACCESSIBILITY_ELEMENT_THROWING_GETTER,
                MODERN_ACCESSIBILITY_ELEMENT_MISMATCHED_PARENT,
                MODERN_ACCESSIBILITY_ELEMENT_MISMATCHED_CHILDREN,
                MODERN_ACCESSIBILITY_ELEMENT_FILTERED,
                MODERN_ACCESSIBILITY_ELEMENT_UNSUPPORTED,
            ] {
                let element = modern_accessibility_element(kind);
                assert_eq!(
                    configure_modern_accessibility_element_fixture(
                        element,
                        "AXGroup",
                        host,
                        &[],
                        frame,
                        None,
                        None,
                        None,
                        None,
                    ),
                    NO,
                    "modern fixture {kind} must remain precommit"
                );
                assert!(modern_accessibility_children(host).is_empty());
                release_modern_accessibility_element(element);
            }

            let standard = modern_accessibility_element(MODERN_ACCESSIBILITY_ELEMENT_STANDARD);
            assert_eq!(
                configure_modern_accessibility_element_fixture(
                    null_mut(),
                    "AXGroup",
                    host,
                    &[],
                    frame,
                    None,
                    None,
                    None,
                    None,
                ),
                NO
            );
            assert_eq!(
                configure_modern_accessibility_element_fixture(
                    standard,
                    "AXGroup",
                    null_mut(),
                    &[],
                    frame,
                    None,
                    None,
                    None,
                    None,
                ),
                NO
            );
            assert_eq!(
                configure_modern_accessibility_element_fixture(
                    host,
                    "AXGroup",
                    host,
                    &[],
                    frame,
                    None,
                    None,
                    None,
                    None,
                ),
                NO,
                "a non-NSAccessibilityElement must not be configured"
            );
            assert!(modern_accessibility_children(host).is_empty());
            release_modern_accessibility_element(standard);
            release_accessibility_children_test_host(host);
        }

        #[test]
        fn native_publication_commits_attachment_and_layout_only_after_host_success() {
            let runtime =
                SurfaceRuntime::new(MappedNumericBridge::new(7.0), Vector2::new(240.0, 120.0));
            let host = accessibility_children_test_host(ACCESSIBILITY_CHILDREN_HOST_STANDARD);
            let mut adapter = native_publication_adapter_fixture(host);
            let ordinary = runtime.automation_snapshot();
            let targets = runtime.automation_target_snapshot();
            assert!(
                adapter
                    .publish_projection(&ordinary, &targets, &[], None)
                    .is_ok()
            );
            assert!(adapter.attached);
            assert_eq!(adapter.committed_objects, adapter.objects);
            assert_eq!(adapter.layout_notifications, 1);
            assert_eq!(adapter.value_notifications, 0);
            let (readback, count) = accessibility_children_readback(host);
            assert_eq!(count, 1);
            assert_eq!(
                readback,
                adapter
                    .callback_state
                    .borrow()
                    .projection
                    .nodes
                    .first()
                    .map_or(null_mut(), |node| node.object)
            );
            let expected_destroyed_notifications = adapter.objects.len();
            assert!(adapter.retire_published_objects());
            assert_eq!(adapter.objects.len(), 0);
            assert_eq!(
                adapter.destroyed_notifications, expected_destroyed_notifications,
                "committed objects receive one destruction notification after verified clear"
            );
            adapter.retire_published_objects();
            assert_eq!(
                adapter.destroyed_notifications,
                expected_destroyed_notifications
            );
            drop(runtime);
            drop(adapter);
            let (readback, count) = accessibility_children_readback(host);
            assert!(readback.is_null());
            assert_eq!(count, 0);
            release_accessibility_children_test_host(host);
        }

        #[test]
        fn native_publication_uses_modern_numeric_value_and_retains_identity() {
            let mut runtime =
                SurfaceRuntime::new(MappedNumericBridge::new(7.0), Vector2::new(240.0, 120.0));
            let host = accessibility_children_test_host(ACCESSIBILITY_CHILDREN_HOST_STANDARD);
            let mut adapter = native_publication_adapter_fixture(host);
            let initial_snapshot = runtime.automation_snapshot();
            let initial_targets = runtime.automation_target_snapshot();
            adapter
                .publish_projection(&initial_snapshot, &initial_targets, &[], None)
                .expect("the modern native projection should publish");

            let initial_projection = adapter.callback_state.borrow().projection.clone();
            let initial_numeric = initial_projection
                .nodes
                .iter()
                .find(|node| node.role == "AXIncrementor")
                .cloned()
                .expect("the production numeric node should be an AXIncrementor");
            let root = initial_projection
                .nodes
                .iter()
                .find(|node| node.kind == NativeNodeKind::Root)
                .expect("the production root should be present");
            assert_eq!(
                unsafe { msg_id(initial_numeric.object, sel(c"accessibilityParent")) },
                root.object
            );
            assert_eq!(
                modern_accessibility_children(root.object),
                vec![initial_numeric.object]
            );
            assert_eq!(
                modern_accessibility_string(initial_numeric.object, unsafe {
                    sel(c"accessibilityValue")
                })
                .as_deref(),
                Some("7")
            );
            assert_eq!(
                unsafe { msg_bool(initial_numeric.object, sel(c"isAccessibilityEnabled")) },
                YES
            );
            let initial_frame =
                unsafe { msg_rect(initial_numeric.object, sel(c"accessibilityFrame")) };
            assert!(rect_is_finite(initial_frame));
            assert!(initial_frame.size.width > 0.0);
            assert!(initial_frame.size.height > 0.0);
            let initial_object = initial_numeric.object;
            let initial_target = initial_targets
                .targets
                .iter()
                .find(|target| {
                    target.id
                        == initial_numeric
                            .action_target
                            .as_ref()
                            .expect("the numeric node should carry an action target")
                            .id
                })
                .cloned()
                .expect("the numeric action target should remain in the target snapshot");
            let request = adapter
                .numeric_accessibility_request(
                    initial_numeric.token,
                    initial_target,
                    NativeNumericAccessibilityAction::Increment,
                )
                .expect("the modern numeric node should admit increment");
            assert!(matches!(
                runtime.dispatch_numeric_accessibility_action(request),
                crate::runtime::NumericAccessibilityDispatchResult::Accepted { .. }
            ));

            let refreshed_snapshot = runtime.automation_snapshot();
            let refreshed_targets = runtime.automation_target_snapshot();
            adapter
                .publish_projection(&refreshed_snapshot, &refreshed_targets, &[], None)
                .expect("a value-only numeric refresh should remain published");
            let refreshed_numeric = adapter
                .callback_state
                .borrow()
                .projection
                .nodes
                .iter()
                .find(|node| node.token == initial_numeric.token)
                .cloned()
                .expect("the numeric object should remain in the refreshed projection");
            assert_eq!(refreshed_numeric.object, initial_object);
            assert_eq!(
                modern_accessibility_string(refreshed_numeric.object, unsafe {
                    sel(c"accessibilityValue")
                })
                .as_deref(),
                Some("8")
            );
            assert_eq!(adapter.value_notifications, 1);
            assert_eq!(adapter.layout_notifications, 1);
            assert!(adapter.retire_published_objects());
            drop(runtime);
            drop(adapter);
            release_accessibility_children_test_host(host);
        }

        #[test]
        fn native_publication_failure_is_inert_for_mismatched_and_throwing_hosts() {
            let runtime =
                SurfaceRuntime::new(MappedNumericBridge::new(7.0), Vector2::new(240.0, 120.0));
            let ordinary = runtime.automation_snapshot();
            let targets = runtime.automation_target_snapshot();
            for kind in [
                ACCESSIBILITY_CHILDREN_HOST_MISMATCHED,
                ACCESSIBILITY_CHILDREN_HOST_THROWING_SETTER,
                ACCESSIBILITY_CHILDREN_HOST_THROWING_GETTER,
            ] {
                let host = accessibility_children_test_host(kind);
                let mut adapter = native_publication_adapter_fixture(host);
                assert!(
                    adapter
                        .publish_projection(&ordinary, &targets, &[], None)
                        .is_err(),
                    "host fixture {kind} must reject the publication"
                );
                assert!(!adapter.attached);
                assert_eq!(adapter.layout_notifications, 0);
                assert_eq!(adapter.value_notifications, 0);
                assert_eq!(adapter.destroyed_notifications, 0);
                assert!(adapter.committed_objects.is_empty());
                assert!(adapter.callback_state.borrow().projection.nodes.is_empty());
                let quarantined_objects = adapter.objects.clone();
                drop(adapter);
                release_accessibility_children_test_host(host);
                if !quarantined_objects.is_empty() {
                    release_native_index_objects(&quarantined_objects);
                }
            }
        }

        #[test]
        fn native_retirement_quarantines_objects_when_host_clear_is_unverified() {
            let runtime =
                SurfaceRuntime::new(MappedNumericBridge::new(7.0), Vector2::new(240.0, 120.0));
            let ordinary = runtime.automation_snapshot();
            let targets = runtime.automation_target_snapshot();
            for kind in [
                ACCESSIBILITY_CHILDREN_HOST_IGNORING_CLEAR,
                ACCESSIBILITY_CHILDREN_HOST_THROWING_CLEAR,
            ] {
                let host = accessibility_children_test_host(kind);
                let mut adapter = native_publication_adapter_fixture(host);
                assert!(
                    adapter
                        .publish_projection(&ordinary, &targets, &[], None)
                        .is_ok(),
                    "host fixture {kind} must accept the initial publication"
                );
                let root = adapter
                    .callback_state
                    .borrow()
                    .projection
                    .nodes
                    .first()
                    .map_or(null_mut(), |node| node.object);
                let quarantined_objects = adapter.objects.clone();
                let (readback, count) = accessibility_children_readback(host);
                assert_eq!(count, 1);
                assert_eq!(readback, root);

                assert!(!adapter.retire_published_objects());
                assert!(!adapter.attached);
                assert_eq!(adapter.destroyed_notifications, 0);
                assert_eq!(adapter.objects, quarantined_objects);
                let (readback, count) = accessibility_children_readback(host);
                assert_eq!(count, 1);
                assert_eq!(readback, root);

                assert!(!adapter.retire_published_objects());
                assert_eq!(adapter.destroyed_notifications, 0);
                assert_eq!(adapter.objects, quarantined_objects);
                drop(adapter);
                release_accessibility_children_test_host(host);
                release_native_index_objects(&quarantined_objects);
            }
        }

        fn numeric_adapter_fixture(target: AutomationTarget) -> NativeSemanticAccessibilityAdapter {
            let node = numeric_callback_node(target, "7");
            numeric_adapter_fixture_with_projection(NativeCallbackProjection {
                nodes: vec![node],
                root_token: Some(91),
            })
        }

        fn numeric_adapter_fixture_with_projection(
            projection: NativeCallbackProjection,
        ) -> NativeSemanticAccessibilityAdapter {
            let callback_state =
                NativeSemanticCallbackState::new_for_test(WindowId::dummy(), 3, null_mut());
            let mut callback_state = callback_state;
            callback_state.projection = projection;
            NativeSemanticAccessibilityAdapter {
                view: null_mut(),
                callback_state: Box::new(RefCell::new(callback_state)),
                objects: Vec::new(),
                committed_objects: Vec::new(),
                tokens: NativeTokenLedger::default(),
                lease: None,
                generation: 3,
                window_generation: 1,
                transform: None,
                current_containers: Vec::new(),
                active_ranges: Vec::new(),
                attached: true,
                layout_notifications: 0,
                value_notifications: 0,
                destroyed_notifications: 0,
            }
        }

        const TEST_ROOT_TOKEN: u64 = 1;
        const TEST_NUMERIC_TOKEN: u64 = 2;

        fn runtime_numeric_adapter_fixture(
            snapshot: &GuiAutomationSnapshot,
            numeric_node: &AutomationNodeSnapshot,
            target: AutomationTarget,
        ) -> NativeSemanticAccessibilityAdapter {
            let root_bounds = AutomationBounds {
                x: 0.0,
                y: 0.0,
                width: snapshot.viewport_width as f32,
                height: snapshot.viewport_height as f32,
            };
            let projection = NativeCallbackProjection {
                nodes: vec![
                    NativeCallbackNode {
                        object: 90_usize as Id,
                        token: TEST_ROOT_TOKEN,
                        kind: NativeNodeKind::Root,
                        parent: None,
                        children: vec![TEST_NUMERIC_TOKEN],
                        logical_children: Vec::new(),
                        role: native_role(NativeNodeKind::Root, AutomationRole::Root, false),
                        frame: native_test_frame(root_bounds),
                        label: None,
                        description: None,
                        value: None,
                        action_target: None,
                        logical_count: None,
                    },
                    NativeCallbackNode {
                        object: 91_usize as Id,
                        token: TEST_NUMERIC_TOKEN,
                        kind: NativeNodeKind::Ordinary,
                        parent: Some(TEST_ROOT_TOKEN),
                        children: Vec::new(),
                        logical_children: Vec::new(),
                        role: native_role(NativeNodeKind::Ordinary, numeric_node.role, true),
                        frame: native_test_frame(numeric_node.bounds),
                        label: numeric_node.semantics.label.clone(),
                        description: numeric_node.semantics.description.clone(),
                        value: numeric_node.semantics.value_text.clone(),
                        action_target: Some(target.clone()),
                        logical_count: None,
                    },
                ],
                root_token: Some(TEST_ROOT_TOKEN),
            };
            let mut adapter = numeric_adapter_fixture_with_projection(projection);
            adapter.transform = Some(NativeCoordinateTransform::for_test(NSRect {
                origin: NSPoint { x: 0.0, y: 0.0 },
                size: NSSize {
                    width: f64::from(snapshot.viewport_width),
                    height: f64::from(snapshot.viewport_height),
                },
            }));
            adapter.tokens.next = TEST_NUMERIC_TOKEN;
            adapter.tokens.root = Some((TEST_ROOT_TOKEN, adapter.lease, adapter.window_generation));
            adapter.tokens.ordinary.push(NativeOrdinaryToken {
                token: TEST_NUMERIC_TOKEN,
                id: target.id.clone(),
                parent: Some(TEST_ROOT_TOKEN),
                lease: adapter.lease,
                window_generation: adapter.window_generation,
            });
            adapter
        }

        fn automation_node_for_id<'a>(
            node: &'a AutomationNodeSnapshot,
            id: &AutomationNodeId,
        ) -> Option<&'a AutomationNodeSnapshot> {
            if node.id == *id {
                return Some(node);
            }
            node.children
                .iter()
                .find_map(|child| automation_node_for_id(child, id))
        }

        fn native_test_frame(bounds: AutomationBounds) -> NSRect {
            NSRect {
                origin: NSPoint {
                    x: f64::from(bounds.x),
                    y: f64::from(bounds.y),
                },
                size: NSSize {
                    width: f64::from(bounds.width),
                    height: f64::from(bounds.height),
                },
            }
        }

        fn numeric_spec_fixture(target: AutomationTarget, value: &str) -> (NativeNodeSpec, NSRect) {
            let (mut node, _) = numeric_target_fixture();
            node.semantics.value_text = Some(value.to_owned());
            node.value = Some(value.to_owned());
            (
                NativeNodeSpec {
                    token: 91,
                    kind: NativeNodeKind::Ordinary,
                    parent: Some(1),
                    children: Vec::new(),
                    logical_children: Vec::new(),
                    bounds: node.bounds,
                    semantics: node.semantics,
                    action_target: Some(target),
                    logical_count: None,
                },
                NSRect {
                    origin: NSPoint { x: 8.0, y: 12.0 },
                    size: NSSize {
                        width: 120.0,
                        height: 24.0,
                    },
                },
            )
        }

        #[test]
        fn mismatched_explicit_retry_is_inert_after_accepted_semantic_projection() {
            let provider = Rc::new(CountingRangeProvider {
                calls: Cell::new(0),
            });
            let parts = VirtualLayoutParts::new(
                Rc::new(TestVirtualLayoutPolicy),
                VirtualLayoutPolicyIdentity::new("native-semantic-retry-test"),
                VirtualLayoutOverscan::new(0.0, 0.0).expect("finite test overscan"),
                VirtualLayoutBudget::new(4),
                VirtualLayoutRevisions::default(),
                Rc::new(|| scroll(spacer::<()>())),
                Rc::new(|_| text::<()>("semantic item")),
                Rc::new(|_| VirtualLayoutPolicyIdentity::new("native-semantic-item")),
            )
            .with_semantic_range_provider(provider.clone())
            .with_semantic_cardinality(
                crate::application::virtual_layout::VirtualLayoutSemanticCardinality::new(4, 1),
            );
            let mut runtime = SurfaceRuntime::new(
                TestBridge {
                    surface: virtual_layout_from_parts(parts).into_surface(),
                },
                Vector2::new(160.0, 80.0),
            );
            let containers = runtime.native_semantic_containers();
            assert_eq!(containers.len(), 1);
            let current = containers[0].clone();
            let provider_calls = provider.calls.get();
            let ordinary_before = runtime.automation_snapshot();

            let session = SemanticAutomationSessionHandle {
                runtime_id: 0xfeed,
                generation: 7,
            };
            let container_token = NativeContainerToken {
                token: 41,
                container_id: current.container_id,
                mount_generation: current.mount_generation,
                registration_generation: current.registration_generation,
                provider_generation: current.provider_generation,
                coordinate_authority: current.coordinate_authority.clone(),
                cardinality: current.cardinality,
                lease: Some(session),
                window_generation: 5,
            };
            let item_tokens = [42_u64, 43_u64];
            let projection = NativeCallbackProjection {
                nodes: vec![
                    NativeCallbackNode {
                        object: 40_usize as Id,
                        token: 40,
                        kind: NativeNodeKind::Root,
                        parent: None,
                        children: vec![container_token.token],
                        logical_children: Vec::new(),
                        role: "AXGroup",
                        frame: NSRect {
                            origin: NSPoint { x: 0.0, y: 0.0 },
                            size: NSSize {
                                width: 160.0,
                                height: 80.0,
                            },
                        },
                        label: None,
                        description: None,
                        value: None,
                        action_target: None,
                        logical_count: None,
                    },
                    NativeCallbackNode {
                        object: container_token.token as Id,
                        token: container_token.token,
                        kind: NativeNodeKind::Container,
                        parent: Some(40),
                        children: item_tokens.to_vec(),
                        logical_children: item_tokens
                            .iter()
                            .enumerate()
                            .map(|(index, token)| (index, *token))
                            .collect(),
                        role: "AXGroup",
                        frame: NSRect {
                            origin: NSPoint { x: 0.0, y: 0.0 },
                            size: NSSize {
                                width: 160.0,
                                height: 80.0,
                            },
                        },
                        label: Some(String::from("accepted semantic container")),
                        description: None,
                        value: None,
                        action_target: None,
                        logical_count: Some(current.cardinality.logical_item_count),
                    },
                    NativeCallbackNode {
                        object: item_tokens[0] as Id,
                        token: item_tokens[0],
                        kind: NativeNodeKind::Item,
                        parent: Some(container_token.token),
                        children: Vec::new(),
                        logical_children: Vec::new(),
                        role: "AXGroup",
                        frame: NSRect {
                            origin: NSPoint { x: 0.0, y: 0.0 },
                            size: NSSize {
                                width: 100.0,
                                height: 20.0,
                            },
                        },
                        label: Some(String::from("item zero")),
                        description: None,
                        value: None,
                        action_target: None,
                        logical_count: None,
                    },
                    NativeCallbackNode {
                        object: item_tokens[1] as Id,
                        token: item_tokens[1],
                        kind: NativeNodeKind::Item,
                        parent: Some(container_token.token),
                        children: Vec::new(),
                        logical_children: Vec::new(),
                        role: "AXGroup",
                        frame: NSRect {
                            origin: NSPoint { x: 0.0, y: 20.0 },
                            size: NSSize {
                                width: 100.0,
                                height: 20.0,
                            },
                        },
                        label: Some(String::from("item one")),
                        description: None,
                        value: None,
                        action_target: None,
                        logical_count: None,
                    },
                ],
                root_token: Some(40),
            };
            let mut callback_state =
                NativeSemanticCallbackState::new_for_test(WindowId::dummy(), 1, null_mut());
            let accepted_key = NativeQueryKey {
                token: container_token.token,
                start_index: 0,
                max_count: 2,
            };
            let retry_key = NativeQueryKey {
                max_count: 3,
                ..accepted_key
            };
            callback_state.projection = projection;
            callback_state.in_flight.push(retry_key);
            let mut adapter = NativeSemanticAccessibilityAdapter {
                view: null_mut(),
                callback_state: Box::new(RefCell::new(callback_state)),
                objects: Vec::new(),
                committed_objects: Vec::new(),
                tokens: NativeTokenLedger {
                    next: item_tokens[1],
                    root: Some((40, Some(session), 5)),
                    ordinary: Vec::new(),
                    containers: vec![container_token.clone()],
                    items: item_tokens
                        .iter()
                        .enumerate()
                        .map(|(index, token)| NativeItemToken {
                            token: *token,
                            container_id: current.container_id,
                            mount_generation: current.mount_generation,
                            logical_index: index,
                            key: VirtualLayoutItemKey::new(index as u32),
                            coordinate_authority: current.coordinate_authority.clone(),
                            fences: crate::runtime::NormalizedSemanticPublicationFenceSet::default(
                            ),
                            lease: Some(session),
                            window_generation: 5,
                        })
                        .collect(),
                },
                lease: Some(session),
                generation: 1,
                window_generation: 5,
                transform: None,
                current_containers: vec![current],
                active_ranges: vec![NativeActiveRange {
                    key: accepted_key,
                    container: container_token,
                    length: 2,
                }],
                attached: true,
                layout_notifications: 9,
                value_notifications: 0,
                destroyed_notifications: 0,
            };

            let tokens_before = adapter.tokens.clone();
            let objects_before = adapter.objects.clone();
            let lease_before = adapter.lease;
            let active_ranges_before = adapter.active_ranges.clone();
            let containers_before = adapter.current_containers.clone();
            let attached_before = adapter.attached;
            let layout_notifications_before = adapter.layout_notifications;
            let state_before = adapter.callback_state.borrow();
            let projection_before = state_before.projection.clone();
            let deferred_before = state_before.deferred.clone();
            let unavailable_before = state_before.last_unavailable;
            drop(state_before);

            adapter.handle_query(
                &mut runtime,
                NativeSemanticAccessibilityQuery::ChildrenRange {
                    token: retry_key.token,
                    start_index: retry_key.start_index,
                    max_count: retry_key.max_count,
                    explicit_retry: true,
                },
            );

            assert_eq!(provider.calls.get(), provider_calls);
            assert_eq!(runtime.automation_snapshot(), ordinary_before);
            let opened = runtime
                .open_semantic_automation_session()
                .expect("the mismatched retry must not open a semantic session");
            runtime
                .close_semantic_automation_session(opened)
                .expect("the probe session should close cleanly");
            assert_eq!(adapter.tokens, tokens_before);
            assert_eq!(adapter.objects, objects_before);
            assert_eq!(adapter.lease, lease_before);
            assert_eq!(adapter.active_ranges, active_ranges_before);
            assert_eq!(adapter.current_containers, containers_before);
            assert_eq!(adapter.attached, attached_before);
            assert_eq!(adapter.layout_notifications, layout_notifications_before);
            let state_after = adapter.callback_state.borrow();
            assert_eq!(state_after.projection, projection_before);
            assert!(state_after.in_flight.is_empty());
            assert_eq!(state_after.deferred, deferred_before);
            assert_eq!(state_after.last_unavailable, unavailable_before);
        }

        #[test]
        fn checked_range_arithmetic_is_bounded() {
            assert_eq!(normalize_range(0, 10, 0, 1, 0), None);
            assert_eq!(normalize_range(4, 10, 4, 1, 0), None);
            assert_eq!(normalize_range(4, 2, 1, 8, 0), Some(2));
            assert_eq!(normalize_range(2000, 2000, 0, 2000, 0), Some(1024));
            assert_eq!(normalize_range(10, 10, usize::MAX, 1, 0), None);
            assert_eq!(normalize_range(10, 10, 0, 2, 1024), None);
        }

        #[test]
        fn zero_cardinality_is_not_a_range_demand() {
            assert_eq!(normalize_range(0, 10, 0, 10, 0), None);
        }

        #[test]
        fn positive_cardinality_without_provider_is_vetoed_before_runtime() {
            let positive = NativeSemanticContainerSnapshot {
                container_id: 1,
                mount_generation: 1,
                registration_generation: 1,
                provider_generation: 1,
                coordinate_authority: NativeSemanticCoordinateAuthority::Logical,
                cardinality:
                    crate::application::virtual_layout::VirtualLayoutSemanticCardinality::new(1, 1),
                has_range_provider: false,
                max_entries: 10,
            };
            assert!(!native_range_query_is_admitted(
                positive.cardinality,
                positive.has_range_provider,
                Some(1),
            ));
        }

        #[test]
        fn active_range_staging_retains_distinct_containers() {
            let a = test_active_range(1, 0, 2);
            let b = test_active_range(2, 4, 3);
            assert_eq!(
                stage_active_range(std::slice::from_ref(&a), b.clone()),
                Some(vec![a, b])
            );
        }

        #[test]
        fn active_range_staging_replaces_only_the_matching_container() {
            let a = test_active_range(1, 0, 2);
            let b = test_active_range(2, 4, 3);
            let replacement = test_active_range(1, 8, 1);
            assert_eq!(
                stage_active_range(&[a, b.clone()], replacement.clone()),
                Some(vec![replacement, b])
            );
        }

        #[test]
        fn active_range_staging_rejects_zero_length_cap_and_overflow() {
            assert_eq!(stage_active_range(&[], test_active_range(1, 0, 0)), None);
            let at_cap = test_active_range(1, 0, MAX_NATIVE_ITEMS);
            assert!(stage_active_range(&[], at_cap.clone()).is_some());
            assert_eq!(
                stage_active_range(&[at_cap], test_active_range(2, 0, 1)),
                None
            );
            assert_eq!(
                stage_active_range(
                    &[test_active_range(1, 0, usize::MAX)],
                    test_active_range(2, 0, 1),
                ),
                None
            );
        }

        #[test]
        fn semantic_demands_preserve_complete_active_range_order_and_bounds() {
            let first = test_active_range(1, 7, 2);
            let second = test_active_range(2, 19, 4);
            let demands = semantic_demands_for_active_ranges(
                &[first, second],
                &[test_handle(1), test_handle(2)],
            )
            .expect("all active ranges should have live handles");

            assert_eq!(
                demands,
                vec![
                    SemanticAutomationDemand::range(test_handle(1), 7, 2),
                    SemanticAutomationDemand::range(test_handle(2), 19, 4),
                ]
            );
        }

        #[test]
        fn semantic_demands_fail_when_an_active_range_handle_is_missing() {
            let first = test_active_range(1, 7, 2);
            let second = test_active_range(2, 19, 4);

            assert_eq!(
                semantic_demands_for_active_ranges(&[first, second], &[test_handle(1)]),
                None
            );
        }

        #[test]
        fn baseline_refresh_replaces_any_previous_semantic_projection_with_ordinary() {
            let reasons = [
                SemanticAutomationFallbackReason::NoDemand,
                SemanticAutomationFallbackReason::IncompleteDemandSet,
                SemanticAutomationFallbackReason::NoProvider,
                SemanticAutomationFallbackReason::Unsupported,
                SemanticAutomationFallbackReason::DataUnavailable,
                SemanticAutomationFallbackReason::Deferred,
                SemanticAutomationFallbackReason::Rejected,
                SemanticAutomationFallbackReason::Malformed,
                SemanticAutomationFallbackReason::Stale,
                SemanticAutomationFallbackReason::Invalidated,
                SemanticAutomationFallbackReason::CounterOverflow,
            ];
            for reason in reasons {
                assert!(!semantic_projection_is_eligible(
                    SemanticAutomationRefreshStatus::Baseline { reason },
                    Some(SemanticAutomationRefreshStatus::Published),
                ));
                assert!(!semantic_projection_is_eligible(
                    SemanticAutomationRefreshStatus::Baseline { reason },
                    Some(SemanticAutomationRefreshStatus::Retained { reason }),
                ));
            }
        }

        #[test]
        fn only_exact_published_or_retained_statuses_keep_semantic_projection() {
            assert!(semantic_projection_is_eligible(
                SemanticAutomationRefreshStatus::Published,
                Some(SemanticAutomationRefreshStatus::Published),
            ));
            assert!(!semantic_projection_is_eligible(
                SemanticAutomationRefreshStatus::Published,
                None,
            ));
            assert!(!semantic_projection_is_eligible(
                SemanticAutomationRefreshStatus::Published,
                Some(SemanticAutomationRefreshStatus::Retained {
                    reason: SemanticAutomationFallbackReason::Deferred,
                }),
            ));
            assert!(semantic_projection_is_eligible(
                SemanticAutomationRefreshStatus::Retained {
                    reason: SemanticAutomationFallbackReason::Deferred,
                },
                Some(SemanticAutomationRefreshStatus::Retained {
                    reason: SemanticAutomationFallbackReason::Deferred,
                }),
            ));
            assert!(!semantic_projection_is_eligible(
                SemanticAutomationRefreshStatus::Retained {
                    reason: SemanticAutomationFallbackReason::Deferred,
                },
                Some(SemanticAutomationRefreshStatus::Retained {
                    reason: SemanticAutomationFallbackReason::Stale,
                }),
            ));
        }

        #[test]
        fn active_range_retry_requires_exact_key_and_container() {
            let active = test_active_range(1, 4, 3);
            assert!(active_range_matches_retry(
                std::slice::from_ref(&active),
                active.key,
                &active.container,
                active.length,
            ));
            assert!(!active_range_matches_retry(
                std::slice::from_ref(&active),
                NativeQueryKey {
                    start_index: active.key.start_index + 1,
                    ..active.key
                },
                &active.container,
                active.length,
            ));
            let mut changed_container = active.container.clone();
            changed_container.mount_generation += 1;
            assert!(!active_range_matches_retry(
                std::slice::from_ref(&active),
                active.key,
                &changed_container,
                active.length,
            ));
        }

        #[test]
        fn active_range_retry_rejects_an_exact_key_with_a_different_normalized_length() {
            let active = test_active_range(1, 4, 3);
            assert!(!active_range_matches_retry(
                std::slice::from_ref(&active),
                active.key,
                &active.container,
                active.length - 1,
            ));
        }

        #[test]
        fn retry_decision_rejects_mismatches_without_refreshing_the_complete_set() {
            assert_eq!(
                native_semantic_query_action(true, true),
                NativeSemanticQueryAction::ExactRetry
            );
            assert_eq!(
                native_semantic_query_action(true, false),
                NativeSemanticQueryAction::RejectRetry
            );
            assert_eq!(
                native_semantic_query_action(false, false),
                NativeSemanticQueryAction::CompleteRefresh
            );
        }

        #[test]
        fn callback_query_keys_coalesce_and_retry_only_after_deferred() {
            let key = NativeQueryKey {
                token: 4,
                start_index: 0,
                max_count: 2,
            };
            assert_eq!(query_retry_mode(key, &[key], &[], 2, &[]), None);
            assert_eq!(query_retry_mode(key, &[], &[key], 2, &[]), Some(true));
            assert_eq!(query_retry_mode(key, &[], &[], 2, &[]), Some(false));
            assert_eq!(query_retry_mode(key, &[], &[], 2, &[(0, 9), (1, 10)]), None);
            assert_eq!(query_retry_mode(key, &[], &[], 0, &[]), None);
            assert_eq!(
                query_retry_mode(
                    NativeQueryKey {
                        start_index: 2,
                        ..key
                    },
                    &[],
                    &[],
                    2,
                    &[],
                ),
                None
            );
            assert_eq!(
                query_retry_mode(
                    NativeQueryKey {
                        start_index: 0,
                        ..key
                    },
                    &[],
                    &[],
                    110,
                    &[(100, 1000), (101, 1001)],
                ),
                Some(false)
            );
        }

        #[test]
        fn retained_child_ranges_use_sidecar_logical_indices_not_compact_offsets() {
            let children = (100..110)
                .map(|logical_index| (logical_index, logical_index as u64 + 1_000))
                .collect::<Vec<_>>();
            assert_eq!(
                retained_logical_child_tokens(&children, 110, 100, 10),
                Some((1_100..1_110).collect::<Vec<_>>())
            );
            assert_eq!(retained_logical_child_tokens(&children, 110, 0, 10), None);
            assert!(!logical_child_range_is_retained(&children, 110, 0, 10));
            assert!(logical_child_range_is_retained(&children, 110, 100, 10));
        }

        #[test]
        fn complete_virtual_child_tokens_requires_an_exact_projection() {
            let mut node = NativeCallbackNode {
                object: null_mut(),
                token: 1,
                kind: NativeNodeKind::Container,
                parent: None,
                children: vec![10, 11, 12],
                logical_children: vec![(0, 10), (1, 11), (2, 12)],
                role: "AXGroup",
                frame: NSRect {
                    origin: NSPoint { x: 0.0, y: 0.0 },
                    size: NSSize {
                        width: 1.0,
                        height: 1.0,
                    },
                },
                label: None,
                description: None,
                value: None,
                action_target: None,
                logical_count: Some(3),
            };
            assert_eq!(complete_virtual_child_tokens(&node), Some(vec![10, 11, 12]));

            node.children = (1_000..1_010).collect();
            node.logical_children = (100..110)
                .map(|logical_index| (logical_index, logical_index as u64 + 900))
                .collect();
            node.logical_count = Some(110);
            assert_eq!(complete_virtual_child_tokens(&node), None);

            node.children = vec![10, 99, 12];
            node.logical_children = vec![(0, 10), (1, 11), (2, 12)];
            node.logical_count = Some(3);
            assert_eq!(complete_virtual_child_tokens(&node), None);
        }

        #[test]
        fn native_index_of_child_is_registered_with_exact_abi_and_dispatches() {
            let class = native_class().expect("the native accessibility class should exist");
            let selector = unsafe { sel(c"accessibilityIndexOfChild:") };
            let method = unsafe { class_getInstanceMethod(class.class(), selector) };
            assert!(!method.is_null());
            assert_eq!(
                unsafe { method_getImplementation(method) },
                native_index_of_child as *const c_void
            );
            let encoding = unsafe { method_getTypeEncoding(method) };
            assert!(!encoding.is_null());
            assert_eq!(
                unsafe { CStr::from_ptr(encoding) },
                INDEX_OF_CHILD_METHOD_TYPE
            );
            assert_eq!(INDEX_OF_CHILD_METHOD_TYPE.to_bytes_with_nul(), b"Q@:@\0");
            assert_eq!(NS_NOT_FOUND, isize::MAX as usize);
            assert_ne!(NS_NOT_FOUND, usize::MAX);

            let projection = NativeCallbackProjection {
                nodes: vec![
                    index_test_node(
                        101,
                        1,
                        NativeNodeKind::Root,
                        None,
                        vec![2],
                        Vec::new(),
                        None,
                    ),
                    index_test_node(
                        102,
                        2,
                        NativeNodeKind::Ordinary,
                        Some(1),
                        Vec::new(),
                        Vec::new(),
                        None,
                    ),
                ],
                root_token: Some(1),
            };
            let (objects, callback_state) = native_index_projection_fixture(projection);
            assert_eq!(unsafe { msg_usize_id(objects[0], selector, objects[1]) }, 0);

            let throwing_child = throwing_foundation_object();
            assert_eq!(
                unsafe { msg_usize_id(objects[0], selector, throwing_child) },
                NS_NOT_FOUND,
                "the child argument must remain an opaque identity"
            );
            unsafe { msg_void(throwing_child, sel(c"release")) };
            release_native_index_objects(&objects);
            drop(callback_state);
        }

        #[test]
        fn native_accessibility_notifies_when_destroyed_is_registered_with_exact_abi() {
            let class = native_class().expect("the native accessibility class should exist");
            let selector = unsafe { sel(c"accessibilityNotifiesWhenDestroyed") };
            let method = unsafe { class_getInstanceMethod(class.class(), selector) };
            assert!(!method.is_null());
            assert_eq!(
                unsafe { method_getImplementation(method) },
                native_accessibility_notifies_when_destroyed as *const c_void
            );
            let encoding = unsafe { method_getTypeEncoding(method) };
            assert!(!encoding.is_null());
            assert_eq!(
                unsafe { CStr::from_ptr(encoding) },
                ACCESSIBILITY_NOTIFIES_WHEN_DESTROYED_METHOD_TYPE
            );
            assert_eq!(
                ACCESSIBILITY_NOTIFIES_WHEN_DESTROYED_METHOD_TYPE.to_bytes_with_nul(),
                b"c@:\0"
            );
        }

        #[test]
        fn native_accessibility_notifies_when_destroyed_is_state_free_through_retirement() {
            let mut adapter = native_destroyed_callback_adapter_fixture();
            let class = native_class().expect("the native accessibility class should exist");
            let selector = unsafe { sel(c"accessibilityNotifiesWhenDestroyed") };
            let object = adapter.objects[0];

            let before = native_destroyed_callback_snapshot(&adapter);
            assert_eq!(unsafe { msg_bool(object, selector) }, YES);
            assert_eq!(native_destroyed_callback_snapshot(&adapter), before);

            let borrowed = adapter.callback_state.borrow_mut();
            assert_eq!(unsafe { msg_bool(object, selector) }, YES);
            drop(borrowed);
            assert_eq!(native_destroyed_callback_snapshot(&adapter), before);

            // Retirement clears the ivar before the object can synchronously
            // invoke this selector; the callback must remain an unconditional
            // YES even in that state.
            unsafe { object_setIvar(object, class.state_ivar(), null_mut()) };
            assert_eq!(unsafe { msg_bool(object, selector) }, YES);
            assert_eq!(native_destroyed_callback_snapshot(&adapter), before);

            adapter.retire_published_objects();
            assert!(adapter.objects.is_empty());
            assert_eq!(adapter.destroyed_notifications, 1);

            let after_first_retirement = native_destroyed_callback_snapshot(&adapter);
            adapter.retire_published_objects();
            let after_second_retirement = native_destroyed_callback_snapshot(&adapter);
            assert_eq!(
                after_second_retirement.destroyed_notifications,
                after_first_retirement.destroyed_notifications,
                "a drained retirement must not post a second AXUIElementDestroyed notification"
            );
            assert_eq!(
                after_second_retirement.objects,
                after_first_retirement.objects
            );
            assert_eq!(
                after_second_retirement.projection,
                after_first_retirement.projection
            );
            assert_eq!(
                after_second_retirement.in_flight,
                after_first_retirement.in_flight
            );
            assert_eq!(
                after_second_retirement.deferred,
                after_first_retirement.deferred
            );
            assert_eq!(
                after_second_retirement.pending_numeric_actions,
                after_first_retirement.pending_numeric_actions
            );
            assert_eq!(
                after_second_retirement.active_ranges,
                after_first_retirement.active_ranges
            );
            assert_eq!(after_second_retirement.lease, after_first_retirement.lease);
            assert_eq!(
                after_second_retirement.tokens,
                after_first_retirement.tokens
            );
            assert_eq!(
                after_second_retirement.current_containers,
                after_first_retirement.current_containers
            );
            assert_eq!(
                after_second_retirement.layout_notifications,
                after_first_retirement.layout_notifications
            );
            assert_eq!(
                after_second_retirement.value_notifications,
                after_first_retirement.value_notifications
            );
        }

        #[test]
        fn native_index_of_child_uses_direct_positions_and_sparse_logical_indices() {
            let projection = index_topology_projection();
            let root = 101_usize as Id;
            let ordinary = 102_usize as Id;
            let container = 103_usize as Id;
            let ordinary_child = 104_usize as Id;
            let item = 106_usize as Id;
            let item_child = 107_usize as Id;
            let ordinary_container_child = 108_usize as Id;

            assert_eq!(
                native_index_of_child_in_projection(&projection, root, ordinary),
                0
            );
            assert_eq!(
                native_index_of_child_in_projection(&projection, root, container),
                1
            );
            assert_eq!(
                native_index_of_child_in_projection(&projection, ordinary, ordinary_child),
                0
            );
            assert_eq!(
                native_index_of_child_in_projection(&projection, item, item_child),
                0
            );
            assert_eq!(
                native_index_of_child_in_projection(&projection, container, item),
                100,
                "a container returns the retained sparse logical index"
            );
            assert_eq!(
                native_index_of_child_in_projection(
                    &projection,
                    container,
                    ordinary_container_child
                ),
                NS_NOT_FOUND,
                "ordinary children are not a container logical mapping"
            );
        }

        #[test]
        fn native_index_of_child_rejects_invalid_or_ambiguous_topology_evidence() {
            let projection = index_topology_projection();
            let root = 101_usize as Id;
            let ordinary = 102_usize as Id;
            let ordinary_child = 104_usize as Id;
            let container = 103_usize as Id;
            let item = 106_usize as Id;
            let item_child = 107_usize as Id;
            let ordinary_container_child = 108_usize as Id;

            for (receiver, child) in [
                (null_mut(), item),
                (root, null_mut()),
                (root, 999_usize as Id),
                (999_usize as Id, item),
                (root, root),
                (root, ordinary_child),
                (ordinary, container),
                (item, container),
            ] {
                assert_eq!(
                    native_index_of_child_in_projection(&projection, receiver, child),
                    NS_NOT_FOUND
                );
            }

            let mut wrong_parent = projection.clone();
            wrong_parent
                .nodes
                .iter_mut()
                .find(|node| node.token == 7)
                .expect("the item child should exist")
                .parent = Some(2);
            assert_eq!(
                native_index_of_child_in_projection(&wrong_parent, item, item_child),
                NS_NOT_FOUND
            );

            let mut wrong_kind = projection.clone();
            wrong_kind
                .nodes
                .iter_mut()
                .find(|node| node.token == 3)
                .expect("the container should exist")
                .logical_children = vec![(100, 8)];
            assert_eq!(
                native_index_of_child_in_projection(
                    &wrong_kind,
                    container,
                    ordinary_container_child
                ),
                NS_NOT_FOUND
            );

            let mut out_of_count = projection.clone();
            out_of_count
                .nodes
                .iter_mut()
                .find(|node| node.token == 3)
                .expect("the container should exist")
                .logical_children = vec![(110, 6)];
            assert_eq!(
                native_index_of_child_in_projection(&out_of_count, container, item),
                NS_NOT_FOUND
            );

            let mut missing_mapping = projection.clone();
            missing_mapping
                .nodes
                .iter_mut()
                .find(|node| node.token == 3)
                .expect("the container should exist")
                .logical_children = vec![(100, 999)];
            assert_eq!(
                native_index_of_child_in_projection(&missing_mapping, container, item),
                NS_NOT_FOUND
            );

            let mut malformed = projection.clone();
            malformed
                .nodes
                .iter_mut()
                .find(|node| node.token == 3)
                .expect("the container should exist")
                .logical_children = vec![(101, 6), (100, 6)];
            assert_eq!(
                native_index_of_child_in_projection(&malformed, container, item),
                NS_NOT_FOUND
            );

            let mut sentinel_collision = projection.clone();
            sentinel_collision
                .nodes
                .iter_mut()
                .find(|node| node.token == 3)
                .expect("the container should exist")
                .logical_children = vec![(NS_NOT_FOUND, 6)];
            assert_eq!(
                native_index_of_child_in_projection(&sentinel_collision, container, item),
                NS_NOT_FOUND
            );

            let mut duplicate_direct_mapping = projection.clone();
            duplicate_direct_mapping
                .nodes
                .iter_mut()
                .find(|node| node.token == 1)
                .expect("the root should exist")
                .children = vec![2, 2];
            assert_eq!(
                native_index_of_child_in_projection(&duplicate_direct_mapping, root, ordinary),
                NS_NOT_FOUND
            );

            let mut ambiguous_object = projection.clone();
            let duplicate_object = ambiguous_object
                .nodes
                .iter()
                .find(|node| node.token == 2)
                .expect("the ordinary node should exist")
                .object;
            ambiguous_object
                .nodes
                .iter_mut()
                .find(|node| node.token == 4)
                .expect("the ordinary child should exist")
                .object = duplicate_object;
            assert_eq!(
                native_index_of_child_in_projection(&ambiguous_object, root, ordinary),
                NS_NOT_FOUND
            );
        }

        #[test]
        fn native_index_of_child_is_immutable_and_handles_borrow_retirement_and_stale_state() {
            let projection = NativeCallbackProjection {
                nodes: vec![
                    index_test_node(
                        101,
                        1,
                        NativeNodeKind::Root,
                        None,
                        vec![2],
                        Vec::new(),
                        None,
                    ),
                    index_test_node(
                        102,
                        2,
                        NativeNodeKind::Ordinary,
                        Some(1),
                        Vec::new(),
                        Vec::new(),
                        None,
                    ),
                ],
                root_token: Some(1),
            };
            let (objects, callback_state) = native_index_projection_fixture(projection);
            let before = callback_state.borrow().projection.clone();
            assert_eq!(
                unsafe { msg_usize_id(objects[0], sel(c"accessibilityIndexOfChild:"), objects[1]) },
                0
            );
            assert_eq!(callback_state.borrow().projection, before);

            let borrowed = callback_state.borrow_mut();
            assert_eq!(
                unsafe { msg_usize_id(objects[0], sel(c"accessibilityIndexOfChild:"), objects[1]) },
                NS_NOT_FOUND
            );
            drop(borrowed);

            let class = native_class().expect("the native accessibility class should exist");
            let state_ptr = (&*callback_state as *const RefCell<NativeSemanticCallbackState>) as Id;
            let foreign_class = unsafe {
                class_named(c"NSObject").expect("the Foundation NSObject class should exist")
            };
            let foreign = unsafe { msg_id(foreign_class, sel(c"alloc")) };
            let foreign = unsafe { msg_id(foreign, sel(c"init")) };
            assert_eq!(
                native_index_of_child(foreign, null_mut(), objects[1]),
                NS_NOT_FOUND
            );
            let stale = unsafe { msg_id(class.class(), sel(c"alloc")) };
            let stale = unsafe { msg_id(stale, sel(c"init")) };
            unsafe { object_setIvar(stale, class.state_ivar(), state_ptr) };
            assert_eq!(
                unsafe { msg_usize_id(stale, sel(c"accessibilityIndexOfChild:"), objects[1],) },
                NS_NOT_FOUND
            );
            unsafe { object_setIvar(objects[0], class.state_ivar(), null_mut()) };
            assert_eq!(
                unsafe { msg_usize_id(objects[0], sel(c"accessibilityIndexOfChild:"), objects[1]) },
                NS_NOT_FOUND
            );
            unsafe { msg_void(foreign, sel(c"release")) };
            unsafe { msg_void(stale, sel(c"release")) };
            release_native_index_objects(&objects);
            drop(callback_state);
        }

        #[test]
        fn native_index_of_child_ffi_boundary_uses_nsnotfound_for_panic() {
            let result = ffi_boundary(NS_NOT_FOUND, || -> usize {
                std::panic::resume_unwind(Box::new("index callback panic"));
            });
            assert_eq!(result, NS_NOT_FOUND);
        }

        #[test]
        fn array_attribute_count_prefers_exact_declared_cardinality() {
            let node = NativeCallbackNode {
                object: null_mut(),
                token: 1,
                kind: NativeNodeKind::Container,
                parent: None,
                children: vec![2, 3],
                logical_children: vec![(100, 2), (101, 3)],
                role: "AXGroup",
                frame: NSRect {
                    origin: NSPoint { x: 0.0, y: 0.0 },
                    size: NSSize {
                        width: 1.0,
                        height: 1.0,
                    },
                },
                label: None,
                description: None,
                value: None,
                action_target: None,
                logical_count: Some(10_000),
            };
            assert_eq!(accessibility_children_count(&node), 10_000);
        }

        #[test]
        fn array_attribute_count_uses_retained_children_for_non_virtual_nodes() {
            let node = NativeCallbackNode {
                object: null_mut(),
                token: 1,
                kind: NativeNodeKind::Ordinary,
                parent: None,
                children: (0..9).collect(),
                logical_children: Vec::new(),
                role: "AXGroup",
                frame: NSRect {
                    origin: NSPoint { x: 0.0, y: 0.0 },
                    size: NSSize {
                        width: 1.0,
                        height: 1.0,
                    },
                },
                label: None,
                description: None,
                value: None,
                action_target: None,
                logical_count: None,
            };
            assert_eq!(accessibility_children_count(&node), 9);
        }

        #[test]
        fn native_tokens_are_continuous_only_for_the_exact_container_fence() {
            let cardinality =
                crate::application::virtual_layout::VirtualLayoutSemanticCardinality::new(2, 7);
            let current = NativeSemanticContainerSnapshot {
                container_id: 4,
                mount_generation: 3,
                registration_generation: 5,
                provider_generation: 6,
                coordinate_authority: NativeSemanticCoordinateAuthority::Logical,
                cardinality,
                has_range_provider: true,
                max_entries: 8,
            };
            let previous = NativeContainerToken {
                token: 11,
                container_id: 4,
                mount_generation: 3,
                registration_generation: 5,
                provider_generation: 6,
                coordinate_authority: NativeSemanticCoordinateAuthority::Logical,
                cardinality,
                lease: None,
                window_generation: 9,
            };
            assert!(container_token_fence_matches(&previous, &current, None, 9));
            assert!(!container_token_fence_matches(
                &previous, &current, None, 10
            ));
            let mut changed = current;
            changed.cardinality.cardinality_revision = 8;
            assert!(!container_token_fence_matches(&previous, &changed, None, 9));
            let mut coordinate_changed = changed.clone();
            coordinate_changed.coordinate_authority = NativeSemanticCoordinateAuthority::Custom {
                identity: crate::layout::VirtualLayoutPolicyIdentity::new("custom"),
                transform_revision: 1,
                transform_generation: 1,
                resolver_token: 1,
            };
            assert!(!container_token_fence_matches(
                &previous,
                &coordinate_changed,
                None,
                9
            ));
            let mut ledger = NativeTokenLedger::default();
            assert_eq!(ledger.issue(), Some(1));
            ledger.retire_all();
            assert!(ledger.ordinary.is_empty());
            assert!(ledger.containers.is_empty());
            assert!(ledger.items.is_empty());
        }

        #[test]
        fn unchanged_projection_does_not_emit_a_second_layout_notification() {
            let projection = NativeCallbackProjection::default();
            assert!(!callback_projection_changed(&projection, &projection));
        }

        #[test]
        fn native_numeric_qualification_requires_current_ordinary_materialized_evidence() {
            let (node, target) = numeric_target_fixture();
            let path = target.path.clone();
            let targets = GuiAutomationTargetSnapshot {
                schema_version: 2,
                viewport_width: 320,
                viewport_height: 120,
                targets: vec![target.clone()],
            };
            assert!(
                qualified_numeric_action_target(&targets, &node, &path, NativeNodeKind::Ordinary,)
                    .is_some()
            );
            assert!(
                native_numeric_action_target(
                    &targets,
                    &node,
                    &path,
                    NativeNodeKind::Ordinary,
                    true,
                )
                .is_none()
            );

            let mut disabled = node.clone();
            disabled.enabled = false;
            disabled.semantics.disabled = true;
            assert!(
                qualified_numeric_action_target(
                    &targets,
                    &disabled,
                    &path,
                    NativeNodeKind::Ordinary,
                )
                .is_none()
            );

            let mut read_only = node.clone();
            read_only.semantics.read_only = true;
            assert!(
                qualified_numeric_action_target(
                    &targets,
                    &read_only,
                    &path,
                    NativeNodeKind::Ordinary,
                )
                .is_none()
            );

            let mut unfocusable = node.clone();
            unfocusable.semantics.focusable = false;
            assert!(
                qualified_numeric_action_target(
                    &targets,
                    &unfocusable,
                    &path,
                    NativeNodeKind::Ordinary,
                )
                .is_none()
            );

            let mut no_value = node.clone();
            no_value.semantics.value_text = None;
            assert!(
                qualified_numeric_action_target(
                    &targets,
                    &no_value,
                    &path,
                    NativeNodeKind::Ordinary,
                )
                .is_none()
            );

            for kind in [
                NativeNodeKind::Root,
                NativeNodeKind::Container,
                NativeNodeKind::Item,
            ] {
                assert!(qualified_numeric_action_target(&targets, &node, &path, kind).is_none());
            }

            let mut wrong_actions = target.clone();
            wrong_actions
                .available_actions
                .retain(|action| action == AUTOMATION_ACTION_INCREMENT);
            let wrong_action_targets = GuiAutomationTargetSnapshot {
                targets: vec![wrong_actions],
                ..targets.clone()
            };
            assert!(
                qualified_numeric_action_target(
                    &wrong_action_targets,
                    &node,
                    &path,
                    NativeNodeKind::Ordinary,
                )
                .is_none()
            );

            let mut without_set_text = target.clone();
            without_set_text.available_actions.retain(|action| {
                action == AUTOMATION_ACTION_INCREMENT || action == AUTOMATION_ACTION_DECREMENT
            });
            let without_set_text_targets = GuiAutomationTargetSnapshot {
                targets: vec![without_set_text],
                ..targets.clone()
            };
            assert!(
                qualified_numeric_action_target(
                    &without_set_text_targets,
                    &node,
                    &path,
                    NativeNodeKind::Ordinary,
                )
                .is_none()
            );

            let mut unmaterialized = target.clone();
            unmaterialized.authority = Some(AutomationTargetAuthority {
                runtime_generation: 9,
                materialized: false,
            });
            let unmaterialized_targets = GuiAutomationTargetSnapshot {
                targets: vec![unmaterialized],
                ..targets.clone()
            };
            assert!(
                qualified_numeric_action_target(
                    &unmaterialized_targets,
                    &node,
                    &path,
                    NativeNodeKind::Ordinary,
                )
                .is_none()
            );

            let ambiguous_targets = GuiAutomationTargetSnapshot {
                targets: vec![target.clone(), target],
                ..targets
            };
            assert!(
                qualified_numeric_action_target(
                    &ambiguous_targets,
                    &node,
                    &path,
                    NativeNodeKind::Ordinary,
                )
                .is_none()
            );
        }

        #[test]
        fn native_numeric_contract_maps_exact_role_actions_and_abi() {
            assert_eq!(
                native_role(NativeNodeKind::Ordinary, AutomationRole::TextInput, true),
                "AXIncrementor"
            );
            assert_eq!(
                native_role(NativeNodeKind::Ordinary, AutomationRole::TextInput, false),
                "AXGroup"
            );
            assert_eq!(
                numeric_action_names(),
                [
                    NativeNumericAccessibilityAction::Increment,
                    NativeNumericAccessibilityAction::Decrement,
                ]
            );
            assert_eq!(
                native_numeric_action_name(&NativeNumericAccessibilityAction::Increment),
                Some(c"AXIncrement")
            );
            assert_eq!(
                native_numeric_action_name(&NativeNumericAccessibilityAction::Decrement),
                Some(c"AXDecrement")
            );
            assert_eq!(
                native_numeric_action_name(&NativeNumericAccessibilityAction::SetValueText(
                    String::from("8"),
                )),
                None
            );
            assert_eq!(
                native_numeric_action_from_name(c"AXIncrement"),
                Some(NativeNumericAccessibilityAction::Increment)
            );
            assert_eq!(
                native_numeric_action_from_name(c"AXDecrement"),
                Some(NativeNumericAccessibilityAction::Decrement)
            );
            assert_eq!(native_numeric_action_from_name(c"AXSetValue"), None);
            assert_eq!(MODERN_ACTION_METHOD_TYPE.to_bytes_with_nul(), b"c@:\0");
            assert_eq!(DEPRECATED_ACTION_METHOD_TYPE.to_bytes_with_nul(), b"v@:@\0");
            assert_eq!(MODERN_VALUE_METHOD_TYPE.to_bytes_with_nul(), b"@@:\0");
            assert_eq!(VALUE_SETTER_METHOD_TYPE.to_bytes_with_nul(), b"v@:@\0");
            assert_eq!(
                LEGACY_VALUE_SETTER_METHOD_TYPE.to_bytes_with_nul(),
                b"v@:@@\0"
            );
            assert_eq!(
                native_attribute_settable(null_mut(), null_mut(), null_mut()),
                NO
            );
            assert_eq!(native_perform_increment(null_mut(), null_mut()), NO);
            assert_eq!(native_perform_decrement(null_mut(), null_mut()), NO);
            assert!(native_action_names(null_mut(), null_mut()).is_null());
            native_perform_action(null_mut(), null_mut(), null_mut());
        }

        #[test]
        fn native_numeric_event_fence_ignores_geometry_and_requires_identity() {
            let (_, target) = numeric_target_fixture();
            let mut geometry_changed = target.clone();
            geometry_changed.bounds.x += 100.0;
            assert!(numeric_action_target_fence_matches(
                &target,
                &geometry_changed
            ));

            let mut path_changed = target.clone();
            path_changed.path.push(AutomationNodeId::new("nested"));
            assert!(!numeric_action_target_fence_matches(&target, &path_changed));

            let mut authority_changed = target.clone();
            authority_changed.authority = Some(AutomationTargetAuthority::materialized(10));
            assert!(!numeric_action_target_fence_matches(
                &target,
                &authority_changed
            ));
            assert!(numeric_action_target_continuity_matches(
                &target,
                &authority_changed
            ));

            let mut adapter = numeric_adapter_fixture(target.clone());
            assert!(
                adapter
                    .numeric_accessibility_request(
                        91,
                        target.clone(),
                        NativeNumericAccessibilityAction::Increment,
                    )
                    .is_some()
            );
            let set_value_request = adapter
                .numeric_accessibility_request(
                    91,
                    target.clone(),
                    NativeNumericAccessibilityAction::SetValueText(String::from("12")),
                )
                .expect("the current native target should admit AXValue text");
            assert_eq!(
                set_value_request.action,
                NumericAccessibilityAction::SetValueText(String::from("12"))
            );
            assert!(
                adapter
                    .numeric_accessibility_request(
                        90,
                        target,
                        NativeNumericAccessibilityAction::Increment,
                    )
                    .is_none()
            );
            let (_, changed_target) = numeric_target_fixture();
            let mut changed_target = changed_target;
            changed_target.path.push(AutomationNodeId::new("stale"));
            assert!(
                adapter
                    .numeric_accessibility_request(
                        91,
                        changed_target,
                        NativeNumericAccessibilityAction::Decrement,
                    )
                    .is_none()
            );

            let state_borrow = adapter.callback_state.borrow_mut();
            assert!(
                adapter
                    .numeric_accessibility_request(
                        91,
                        numeric_target_fixture().1,
                        NativeNumericAccessibilityAction::Increment,
                    )
                    .is_none()
            );
            drop(state_borrow);
            adapter.finish_numeric_action();
        }

        #[test]
        fn native_ax_value_setter_requires_the_complete_current_numeric_capability() {
            let (_, target) = numeric_target_fixture();
            let node = numeric_callback_node(target.clone(), "7");
            assert_eq!(
                native_value_settable_target(&node),
                Some((91, target.clone()))
            );

            let mut without_set_text = node.clone();
            without_set_text
                .action_target
                .as_mut()
                .expect("numeric fixture has a target")
                .available_actions
                .retain(|action| {
                    action == AUTOMATION_ACTION_INCREMENT || action == AUTOMATION_ACTION_DECREMENT
                });
            assert!(native_value_settable_target(&without_set_text).is_none());

            let mut wrong_value = node.clone();
            wrong_value
                .action_target
                .as_mut()
                .expect("numeric fixture has a target")
                .value = Some(String::from("8"));
            assert!(native_value_settable_target(&wrong_value).is_none());

            let mut stale_authority = node;
            stale_authority
                .action_target
                .as_mut()
                .expect("numeric fixture has a target")
                .authority = Some(AutomationTargetAuthority {
                runtime_generation: 9,
                materialized: false,
            });
            assert!(native_value_settable_target(&stale_authority).is_none());
        }

        #[test]
        fn bounded_native_value_extraction_rejects_wrong_types_and_overflow() {
            let valid = unsafe { ns_string("12\0.5") };
            assert_eq!(
                unsafe { bounded_ns_string_to_rust(valid) }.as_deref(),
                Some("12\0.5")
            );
            let empty = unsafe { ns_string("") };
            assert_eq!(
                unsafe { bounded_ns_string_to_rust(empty) }.as_deref(),
                Some("")
            );

            let oversized_utf16 = "x".repeat(MAX_NATIVE_VALUE_UTF16_UNITS + 1);
            let oversized_utf16 = unsafe { ns_string(&oversized_utf16) };
            assert!(unsafe { bounded_ns_string_to_rust(oversized_utf16) }.is_none());

            let oversized_utf8 = "€".repeat(MAX_NATIVE_VALUE_UTF8_BYTES / 3 + 1);
            let oversized_utf8 = unsafe { ns_string(&oversized_utf8) };
            assert!(unsafe { bounded_ns_string_to_rust(oversized_utf8) }.is_none());

            let number = unsafe { ns_number_usize(12) };
            assert!(unsafe { bounded_ns_string_to_rust(number) }.is_none());
            assert!(unsafe { bounded_ns_string_to_rust(null_mut()) }.is_none());
        }

        #[test]
        fn throwing_ax_value_objects_are_inert_and_do_not_enqueue_events() {
            let (receiver, callback_state) = native_numeric_callback_receiver();
            let throwing = throwing_foundation_object();

            native_set_accessibility_value(receiver, null_mut(), throwing);
            assert_eq!(callback_state.borrow().pending_numeric_actions, 0);
            assert!(!enqueue_native_value(receiver, throwing));

            let valid_value = unsafe { ns_string("12") };
            native_set_accessibility_value_for_attribute(
                receiver,
                null_mut(),
                valid_value,
                throwing,
            );
            assert_eq!(callback_state.borrow().pending_numeric_actions, 0);

            unsafe { msg_void(throwing, sel(c"release")) };
            unsafe { msg_void(receiver, sel(c"release")) };
        }

        #[test]
        fn legacy_ax_value_attribute_matching_rejects_embedded_nul_suffixes() {
            let exact = unsafe { ns_string("AXValue") };
            let malformed = unsafe { ns_string("AXValue\0suffix") };
            assert!(unsafe { objc_attribute_is(exact, NS_ACCESSIBILITY_VALUE_ATTRIBUTE) });
            assert!(!unsafe { objc_attribute_is(malformed, NS_ACCESSIBILITY_VALUE_ATTRIBUTE) });
            assert_eq!(
                unsafe { bounded_ns_string_to_rust(malformed) }.as_deref(),
                Some("AXValue\0suffix")
            );
        }

        #[test]
        fn native_numeric_action_connected_production_seam_updates_value_without_rebuilding_object()
        {
            let mut runtime =
                SurfaceRuntime::new(MappedNumericBridge::new(7.0), Vector2::new(240.0, 120.0));
            assert_eq!(runtime.focused_widget(), None);

            let initial_snapshot = runtime.automation_snapshot();
            let initial_targets = runtime.automation_target_snapshot();
            let initial_target = initial_targets
                .targets
                .iter()
                .find(|target| {
                    target.role == AutomationRole::TextInput
                        && target
                            .available_actions
                            .iter()
                            .any(|action| action == AUTOMATION_ACTION_INCREMENT)
                        && target
                            .available_actions
                            .iter()
                            .any(|action| action == AUTOMATION_ACTION_DECREMENT)
                })
                .cloned()
                .expect("the production numeric target should be published");
            assert!(!initial_target.focused);
            assert_eq!(
                initial_target
                    .available_actions
                    .iter()
                    .filter(|action| {
                        action.as_str() == AUTOMATION_ACTION_INCREMENT
                            || action.as_str() == AUTOMATION_ACTION_DECREMENT
                    })
                    .count(),
                2
            );
            assert!(
                initial_target
                    .available_actions
                    .iter()
                    .any(|action| action == AUTOMATION_ACTION_INCREMENT)
            );
            assert!(
                initial_target
                    .available_actions
                    .iter()
                    .any(|action| action == AUTOMATION_ACTION_DECREMENT)
            );
            let initial_node = automation_node_for_id(&initial_snapshot.root, &initial_target.id)
                .cloned()
                .expect("the production numeric node should be published");
            let initial_authority = initial_target
                .authority
                .expect("the initial target should carry runtime authority");

            let mut adapter = runtime_numeric_adapter_fixture(
                &initial_snapshot,
                &initial_node,
                initial_target.clone(),
            );
            let initial_projection = adapter.callback_state.borrow().projection.clone();
            let initial_objects = initial_projection
                .nodes
                .iter()
                .map(|node| node.object)
                .collect::<Vec<_>>();
            assert_eq!(
                adapter.tokens.root,
                Some((TEST_ROOT_TOKEN, adapter.lease, adapter.window_generation))
            );
            let seeded_node = &initial_projection.nodes[1];
            assert_eq!(seeded_node.parent, Some(TEST_ROOT_TOKEN));
            assert_eq!(seeded_node.action_target.as_ref(), Some(&initial_target));
            let request = adapter
                .numeric_accessibility_request(
                    TEST_NUMERIC_TOKEN,
                    initial_target.clone(),
                    NativeNumericAccessibilityAction::Increment,
                )
                .expect("the seeded native projection should admit the initial target");
            let result = runtime.dispatch_numeric_accessibility_action(request);
            assert!(matches!(
                result,
                crate::runtime::NumericAccessibilityDispatchResult::Accepted { widget_id: 42, .. }
            ));
            assert_eq!(runtime.bridge().mapped_actions.get(), 1);
            assert_eq!(runtime.bridge().value.get(), 8.0);
            assert_eq!(runtime.focused_widget(), Some(42));

            let refreshed_snapshot = runtime.automation_snapshot();
            let refreshed_targets = runtime.automation_target_snapshot();
            let refreshed_target = refreshed_targets
                .targets
                .iter()
                .find(|target| target.id == initial_target.id)
                .cloned()
                .expect("the refreshed numeric target should be published");
            let refreshed_authority = refreshed_target
                .authority
                .expect("the refreshed target should carry runtime authority");
            assert!(
                refreshed_authority.runtime_generation > initial_authority.runtime_generation,
                "the runtime target authority should advance after the mapped action"
            );
            adapter
                .publish_projection(&refreshed_snapshot, &refreshed_targets, &[], None)
                .expect("the refreshed production projection should publish");
            assert_eq!(adapter.value_notifications, 1);
            assert_eq!(adapter.layout_notifications, 0);
            {
                let state = adapter.callback_state.borrow();
                let objects = state
                    .projection
                    .nodes
                    .iter()
                    .map(|node| node.object)
                    .collect::<Vec<_>>();
                assert_eq!(objects, initial_objects);
                assert_eq!(state.projection.root_token, initial_projection.root_token);
                assert_eq!(state.projection.nodes[0].token, TEST_ROOT_TOKEN);
                assert_eq!(state.projection.nodes[1].token, TEST_NUMERIC_TOKEN);
                let node = &state.projection.nodes[1];
                assert_eq!(node.value.as_deref(), Some("8"));
                assert_eq!(node.action_target.as_ref(), Some(&refreshed_target));
            }

            let set_text_request = adapter
                .numeric_accessibility_request(
                    TEST_NUMERIC_TOKEN,
                    refreshed_target.clone(),
                    NativeNumericAccessibilityAction::SetValueText(String::from("12")),
                )
                .expect("the refreshed production target should admit AXValue text");
            let set_text_result = runtime.dispatch_numeric_accessibility_action(set_text_request);
            assert!(matches!(
                set_text_result,
                crate::runtime::NumericAccessibilityDispatchResult::Accepted { widget_id: 42, .. }
            ));
            assert_eq!(runtime.bridge().mapped_actions.get(), 2);
            assert_eq!(runtime.bridge().value.get(), 12.0);
            assert_eq!(runtime.focused_widget(), Some(42));

            let value_notifications_before = adapter.value_notifications;
            let layout_notifications_before = adapter.layout_notifications;
            adapter
                .publish_projection(&refreshed_snapshot, &refreshed_targets, &[], None)
                .expect("the unchanged production projection should remain published");
            assert_eq!(adapter.value_notifications, value_notifications_before);
            assert_eq!(adapter.layout_notifications, layout_notifications_before);

            assert!(
                adapter
                    .numeric_accessibility_request(
                        TEST_NUMERIC_TOKEN,
                        initial_target,
                        NativeNumericAccessibilityAction::Increment,
                    )
                    .is_none(),
                "the old pre-update target must fail the exact native stale fence"
            );
        }

        #[test]
        fn stable_numeric_value_updates_retain_object_and_deduplicate_notifications() {
            let (_, target) = numeric_target_fixture();
            let current_node = numeric_callback_node(target.clone(), "7");
            let projection = NativeCallbackProjection {
                nodes: vec![current_node],
                root_token: Some(91),
            };
            let mut value_target = target.clone();
            value_target.value = Some(String::from("8"));
            let (changed_spec, frame) = numeric_spec_fixture(value_target.clone(), "8");
            let updates =
                collect_stable_value_projection_updates(&projection, &[changed_spec], &[frame])
                    .expect("the value-only native evidence should be stable");
            assert_eq!(updates.len(), 1);
            let object = projection.nodes[0].object;
            let mut changed_projection = projection.clone();
            assert_eq!(
                apply_stable_value_projection_updates(&mut changed_projection, &updates),
                Some(vec![object])
            );
            assert_eq!(changed_projection.nodes[0].object, object);
            assert_eq!(changed_projection.nodes[0].value.as_deref(), Some("8"));

            let (same_spec, same_frame) = numeric_spec_fixture(value_target, "8");
            let same_updates = collect_stable_value_projection_updates(
                &changed_projection,
                &[same_spec],
                &[same_frame],
            )
            .expect("the unchanged native evidence should remain stable");
            assert!(same_updates.is_empty());

            let mut changed_target = target;
            changed_target.authority = Some(AutomationTargetAuthority::materialized(10));
            let (stale_spec, stale_frame) = numeric_spec_fixture(changed_target, "9");
            assert!(
                collect_stable_value_projection_updates(
                    &changed_projection,
                    &[stale_spec],
                    &[stale_frame],
                )
                .is_some()
            );
        }

        #[test]
        fn ffi_boundary_contains_panics() {
            let value = ffi_boundary(7_u32, || {
                std::panic::resume_unwind(Box::new("callback panic"));
            });
            assert_eq!(value, 7);
        }

        #[test]
        fn custom_zero_cardinality_never_reaches_native_container_view() {
            let custom = crate::layout::VirtualLayoutCoordinateSpace::custom(
                crate::layout::VirtualLayoutPolicyIdentity::new("custom".to_owned()),
            );
            assert_ne!(custom, crate::layout::VirtualLayoutCoordinateSpace::Logical);
        }

        #[test]
        fn qualified_custom_native_consumption_is_passive_and_uses_normalized_bounds() {
            let provider_calls = Rc::new(Cell::new(0_usize));
            let transform_calls = Rc::new(Cell::new(0_usize));
            let provider: Rc<dyn VirtualLayoutSemanticRangeProvider> = Rc::new({
                let provider_calls = Rc::clone(&provider_calls);
                move |_request: &VirtualLayoutSemanticRangeRequest| {
                    provider_calls.set(provider_calls.get().saturating_add(1));
                    VirtualLayoutSemanticProviderOutcome::Found(vec![
                        VirtualLayoutSemanticEntry::new(
                            VirtualLayoutItemKey::new(1_u32),
                            0,
                            Rect::from_xy_size(1.0, 2.0, 3.0, 4.0),
                            AutomationNodeSemantics::new(AutomationRole::Row)
                                .with_label("custom item"),
                            AutomationNodeId::new("custom-native-item"),
                        ),
                    ])
                }
            });
            let transform: Rc<dyn VirtualLayoutSemanticCoordinateTransform> = Rc::new({
                let transform_calls = Rc::clone(&transform_calls);
                move |_request: &crate::runtime::virtual_layout::VirtualLayoutSemanticCoordinateTransformRequest| {
                    transform_calls.set(transform_calls.get().saturating_add(1));
                    VirtualLayoutSemanticCoordinateTransformOutcome::Found(
                        Rect::from_xy_size(73.0, 19.0, 7.0, 11.0),
                    )
                }
            });
            let parts = VirtualLayoutParts::new(
                Rc::new(TestVirtualLayoutPolicy),
                VirtualLayoutPolicyIdentity::new("native-custom-policy"),
                VirtualLayoutOverscan::new(0.0, 0.0).expect("finite test overscan"),
                crate::layout::VirtualLayoutBudget::new(4),
                VirtualLayoutRevisions::new(1, 2, 3, 4),
                Rc::new(|| scroll(spacer::<()>().size(240.0, 100.0))),
                Rc::new(|_| text::<()>("semantic item")),
                Rc::new(|_| VirtualLayoutPolicyIdentity::new("native-custom-item")),
            )
            .with_semantic_range_provider(Rc::clone(&provider))
            .with_semantic_cardinality(
                crate::application::virtual_layout::VirtualLayoutSemanticCardinality::new(1, 1),
            )
            .with_semantic_coordinate_transform(
                VirtualLayoutPolicyIdentity::new("native-custom-space"),
                23,
                Rc::clone(&transform),
            );
            let mut runtime = SurfaceRuntime::new(
                TestBridge {
                    surface: row([virtual_layout_from_parts(parts).fill()])
                        .fill()
                        .into_surface(),
                },
                Vector2::new(240.0, 100.0),
            );

            let containers = runtime.native_semantic_containers();
            assert_eq!(containers.len(), 1);
            assert!(matches!(
                containers[0].coordinate_authority,
                NativeSemanticCoordinateAuthority::Custom { .. }
            ));
            assert_eq!(provider_calls.get(), 0);
            assert_eq!(transform_calls.get(), 0);

            let callback_state =
                NativeSemanticCallbackState::new_for_test(WindowId::dummy(), 1, null_mut());
            let mut adapter = NativeSemanticAccessibilityAdapter {
                view: null_mut(),
                callback_state: Box::new(RefCell::new(callback_state)),
                objects: Vec::new(),
                committed_objects: Vec::new(),
                tokens: NativeTokenLedger::default(),
                lease: None,
                generation: 1,
                window_generation: 1,
                transform: None,
                current_containers: Vec::new(),
                active_ranges: Vec::new(),
                attached: false,
                layout_notifications: 0,
                value_notifications: 0,
                destroyed_notifications: 0,
            };
            let targets = runtime.automation_target_snapshot();
            let passive_specs = adapter
                .build_specs(&runtime.automation_snapshot(), &targets, &containers, None)
                .expect("passive custom topology should build");
            assert!(
                passive_specs
                    .iter()
                    .all(|spec| spec.kind != NativeNodeKind::Item)
            );

            let session = runtime
                .open_semantic_automation_session()
                .expect("passive observation must not own the session");
            let handles = runtime
                .semantic_automation_containers(session)
                .expect("the admitted custom container should have a session handle");
            assert_eq!(handles.len(), 1);
            let refresh = runtime
                .refresh_semantic_automation_session(
                    session,
                    &[SemanticAutomationDemand::range(handles[0], 0, 1)],
                )
                .expect("explicit custom range refresh should publish");
            assert_eq!(refresh.status, SemanticAutomationRefreshStatus::Published);
            assert_eq!(provider_calls.get(), 1);
            assert_eq!(transform_calls.get(), 1);
            let (composition, status) = runtime
                .native_semantic_automation_composition(session)
                .expect("selected composition lookup succeeds")
                .expect("the published composition is selected");
            assert_eq!(status, SemanticAutomationRefreshStatus::Published);
            assert_eq!(composition.normalized_sidecar().entries().len(), 1);
            let targets = runtime.automation_target_snapshot();
            let specs = adapter
                .build_specs(
                    composition.snapshot(),
                    &targets,
                    &containers,
                    Some(composition.normalized_sidecar()),
                )
                .expect("matching custom witness should be consumable");
            assert_eq!(
                specs
                    .iter()
                    .filter(|spec| spec.kind == NativeNodeKind::Container)
                    .count(),
                1,
                "the composed snapshot should retain the virtual container"
            );
            let item = specs
                .iter()
                .find(|spec| spec.kind == NativeNodeKind::Item)
                .expect("the explicit custom query should expose one item");
            assert_eq!(
                item.bounds,
                AutomationBounds::from_rect(Rect::from_xy_size(73.0, 19.0, 7.0, 11.0))
            );
            assert_eq!(provider_calls.get(), 1);
            assert_eq!(transform_calls.get(), 1);

            let mut mismatched_containers = containers.clone();
            let NativeSemanticCoordinateAuthority::Custom {
                identity,
                transform_revision,
                transform_generation,
                resolver_token,
            } = mismatched_containers[0].coordinate_authority.clone()
            else {
                panic!("the fixture must be custom");
            };
            mismatched_containers[0].coordinate_authority =
                NativeSemanticCoordinateAuthority::Custom {
                    identity,
                    transform_revision,
                    transform_generation: transform_generation.saturating_add(1),
                    resolver_token,
                };
            assert!(
                adapter
                    .build_specs(
                        composition.snapshot(),
                        &targets,
                        &mismatched_containers,
                        Some(composition.normalized_sidecar()),
                    )
                    .is_err()
            );
            assert_eq!(provider_calls.get(), 1);
            assert_eq!(transform_calls.get(), 1);
            runtime
                .close_semantic_automation_session(session)
                .expect("the explicit test session should close");
        }
    }
}

#[cfg(target_os = "macos")]
pub(super) use macos::NativeSemanticAccessibilityAdapter;

#[cfg(not(target_os = "macos"))]
pub(super) struct NativeSemanticAccessibilityAdapter;
