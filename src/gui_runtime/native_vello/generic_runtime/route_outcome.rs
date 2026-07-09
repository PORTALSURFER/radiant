/// Routing result consumed by redraw, scene refresh, and runtime wakeup policy.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(in crate::gui_runtime::native_vello) struct GenericRouteOutcome {
    pub(in crate::gui_runtime::native_vello) routed: bool,
    pub(in crate::gui_runtime::native_vello) frame_work: FrameWork,
    pub(in crate::gui_runtime::native_vello) exit_requested: bool,
    pub(in crate::gui_runtime::native_vello) runtime_work_remaining: bool,
    pub(in crate::gui_runtime::native_vello) dpi_scale_override: Option<crate::theme::DpiScale>,
    pub(in crate::gui_runtime::native_vello) window_logical_size: Option<crate::layout::Vector2>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::gui_runtime::native_vello) enum FrameWork {
    #[default]
    None,
    PaintOnly {
        reason: FrameWorkReason,
    },
    RefreshSurface {
        reason: FrameWorkReason,
    },
    ResizeSurface {
        reason: FrameWorkReason,
    },
    RebuildScene {
        reason: FrameWorkReason,
        mode: SceneRebuildMode,
    },
    ResizeAndRebuild {
        reason: FrameWorkReason,
    },
    Exit {
        reason: FrameWorkReason,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::gui_runtime::native_vello) enum SceneRebuildMode {
    Immediate,
    ImmediateWithSurfaceRefresh,
    Interactive,
    InteractiveWithSurfaceRefresh,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::gui_runtime::native_vello) enum FrameWorkReason {
    None,
    RoutedInput,
    PointerHover,
    DeferredSurfaceRefresh,
    InteractiveSurfaceRefresh,
    RuntimeSurfaceRefresh,
    RuntimeSurfaceRepaint,
    RuntimePaintOnly,
    TimedPaintOnlyAnimation,
    TextCaretAnimation,
    NativePointerClear,
    NativeResize,
    NativeDpiScale,
    NativeFocusRegained,
    ExternalDragPreview,
    CommandResize,
    Exit,
}

impl GenericRouteOutcome {
    pub(in crate::gui_runtime::native_vello) fn frame_work(self) -> FrameWork {
        self.frame_work
    }

    pub(in crate::gui_runtime::native_vello) fn frame_work_kind(self) -> &'static str {
        self.frame_work.kind()
    }

    pub(in crate::gui_runtime::native_vello) fn frame_work_reason(self) -> &'static str {
        self.frame_work.reason().name()
    }

    pub(in crate::gui_runtime::native_vello) fn needs_redraw(self) -> bool {
        self.frame_work.needs_redraw()
    }

    pub(in crate::gui_runtime::native_vello) fn needs_scene_rebuild(self) -> bool {
        self.frame_work.needs_scene_rebuild()
    }

    pub(in crate::gui_runtime::native_vello) fn is_paint_only(self) -> bool {
        matches!(self.frame_work, FrameWork::PaintOnly { .. })
    }

    pub(in crate::gui_runtime::native_vello) fn is_deferred_surface_refresh(self) -> bool {
        matches!(self.frame_work, FrameWork::RefreshSurface { .. })
    }

    pub(in crate::gui_runtime::native_vello) fn interactive_scene_rebuild_mode(
        self,
    ) -> Option<SceneRebuildMode> {
        match self.frame_work {
            FrameWork::RebuildScene {
                mode:
                    mode @ (SceneRebuildMode::Interactive
                    | SceneRebuildMode::InteractiveWithSurfaceRefresh),
                ..
            } => Some(mode),
            _ => None,
        }
    }

    pub(in crate::gui_runtime::native_vello) fn is_interactive_scene_rebuild(self) -> bool {
        self.interactive_scene_rebuild_mode().is_some()
    }

    pub(in crate::gui_runtime::native_vello) fn is_interactive_surface_refresh(self) -> bool {
        matches!(
            self.frame_work,
            FrameWork::RebuildScene {
                mode: SceneRebuildMode::InteractiveWithSurfaceRefresh,
                ..
            }
        )
    }

    pub(in crate::gui_runtime::native_vello) fn request_frame_work(&mut self, work: FrameWork) {
        self.frame_work = self.frame_work.merge(work);
        if matches!(work, FrameWork::Exit { .. }) {
            self.exit_requested = true;
        }
    }

    pub(in crate::gui_runtime::native_vello) fn request_paint_only(
        &mut self,
        reason: FrameWorkReason,
    ) {
        self.request_frame_work(FrameWork::PaintOnly { reason });
    }

    pub(in crate::gui_runtime::native_vello) fn request_surface_refresh(
        &mut self,
        reason: FrameWorkReason,
    ) {
        self.request_frame_work(FrameWork::RefreshSurface { reason });
    }

    pub(in crate::gui_runtime::native_vello) fn request_scene_rebuild(
        &mut self,
        reason: FrameWorkReason,
    ) {
        self.request_frame_work(FrameWork::RebuildScene {
            reason,
            mode: SceneRebuildMode::Immediate,
        });
    }

    #[allow(dead_code)]
    pub(in crate::gui_runtime::native_vello) fn request_interactive_scene_rebuild(
        &mut self,
        reason: FrameWorkReason,
    ) {
        self.request_frame_work(FrameWork::RebuildScene {
            reason,
            mode: SceneRebuildMode::Interactive,
        });
    }

    pub(in crate::gui_runtime::native_vello) fn request_interactive_surface_refresh(
        &mut self,
        reason: FrameWorkReason,
    ) {
        self.request_frame_work(FrameWork::RebuildScene {
            reason,
            mode: SceneRebuildMode::InteractiveWithSurfaceRefresh,
        });
    }

    pub(in crate::gui_runtime::native_vello) fn request_resize_and_rebuild(
        &mut self,
        reason: FrameWorkReason,
    ) {
        self.request_frame_work(FrameWork::ResizeAndRebuild { reason });
    }

    pub(in crate::gui_runtime::native_vello) fn request_exit(&mut self) {
        self.request_frame_work(FrameWork::Exit {
            reason: FrameWorkReason::Exit,
        });
    }

    pub(in crate::gui_runtime::native_vello) fn merge(&mut self, other: Self) {
        self.routed |= other.routed;
        self.frame_work = self.frame_work.merge(other.frame_work);
        self.exit_requested |= other.exit_requested;
        if self.exit_requested {
            self.frame_work = self.frame_work.merge(FrameWork::Exit {
                reason: FrameWorkReason::Exit,
            });
        }
        self.runtime_work_remaining |= other.runtime_work_remaining;
        self.dpi_scale_override = other.dpi_scale_override.or(self.dpi_scale_override);
        self.window_logical_size = other.window_logical_size.or(self.window_logical_size);
    }
}

impl FrameWork {
    pub(in crate::gui_runtime::native_vello) fn needs_redraw(self) -> bool {
        matches!(
            self,
            Self::PaintOnly { .. }
                | Self::RefreshSurface { .. }
                | Self::ResizeSurface { .. }
                | Self::RebuildScene { .. }
                | Self::ResizeAndRebuild { .. }
        )
    }

    pub(in crate::gui_runtime::native_vello) fn needs_scene_rebuild(self) -> bool {
        matches!(
            self,
            Self::RebuildScene { .. } | Self::ResizeAndRebuild { .. }
        )
    }

    pub(in crate::gui_runtime::native_vello) fn is_paint_only(self) -> bool {
        matches!(self, Self::PaintOnly { .. })
    }

    pub(in crate::gui_runtime::native_vello) fn merge(self, other: Self) -> Self {
        match (self, other) {
            (Self::Exit { .. }, _) => self,
            (_, Self::Exit { .. }) => other,
            (Self::ResizeAndRebuild { .. }, _) => self,
            (_, Self::ResizeAndRebuild { .. }) => other,
            (Self::ResizeSurface { reason }, Self::RebuildScene { .. })
            | (Self::RebuildScene { .. }, Self::ResizeSurface { reason }) => {
                Self::ResizeAndRebuild { reason }
            }
            (Self::ResizeSurface { reason }, Self::RefreshSurface { .. })
            | (Self::RefreshSurface { .. }, Self::ResizeSurface { reason }) => {
                Self::ResizeAndRebuild { reason }
            }
            (Self::ResizeSurface { .. }, Self::ResizeSurface { .. }) => other,
            (Self::ResizeSurface { .. }, _) => self,
            (_, Self::ResizeSurface { .. }) => other,
            (
                Self::RebuildScene {
                    reason: _,
                    mode: left_mode,
                },
                Self::RebuildScene {
                    reason: right_reason,
                    mode: right_mode,
                },
            ) => Self::RebuildScene {
                reason: right_reason,
                mode: left_mode.merge(right_mode),
            },
            (Self::RebuildScene { reason, mode }, Self::RefreshSurface { .. }) => {
                Self::RebuildScene {
                    reason,
                    mode: mode.with_surface_refresh(),
                }
            }
            (Self::RefreshSurface { .. }, Self::RebuildScene { reason, mode }) => {
                Self::RebuildScene {
                    reason,
                    mode: mode.with_surface_refresh(),
                }
            }
            (Self::RebuildScene { .. }, _) => self,
            (_, Self::RebuildScene { .. }) => other,
            (Self::RefreshSurface { .. }, _) => self,
            (_, Self::RefreshSurface { .. }) => other,
            (Self::PaintOnly { .. }, Self::None) => self,
            (Self::None, other) => other,
            (Self::PaintOnly { .. }, Self::PaintOnly { .. }) => other,
        }
    }

    pub(in crate::gui_runtime::native_vello) fn reason(self) -> FrameWorkReason {
        match self {
            Self::None => FrameWorkReason::None,
            Self::PaintOnly { reason }
            | Self::RefreshSurface { reason }
            | Self::ResizeSurface { reason }
            | Self::RebuildScene { reason, .. }
            | Self::ResizeAndRebuild { reason }
            | Self::Exit { reason } => reason,
        }
    }

    pub(in crate::gui_runtime::native_vello) fn kind(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::PaintOnly { .. } => "paint_only",
            Self::RefreshSurface { .. } => "refresh_surface",
            Self::ResizeSurface { .. } => "resize_surface",
            Self::RebuildScene {
                mode: SceneRebuildMode::Immediate,
                ..
            } => "rebuild_scene",
            Self::RebuildScene {
                mode: SceneRebuildMode::ImmediateWithSurfaceRefresh,
                ..
            } => "refresh_surface_and_rebuild_scene",
            Self::RebuildScene {
                mode: SceneRebuildMode::Interactive,
                ..
            } => "interactive_rebuild_scene",
            Self::RebuildScene {
                mode: SceneRebuildMode::InteractiveWithSurfaceRefresh,
                ..
            } => "interactive_refresh_surface",
            Self::ResizeAndRebuild { .. } => "resize_and_rebuild",
            Self::Exit { .. } => "exit",
        }
    }
}

impl SceneRebuildMode {
    fn merge(self, other: Self) -> Self {
        match (self, other) {
            (Self::InteractiveWithSurfaceRefresh, _) | (_, Self::InteractiveWithSurfaceRefresh) => {
                Self::InteractiveWithSurfaceRefresh
            }
            (Self::ImmediateWithSurfaceRefresh, Self::Interactive)
            | (Self::Interactive, Self::ImmediateWithSurfaceRefresh) => {
                Self::InteractiveWithSurfaceRefresh
            }
            (Self::ImmediateWithSurfaceRefresh, _) | (_, Self::ImmediateWithSurfaceRefresh) => {
                Self::ImmediateWithSurfaceRefresh
            }
            (Self::Interactive, _) | (_, Self::Interactive) => Self::Interactive,
            (Self::Immediate, Self::Immediate) => Self::Immediate,
        }
    }

    fn with_surface_refresh(self) -> Self {
        match self {
            Self::Immediate | Self::ImmediateWithSurfaceRefresh => {
                Self::ImmediateWithSurfaceRefresh
            }
            Self::Interactive | Self::InteractiveWithSurfaceRefresh => {
                Self::InteractiveWithSurfaceRefresh
            }
        }
    }
}

impl FrameWorkReason {
    pub(in crate::gui_runtime::native_vello) fn name(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::RoutedInput => "routed_input",
            Self::PointerHover => "pointer_hover",
            Self::DeferredSurfaceRefresh => "deferred_surface_refresh",
            Self::InteractiveSurfaceRefresh => "interactive_surface_refresh",
            Self::RuntimeSurfaceRefresh => "runtime_surface_refresh",
            Self::RuntimeSurfaceRepaint => "runtime_surface_repaint",
            Self::RuntimePaintOnly => "runtime_paint_only",
            Self::TimedPaintOnlyAnimation => "timed_paint_only_animation",
            Self::TextCaretAnimation => "text_caret_animation",
            Self::NativePointerClear => "native_pointer_clear",
            Self::NativeResize => "native_resize",
            Self::NativeDpiScale => "native_dpi_scale",
            Self::NativeFocusRegained => "native_focus_regained",
            Self::ExternalDragPreview => "external_drag_preview",
            Self::CommandResize => "command_resize",
            Self::Exit => "exit",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{FrameWork, FrameWorkReason, GenericRouteOutcome, SceneRebuildMode};

    #[test]
    fn route_outcome_frame_work_distinguishes_paint_only_from_scene_rebuild() {
        let mut paint_only = GenericRouteOutcome::default();
        paint_only.request_paint_only(FrameWorkReason::RuntimePaintOnly);
        let mut deferred = GenericRouteOutcome::default();
        deferred.request_surface_refresh(FrameWorkReason::DeferredSurfaceRefresh);
        let mut scene = GenericRouteOutcome::default();
        scene.request_scene_rebuild(FrameWorkReason::RuntimeSurfaceRepaint);

        assert!(paint_only.needs_redraw());
        assert!(!paint_only.needs_scene_rebuild());
        assert_eq!(
            paint_only.frame_work(),
            FrameWork::PaintOnly {
                reason: FrameWorkReason::RuntimePaintOnly
            }
        );
        assert!(deferred.needs_redraw());
        assert!(!deferred.needs_scene_rebuild());
        assert_eq!(
            deferred.frame_work(),
            FrameWork::RefreshSurface {
                reason: FrameWorkReason::DeferredSurfaceRefresh
            }
        );
        assert!(scene.needs_redraw());
        assert!(scene.needs_scene_rebuild());
    }

    #[test]
    fn route_outcome_merge_escalates_to_single_strongest_frame_work() {
        let mut outcome = GenericRouteOutcome {
            routed: true,
            ..GenericRouteOutcome::default()
        };
        outcome.request_scene_rebuild(FrameWorkReason::RuntimeSurfaceRepaint);

        let mut other = GenericRouteOutcome::default();
        other.request_paint_only(FrameWorkReason::RuntimePaintOnly);
        other.request_surface_refresh(FrameWorkReason::DeferredSurfaceRefresh);
        other.request_interactive_surface_refresh(FrameWorkReason::InteractiveSurfaceRefresh);
        other.runtime_work_remaining = true;
        outcome.merge(other);

        assert!(outcome.routed);
        assert_eq!(
            outcome.frame_work(),
            FrameWork::RebuildScene {
                reason: FrameWorkReason::InteractiveSurfaceRefresh,
                mode: SceneRebuildMode::InteractiveWithSurfaceRefresh,
            }
        );
        assert!(outcome.runtime_work_remaining);
    }

    #[test]
    fn route_outcome_merge_preserves_surface_refresh_when_rebuilds_scene() {
        let mut refresh_then_rebuild = GenericRouteOutcome::default();
        refresh_then_rebuild.request_surface_refresh(FrameWorkReason::DeferredSurfaceRefresh);
        refresh_then_rebuild.request_scene_rebuild(FrameWorkReason::RuntimeSurfaceRepaint);

        assert_eq!(
            refresh_then_rebuild.frame_work(),
            FrameWork::RebuildScene {
                reason: FrameWorkReason::RuntimeSurfaceRepaint,
                mode: SceneRebuildMode::ImmediateWithSurfaceRefresh,
            }
        );

        let mut rebuild_then_refresh = GenericRouteOutcome::default();
        rebuild_then_refresh.request_scene_rebuild(FrameWorkReason::RuntimeSurfaceRepaint);
        rebuild_then_refresh.request_surface_refresh(FrameWorkReason::DeferredSurfaceRefresh);

        assert_eq!(
            rebuild_then_refresh.frame_work(),
            FrameWork::RebuildScene {
                reason: FrameWorkReason::RuntimeSurfaceRepaint,
                mode: SceneRebuildMode::ImmediateWithSurfaceRefresh,
            }
        );
    }

    #[test]
    fn resize_surface_work_upgrades_after_refresh_or_scene_rebuild() {
        let resize = FrameWork::ResizeSurface {
            reason: FrameWorkReason::CommandResize,
        };

        assert!(resize.needs_redraw());
        assert!(!resize.needs_scene_rebuild());
        assert_eq!(resize.kind(), "resize_surface");
        assert_eq!(
            resize.merge(FrameWork::PaintOnly {
                reason: FrameWorkReason::RuntimePaintOnly,
            }),
            resize
        );
        assert_eq!(
            resize.merge(FrameWork::RebuildScene {
                reason: FrameWorkReason::RuntimeSurfaceRepaint,
                mode: SceneRebuildMode::Immediate,
            }),
            FrameWork::ResizeAndRebuild {
                reason: FrameWorkReason::CommandResize,
            }
        );
        assert_eq!(
            resize.merge(FrameWork::RefreshSurface {
                reason: FrameWorkReason::DeferredSurfaceRefresh,
            }),
            FrameWork::ResizeAndRebuild {
                reason: FrameWorkReason::CommandResize,
            }
        );
        assert_eq!(
            FrameWork::RefreshSurface {
                reason: FrameWorkReason::DeferredSurfaceRefresh,
            }
            .merge(resize),
            FrameWork::ResizeAndRebuild {
                reason: FrameWorkReason::CommandResize,
            }
        );
    }

    #[test]
    fn route_outcome_covers_every_frame_work_variant() {
        assert_eq!(FrameWork::None.kind(), "none");
        assert_eq!(FrameWork::None.reason().name(), "none");

        let no_work = GenericRouteOutcome::default();
        assert_eq!(no_work.frame_work_kind(), "none");
        assert_eq!(no_work.frame_work_reason(), "none");

        let resize_surface = FrameWork::ResizeSurface {
            reason: FrameWorkReason::NativeResize,
        };
        assert_eq!(resize_surface.kind(), "resize_surface");
        assert_eq!(resize_surface.reason().name(), "native_resize");

        let mut interactive = GenericRouteOutcome::default();
        interactive.request_interactive_scene_rebuild(FrameWorkReason::RoutedInput);
        assert_eq!(
            interactive.frame_work(),
            FrameWork::RebuildScene {
                reason: FrameWorkReason::RoutedInput,
                mode: SceneRebuildMode::Interactive,
            }
        );

        let mut refreshed_rebuild = GenericRouteOutcome::default();
        refreshed_rebuild.request_surface_refresh(FrameWorkReason::DeferredSurfaceRefresh);
        refreshed_rebuild.request_scene_rebuild(FrameWorkReason::RuntimeSurfaceRepaint);
        assert_eq!(
            refreshed_rebuild.frame_work_kind(),
            "refresh_surface_and_rebuild_scene"
        );
        assert_eq!(
            refreshed_rebuild.frame_work(),
            FrameWork::RebuildScene {
                reason: FrameWorkReason::RuntimeSurfaceRepaint,
                mode: SceneRebuildMode::ImmediateWithSurfaceRefresh,
            }
        );

        let mut resize = GenericRouteOutcome::default();
        resize.request_resize_and_rebuild(FrameWorkReason::CommandResize);
        assert!(resize.needs_redraw());
        assert!(resize.needs_scene_rebuild());
        assert_eq!(resize.frame_work_kind(), "resize_and_rebuild");
        assert_eq!(
            resize.frame_work(),
            FrameWork::ResizeAndRebuild {
                reason: FrameWorkReason::CommandResize,
            }
        );
    }

    #[test]
    fn route_outcome_resize_and_exit_escalate_above_render_work() {
        let mut outcome = GenericRouteOutcome::default();
        outcome.request_paint_only(FrameWorkReason::RuntimePaintOnly);
        outcome.request_surface_refresh(FrameWorkReason::DeferredSurfaceRefresh);
        outcome.request_scene_rebuild(FrameWorkReason::RuntimeSurfaceRepaint);
        outcome.request_resize_and_rebuild(FrameWorkReason::CommandResize);

        assert_eq!(
            outcome.frame_work(),
            FrameWork::ResizeAndRebuild {
                reason: FrameWorkReason::CommandResize,
            }
        );

        outcome.request_exit();
        assert_eq!(
            outcome.frame_work(),
            FrameWork::Exit {
                reason: FrameWorkReason::Exit,
            }
        );
    }

    #[test]
    fn route_outcome_exit_dominates_frame_work() {
        let mut outcome = GenericRouteOutcome::default();
        outcome.request_scene_rebuild(FrameWorkReason::RuntimeSurfaceRepaint);

        let mut exit = GenericRouteOutcome::default();
        exit.request_exit();
        outcome.merge(exit);

        assert!(outcome.exit_requested);
        assert_eq!(
            outcome.frame_work(),
            FrameWork::Exit {
                reason: FrameWorkReason::Exit
            }
        );
    }
}
