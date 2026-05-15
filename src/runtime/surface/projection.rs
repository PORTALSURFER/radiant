use super::{UiSurface, clear_paint_plan_for_layout, empty_paint_plan_for_layout};
use crate::{layout::LayoutOutput, runtime::paint::SurfacePaintPlan, theme::ThemeTokens};

impl<Message> UiSurface<Message> {
    /// Project the surface and its layout output into backend-neutral paint data.
    ///
    /// Primitives are emitted in declarative tree order so backends and tests can
    /// compare output deterministically without depending on the native shell.
    pub fn paint_plan(&self, layout: &LayoutOutput, theme: &ThemeTokens) -> SurfacePaintPlan {
        let mut plan = empty_paint_plan_for_layout(layout, theme);
        self.paint_plan_into(layout, theme, &mut plan);
        plan
    }

    /// Project backend-neutral paint data into an existing plan buffer.
    ///
    /// This is the allocation-lean counterpart to [`Self::paint_plan`] for
    /// hosts and renderers that rebuild paint data every frame.
    pub fn paint_plan_into(
        &self,
        layout: &LayoutOutput,
        theme: &ThemeTokens,
        plan: &mut SurfacePaintPlan,
    ) {
        self.paint_plan_with_hover_into(layout, theme, None, None, plan);
    }

    pub(in crate::runtime) fn paint_plan_with_hover_into(
        &self,
        layout: &LayoutOutput,
        theme: &ThemeTokens,
        hovered_container: Option<crate::layout::NodeId>,
        active_scroll_affordance: Option<crate::layout::NodeId>,
        plan: &mut SurfacePaintPlan,
    ) {
        clear_paint_plan_for_layout(plan, layout, theme);
        self.root.append_paint(
            layout,
            theme,
            plan,
            hovered_container,
            active_scroll_affordance,
        );
        crate::runtime::paint::push_layout_debug_overlay(layout, plan);
    }
}
