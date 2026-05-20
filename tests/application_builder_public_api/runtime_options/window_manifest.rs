use super::*;

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
fn window_manifest_rejects_empty_window_keys() {
    let empty = WindowManifest::from_specs([WindowSpec::new("", "Untitled")]);

    let error = empty.expect_err("empty key should fail");

    assert_eq!(
        error,
        WindowManifestError::InvalidSpec(WindowSpecError::InvalidKey { key: String::new() })
    );
    assert_eq!(
        error.to_string(),
        "window key '' is invalid; stable window keys must not be empty"
    );

    let whitespace = WindowManifest::from_specs([WindowSpec::new("   ", "Untitled")]);

    let error = whitespace.expect_err("whitespace key should fail");

    assert_eq!(
        error,
        WindowManifestError::InvalidSpec(WindowSpecError::InvalidKey {
            key: String::from("   "),
        })
    );
    assert_eq!(
        error.to_string(),
        "window key '   ' is invalid; stable window keys must not be empty"
    );
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
