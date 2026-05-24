use super::read_runtime_source;

#[test]
fn native_window_platform_attributes_stay_in_focused_module() {
    let window = read_runtime_source("src/gui_runtime/native_vello/generic_runtime/window.rs");
    let platform =
        read_runtime_source("src/gui_runtime/native_vello/generic_runtime/window/platform.rs");
    let tests =
        read_runtime_source("src/gui_runtime/native_vello/generic_runtime/tests/window_policy.rs");

    assert!(
        window.contains("mod platform;")
            && window.contains("platform::apply_drag_and_drop_attributes")
            && window.contains("platform::apply_popup_attributes")
            && !window.contains("WindowAttributesExtWindows")
            && !window.contains("cfg(target_os"),
        "generic window attributes should delegate platform extension hooks"
    );
    assert!(
        platform.contains("#[cfg(target_os = \"windows\")]")
            && platform.contains("#[cfg(not(target_os = \"windows\"))]")
            && platform.contains("WindowAttributesExtWindows")
            && platform.contains("with_drag_and_drop(true)")
            && platform.contains("with_skip_taskbar(true)"),
        "target-specific window attribute extensions should stay in window/platform.rs"
    );
    assert!(
        tests.contains("generic_native_window_uses_configured_drag_and_drop_policy")
            && tests.contains("generic_native_window_applies_floating_popup_policy"),
        "generic window policy tests should continue covering platform-neutral decisions"
    );
}
