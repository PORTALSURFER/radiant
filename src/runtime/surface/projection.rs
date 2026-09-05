use super::{UiSurface, clear_paint_plan_for_layout, empty_paint_plan_for_layout};
use crate::{
    layout::LayoutOutput,
    runtime::paint::SurfacePaintPlan,
    theme::{AppearancePolicy, ResolvedAppearance, ThemeTokens},
};

impl<Message> UiSurface<Message> {
    /// Resolve one appearance policy against this surface's current snapshot.
    pub fn resolved_appearance(&self, policy: AppearancePolicy) -> ResolvedAppearance {
        policy.resolve(&self.resolved_environment())
    }

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

    /// Project the surface using a policy resolved once for this paint pass.
    pub fn paint_plan_with_policy(
        &self,
        layout: &LayoutOutput,
        policy: AppearancePolicy,
    ) -> SurfacePaintPlan {
        let environment = self.resolved_environment();
        let appearance = policy.resolve(&environment);
        let theme = appearance.tokens();
        let mut plan = empty_paint_plan_for_layout(layout, &theme);
        self.paint_plan_with_appearance_into(layout, appearance, &mut plan);
        plan
    }

    /// Alias for [`Self::paint_plan_with_policy`].
    pub fn paint_plan_with_appearance_policy(
        &self,
        layout: &LayoutOutput,
        policy: AppearancePolicy,
    ) -> SurfacePaintPlan {
        self.paint_plan_with_policy(layout, policy)
    }

    /// Fill a reusable plan using one resolved appearance snapshot.
    pub fn paint_plan_with_policy_into(
        &self,
        layout: &LayoutOutput,
        policy: AppearancePolicy,
        plan: &mut SurfacePaintPlan,
    ) {
        let environment = self.resolved_environment();
        let appearance = policy.resolve(&environment);
        self.paint_plan_with_appearance_into(layout, appearance, plan);
    }

    pub(in crate::runtime) fn paint_plan_with_appearance_into(
        &self,
        layout: &LayoutOutput,
        appearance: ResolvedAppearance,
        plan: &mut SurfacePaintPlan,
    ) {
        let theme = appearance.tokens();
        self.paint_plan_with_hover_and_environment_and_appearance_into(
            layout,
            &theme,
            self.resolved_environment(),
            appearance,
            None,
            None,
            plan,
        );
    }

    pub(in crate::runtime) fn paint_plan_with_hover_into(
        &self,
        layout: &LayoutOutput,
        theme: &ThemeTokens,
        hovered_container: Option<crate::layout::NodeId>,
        active_scroll_affordance: Option<crate::layout::NodeId>,
        plan: &mut SurfacePaintPlan,
    ) {
        self.paint_plan_with_hover_and_environment_into(
            layout,
            theme,
            self.resolved_environment(),
            hovered_container,
            active_scroll_affordance,
            plan,
        );
    }

    pub(in crate::runtime) fn paint_plan_with_hover_and_environment_into(
        &self,
        layout: &LayoutOutput,
        theme: &ThemeTokens,
        environment: crate::runtime::ResolvedEnvironment,
        hovered_container: Option<crate::layout::NodeId>,
        active_scroll_affordance: Option<crate::layout::NodeId>,
        plan: &mut SurfacePaintPlan,
    ) {
        self.paint_plan_with_hover_and_environment_and_appearance_into(
            layout,
            theme,
            environment,
            ResolvedAppearance::fixed(*theme),
            hovered_container,
            active_scroll_affordance,
            plan,
        );
    }

    // The explicit arguments document the immutable pass inputs and preserve
    // the existing hover/clip projection boundary.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::runtime) fn paint_plan_with_hover_and_environment_and_appearance_into(
        &self,
        layout: &LayoutOutput,
        theme: &ThemeTokens,
        environment: crate::runtime::ResolvedEnvironment,
        appearance: ResolvedAppearance,
        hovered_container: Option<crate::layout::NodeId>,
        active_scroll_affordance: Option<crate::layout::NodeId>,
        plan: &mut SurfacePaintPlan,
    ) {
        clear_paint_plan_for_layout(plan, layout, theme);
        self.root.append_paint(
            layout,
            theme,
            environment,
            appearance,
            plan,
            hovered_container,
            active_scroll_affordance,
        );
    }
}
