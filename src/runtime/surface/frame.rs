use crate::{
    gui::types::{Point, Rect},
    layout::{LayoutDebugOptions, LayoutOutput, LayoutState, layout_tree_with_direction},
    runtime::SurfacePaintPlan,
    theme::ThemeTokens,
};

use super::UiSurface;

/// One host-controlled rendering frame prepared from a declarative surface.
///
/// `SurfaceFrame` packages the logical viewport, resolved layout, and
/// backend-neutral paint plan that a host renderer needs to draw a projected
/// [`UiSurface`]. It is intended for embedded or custom-host integrations that
/// own the surrounding window, native surface, or render pass.
#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceFrame {
    /// Logical viewport rectangle supplied by the host.
    pub viewport: Rect,
    /// Resolved layout for the projected surface.
    pub layout: LayoutOutput,
    /// Backend-neutral paint plan for the resolved layout.
    pub paint_plan: SurfacePaintPlan,
}

impl<Message> UiSurface<Message> {
    /// Resolve this surface into layout rectangles for a host-controlled viewport.
    ///
    /// This is the layout-only counterpart to [`Self::frame`] for hosts that
    /// project declarative Radiant surfaces into an existing renderer or
    /// compatibility layer and only need geometry.
    pub fn layout(&self, viewport: Rect) -> LayoutOutput {
        layout_tree_with_direction(
            &self.layout_node(),
            viewport,
            self.resolved_environment().writing_direction(),
        )
    }

    /// Resolve this surface into layout rectangles for an origin-based viewport.
    ///
    /// This is a convenience for tests, automation, plugin previews, and
    /// embedded hosts that render a surface into a logical size rather than an
    /// already-positioned viewport rectangle.
    pub fn layout_at_size(&self, size: crate::layout::Vector2) -> LayoutOutput {
        self.layout(Rect::from_min_size(Point::default(), size))
    }

    /// Resolve this surface into layout rectangles with explicit state/options.
    ///
    /// Use this variant when a host needs scroll offsets, virtualization state,
    /// or debug primitives/diagnostics without also building a paint plan.
    pub fn layout_with_options(
        &self,
        viewport: Rect,
        layout_state: &LayoutState,
        debug_options: LayoutDebugOptions,
    ) -> LayoutOutput {
        let mut engine = crate::layout::LayoutEngine::default();
        let mut output = LayoutOutput::default();
        engine.layout_with_state_and_direction_and_source_into(
            &self.layout_node(),
            viewport,
            layout_state,
            debug_options,
            self.resolved_environment().writing_direction(),
            None,
            &mut output,
        );
        output
    }

    /// Prepare one layout plus paint-plan frame for a host-controlled viewport.
    ///
    /// This is the direct embedding path for hosts that already own a platform
    /// surface or render pass and only need Radiant to project widgets into
    /// backend-neutral layout and paint data.
    pub fn frame(&self, viewport: Rect, theme: &ThemeTokens) -> SurfaceFrame {
        let layout = self.layout(viewport);
        let paint_plan = self.paint_plan(&layout, theme);
        SurfaceFrame {
            viewport,
            layout,
            paint_plan,
        }
    }

    /// Prepare one frame for an origin-based viewport.
    ///
    /// Use this when a host or test only cares about rendering into a logical
    /// size and does not need to supply a non-zero viewport origin.
    pub fn frame_at_size(&self, size: crate::layout::Vector2, theme: &ThemeTokens) -> SurfaceFrame {
        self.frame(Rect::from_min_size(Point::default(), size), theme)
    }

    /// Prepare one frame with Radiant's default theme.
    ///
    /// This is intended for smoke tests, automation, examples, and embedded
    /// previews where custom theme tokens are not part of the behavior under
    /// test.
    pub fn frame_with_default_theme(&self, viewport: Rect) -> SurfaceFrame {
        self.frame(viewport, &ThemeTokens::default())
    }

    /// Prepare one origin-based frame with Radiant's default theme.
    ///
    /// This combines [`Self::frame_at_size`] with the default theme for common
    /// GUI smoke tests and examples.
    pub fn frame_at_size_with_default_theme(&self, size: crate::layout::Vector2) -> SurfaceFrame {
        self.frame_at_size(size, &ThemeTokens::default())
    }

    /// Prepare one host-controlled frame with explicit layout state and diagnostics.
    ///
    /// Use this variant when a host needs scroll offsets, virtualization state,
    /// or debug primitives/diagnostics in the returned layout output.
    pub fn frame_with_layout_options(
        &self,
        viewport: Rect,
        theme: &ThemeTokens,
        layout_state: &LayoutState,
        debug_options: LayoutDebugOptions,
    ) -> SurfaceFrame {
        let layout = self.layout_with_options(viewport, layout_state, debug_options);
        let paint_plan = self.paint_plan(&layout, theme);
        SurfaceFrame {
            viewport,
            layout,
            paint_plan,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        application::{ApplicationEnvironment, LocaleId, TextScale},
        layout::Vector2,
        prelude::{IntoView, column, text},
        runtime::{SurfaceNode, UiSurface},
        widgets::{TextInputWidget, TextWidget, WidgetSizing},
    };

    #[test]
    fn frame_at_size_uses_origin_viewport() {
        let theme = crate::theme::ThemeTokens::default();
        let frame = UiSurface::<()>::new(text("Status").into_node())
            .frame_at_size(Vector2::new(120.0, 40.0), &theme);

        assert_eq!(frame.viewport.min, crate::gui::types::Point::default());
        assert_eq!(frame.viewport.width(), 120.0);
        assert_eq!(frame.viewport.height(), 40.0);
        assert!(frame.paint_plan.contains_text("Status"));
    }

    #[test]
    fn frame_at_size_with_default_theme_builds_paint_plan() {
        let frame = UiSurface::<()>::new(text("Ready").into_node())
            .frame_at_size_with_default_theme(Vector2::new(120.0, 40.0));

        assert_eq!(frame.viewport.min, crate::gui::types::Point::default());
        assert!(frame.paint_plan.contains_text("Ready"));
    }

    #[test]
    fn text_scale_resolves_intrinsic_layout_and_paint_metrics_once() {
        let environment = ApplicationEnvironment::new(LocaleId::english())
            .with_text_scale(TextScale::new(2.0).expect("valid scale"));
        let surface: UiSurface<()> = UiSurface::new(SurfaceNode::static_widget(TextWidget::new(
            7,
            "Scaled",
            WidgetSizing::fixed(Vector2::new(80.0, 24.0)),
        )))
        .with_application_environment(environment);
        let crate::layout::LayoutNode::Widget(node) = surface.layout_node() else {
            panic!("text surface should project one widget leaf");
        };
        assert_eq!(node.intrinsic, Vector2::new(160.0, 48.0));
        let frame = surface.frame_at_size_with_default_theme(Vector2::new(300.0, 100.0));
        let rect = frame.layout.rects.get(&7).expect("text layout rect");
        assert_eq!(rect.width(), 300.0);
        assert_eq!(rect.height(), 100.0);
        let text = frame.paint_plan.first_text_run("Scaled").expect("text run");
        assert_eq!(text.font_size, 26.0);
    }

    #[test]
    fn text_scale_keeps_parent_assigned_bounds_physical_and_allows_clipping() {
        let environment = ApplicationEnvironment::new(LocaleId::english())
            .with_text_scale(TextScale::new(2.0).expect("valid scale"));
        let surface: UiSurface<()> = column([text::<()>("Scaled").id(7).width(80.0).height(24.0)])
            .align_cross(crate::layout::CrossAlign::Start)
            .into_surface()
            .with_application_environment(environment);
        let frame = surface.frame_at_size_with_default_theme(Vector2::new(320.0, 120.0));
        let rect = frame.layout.rects.get(&7).expect("text layout rect");
        assert_eq!(rect.width(), 80.0);
        assert_eq!(rect.height(), 24.0);
        let text = frame.paint_plan.first_text_run("Scaled").expect("text run");
        assert_eq!(text.font_size, 26.0);
    }

    #[test]
    fn text_input_uses_the_same_resolved_font_metrics_as_text() {
        let environment = ApplicationEnvironment::new(LocaleId::english())
            .with_text_scale(TextScale::new(2.0).expect("valid scale"));
        let surface: UiSurface<()> = UiSurface::new(SurfaceNode::static_widget(
            TextInputWidget::new(8, "value", WidgetSizing::fixed(Vector2::new(120.0, 28.0))),
        ))
        .with_application_environment(environment);
        let frame = surface.frame_at_size_with_default_theme(Vector2::new(300.0, 100.0));
        let input = frame
            .paint_plan
            .first_text_input()
            .expect("text input paint primitive");
        assert_eq!(input.font_size, 26.0);
        assert_eq!(input.align, crate::runtime::PaintTextAlign::Left);
    }

    #[test]
    fn text_scale_and_direction_are_surface_local_and_repeated_frames_reuse_values() {
        let first: UiSurface<()> = UiSurface::new(SurfaceNode::static_widget(
            TextWidget::new(1, "first", WidgetSizing::fixed(Vector2::new(80.0, 24.0)))
                .with_align(crate::widgets::TextAlign::Start),
        ))
        .with_application_environment(
            ApplicationEnvironment::new(LocaleId::english())
                .with_text_scale(TextScale::new(1.0).expect("valid scale")),
        );
        let second: UiSurface<()> = UiSurface::new(SurfaceNode::static_widget(
            TextWidget::new(2, "second", WidgetSizing::fixed(Vector2::new(80.0, 24.0)))
                .with_align(crate::widgets::TextAlign::Start),
        ))
        .with_application_environment(
            ApplicationEnvironment::new(LocaleId::english())
                .with_writing_direction(crate::application::WritingDirection::Rtl)
                .with_text_scale(TextScale::new(1.5).expect("valid scale")),
        );

        let crate::layout::LayoutNode::Widget(first_node) = first.layout_node() else {
            panic!("first surface should project one widget leaf");
        };
        let crate::layout::LayoutNode::Widget(second_node) = second.layout_node() else {
            panic!("second surface should project one widget leaf");
        };
        assert_eq!(first_node.intrinsic, Vector2::new(80.0, 24.0));
        assert_eq!(second_node.intrinsic, Vector2::new(120.0, 36.0));

        let first_frame = first.frame_at_size_with_default_theme(Vector2::new(200.0, 80.0));
        let second_frame = second.frame_at_size_with_default_theme(Vector2::new(200.0, 80.0));
        let first_run = first_frame
            .paint_plan
            .first_text_run("first")
            .expect("first text run");
        let second_run = second_frame
            .paint_plan
            .first_text_run("second")
            .expect("second text run");
        assert_eq!(first_run.font_size, 13.0);
        assert_eq!(first_run.align, crate::runtime::PaintTextAlign::Left);
        assert_eq!(second_run.font_size, 19.5);
        assert_eq!(second_run.align, crate::runtime::PaintTextAlign::Right);
        assert_eq!(
            second_frame,
            second.frame_at_size_with_default_theme(Vector2::new(200.0, 80.0))
        );
    }

    #[test]
    fn text_input_start_alignment_resolves_against_rtl_environment() {
        let surface: UiSurface<()> = UiSurface::new(SurfaceNode::static_widget(
            TextInputWidget::new(9, "value", WidgetSizing::fixed(Vector2::new(120.0, 28.0))),
        ))
        .with_application_environment(
            ApplicationEnvironment::new(LocaleId::english())
                .with_writing_direction(crate::application::WritingDirection::Rtl)
                .with_text_scale(TextScale::new(2.0).expect("valid scale")),
        );
        let frame = surface.frame_at_size_with_default_theme(Vector2::new(300.0, 100.0));
        let input = frame
            .paint_plan
            .first_text_input()
            .expect("text input paint primitive");
        assert_eq!(input.font_size, 26.0);
        assert_eq!(input.align, crate::runtime::PaintTextAlign::Right);
    }
}
