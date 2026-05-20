use super::*;

#[test]
fn native_runtime_test_fixtures_stay_grouped_by_runtime_concern() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/tests/fixtures.rs"),
    )
    .expect("native runtime fixture root should be readable");
    let demo = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/tests/fixtures/demo.rs"),
    )
    .expect("native runtime demo fixtures should be readable");
    let frame = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/tests/fixtures/frame.rs"),
    )
    .expect("native runtime frame fixtures should be readable");
    let input = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/tests/fixtures/input.rs"),
    )
    .expect("native runtime input fixtures should be readable");
    let gpu_wheel = fs::read_to_string(
        manifest_dir
            .join("src/gui_runtime/native_vello/generic_runtime/tests/fixtures/gpu_wheel.rs"),
    )
    .expect("native runtime GPU wheel fixtures should be readable");

    assert!(
        root.contains("mod demo;")
            && root.contains("mod frame;")
            && root.contains("mod input;")
            && root.contains("mod gpu_wheel;")
            && !root.contains("struct TestGpuWheelWidget"),
        "native runtime fixture root should index focused fixture groups instead of owning all test support"
    );
    assert!(
        demo.contains("fn demo_surface")
            && frame.contains("struct PaintOnlyFrameBridge")
            && input.contains("struct WheelRefreshBridge")
            && gpu_wheel.contains("struct TestGpuWheelWidget"),
        "native runtime fixtures should stay grouped by demo, frame, input, and GPU wheel concerns"
    );
}
