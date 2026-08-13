//! Private primary-window macOS semantic accessibility consumer.
//!
//! The adapter is deliberately a native boundary, not a public runtime
//! capability.  AppKit calls below are made on the main thread and are
//! wrapped in `catch_unwind` so no Rust panic can unwind through Objective-C.
//! Callback state is callback-local and bounded; it never borrows or enters a
//! `SurfaceRuntime`.  Runtime/provider work happens only after the event-loop
//! turn owns `SurfaceRuntime`.

#[cfg(target_os = "macos")]
mod macos {
    use crate::{
        gui::automation::{
            AutomationBounds, AutomationNodeId, AutomationNodeSemantics, AutomationNodeSnapshot,
            AutomationRole, GuiAutomationSnapshot,
        },
        gui_runtime::native_vello::runtime_event::{
            NativeSemanticAccessibilityQuery, RuntimeUserEvent,
        },
        layout::{VIRTUAL_LAYOUT_MAX_QUERY_ENTRIES, VirtualLayoutItemKey},
        runtime::{
            NativeSemanticContainerSnapshot, NativeSemanticCoordinateAuthority, RuntimeBridge,
            SemanticAutomationContainerHandle, SemanticAutomationDemand,
            SemanticAutomationFallbackReason, SemanticAutomationRefreshStatus,
            SemanticAutomationSessionError, SemanticAutomationSessionHandle, SurfaceRuntime,
        },
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
    const NATIVE_STATE_IVAR_NAME: &CStr = c"radiantNativeSemanticState";
    const NATIVE_STATE_IVAR_TYPE: &CStr = c"^v";
    const NATIVE_CLASS_NAME: &CStr = c"RadiantNativeSemanticAccessibilityElement";
    const NS_UTF8_STRING_ENCODING: usize = 4;
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
        fn class_addMethod(
            class: Id,
            name: Sel,
            imp: *const c_void,
            types: *const c_char,
        ) -> ObjcBool;
        fn object_getIvar(object: Id, ivar: Ivar) -> Id;
        fn object_setIvar(object: Id, ivar: Ivar, value: Id);
        fn sel_registerName(name: *const c_char) -> Sel;
        fn objc_msgSend();
        #[cfg(target_arch = "x86_64")]
        fn objc_msgSend_stret();
        fn objc_msgSendSuper();
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
                    c"accessibilityArrayAttributeCount:",
                    native_array_attribute_count as *const c_void,
                    c"Q@:@",
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
                    c"accessibilityActionNames",
                    native_action_names as *const c_void,
                    c"@@:",
                ),
                (
                    c"accessibilityPerformAction:",
                    native_perform_action as *const c_void,
                    c"c@:@",
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

    unsafe fn msg_void_id_id(receiver: Id, selector: Sel, first: Id, second: Id) {
        let message: unsafe extern "C" fn(Id, Sel, Id, Id) =
            unsafe { transmute(objc_msgSend as *const ()) };
        unsafe { message(receiver, selector, first, second) };
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

    unsafe fn msg_rect_rect(receiver: Id, selector: Sel, rect: NSRect) -> NSRect {
        #[cfg(target_arch = "x86_64")]
        {
            let message: unsafe extern "C" fn(*mut NSRect, Id, Sel, NSRect) =
                unsafe { transmute(objc_msgSend_stret as *const ()) };
            let mut result = MaybeUninit::<NSRect>::uninit();
            unsafe { message(result.as_mut_ptr(), receiver, selector, rect) };
            unsafe { result.assume_init() }
        }

        #[cfg(not(target_arch = "x86_64"))]
        {
            let message: unsafe extern "C" fn(Id, Sel, NSRect) -> NSRect =
                unsafe { transmute(objc_msgSend as *const ()) };
            unsafe { message(receiver, selector, rect) }
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
            })
        }

        fn convert(&self, bounds: AutomationBounds) -> Option<NSRect> {
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
            let screen = unsafe { msg_rect_rect(self.view, sel(c"convertRectToScreen:"), source) };
            if !rect_is_finite(screen)
                || screen.size.width < 0.0
                || screen.size.height < 0.0
                || !self.scale_factor.is_finite()
                || self.scale_factor <= 0.0
            {
                None
            } else {
                Some(screen)
            }
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
        logical_count: Option<usize>,
    }

    /// One owned primary-window native semantic adapter.  It is intentionally
    /// not `Send`/`Sync`; its Objective-C objects, lease, and callback state
    /// are owned by the primary event-loop thread.
    pub struct NativeSemanticAccessibilityAdapter {
        view: Id,
        callback_state: Box<RefCell<NativeSemanticCallbackState>>,
        objects: Vec<Id>,
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
            })
        }

        pub(crate) fn publish_passive<Bridge, Message>(
            &mut self,
            runtime: &SurfaceRuntime<Bridge, Message>,
        ) where
            Bridge: RuntimeBridge<Message>,
        {
            let ordinary = runtime.automation_snapshot();
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
            let _ = self.publish_projection(&ordinary, &containers, selected);
        }

        fn publish_ordinary_projection<Bridge, Message>(
            &mut self,
            runtime: &SurfaceRuntime<Bridge, Message>,
        ) where
            Bridge: RuntimeBridge<Message>,
        {
            let ordinary = runtime.automation_snapshot();
            let containers = runtime.native_semantic_containers();
            let _ = self.publish_projection(&ordinary, &containers, None);
        }

        pub(crate) fn accepts_generation(&self, generation: u64) -> bool {
            self.generation == generation
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
                    self.publish_ordinary_projection(runtime);
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
                        self.publish_ordinary_projection(runtime);
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
                    self.publish_ordinary_projection(runtime);
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
            let containers = runtime.native_semantic_containers();
            let _ = self.publish_projection(&ordinary, &containers, projection);
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
            self.generation = self.generation.saturating_add(1);
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
            containers: &[NativeSemanticContainerSnapshot],
            selected: Option<(
                crate::runtime::VirtualLayoutAutomationComposition,
                SemanticAutomationRefreshStatus,
            )>,
        ) -> Result<(), String> {
            let composition = selected.as_ref().map(|(composition, _)| composition);
            let snapshot = composition.map_or(ordinary, |composition| composition.snapshot());
            let sidecar = composition.map(|composition| composition.normalized_sidecar());
            let specs = match self.build_specs(snapshot, containers, sidecar) {
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
            if self.specs_match_projection(&specs) {
                self.current_containers = containers.to_vec();
                self.attached = true;
                if let Ok(mut state) = self.callback_state.try_borrow_mut() {
                    state.last_unavailable = None;
                }
                return Ok(());
            }
            self.retire_published_objects();
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
            let changed = self.replace_callback_projection(projection);
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
                let token = self.build_snapshot_node(
                    snapshot,
                    child,
                    Some(root_token),
                    &mut path,
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
            let spec_index = specs.len();
            specs.push(NativeNodeSpec {
                token,
                kind,
                parent,
                children: Vec::new(),
                logical_children: Vec::new(),
                bounds: node.bounds,
                semantics: node.semantics.clone(),
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
                            logical_count: None,
                        });
                        let mut nested = Vec::new();
                        let mut item_path = entry.normalized_path().to_vec();
                        for (child_index, child) in item_node.children.iter().enumerate() {
                            item_path.push(child_index);
                            let child_token = self.build_snapshot_node(
                                snapshot,
                                child,
                                Some(item_token),
                                &mut item_path,
                                containers,
                                anchor_ids,
                                Some(sidecar),
                                item_paths,
                                specs,
                            )?;
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
                    let is_sidecar_path = item_paths.iter().any(|(_, item_path)| {
                        item_path.len() >= path.len() && item_path.starts_with(path)
                    });
                    if !is_sidecar_path {
                        let child_token = self.build_snapshot_node(
                            snapshot,
                            child,
                            Some(token),
                            path,
                            containers,
                            anchor_ids,
                            sidecar,
                            item_paths,
                            specs,
                        )?;
                        children.push(child_token);
                    }
                    path.pop();
                }
            } else {
                for (child_index, child) in node.children.iter().enumerate() {
                    path.push(child_index);
                    let child_token = self.build_snapshot_node(
                        snapshot,
                        child,
                        Some(token),
                        path,
                        containers,
                        anchor_ids,
                        sidecar,
                        item_paths,
                        specs,
                    )?;
                    path.pop();
                    children.push(child_token);
                }
            }
            specs[spec_index].children = children;
            specs[spec_index].logical_children = logical_children;
            Ok(token)
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
                                && node.role == native_role(spec.kind, spec.semantics.role)
                                && node.label == spec.semantics.label
                                && node.description == spec.semantics.description
                                && node.value == spec.semantics.value_text
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
                    role: native_role(spec.kind, spec.semantics.role),
                    frame,
                    label: spec.semantics.label,
                    description: spec.semantics.description,
                    value: spec.semantics.value_text,
                    logical_count: spec.logical_count,
                });
            }
            let root_token = nodes.first().map(|node| node.token);
            Ok(NativeCallbackProjection { nodes, root_token })
        }

        fn replace_callback_projection(&mut self, projection: NativeCallbackProjection) -> bool {
            let changed =
                callback_projection_changed(&self.callback_state.borrow().projection, &projection);
            if let Ok(mut state) = self.callback_state.try_borrow_mut() {
                state.projection = projection.clone();
                state.last_unavailable = None;
            }
            let root = projection
                .root_token
                .and_then(|token| projection.nodes.iter().find(|node| node.token == token))
                .map(|node| node.object);
            if let Some(root) = root {
                unsafe {
                    let children = ns_array(&[root]);
                    let attribute = ns_string("AXChildren");
                    if !children.is_null() && !attribute.is_null() {
                        msg_void_id_id(
                            self.view,
                            sel(c"accessibilitySetOverrideValue:forAttribute:"),
                            children,
                            attribute,
                        );
                    }
                }
            }
            changed
        }

        fn retire_published_objects(&mut self) {
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
            unsafe {
                let attribute = ns_string("AXChildren");
                if !attribute.is_null() {
                    msg_void_id_id(
                        self.view,
                        sel(c"accessibilitySetOverrideValue:forAttribute:"),
                        null_mut(),
                        attribute,
                    );
                }
            }
            let Some(class) = class else {
                // The state has already been emptied where possible. Retain
                // objects if Objective-C cannot expose their ivar, so no
                // object is released while it may still point at live state.
                return;
            };
            for object in self.objects.drain(..) {
                unsafe {
                    // Keep this immediately before notification/release as a
                    // final per-object lifecycle fence, including reentrant
                    // or partially borrowed callback-state cases.
                    object_setIvar(object, class.state_ivar(), null_mut());
                    let notification = ns_string("AXUIElementDestroyed");
                    if !notification.is_null() {
                        NSAccessibilityPostNotification(object, notification);
                    }
                    msg_void(object, sel(c"release"));
                }
            }
        }

        fn clear_callback_state(&mut self) {
            let state = self.callback_state.get_mut();
            state.projection = NativeCallbackProjection::default();
            state.in_flight.clear();
            state.deferred.clear();
            state.last_unavailable = None;
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
            self.retire_published_objects();
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

    fn native_role(_kind: NativeNodeKind, role: AutomationRole) -> &'static str {
        if matches!(role, AutomationRole::Text | AutomationRole::Readout) {
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
        let class = native_class().ok()?;
        let state = unsafe { object_getIvar(receiver, class.state_ivar()) };
        if state.is_null() {
            return None;
        }
        // The adapter owns this box until all custom elements are detached and
        // released.  AppKit callbacks are main-thread-only during that window.
        Some(unsafe { &*(state.cast::<RefCell<NativeSemanticCallbackState>>()) })
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
                node.value
                    .as_deref()
                    .map_or(null_mut(), |value| unsafe { ns_string(value) })
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

    extern "C" fn native_attribute_settable(_: Id, _: Sel, _: Id) -> ObjcBool {
        ffi_boundary(NO, || NO)
    }

    extern "C" fn native_action_names(_: Id, _: Sel) -> Id {
        ffi_boundary(null_mut(), || unsafe { ns_array(&[]) })
    }

    extern "C" fn native_perform_action(_: Id, _: Sel, _: Id) -> ObjcBool {
        ffi_boundary(NO, || NO)
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::{
            application::IntoView,
            application::{
                VirtualLayoutParts, row, scroll, spacer, text, virtual_layout_from_parts,
            },
            gui::types::{Rect, Vector2},
            layout::{
                VirtualLayoutBoundsConfidence, VirtualLayoutBudget, VirtualLayoutExtentCandidate,
                VirtualLayoutItemCandidate, VirtualLayoutItemKey, VirtualLayoutOverscan,
                VirtualLayoutPolicy, VirtualLayoutPolicyDecision, VirtualLayoutPolicyIdentity,
                VirtualLayoutQueryInput, VirtualLayoutQuerySink, VirtualLayoutVisibility,
            },
            runtime::{
                RuntimeBridge, SurfaceRuntime, UiSurface, VirtualLayoutRevisions,
                VirtualLayoutSemanticEntry, VirtualLayoutSemanticProviderOutcome,
                VirtualLayoutSemanticRangeProvider, VirtualLayoutSemanticRangeRequest,
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
                tokens: NativeTokenLedger::default(),
                lease: None,
                generation: 1,
                window_generation: 1,
                transform: None,
                current_containers: Vec::new(),
                active_ranges: Vec::new(),
                attached: false,
                layout_notifications: 0,
            };
            let passive_specs = adapter
                .build_specs(&runtime.automation_snapshot(), &containers, None)
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
            let specs = adapter
                .build_specs(
                    composition.snapshot(),
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
