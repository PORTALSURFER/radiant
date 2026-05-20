use super::*;

#[test]
fn estimated_paint_primitive_capacity_scales_for_small_layouts() {
    let mut layout = LayoutOutput::default();
    for node_id in 0..16 {
        layout.rects.insert(node_id, Default::default());
    }

    assert_eq!(estimated_paint_primitive_capacity(&layout), 48);
}

#[test]
fn estimated_paint_primitive_capacity_caps_large_initial_reserves() {
    let mut layout = LayoutOutput::default();
    for node_id in 0..10_000 {
        layout.rects.insert(node_id, Default::default());
    }

    assert_eq!(estimated_paint_primitive_capacity(&layout), 1024);
}

#[test]
fn empty_paint_plan_for_layout_presizes_with_layout_estimate() {
    let mut layout = LayoutOutput::default();
    for node_id in 0..16 {
        layout.rects.insert(node_id, Default::default());
    }

    let plan = empty_paint_plan_for_layout(&layout, &ThemeTokens::default());

    assert!(plan.primitives.is_empty());
    assert!(plan.primitives.capacity() >= 48);
}

#[test]
fn clear_paint_plan_for_layout_reuses_existing_capacity() {
    let mut layout = LayoutOutput::default();
    for node_id in 0..16 {
        layout.rects.insert(node_id, Default::default());
    }
    let theme = ThemeTokens::default();
    let mut plan = SurfacePaintPlan::empty(&theme);
    plan.primitives.reserve(128);
    let capacity = plan.primitives.capacity();

    clear_paint_plan_for_layout(&mut plan, &layout, &theme);

    assert!(plan.primitives.is_empty());
    assert_eq!(plan.primitives.capacity(), capacity);
}
