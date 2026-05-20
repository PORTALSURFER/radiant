use super::*;

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
fn window_specs_support_named_parts_construction() {
    let spec = WindowSpec::from_parts(WindowSpecParts {
        key: "main".to_owned(),
        options: NativeRunOptions {
            title: "Main".to_owned(),
            inner_size: Some([640.0, 480.0]),
            ..NativeRunOptions::default()
        },
    });

    assert_eq!(spec.key, "main");
    assert_eq!(spec.title(), "Main");
    assert_eq!(spec.inner_size(), Some([640.0, 480.0]));
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
fn window_specs_describe_prewarmed_popup_windows() {
    let popup = WindowSpec::prewarmed_popup("drag-preview", "Drag Preview", -32_000.0, -32_000.0)
        .logical_size(180.0, 64.0);

    assert_eq!(popup.key, "drag-preview");
    assert_eq!(popup.title(), "Drag Preview");
    assert!(popup.is_popup());
    assert_eq!(
        popup.popup_options().map(|popup| popup.position),
        Some(Some([-32_000.0, -32_000.0]))
    );
    assert_eq!(
        popup
            .popup_options()
            .map(|popup| popup.hide_after_first_present),
        Some(true)
    );
}
