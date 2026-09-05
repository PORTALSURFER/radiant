//! Environment-aware text metrics shared by layout and paint.

use crate::{gui::types::Vector2, layout::WidgetNode, runtime::ResolvedEnvironment};

use super::{WidgetId, WidgetSizing};

/// Whether a widget's declared text metrics participate in application text scaling.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TextScaleParticipation {
    /// Preserve the legacy unscaled contract for custom widgets.
    #[default]
    Unscaled,
    /// Resolve declared metrics against the current application text scale.
    Scaled,
}

/// Metrics declared by a widget before environment projection.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DeclaredTextMetrics {
    /// Intrinsic sizing declaration.
    pub sizing: WidgetSizing,
    /// Declared logical font size in pixels.
    pub font_size: f32,
    /// Declared horizontal and vertical insets.
    pub insets: Vector2,
}

impl DeclaredTextMetrics {
    /// Construct declared text metrics.
    pub const fn new(sizing: WidgetSizing, font_size: f32, insets: Vector2) -> Self {
        Self {
            sizing,
            font_size,
            insets,
        }
    }

    /// Project the declaration once against the application text scale.
    pub fn resolve(
        self,
        environment: &ResolvedEnvironment,
        participation: TextScaleParticipation,
    ) -> ResolvedTextMetrics {
        let factor = match participation {
            TextScaleParticipation::Unscaled => 1.0,
            TextScaleParticipation::Scaled => environment.text_scale().factor(),
        };
        ResolvedTextMetrics {
            sizing: WidgetSizing::from_parts(crate::widgets::WidgetSizingParts {
                min: Vector2::new(self.sizing.min.x * factor, self.sizing.min.y * factor),
                preferred: Vector2::new(
                    self.sizing.preferred.x * factor,
                    self.sizing.preferred.y * factor,
                ),
                baseline: self.sizing.baseline.map(|baseline| baseline * factor),
            }),
            font_size: self.font_size * factor,
            insets: Vector2::new(self.insets.x * factor, self.insets.y * factor),
        }
    }
}

/// Text metrics resolved for one immutable environment projection.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResolvedTextMetrics {
    /// Environment-resolved intrinsic sizing.
    pub sizing: WidgetSizing,
    /// Environment-resolved logical font size.
    pub font_size: f32,
    /// Environment-resolved insets.
    pub insets: Vector2,
}

impl ResolvedTextMetrics {
    /// Project resolved sizing into the layout leaf representation.
    pub fn layout_node(self, id: WidgetId) -> WidgetNode {
        WidgetNode::new(id, self.sizing.preferred)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        application::{ApplicationEnvironment, LocaleId, TextScale},
        gui::types::{Point, Rect},
        layout::{LayoutOutput, Vector2},
        runtime::{PaintPrimitive, PaintText, PaintTextAlign, PaintTextRun},
        theme::ThemeTokens,
        widgets::{Widget, WidgetCommon, WidgetPaintContext},
    };
    use std::sync::Arc;

    fn environment(scale: f32) -> ResolvedEnvironment {
        ResolvedEnvironment::from_snapshots(
            crate::runtime::WindowEnvironment::default(),
            Arc::new(
                ApplicationEnvironment::new(LocaleId::english())
                    .with_text_scale(TextScale::new(scale).expect("valid scale")),
            ),
        )
    }

    #[test]
    fn scaled_metrics_project_all_declared_dimensions_once() {
        let declared = DeclaredTextMetrics::new(
            WidgetSizing::new(Vector2::new(40.0, 12.0), Vector2::new(80.0, 24.0))
                .with_baseline(16.0),
            13.0,
            Vector2::new(4.0, 2.0),
        );
        let resolved = declared.resolve(&environment(2.0), TextScaleParticipation::Scaled);
        assert_eq!(resolved.sizing.min, Vector2::new(80.0, 24.0));
        assert_eq!(resolved.sizing.preferred, Vector2::new(160.0, 48.0));
        assert_eq!(resolved.sizing.baseline, Some(32.0));
        assert_eq!(resolved.font_size, 26.0);
        assert_eq!(resolved.insets, Vector2::new(8.0, 4.0));
    }

    #[test]
    fn scaled_metrics_cover_supported_scale_table_without_dpi_involvement() {
        let declared = DeclaredTextMetrics::new(
            WidgetSizing::new(Vector2::new(40.0, 12.0), Vector2::new(80.0, 24.0))
                .with_baseline(16.0),
            13.0,
            Vector2::new(4.0, 2.0),
        );
        for (scale, expected) in [
            (1.0, (40.0, 80.0, 13.0, 16.0, 4.0, 2.0)),
            (1.5, (60.0, 120.0, 19.5, 24.0, 6.0, 3.0)),
            (2.0, (80.0, 160.0, 26.0, 32.0, 8.0, 4.0)),
        ] {
            let resolved = declared.resolve(&environment(scale), TextScaleParticipation::Scaled);
            assert_eq!(resolved.sizing.min.x, expected.0);
            assert_eq!(resolved.sizing.preferred.x, expected.1);
            assert_eq!(resolved.font_size, expected.2);
            assert_eq!(resolved.sizing.baseline, Some(expected.3));
            assert_eq!(resolved.insets, Vector2::new(expected.4, expected.5));
        }
    }

    #[test]
    fn unscaled_metrics_ignore_application_text_scale() {
        let declared = DeclaredTextMetrics::new(
            WidgetSizing::fixed(Vector2::new(80.0, 24.0)),
            13.0,
            Vector2::new(4.0, 2.0),
        );
        let resolved = declared.resolve(&environment(2.0), TextScaleParticipation::Unscaled);
        assert_eq!(resolved.sizing.preferred, Vector2::new(80.0, 24.0));
        assert_eq!(resolved.font_size, 13.0);
        assert_eq!(resolved.insets, Vector2::new(4.0, 2.0));
    }

    #[derive(Clone)]
    struct CustomTextProbe {
        common: WidgetCommon,
        participation: TextScaleParticipation,
    }

    impl Widget for CustomTextProbe {
        fn common(&self) -> &WidgetCommon {
            &self.common
        }

        fn common_mut(&mut self) -> &mut WidgetCommon {
            &mut self.common
        }

        fn text_scale_participation(&self) -> TextScaleParticipation {
            self.participation
        }

        fn handle_input(
            &mut self,
            _bounds: Rect,
            _input: crate::widgets::WidgetInput,
        ) -> Option<crate::widgets::WidgetOutput> {
            None
        }

        fn append_paint(
            &self,
            primitives: &mut Vec<PaintPrimitive>,
            bounds: Rect,
            _layout: &LayoutOutput,
            _theme: &ThemeTokens,
        ) {
            primitives.push(PaintPrimitive::Text(PaintTextRun {
                widget_id: self.common.id,
                text: PaintText::from_static("probe"),
                rect: bounds,
                baseline: None,
                color: crate::gui::types::Rgba8::default(),
                align: PaintTextAlign::Left,
                wrap: crate::widgets::TextWrap::None,
                font_size: 13.0,
            }));
        }

        fn append_paint_with_context(&self, context: &mut WidgetPaintContext<'_>) {
            if self.participation == TextScaleParticipation::Unscaled {
                let bounds = context.bounds();
                let layout = context.layout();
                let theme = context.theme();
                let primitives = context.primitives();
                self.append_paint(primitives, bounds, layout, theme);
                return;
            }
            let bounds = context.bounds();
            let environment = context.environment().clone();
            let metrics =
                DeclaredTextMetrics::new(self.common.sizing, 13.0, Vector2::new(0.0, 0.0))
                    .resolve(&environment, self.participation);
            context
                .primitives()
                .push(PaintPrimitive::Text(PaintTextRun {
                    widget_id: self.common.id,
                    text: PaintText::from_static("probe"),
                    rect: bounds,
                    baseline: None,
                    color: crate::gui::types::Rgba8::default(),
                    align: PaintTextAlign::Left,
                    wrap: crate::widgets::TextWrap::None,
                    font_size: metrics.font_size,
                }));
        }
    }

    #[test]
    fn custom_widget_legacy_paint_stays_unscaled_and_opt_in_uses_same_resolver() {
        let environment = environment(2.0);
        let make = |participation| CustomTextProbe {
            common: WidgetCommon::fixed(9, 80.0, 24.0),
            participation,
        };
        let legacy = make(TextScaleParticipation::Unscaled);
        let opted = make(TextScaleParticipation::Scaled);
        let crate::layout::LayoutNode::Widget(legacy_node) =
            legacy.layout_node_with_environment(&environment)
        else {
            panic!("legacy probe should project one widget leaf");
        };
        let crate::layout::LayoutNode::Widget(opted_node) =
            opted.layout_node_with_environment(&environment)
        else {
            panic!("opted probe should project one widget leaf");
        };
        assert_eq!(legacy_node.intrinsic, Vector2::new(80.0, 24.0));
        assert_eq!(opted_node.intrinsic, Vector2::new(160.0, 48.0));

        let layout = LayoutOutput::default();
        let theme = ThemeTokens::default();
        let bounds = Rect::from_min_size(Point::default(), Vector2::new(80.0, 24.0));
        let mut legacy_primitives = Vec::new();
        let mut legacy_context = WidgetPaintContext::new(
            &mut legacy_primitives,
            bounds,
            &layout,
            &theme,
            &environment,
        );
        legacy.append_paint_with_context(&mut legacy_context);
        let mut opted_primitives = Vec::new();
        let mut opted_context =
            WidgetPaintContext::new(&mut opted_primitives, bounds, &layout, &theme, &environment);
        opted.append_paint_with_context(&mut opted_context);
        let PaintPrimitive::Text(legacy_text) = &legacy_primitives[0] else {
            panic!("legacy probe should emit text");
        };
        let PaintPrimitive::Text(opted_text) = &opted_primitives[0] else {
            panic!("opted probe should emit text");
        };
        assert_eq!(legacy_text.font_size, 13.0);
        assert_eq!(opted_text.font_size, 26.0);
    }
}
