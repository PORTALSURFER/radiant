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
        layout::{SizeModeCross, SizeModeMain, SlotParams, Vector2},
        prelude::{IntoView, column, text},
        runtime::{SurfaceChild, SurfaceNode, UiSurface, WidgetMessageMapper},
        widgets::{
            BadgeWidget, ButtonWidget, ListItemWidget, SelectableWidget, TextInputWidget,
            TextWidget, ToggleWidget, Widget, WidgetSizing,
        },
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

    #[test]
    fn built_in_text_controls_resolve_declared_metrics_once_across_scale_table() {
        let sizing = WidgetSizing::new(Vector2::new(40.0, 20.0), Vector2::new(80.0, 28.0))
            .with_baseline(20.0);
        for (scale, expected_min, expected_preferred, expected_baseline, expected_font) in [
            (
                1.0,
                Vector2::new(40.0, 20.0),
                Vector2::new(80.0, 28.0),
                20.0,
                13.0,
            ),
            (
                1.5,
                Vector2::new(60.0, 30.0),
                Vector2::new(120.0, 42.0),
                30.0,
                19.5,
            ),
            (
                2.0,
                Vector2::new(80.0, 40.0),
                Vector2::new(160.0, 56.0),
                40.0,
                26.0,
            ),
        ] {
            let environment = ApplicationEnvironment::new(LocaleId::english())
                .with_text_scale(TextScale::new(scale).expect("valid scale"));
            let resolved =
                crate::widgets::DeclaredTextMetrics::new(sizing, 13.0, Vector2::new(8.0, 3.0))
                    .resolve(
                        &crate::runtime::ResolvedEnvironment::from_snapshots(
                            crate::runtime::WindowEnvironment::default(),
                            std::sync::Arc::new(environment.clone()),
                        ),
                        crate::widgets::TextScaleParticipation::Scaled,
                    );
            assert_eq!(resolved.sizing.min, expected_min);
            assert_eq!(resolved.sizing.preferred, expected_preferred);
            assert_eq!(resolved.sizing.baseline, Some(expected_baseline));
            assert_eq!(resolved.font_size, expected_font);
            assert_eq!(resolved.insets, Vector2::new(8.0 * scale, 3.0 * scale));

            macro_rules! assert_control {
                ($widget:expr, $id:expr, $label:expr, $inset_y:expr, $control_font:expr) => {{
                    let control = $widget;
                    let surface: UiSurface<()> =
                        UiSurface::new(SurfaceNode::static_widget(control))
                            .with_application_environment(environment.clone());
                    let crate::layout::LayoutNode::Widget(node) = surface.layout_node() else {
                        panic!("control should project one widget leaf");
                    };
                    assert_eq!(node.intrinsic, expected_preferred);
                    let frame = surface.frame_at_size_with_default_theme(Vector2::new(100.0, 36.0));
                    let bounds = frame.layout.rects.get(&$id).expect("control bounds");
                    assert_eq!(bounds.width(), 100.0);
                    assert_eq!(bounds.height(), 36.0);
                    let text = frame
                        .paint_plan
                        .first_text_run($label)
                        .expect("control text");
                    assert_eq!(text.font_size, $control_font * scale);
                    assert_eq!(text.rect.min.x, 8.0 * scale);
                    assert_eq!(text.rect.min.y, $inset_y * scale);
                }};
            }

            assert_control!(
                ButtonWidget::new(101, "Button", sizing),
                101,
                "Button",
                4.0,
                13.0
            );
            assert_control!(
                BadgeWidget::new(102, "Badge", sizing),
                102,
                "Badge",
                3.0,
                13.0
            );
            assert_control!(
                ToggleWidget::new(103, "Toggle", sizing),
                103,
                "Toggle",
                4.0,
                14.0
            );
            assert_control!(
                SelectableWidget::new(104, "Selectable", false, sizing),
                104,
                "Selectable",
                3.0,
                14.0
            );
            assert_control!(
                ListItemWidget::new(105, "List item", sizing),
                105,
                "List item",
                3.0,
                14.0
            );
        }
    }

    #[test]
    fn scaled_control_keeps_declarative_parent_and_clip_bounds_physical_across_frames() {
        for (scale, expected_font, expected_x, expected_y) in [
            (1.0, 13.0, 8.0, 4.0),
            (1.5, 19.5, 12.0, 6.0),
            (2.0, 26.0, 16.0, 8.0),
        ] {
            let environment = ApplicationEnvironment::new(LocaleId::english())
                .with_text_scale(TextScale::new(scale).expect("valid scale"));
            let surface: UiSurface<()> = UiSurface::new(SurfaceNode::column(
                109,
                0.0,
                vec![SurfaceChild::new(
                    SlotParams {
                        size_main: SizeModeMain::Fixed(24.0),
                        size_cross: SizeModeCross::Fixed(48.0),
                        constraints: crate::layout::Constraints::unconstrained(),
                        margin: Default::default(),
                        align_cross_override: Some(crate::layout::CrossAlign::Start),
                        allow_fixed_compress: false,
                    },
                    SurfaceNode::column(
                        108,
                        0.0,
                        vec![SurfaceChild::new(
                            SlotParams {
                                size_main: SizeModeMain::Fixed(20.0),
                                size_cross: SizeModeCross::Fixed(40.0),
                                constraints: crate::layout::Constraints::unconstrained(),
                                margin: Default::default(),
                                align_cross_override: Some(crate::layout::CrossAlign::Start),
                                allow_fixed_compress: false,
                            },
                            SurfaceNode::widget(
                                ButtonWidget::new(
                                    107,
                                    "Scaled",
                                    WidgetSizing::fixed(Vector2::new(40.0, 20.0)),
                                ),
                                WidgetMessageMapper::none(),
                            ),
                        )],
                    ),
                )],
            ))
            .with_application_environment(environment);

            let frame = surface.frame_at_size_with_default_theme(Vector2::new(160.0, 80.0));
            let parent = frame.layout.rects.get(&108).expect("parent bounds");
            let child = frame.layout.rects.get(&107).expect("button bounds");
            assert_eq!(parent.width(), 48.0);
            assert_eq!(parent.height(), 24.0);
            assert_eq!(child.width(), 40.0);
            assert_eq!(child.height(), 20.0);

            let clip = frame
                .paint_plan
                .primitives
                .iter()
                .find_map(|primitive| match primitive {
                    crate::runtime::PaintPrimitive::ClipStart(clip) if clip.node_id == 107 => {
                        Some(clip.rect)
                    }
                    _ => None,
                })
                .expect("button clip bounds");
            assert_eq!(clip, *child);

            let text = frame
                .paint_plan
                .first_text_run("Scaled")
                .expect("button text");
            assert_eq!(text.font_size, expected_font);
            assert_eq!(text.rect.min.x, expected_x);
            assert_eq!(text.rect.min.y, expected_y);
            assert_eq!(
                frame,
                surface.frame_at_size_with_default_theme(Vector2::new(160.0, 80.0))
            );
        }
    }

    #[test]
    fn button_start_alignment_uses_rtl_environment_in_context_paint() {
        let environment = ApplicationEnvironment::new(LocaleId::english())
            .with_writing_direction(crate::application::WritingDirection::Rtl)
            .with_text_scale(TextScale::new(1.5).expect("valid scale"));
        let mut button =
            ButtonWidget::new(106, "Start", WidgetSizing::fixed(Vector2::new(80.0, 28.0)));
        assert!(Widget::set_text_align(
            &mut button,
            crate::widgets::TextAlign::Start
        ));
        let surface: UiSurface<()> = UiSurface::new(SurfaceNode::static_widget(button))
            .with_application_environment(environment);
        let frame = surface.frame_at_size_with_default_theme(Vector2::new(120.0, 36.0));
        assert_eq!(
            frame
                .paint_plan
                .first_text_run("Start")
                .expect("button text")
                .align,
            crate::runtime::PaintTextAlign::Right
        );
    }
}
