use super::*;

#[test]
fn surface_paint_plan_buffering_stays_with_capacity_policy() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let projection = fs::read_to_string(manifest_dir.join("src/runtime/surface/projection.rs"))
        .expect("surface paint projection should be readable");
    let frame = fs::read_to_string(manifest_dir.join("src/runtime/controller/context/frame.rs"))
        .expect("runtime frame paint projection should be readable");
    let capacity = fs::read_to_string(manifest_dir.join("src/runtime/surface/paint/capacity.rs"))
        .expect("surface paint capacity policy should be readable");
    let capacity_tests =
        fs::read_to_string(manifest_dir.join("src/runtime/surface/paint/capacity/tests.rs"))
            .expect("surface paint capacity tests should be readable");

    assert!(
        capacity.contains("fn empty_paint_plan_for_layout")
            && capacity.contains("fn clear_paint_plan_for_layout")
            && capacity.contains("fn estimated_paint_primitive_capacity")
            && capacity.contains("#[path = \"capacity/tests.rs\"]")
            && !capacity.contains("fn estimated_paint_primitive_capacity_scales_for_small_layouts"),
        "layout-aware paint-plan buffer lifecycle should live with the capacity policy while behavior tests stay delegated"
    );
    assert!(
        capacity_tests.contains("fn estimated_paint_primitive_capacity_scales_for_small_layouts")
            && capacity_tests.contains("fn clear_paint_plan_for_layout_reuses_existing_capacity"),
        "surface paint capacity behavior coverage should live in surface/paint/capacity/tests.rs"
    );
    assert!(
        projection.contains("empty_paint_plan_for_layout(layout, theme)")
            && projection.contains("clear_paint_plan_for_layout(plan, layout, theme)")
            && frame.contains("empty_paint_plan_for_layout(&self.layout, theme)"),
        "surface and runtime paint projection should consume layout-aware plan helpers"
    );
    assert!(
        !projection.contains("estimated_paint_primitive_capacity")
            && !frame.contains("estimated_paint_primitive_capacity")
            && !projection.contains("SurfacePaintPlan::empty_with_capacity")
            && !frame.contains("SurfacePaintPlan::empty_with_capacity"),
        "paint projection callers should not duplicate capacity-policy mechanics"
    );
}
