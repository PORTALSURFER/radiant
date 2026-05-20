use super::*;
use crate::rust_sources_under;
use std::collections::BTreeSet;

const ALLOWED_PLATFORM_SPECIFIC_SOURCE_FILES: &[&str] = &[
    "examples/popup_window/host/child.rs",
    "examples/popup_window/host/prewarm.rs",
    "examples/popup_window/host/process.rs",
    "examples/popup_window/platform.rs",
    "examples/popup_window/platform/readiness.rs",
    "src/gui_runtime/native_vello/generic_runtime/external_drag/platform.rs",
    "src/gui_runtime/native_vello/generic_runtime/window/platform.rs",
    "src/gui_runtime/native_vello/text_renderer/font.rs",
];

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

#[test]
fn target_specific_platform_code_stays_in_documented_adapters() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let architecture = fs::read_to_string(manifest_dir.join("docs/ARCHITECTURE.md"))
        .expect("architecture docs should be readable");
    let allowed = ALLOWED_PLATFORM_SPECIFIC_SOURCE_FILES
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let mut undocumented = Vec::new();

    for path in rust_sources_under(&manifest_dir.join("src"))
        .into_iter()
        .chain(rust_sources_under(&manifest_dir.join("examples")))
    {
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
        if !contains_target_specific_platform_code(&source) {
            continue;
        }

        let relative = relative_path(&manifest_dir, &path);
        if !allowed.contains(relative.as_str()) {
            undocumented.push(relative);
            continue;
        }

        assert!(
            architecture.contains(&format!("`{relative}`")),
            "docs/ARCHITECTURE.md should document target-specific adapter `{relative}`"
        );
    }

    undocumented.sort();
    assert!(
        undocumented.is_empty(),
        "target-specific platform code should stay in documented adapters:\n{}",
        undocumented.join("\n")
    );
}

fn contains_target_specific_platform_code(source: &str) -> bool {
    source.contains("target_os = ")
        || source.contains("target_os=")
        || source.contains("platform::windows")
        || source.contains("windows_sys")
        || source.contains("WindowAttributesExtWindows")
}
