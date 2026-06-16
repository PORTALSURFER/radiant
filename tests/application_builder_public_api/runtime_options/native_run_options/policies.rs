use super::*;

#[test]
fn native_run_options_expose_utility_window_policy() {
    let options = NativeRunOptions::utility_window("Audio Settings", 360.0, 420.0);

    assert!(!options.is_popup());
    assert_eq!(options.window.title, "Audio Settings");
    assert_eq!(options.window.geometry.inner_size, Some([360.0, 420.0]));
    assert_eq!(options.window.geometry.min_inner_size, Some([360.0, 420.0]));
    assert!(options.window.behavior.decorations);
    assert!(!options.window.behavior.drag_and_drop);
    assert!(options.window.behavior.skip_taskbar);
    assert_eq!(options.window.behavior.mode, NativeWindowMode::Window);
}

#[test]
fn native_run_options_expose_floating_popup_policy() {
    let popup_policy = NativePopupOptions::default()
        .position(120.0, 240.0)
        .transparent(false)
        .always_on_top(false)
        .resizable(true)
        .initially_focused(true)
        .skip_taskbar(false)
        .initially_visible(false)
        .hide_after_first_present(true)
        .drag_region_height(36.0);
    let options = NativeRunOptions::popup("Drag Preview").popup_policy(popup_policy);

    assert!(!NativeRunOptions::default().is_popup());
    assert!(options.is_popup());
    assert!(!options.window.behavior.decorations);
    assert!(!options.window.behavior.drag_and_drop);
    assert_eq!(options.window.title, "Drag Preview");
    assert_eq!(options.popup_options(), Some(&popup_policy));
    assert_eq!(
        options.popup_options().map(|popup| popup.initially_visible),
        Some(false)
    );
    assert_eq!(
        options
            .popup_options()
            .map(|popup| popup.hide_after_first_present),
        Some(true)
    );
    assert_eq!(
        options.window.behavior.mode,
        NativeWindowMode::Popup(popup_policy)
    );
}

#[test]
fn native_run_options_expose_prewarmed_popup_policy() {
    let popup_policy = NativePopupOptions::prewarmed_at(-32_000.0, -32_000.0);
    let options = NativeRunOptions::prewarmed_popup("Drag Preview", -32_000.0, -32_000.0);

    assert!(options.is_popup());
    assert!(!options.window.behavior.decorations);
    assert!(!options.window.behavior.drag_and_drop);
    assert_eq!(options.window.title, "Drag Preview");
    assert_eq!(options.popup_options(), Some(&popup_policy));
    assert_eq!(popup_policy.position, Some([-32_000.0, -32_000.0]));
    assert!(popup_policy.initially_visible);
    assert!(popup_policy.hide_after_first_present);
    assert!(!popup_policy.initially_focused);
}

#[test]
fn native_run_options_expose_retained_surface_cache_policy() {
    let options = NativeRunOptions {
        frame: NativeFrameOptions {
            retained_surface_cache: RetainedSurfaceCachePolicy::max_frames(8),
            ..NativeFrameOptions::default()
        },
        ..NativeRunOptions::default()
    };

    assert_eq!(
        NativeRunOptions::default()
            .frame
            .retained_surface_cache
            .max_frames,
        64
    );
    assert_eq!(options.frame.retained_surface_cache.max_frames, 8);
    assert_eq!(
        RetainedSurfaceCachePolicy::max_frames(0).max_frames,
        0,
        "zero is the documented opt-out for retained-frame reuse"
    );
}

#[test]
fn native_run_options_expose_devtools_overlay_policy() {
    let options = NativeRunOptions::default().devtools_overlay(DevtoolsOverlayOptions::enabled());

    assert!(!NativeRunOptions::default().frame.devtools.is_enabled());
    assert!(options.frame.devtools.is_enabled());
    let disabled = options.devtools_overlay_enabled(false);
    assert!(!disabled.frame.devtools.is_enabled());
}

#[test]
fn native_run_options_expose_gpu_backend_policy() {
    let options = NativeRunOptions {
        gpu: NativeGpuOptions {
            backend: NativeGpuBackend::Dx12,
        },
        ..NativeRunOptions::default()
    };
    let spec = WindowSpec::new("main", "Main").gpu_backend(NativeGpuBackend::Vulkan);

    assert_eq!(
        NativeRunOptions::default().gpu.backend,
        NativeGpuBackend::Auto
    );
    assert_eq!(options.gpu.backend, NativeGpuBackend::Dx12);
    assert_eq!(spec.native_options().gpu.backend, NativeGpuBackend::Vulkan);
}

#[test]
fn native_run_options_expose_text_font_policy() {
    let options = NativeRunOptions {
        text: NativeTextOptions {
            embedded_fonts: vec![EmbeddedFont::from_static(b"app-font").with_index(1)],
            font_paths: vec!["fonts/App.ttf".into()],
        },
        ..NativeRunOptions::default()
    };
    let spec = WindowSpec::new("main", "Main")
        .embedded_font(EmbeddedFont::from_static(b"spec-font"))
        .font_path("fonts/Spec.ttf");

    assert!(NativeRunOptions::default().text.embedded_fonts.is_empty());
    assert!(NativeRunOptions::default().text.font_paths.is_empty());
    assert_eq!(options.text.embedded_fonts[0].bytes(), b"app-font");
    assert_eq!(options.text.embedded_fonts[0].index(), 1);
    assert_eq!(
        options.text.font_paths[0],
        std::path::PathBuf::from("fonts/App.ttf")
    );
    assert_eq!(
        spec.native_options().text.embedded_fonts[0].bytes(),
        b"spec-font"
    );
    assert_eq!(
        spec.native_options().text.font_paths[0],
        std::path::PathBuf::from("fonts/Spec.ttf")
    );
}
