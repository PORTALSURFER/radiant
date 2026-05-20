use super::*;

#[test]
fn native_run_options_default_uses_generic_radiant_title() {
    let options = NativeRunOptions::default();

    assert_eq!(options.title, DEFAULT_NATIVE_WINDOW_TITLE);
    assert_eq!(options.title, "Radiant");
    assert!(options.drag_and_drop);
}

#[test]
fn native_run_options_expose_platform_neutral_drag_and_drop_policy() {
    let options = NativeRunOptions {
        drag_and_drop: false,
        ..NativeRunOptions::default()
    };

    assert!(!options.drag_and_drop);
}

#[test]
fn native_run_options_normalize_animation_frame_rate_policy() {
    let zero = NativeRunOptions {
        target_fps: 0,
        ..NativeRunOptions::default()
    };
    let default = NativeRunOptions::default();
    let high = NativeRunOptions {
        target_fps: u32::MAX,
        ..NativeRunOptions::default()
    };

    assert_eq!(zero.normalized_target_fps(), MIN_NATIVE_TARGET_FPS);
    assert_eq!(default.normalized_target_fps(), default.target_fps);
    assert_eq!(high.normalized_target_fps(), MAX_NATIVE_TARGET_FPS);
}

#[test]
fn native_run_options_validate_direct_launch_geometry() {
    let invalid_size = NativeRunOptions {
        inner_size: Some([f32::NAN, 480.0]),
        ..NativeRunOptions::default()
    };
    let invalid_popup = NativeRunOptions::popup("Drag Preview").popup_position(10.0, f32::INFINITY);

    assert!(matches!(
        invalid_size.validate(),
        Err(NativeRunOptionsError::InvalidSize {
            field: "inner_size",
            width,
            height: 480.0,
        }) if width.is_nan()
    ));
    assert_eq!(
        invalid_popup.validate(),
        Err(NativeRunOptionsError::InvalidPopupPosition {
            field: "popup_position",
            x: 10.0,
            y: f32::INFINITY,
        })
    );
    assert_eq!(
        invalid_popup.validate().unwrap_err().to_string(),
        "invalid native popup_position [10, inf]; popup positions must be finite"
    );

    let invalid_drag_region = NativeRunOptions::popup("Drag Preview")
        .popup_policy(NativePopupOptions::default().drag_region_height(f32::NAN));

    assert!(matches!(
        invalid_drag_region.validate(),
        Err(NativeRunOptionsError::InvalidPopupDragRegionHeight { height }) if height.is_nan()
    ));
}

#[test]
fn native_runtime_rejects_invalid_options_before_platform_startup() {
    let options = NativeRunOptions {
        inner_size: Some([0.0, 100.0]),
        ..NativeRunOptions::default()
    };
    let report = radiant::runtime::run_native_vello_runtime_with_artifacts(
        options,
        radiant::app(())
            .view(|_| radiant::prelude::text("invalid"))
            .into_bridge(),
    );

    assert!(matches!(
        report.result,
        Err(NativeGenericRunError::InvalidWindowOptions(
            NativeRunOptionsError::InvalidSize {
                field: "inner_size",
                width: 0.0,
                height: 100.0,
            }
        ))
    ));
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
    assert!(!options.decorations);
    assert!(!options.drag_and_drop);
    assert_eq!(options.title, "Drag Preview");
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
    assert_eq!(options.window_mode, NativeWindowMode::Popup(popup_policy));
}

#[test]
fn native_run_options_expose_prewarmed_popup_policy() {
    let popup_policy = NativePopupOptions::prewarmed_at(-32_000.0, -32_000.0);
    let options = NativeRunOptions::prewarmed_popup("Drag Preview", -32_000.0, -32_000.0);

    assert!(options.is_popup());
    assert!(!options.decorations);
    assert!(!options.drag_and_drop);
    assert_eq!(options.title, "Drag Preview");
    assert_eq!(options.popup_options(), Some(&popup_policy));
    assert_eq!(popup_policy.position, Some([-32_000.0, -32_000.0]));
    assert!(popup_policy.initially_visible);
    assert!(popup_policy.hide_after_first_present);
    assert!(!popup_policy.initially_focused);
}

#[test]
fn native_run_options_expose_layout_debug_overlay_policy() {
    let options = NativeRunOptions {
        debug_layout: true,
        ..NativeRunOptions::default()
    };

    assert!(!NativeRunOptions::default().debug_layout);
    assert!(options.debug_layout);
}

#[test]
fn native_run_options_expose_retained_surface_cache_policy() {
    let options = NativeRunOptions {
        retained_surface_cache: RetainedSurfaceCachePolicy::max_frames(8),
        ..NativeRunOptions::default()
    };

    assert_eq!(
        NativeRunOptions::default()
            .retained_surface_cache
            .max_frames,
        64
    );
    assert_eq!(options.retained_surface_cache.max_frames, 8);
    assert_eq!(
        RetainedSurfaceCachePolicy::max_frames(0).max_frames,
        0,
        "zero is the documented opt-out for retained-frame reuse"
    );
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
