use std::{
    ffi::{CStr, c_void},
    sync::{
        Arc, OnceLock,
        atomic::{AtomicBool, Ordering},
    },
};

use super::bridge::{
    Id, Ivar, NO, NSPoint, ObjcBool, Sel, YES, class, class_addIvar, class_addMethod,
    class_getInstanceVariable, msg_id, msg_super_void, msg_void, objc_allocateClassPair,
    objc_disposeClassPair, objc_getClass, objc_registerClassPair, object_getIvar, object_setIvar,
    selector,
};
use crate::{
    gui_runtime::native_vello::RuntimeUserEvent,
    runtime::{ExternalDragEffect, ExternalDragIdentity, ExternalDragOutcome},
};
use winit::{event_loop::EventLoopProxy, window::WindowId};

const NS_DRAG_OPERATION_COPY: usize = 1;
const SOURCE_CLASS_NAME: &CStr = c"RadiantExternalFileDraggingSource";
const SOURCE_STATE_IVAR_NAME: &CStr = c"radiantCallbackState";
const SOURCE_STATE_IVAR_TYPE: &CStr = c"^v";

#[derive(Clone, Copy)]
struct SourceClass {
    class: usize,
    state_ivar: usize,
}

impl SourceClass {
    unsafe fn class(self) -> Id {
        self.class as Id
    }

    unsafe fn state_ivar(self) -> Ivar {
        self.state_ivar as Ivar
    }
}

struct DraggingSourceState {
    proxy: EventLoopProxy<RuntimeUserEvent>,
    window_id: WindowId,
    identity: ExternalDragIdentity,
    source_ownership: Arc<AtomicBool>,
}

pub(super) struct DraggingSourceRegistration {
    source: Id,
    source_ownership: Arc<AtomicBool>,
    owns_registration: bool,
}

impl DraggingSourceRegistration {
    pub(super) fn source(&self) -> Id {
        self.source
    }

    pub(super) fn commit_to_session(&mut self) {
        self.owns_registration = false;
    }
}

impl Drop for DraggingSourceRegistration {
    fn drop(&mut self) {
        if self.owns_registration && self.source_ownership.swap(false, Ordering::AcqRel) {
            unsafe { msg_void(self.source, selector(c"release")) };
        }
    }
}

pub(super) unsafe fn dragging_source(
    proxy: EventLoopProxy<RuntimeUserEvent>,
    window_id: WindowId,
    identity: ExternalDragIdentity,
) -> Result<DraggingSourceRegistration, String> {
    let source_class = source_class()?;
    let source = unsafe { msg_id(source_class.class(), selector(c"new")) };
    if source.is_null() {
        return Err(String::from("Failed to instantiate NSDraggingSource"));
    }

    let source_ownership = Arc::new(AtomicBool::new(true));
    let state = Box::new(DraggingSourceState {
        proxy,
        window_id,
        identity,
        source_ownership: Arc::clone(&source_ownership),
    });
    let state = Box::into_raw(state);
    unsafe {
        object_setIvar(source, source_class.state_ivar(), state.cast());
    }

    Ok(DraggingSourceRegistration {
        source,
        source_ownership,
        owns_registration: true,
    })
}

fn source_class() -> Result<SourceClass, String> {
    static SOURCE_CLASS: OnceLock<Result<SourceClass, String>> = OnceLock::new();
    SOURCE_CLASS
        .get_or_init(|| unsafe {
            create_source_class().map(|(class, state_ivar)| SourceClass {
                class: class as usize,
                state_ivar: state_ivar as usize,
            })
        })
        .clone()
}

unsafe fn create_source_class() -> Result<(Id, Ivar), String> {
    let superclass = unsafe { class(c"NSObject")? };
    let mut source_class = unsafe { objc_getClass(SOURCE_CLASS_NAME.as_ptr()) };
    if source_class.is_null() {
        source_class = unsafe { objc_allocateClassPair(superclass, SOURCE_CLASS_NAME.as_ptr(), 0) };
        if source_class.is_null() {
            return Err(String::from("objc_allocateClassPair failed"));
        }

        let added_ivar = unsafe {
            class_addIvar(
                source_class,
                SOURCE_STATE_IVAR_NAME.as_ptr(),
                std::mem::size_of::<*mut c_void>(),
                std::mem::align_of::<*mut c_void>().trailing_zeros() as u8,
                SOURCE_STATE_IVAR_TYPE.as_ptr(),
            )
        };
        if added_ivar == NO {
            unsafe { objc_disposeClassPair(source_class) };
            return Err(String::from("class_addIvar failed for callback state"));
        }

        let methods = [
            (c"dealloc", dragging_source_dealloc as *const c_void, c"v@:"),
            (
                c"draggingSession:sourceOperationMaskForDraggingContext:",
                dragging_source_operation_mask as *const c_void,
                c"Q@:@@q",
            ),
            (
                c"ignoreModifierKeysForDraggingSession:",
                dragging_source_ignores_modifier_keys as *const c_void,
                c"c@:@",
            ),
            (
                c"draggingSession:endedAtPoint:operation:",
                dragging_session_ended_at_point as *const c_void,
                c"v@:@{CGPoint=dd}Q",
            ),
        ];
        for (name, imp, types) in methods {
            if let Err(error) = unsafe { add_method(source_class, name, imp, types) } {
                unsafe { objc_disposeClassPair(source_class) };
                return Err(error);
            }
        }
        unsafe { objc_registerClassPair(source_class) };
    }

    let state_ivar =
        unsafe { class_getInstanceVariable(source_class, SOURCE_STATE_IVAR_NAME.as_ptr()) };
    if state_ivar.is_null() {
        return Err(String::from(
            "NSDraggingSource callback state ivar was not installed",
        ));
    }
    Ok((source_class, state_ivar))
}

unsafe fn add_method(
    class: Id,
    name: &'static CStr,
    imp: *const c_void,
    types: &'static CStr,
) -> Result<(), String> {
    let added = unsafe { class_addMethod(class, selector(name), imp, types.as_ptr()) };
    if added == NO {
        Err(format!(
            "class_addMethod failed for {}",
            name.to_string_lossy()
        ))
    } else {
        Ok(())
    }
}

extern "C" fn dragging_source_operation_mask(_: Id, _: Sel, _: Id, _: isize) -> usize {
    NS_DRAG_OPERATION_COPY
}

extern "C" fn dragging_source_ignores_modifier_keys(_: Id, _: Sel, _: Id) -> ObjcBool {
    YES
}

extern "C" fn dragging_session_ended_at_point(
    receiver: Id,
    _: Sel,
    _: Id,
    _point: NSPoint,
    operation: usize,
) {
    let Ok(source_class) = source_class() else {
        return;
    };
    let state = unsafe { object_getIvar(receiver, source_class.state_ivar()) };
    if state.is_null() {
        return;
    }
    unsafe { object_setIvar(receiver, source_class.state_ivar(), std::ptr::null_mut()) };
    let state = unsafe { Box::from_raw(state.cast::<DraggingSourceState>()) };
    let result = terminal_result(operation);
    let event = RuntimeUserEvent::ExternalDragCompleted {
        window_id: state.window_id,
        identity: state.identity,
        result: Ok(result),
    };
    if state.proxy.send_event(event).is_err() {
        tracing::warn!(
            window_id = ?state.window_id,
            identity = ?state.identity,
            "radiant generic native vello: macOS external drag completion event was rejected"
        );
    }
    if state.source_ownership.swap(false, Ordering::AcqRel) {
        unsafe { msg_void(receiver, selector(c"release")) };
    }
}

extern "C" fn dragging_source_dealloc(receiver: Id, _: Sel) {
    if let Ok(source_class) = source_class() {
        let state = unsafe { object_getIvar(receiver, source_class.state_ivar()) };
        if !state.is_null() {
            unsafe { object_setIvar(receiver, source_class.state_ivar(), std::ptr::null_mut()) };
            unsafe { drop(Box::from_raw(state.cast::<DraggingSourceState>())) };
        }
    }
    if let Ok(superclass) = unsafe { class(c"NSObject") } {
        unsafe { msg_super_void(receiver, superclass, selector(c"dealloc")) };
    }
}

fn terminal_result(operation: usize) -> ExternalDragOutcome {
    ExternalDragOutcome {
        effect: if operation == NS_DRAG_OPERATION_COPY {
            ExternalDragEffect::Copy
        } else {
            ExternalDragEffect::None
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appkit_copy_operation_is_the_only_accepted_terminal_result() {
        assert_eq!(
            terminal_result(NS_DRAG_OPERATION_COPY).effect,
            ExternalDragEffect::Copy
        );
        assert_eq!(terminal_result(0).effect, ExternalDragEffect::None);
        assert_eq!(terminal_result(2).effect, ExternalDragEffect::None);
    }

    #[test]
    fn source_advertises_copy_only_and_ignores_modifiers() {
        let nil = std::ptr::null_mut();

        assert_eq!(dragging_source_operation_mask(nil, nil, nil, 0), 1);
        assert_eq!(dragging_source_ignores_modifier_keys(nil, nil, nil), YES);
    }
}
