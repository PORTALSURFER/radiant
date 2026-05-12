use radiant::runtime::{
    DEFAULT_NATIVE_WINDOW_TITLE, NativeGpuBackend, NativeGpuOptions, NativeRunOptions,
    NativeTextOptions, WindowManifest, WindowSpec,
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
            font_paths: vec!["fonts/App.ttf".into()],
        },
        ..NativeRunOptions::default()
    };
    let spec = WindowSpec::new("main", "Main").font_path("fonts/Spec.ttf");

    assert!(NativeRunOptions::default().text.font_paths.is_empty());
    assert_eq!(
        options.text.font_paths[0],
        std::path::PathBuf::from("fonts/App.ttf")
    );
    assert_eq!(
        spec.native_options().text.font_paths[0],
        std::path::PathBuf::from("fonts/Spec.ttf")
    );
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
    let options: NativeRunOptions = inspector.into();
    assert_eq!(options.inner_size, Some([320.5, 500.25]));
    assert_eq!(options.min_inner_size, Some([300.25, 420.5]));
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

    assert_eq!(duplicate, Err(String::from("duplicate window key 'main'")));
}
