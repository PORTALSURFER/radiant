use super::*;

#[test]
fn native_run_options_validate_direct_launch_geometry() {
    let invalid_size = NativeRunOptions {
        window: NativeWindowOptions {
            geometry: NativeWindowGeometry {
                inner_size: Some([f32::NAN, 480.0]),
                ..NativeWindowGeometry::default()
            },
            ..NativeWindowOptions::default()
        },
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
        window: NativeWindowOptions {
            geometry: NativeWindowGeometry {
                inner_size: Some([0.0, 100.0]),
                ..NativeWindowGeometry::default()
            },
            ..NativeWindowOptions::default()
        },
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
