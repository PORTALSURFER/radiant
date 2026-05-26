/// Routing result consumed by redraw, scene refresh, and runtime wakeup policy.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(in crate::gui_runtime::native_vello) struct GenericRouteOutcome {
    pub(in crate::gui_runtime::native_vello) routed: bool,
    pub(in crate::gui_runtime::native_vello) redraw_requested: bool,
    pub(in crate::gui_runtime::native_vello) repaint_requested: bool,
    pub(in crate::gui_runtime::native_vello) paint_only_requested: bool,
    pub(in crate::gui_runtime::native_vello) deferred_surface_refresh_requested: bool,
    pub(in crate::gui_runtime::native_vello) interactive_surface_refresh_requested: bool,
    pub(in crate::gui_runtime::native_vello) interactive_scene_rebuild_requested: bool,
    pub(in crate::gui_runtime::native_vello) exit_requested: bool,
    pub(in crate::gui_runtime::native_vello) runtime_work_remaining: bool,
    pub(in crate::gui_runtime::native_vello) dpi_scale_override: Option<crate::theme::DpiScale>,
    pub(in crate::gui_runtime::native_vello) window_logical_size: Option<crate::layout::Vector2>,
}

impl GenericRouteOutcome {
    pub(in crate::gui_runtime::native_vello) fn needs_redraw(self) -> bool {
        self.needs_scene_rebuild()
            || self.paint_only_requested
            || self.deferred_surface_refresh_requested
    }

    pub(in crate::gui_runtime::native_vello) fn needs_scene_rebuild(self) -> bool {
        self.redraw_requested || self.repaint_requested
    }

    pub(in crate::gui_runtime::native_vello) fn merge(&mut self, other: Self) {
        self.routed |= other.routed;
        self.redraw_requested |= other.redraw_requested;
        self.repaint_requested |= other.repaint_requested;
        self.paint_only_requested |= other.paint_only_requested;
        self.deferred_surface_refresh_requested |= other.deferred_surface_refresh_requested;
        self.interactive_surface_refresh_requested |= other.interactive_surface_refresh_requested;
        self.interactive_scene_rebuild_requested |= other.interactive_scene_rebuild_requested;
        self.exit_requested |= other.exit_requested;
        self.runtime_work_remaining |= other.runtime_work_remaining;
        self.dpi_scale_override = other.dpi_scale_override.or(self.dpi_scale_override);
        self.window_logical_size = other.window_logical_size.or(self.window_logical_size);
    }
}

#[cfg(test)]
mod tests {
    use super::GenericRouteOutcome;

    #[test]
    fn route_outcome_redraw_policy_distinguishes_paint_only_from_scene_rebuild() {
        let paint_only = GenericRouteOutcome {
            paint_only_requested: true,
            ..GenericRouteOutcome::default()
        };
        let deferred = GenericRouteOutcome {
            deferred_surface_refresh_requested: true,
            ..GenericRouteOutcome::default()
        };
        let scene = GenericRouteOutcome {
            repaint_requested: true,
            ..GenericRouteOutcome::default()
        };

        assert!(paint_only.needs_redraw());
        assert!(!paint_only.needs_scene_rebuild());
        assert!(deferred.needs_redraw());
        assert!(!deferred.needs_scene_rebuild());
        assert!(scene.needs_redraw());
        assert!(scene.needs_scene_rebuild());
    }

    #[test]
    fn route_outcome_merge_preserves_all_work_flags() {
        let mut outcome = GenericRouteOutcome {
            routed: true,
            repaint_requested: true,
            ..GenericRouteOutcome::default()
        };

        outcome.merge(GenericRouteOutcome {
            paint_only_requested: true,
            deferred_surface_refresh_requested: true,
            interactive_surface_refresh_requested: true,
            interactive_scene_rebuild_requested: true,
            exit_requested: true,
            runtime_work_remaining: true,
            ..GenericRouteOutcome::default()
        });

        assert!(outcome.routed);
        assert!(outcome.repaint_requested);
        assert!(outcome.paint_only_requested);
        assert!(outcome.deferred_surface_refresh_requested);
        assert!(outcome.interactive_surface_refresh_requested);
        assert!(outcome.interactive_scene_rebuild_requested);
        assert!(outcome.exit_requested);
        assert!(outcome.runtime_work_remaining);
    }
}
