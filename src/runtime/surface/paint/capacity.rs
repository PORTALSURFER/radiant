use crate::{layout::LayoutOutput, runtime::paint::SurfacePaintPlan, theme::ThemeTokens};

#[cfg(test)]
#[path = "capacity/tests.rs"]
mod tests;

const MAX_INITIAL_PAINT_PRIMITIVE_CAPACITY: usize = 1024;

fn estimated_paint_primitive_capacity(layout: &LayoutOutput) -> usize {
    layout
        .rects
        .len()
        .saturating_mul(3)
        .min(MAX_INITIAL_PAINT_PRIMITIVE_CAPACITY)
}

pub(in crate::runtime) fn empty_paint_plan_for_layout(
    layout: &LayoutOutput,
    theme: &ThemeTokens,
) -> SurfacePaintPlan {
    SurfacePaintPlan::empty_with_capacity(theme, estimated_paint_primitive_capacity(layout))
}

pub(in crate::runtime) fn clear_paint_plan_for_layout(
    plan: &mut SurfacePaintPlan,
    layout: &LayoutOutput,
    theme: &ThemeTokens,
) {
    plan.clear_for_theme_with_capacity(theme, estimated_paint_primitive_capacity(layout));
}
