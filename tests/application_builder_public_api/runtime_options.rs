use radiant::runtime::{
    DEFAULT_NATIVE_WINDOW_TITLE, EmbeddedFont, MAX_NATIVE_TARGET_FPS, MIN_NATIVE_TARGET_FPS,
    NativeGenericRunError, NativeGpuBackend, NativeGpuOptions, NativePopupOptions,
    NativeRunOptions, NativeRunOptionsError, NativeTextOptions, NativeWindowMode, WindowManifest,
    WindowManifestError, WindowSpec, WindowSpecError,
};

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
    assert_eq!(options.window_mode, NativeWindowMode::Popup(popup_policy));
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

#[test]
fn launch_builders_expose_embedded_font_policy() {
    let no_state = radiant::window("Main")
        .embedded_font(EmbeddedFont::from_static(b"window-font"))
        .font_path("fonts/Window.ttf")
        .spec("main");
    let stateful = radiant::app(())
        .embedded_font(EmbeddedFont::from_static(b"state-font"))
        .font_path("fonts/State.ttf");

    assert_eq!(
        no_state.native_options().text.embedded_fonts[0].bytes(),
        b"window-font"
    );
    assert_eq!(
        no_state.native_options().text.font_paths[0],
        std::path::PathBuf::from("fonts/Window.ttf")
    );
    let _ = stateful;
}

#[test]
fn window_specs_describe_multiple_windows_without_opening_runtime() {
    let main = radiant::window("Main")
        .size(800, 600)
        .min_size(640, 480)
        .spec("main");
    let inspector = WindowSpec::new("inspector", "Inspector")
        .logical_size(320.5, 500.25)
        .min_logical_size(300.25, 420.5)
        .drag_and_drop(false)
        .target_fps(60);

    assert_eq!(main.key, "main");
    assert_eq!(main.title(), "Main");
    assert_eq!(main.inner_size(), Some([800.0, 600.0]));
    assert_eq!(main.min_inner_size(), Some([640.0, 480.0]));
    assert_eq!(inspector.title(), "Inspector");
    assert_eq!(inspector.inner_size(), Some([320.5, 500.25]));
    assert_eq!(inspector.min_inner_size(), Some([300.25, 420.5]));
    assert!(!inspector.drag_and_drop_enabled());
    assert_eq!(inspector.target_frame_rate(), 60);
    assert_eq!(inspector.normalized_target_frame_rate(), 60);
    let options: NativeRunOptions = inspector.into();
    assert_eq!(options.inner_size, Some([320.5, 500.25]));
    assert_eq!(options.min_inner_size, Some([300.25, 420.5]));
}

#[test]
fn window_specs_describe_floating_popup_windows() {
    let popup = WindowSpec::popup("drag-preview", "Drag Preview")
        .logical_size(180.0, 64.0)
        .popup_position(300.0, 220.0);

    assert_eq!(popup.key, "drag-preview");
    assert_eq!(popup.title(), "Drag Preview");
    assert!(popup.is_popup());
    assert_eq!(popup.inner_size(), Some([180.0, 64.0]));
    assert!(!popup.native_options().decorations);
    assert!(!popup.drag_and_drop_enabled());
    assert_eq!(
        popup.popup_options().and_then(|popup| popup.position),
        Some([300.0, 220.0])
    );
}

#[test]
fn window_manifest_validates_stable_unique_window_keys() {
    let manifest = WindowManifest::from_specs([
        WindowSpec::new("main", "Main").size(800, 600),
        WindowSpec::new("inspector", "Inspector").size(320, 500),
    ])
    .expect("unique keys should be valid");

    assert_eq!(manifest.len(), 2);
    assert_eq!(manifest.keys().collect::<Vec<_>>(), ["main", "inspector"]);
    assert_eq!(
        manifest.get("inspector").unwrap().inner_size(),
        Some([320.0, 500.0])
    );
    assert!(manifest.validate().is_ok());
}

#[test]
fn window_manifest_rejects_duplicate_window_keys() {
    let duplicate = WindowManifest::from_specs([
        WindowSpec::new("main", "Main"),
        WindowSpec::new("main", "Duplicate"),
    ]);

    let error = duplicate.expect_err("duplicate key should fail");

    assert_eq!(
        error,
        WindowManifestError::DuplicateKey {
            key: String::from("main"),
        }
    );
    assert_eq!(error.to_string(), "duplicate window key 'main'");
}

#[test]
fn window_manifest_rejects_invalid_window_geometry() {
    let invalid_size =
        WindowManifest::from_specs([WindowSpec::new("main", "Main").logical_size(f32::NAN, 480.0)]);

    let error = invalid_size.expect_err("invalid size should fail");

    assert!(matches!(
        error,
        WindowManifestError::InvalidSpec(WindowSpecError::InvalidSize {
            ref key,
            field: "inner_size",
            width,
            height: 480.0,
        }) if key == "main" && width.is_nan()
    ));
    assert_eq!(
        error.to_string(),
        "window 'main' has invalid inner_size [NaN, 480]; logical sizes must be finite and positive"
    );

    let invalid_popup =
        WindowSpec::popup("drag-preview", "Drag Preview").popup_position(f32::INFINITY, 220.0);

    let error = invalid_popup
        .validate()
        .expect_err("invalid popup position should fail");

    assert_eq!(
        error,
        WindowSpecError::InvalidPopupPosition {
            key: String::from("drag-preview"),
            x: f32::INFINITY,
            y: 220.0,
        }
    );
    assert_eq!(
        error.to_string(),
        "window 'drag-preview' has invalid popup position [inf, 220]; popup positions must be finite"
    );

    let invalid_drag_region = WindowSpec::popup("drag-preview", "Drag Preview")
        .popup_policy(NativePopupOptions::default().drag_region_height(-1.0));

    let error = invalid_drag_region
        .validate()
        .expect_err("invalid popup drag region should fail");

    assert_eq!(
        error,
        WindowSpecError::InvalidPopupDragRegionHeight {
            key: String::from("drag-preview"),
            height: -1.0,
        }
    );
    assert_eq!(
        error.to_string(),
        "window 'drag-preview' has invalid popup drag region height [-1]; height must be finite and non-negative"
    );
}
