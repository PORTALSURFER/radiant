use super::*;

#[test]
fn empty_with_capacity_presizes_primitive_storage() {
    let theme = ThemeTokens::default();
    let plan = SurfacePaintPlan::empty_with_capacity(&theme, 128);

    assert_eq!(plan.clear_color, theme.clear_color);
    assert!(plan.primitives.is_empty());
    assert!(plan.primitives.capacity() >= 128);
}

#[test]
fn clear_for_theme_with_capacity_reuses_primitive_storage() {
    let theme = ThemeTokens::default();
    let mut plan = SurfacePaintPlan::empty_with_capacity(&theme, 128);
    plan.primitives
        .push(PaintPrimitive::FillRect(PaintFillRect {
            widget_id: 1,
            rect: Default::default(),
            color: theme.accent_copper,
        }));
    let capacity = plan.primitives.capacity();

    plan.clear_for_theme_with_capacity(&theme, 16);

    assert!(plan.primitives.is_empty());
    assert_eq!(plan.primitives.capacity(), capacity);
}

#[test]
fn clear_for_theme_with_capacity_grows_to_requested_capacity() {
    let theme = ThemeTokens::default();
    let mut plan = SurfacePaintPlan::empty_with_capacity(&theme, 32);

    plan.clear_for_theme_with_capacity(&theme, 96);

    assert!(plan.primitives.capacity() >= 96);
}

#[test]
fn additional_reserve_for_capacity_treats_desired_capacity_as_target() {
    assert_eq!(additional_reserve_for_capacity(0, 32, 96), 96);
    assert_eq!(additional_reserve_for_capacity(8, 32, 96), 88);
    assert_eq!(additional_reserve_for_capacity(0, 96, 32), 0);
    assert_eq!(additional_reserve_for_capacity(0, 96, 96), 0);
}
