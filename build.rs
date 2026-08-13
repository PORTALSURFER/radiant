#![allow(missing_docs)]

fn main() {
    println!(
        "cargo:rerun-if-changed=src/gui_runtime/native_vello/generic_runtime/native_semantic_accessibility_exception.m"
    );
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        cc::Build::new()
            .file("src/gui_runtime/native_vello/generic_runtime/native_semantic_accessibility_exception.m")
            .flag("-fobjc-arc")
            .compile("radiant_native_semantic_accessibility_exception");
    }
}
