use super::*;

#[test]
fn window_specs_use_named_parts_for_manifest_identity_and_options() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let spec_path = manifest_dir.join("src/gui_runtime/window_manifest/spec.rs");
    let builder_path = manifest_dir.join("src/gui_runtime/window_manifest/spec/builders.rs");
    let spec = fs::read_to_string(&spec_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", spec_path.display()));
    let builders = fs::read_to_string(&builder_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", builder_path.display()));
    let manifest = fs::read_to_string(manifest_dir.join("src/gui_runtime/window_manifest.rs"))
        .expect("window manifest module should be readable");
    let runtime = fs::read_to_string(manifest_dir.join("src/runtime/mod.rs"))
        .expect("runtime module should be readable");
    let lib = fs::read_to_string(manifest_dir.join("src/lib.rs"))
        .expect("library module should be readable");

    assert!(
        spec.contains("pub struct WindowSpecParts")
            && builders.contains("pub fn from_parts(parts: WindowSpecParts) -> Self"),
        "window specs should expose named parts for stable key and native options"
    );
    assert!(
        builders.contains("Self::from_parts(WindowSpecParts {")
            && manifest.contains("WindowSpecParts")
            && runtime.contains("WindowSpecParts")
            && lib.contains("WindowSpecParts"),
        "window spec compatibility constructors and public exports should keep the named-parts path available"
    );
}
