use std::{ffi::CStr, ffi::c_void, sync::OnceLock};

use super::bridge::{
    Id, NO, ObjcBool, Sel, YES, class, class_addMethod, msg_id, objc_allocateClassPair,
    objc_getClass, objc_registerClassPair, selector,
};

const NS_DRAG_OPERATION_COPY: usize = 1;

pub(super) unsafe fn dragging_source() -> Result<Id, String> {
    static SOURCE: OnceLock<usize> = OnceLock::new();
    let source = *SOURCE.get_or_init(|| unsafe {
        match create_dragging_source() {
            Ok(source) => source as usize,
            Err(_) => 0,
        }
    }) as Id;
    if source.is_null() {
        Err(String::from("Failed to create NSDraggingSource"))
    } else {
        Ok(source)
    }
}

unsafe fn create_dragging_source() -> Result<Id, String> {
    let superclass = unsafe { class(c"NSObject")? };
    let class_name = c"RadiantExternalFileDraggingSource";
    let mut source_class = unsafe { objc_getClass(class_name.as_ptr()) };
    if source_class.is_null() {
        source_class = unsafe { objc_allocateClassPair(superclass, class_name.as_ptr(), 0) };
        if source_class.is_null() {
            return Err(String::from("objc_allocateClassPair failed"));
        }
        unsafe {
            add_method(
                source_class,
                c"draggingSession:sourceOperationMaskForDraggingContext:",
                dragging_source_operation_mask as *const c_void,
                c"Q@:@@q",
            )?;
            add_method(
                source_class,
                c"ignoreModifierKeysForDraggingSession:",
                dragging_source_ignores_modifier_keys as *const c_void,
                c"c@:@",
            )?;
            objc_registerClassPair(source_class);
        }
    }
    let source = unsafe { msg_id(source_class, selector(c"new")) };
    if source.is_null() {
        Err(String::from("Failed to instantiate NSDraggingSource"))
    } else {
        Ok(source)
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dragging_source_callbacks_preserve_copy_semantics() {
        let nil = std::ptr::null_mut();

        assert_eq!(dragging_source_operation_mask(nil, nil, nil, 0), 1);
        assert_eq!(dragging_source_ignores_modifier_keys(nil, nil, nil), YES);
    }
}
