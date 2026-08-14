use std::{fs, path::PathBuf};

#[test]
fn native_semantic_host_attachment_uses_supported_children_property() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let rust_source = fs::read_to_string(
        manifest_dir
            .join("src/gui_runtime/native_vello/generic_runtime/native_semantic_accessibility.rs"),
    )
    .expect("native semantic accessibility source should be readable");
    let objective_c_source = fs::read_to_string(manifest_dir.join(
        "src/gui_runtime/native_vello/generic_runtime/native_semantic_accessibility_exception.m",
    ))
    .expect("native semantic accessibility Objective-C source should be readable");

    assert!(objective_c_source.contains("setAccessibilityChildren:"));
    assert!(objective_c_source.contains("accessibilityChildren"));
    assert!(objective_c_source.contains("id readback = [host accessibilityChildren];"));
    assert!(objective_c_source.contains("[(NSArray *)readback count] == 0"));
    for selector in [
        "setAccessibilityRole:",
        "accessibilityRole",
        "setAccessibilityParent:",
        "accessibilityParent",
        "setAccessibilityChildren:",
        "accessibilityChildren",
        "setAccessibilityFrame:",
        "accessibilityFrame",
        "setAccessibilityLabel:",
        "accessibilityLabel",
        "setAccessibilityTitle:",
        "accessibilityTitle",
        "setAccessibilityHelp:",
        "accessibilityHelp",
        "setAccessibilityEnabled:",
        "isAccessibilityEnabled",
    ] {
        assert!(
            objective_c_source.contains(selector),
            "modern NSAccessibility selector must remain configured and verified: {selector}"
        );
    }
    assert!(objective_c_source.contains("radiant_native_configure_accessibility_element"));
    assert!(objective_c_source.contains("accessibilityValue is left callback-backed"));
    assert!(
        !objective_c_source.contains("setAccessibilityValue"),
        "modern configuration must not seed accessibilityValue through its setter"
    );
    let instantiate = rust_source
        .find("fn instantiate_specs")
        .expect("native instantiation function should remain present");
    let replace = rust_source
        .find("fn replace_callback_projection")
        .expect("native host replacement function should remain present");
    let configure = rust_source[instantiate..replace]
        .find("self.configure_modern_projection")
        .expect("modern properties must be configured during instantiation");
    let host_commit = rust_source[replace..]
        .find("radiant_native_set_accessibility_children(self.view, root)")
        .expect("host attachment commit boundary should remain present");
    assert!(
        instantiate + configure < replace + host_commit,
        "modern property configuration must precede host attachment"
    );
    assert!(rust_source.contains("committed_objects"));
    assert!(rust_source.contains("radiant_native_clear_accessibility_children(self.view) == YES"));
    for source in [&rust_source, &objective_c_source] {
        assert!(
            !source.contains("accessibilitySetOverrideValue:forAttribute:"),
            "native semantic host attachment must not use deprecated AXChildren override"
        );
    }
}
