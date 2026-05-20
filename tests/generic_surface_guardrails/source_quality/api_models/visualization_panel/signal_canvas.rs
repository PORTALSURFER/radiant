use super::*;

#[test]
fn signal_visualization_state_uses_named_parts_for_status_and_preview_fields() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = manifest_dir.join("src/gui/visualization/signal.rs");
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));
    let chrome = fs::read_to_string(manifest_dir.join("src/gui/visualization/signal/chrome.rs"))
        .expect("signal chrome state source should be readable");
    let preview = fs::read_to_string(manifest_dir.join("src/gui/visualization/signal/preview.rs"))
        .expect("signal raster preview source should be readable");
    let tools = fs::read_to_string(manifest_dir.join("src/gui/visualization/signal/tools.rs"))
        .expect("signal tool state source should be readable");

    for required in [
        "mod chrome;",
        "mod preview;",
        "mod tools;",
        "pub use chrome::{ChannelViewMode, SignalChromeParts, SignalChromeState};",
        "pub use preview::{SignalRasterPreview, SignalRasterPreviewParts};",
        "pub use tools::{SignalToolFlags, SignalToolState};",
    ] {
        assert!(
            source.contains(required),
            "signal visualization root should keep public re-exports while delegating `{required}`"
        );
    }

    assert!(
        chrome.contains("pub struct SignalChromeParts")
            && chrome.contains("pub fn from_parts(parts: SignalChromeParts) -> Self"),
        "signal chrome state should expose named parts for readable public construction"
    );
    assert!(
        preview.contains("pub struct SignalRasterPreviewParts")
            && preview.contains("pub fn from_parts(parts: SignalRasterPreviewParts) -> Self"),
        "signal raster preview state should expose named parts for readable public construction"
    );
    assert!(
        chrome.contains("Self::from_parts(SignalChromeParts {")
            && preview.contains("Self::from_parts(SignalRasterPreviewParts {"),
        "signal compatibility constructors should delegate through named parts objects"
    );
    assert!(
        !source.contains("pub struct SignalChromeState")
            && !source.contains("pub struct SignalRasterPreview")
            && !source.contains("pub struct SignalToolState")
            && chrome.contains("pub enum ChannelViewMode")
            && preview.contains("Arc<ImageRgba>")
            && tools.contains("pub struct SignalToolFlags")
            && tools.contains("pub fn from_flags(flags: SignalToolFlags) -> Self"),
        "signal chrome, raster preview, and tool flags should stay in focused visualization modules"
    );
}

#[test]
fn canvas_layer_state_uses_named_parts_for_hit_test_fields() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = manifest_dir.join("src/gui/visualization/canvas.rs");
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));

    assert!(
        source.contains("pub struct CanvasLayerParts")
            && source.contains("pub fn from_parts(parts: CanvasLayerParts) -> Self"),
        "canvas layer state should expose named parts for readable public construction"
    );
    assert!(
        source.contains("Self::from_parts(CanvasLayerParts {"),
        "the positional compatibility constructor should delegate through the named parts object"
    );
}
