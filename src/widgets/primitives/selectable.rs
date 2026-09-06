//! Reusable selectable surface primitive.

use crate::gui::types::{Rect, Rgba8, Vector2};
use crate::layout::LayoutOutput;
use crate::runtime::{PaintPrimitive, PaintText, ResolvedEnvironment, text_font_size_for_height};
use crate::theme::ThemeTokens;

use super::{ColorMarkerProps, support::WidgetCommon};
use crate::widgets::contract::{
    FocusBehavior, Widget, WidgetCapabilities, WidgetId, WidgetPaintContext, WidgetPointerMotion,
    WidgetPointerMotionRevision, WidgetSemantics, WidgetSizing,
};
use crate::widgets::interaction::{SelectableMessage, WidgetInput, WidgetOutput};

mod builders;
mod input;
mod model;
mod paint;

pub use model::SelectableProps;

/// Public selectable primitive for cards, rows, tiles, and options.
#[derive(Clone, Debug, PartialEq)]
pub struct SelectableWidget {
    /// Shared widget contract.
    pub common: WidgetCommon,
    /// Immutable user-facing selectable configuration.
    pub props: SelectableProps,
}

/// Named construction fields for [`SelectableWidget`].
#[derive(Clone, Debug, PartialEq)]
pub struct SelectableWidgetParts {
    /// Stable widget identity used by layout, events, and state synchronization.
    pub id: WidgetId,
    /// User-facing selectable label.
    pub label: PaintText,
    /// Initial selected state.
    pub selected: bool,
    /// Intrinsic selectable sizing contract.
    pub sizing: WidgetSizing,
}

impl SelectableWidget {
    pub(super) fn declared_text_metrics(&self) -> crate::widgets::DeclaredTextMetrics {
        crate::widgets::DeclaredTextMetrics::new(
            self.common.sizing,
            text_font_size_for_height(self.common.sizing.preferred.y),
            Vector2::new(8.0, 3.0),
        )
    }

    /// Build a selectable descriptor from named identity, content, state, and sizing fields.
    pub fn from_parts(parts: SelectableWidgetParts) -> Self {
        let mut common = WidgetCommon::new(parts.id, parts.sizing);
        common.focus = FocusBehavior::Keyboard;
        common.state.selected = parts.selected;
        Self {
            common,
            props: SelectableProps {
                label: parts.label,
                color_marker: None,
            },
        }
    }

    /// Build a selectable descriptor with the provided selected state.
    pub fn new(
        id: WidgetId,
        label: impl Into<PaintText>,
        selected: bool,
        sizing: WidgetSizing,
    ) -> Self {
        Self::from_parts(SelectableWidgetParts {
            id,
            label: label.into(),
            selected,
            sizing,
        })
    }

    /// Route one backend-neutral interaction into the selectable.
    pub fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<SelectableMessage> {
        input::handle_selectable_input(self, bounds, input)
    }

    /// Paint an optional passive color marker inside this selectable.
    pub fn with_color_marker(mut self, color: Option<Rgba8>) -> Self {
        self.props.color_marker = Some(ColorMarkerProps::new(color));
        self
    }

    /// Paint an optional passive color marker with explicit marker geometry.
    pub fn with_color_marker_props(mut self, props: ColorMarkerProps) -> Self {
        self.props.color_marker = Some(props);
        self
    }
}

impl WidgetSemantics for SelectableWidget {
    fn automation_role(&self) -> crate::gui::automation::AutomationRole {
        crate::gui::automation::AutomationRole::Selectable
    }

    fn automation_label(&self) -> Option<String> {
        Some(self.props.label.as_str().to_owned())
    }
}

impl WidgetPointerMotion for SelectableWidget {
    fn revision(&self) -> WidgetPointerMotionRevision {
        WidgetPointerMotionRevision::exact(false)
    }

    fn accepts_pointer_move(&self) -> bool {
        false
    }
}

impl crate::widgets::WidgetSemanticActions for SelectableWidget {
    fn revision(&self) -> crate::widgets::WidgetSemanticActionRevision {
        crate::widgets::WidgetSemanticActionRevision::exact(())
    }

    fn supports(&self, action: &crate::widgets::SemanticAction) -> bool {
        matches!(action, crate::widgets::SemanticAction::Select)
    }

    fn dispatch(
        &mut self,
        action: crate::widgets::SemanticAction,
        _source: crate::widgets::SemanticActionSource,
    ) -> crate::widgets::WidgetSemanticActionResult {
        if !self.supports(&action) || self.common.state.disabled || self.common.state.read_only {
            return crate::widgets::WidgetSemanticActionResult::Unsupported;
        }
        if self.common.state.selected {
            return crate::widgets::WidgetSemanticActionResult::Accepted(None);
        }
        self.common.state.selected = true;
        crate::widgets::WidgetSemanticActionResult::Accepted(Some(WidgetOutput::typed(
            SelectableMessage::SelectionChanged { selected: true },
        )))
    }
}

impl Widget for SelectableWidget {
    fn focused_key_disposition(
        &self,
        key: crate::widgets::WidgetKey,
    ) -> crate::widgets::FocusedKeyDisposition {
        if matches!(
            key,
            crate::widgets::WidgetKey::Enter | crate::widgets::WidgetKey::Space
        ) {
            crate::widgets::FocusedKeyDisposition::Consumed
        } else {
            crate::widgets::FocusedKeyDisposition::Unhandled
        }
    }

    fn text_scale_participation(&self) -> crate::widgets::TextScaleParticipation {
        crate::widgets::TextScaleParticipation::Scaled
    }

    fn layout_node_with_environment(
        &self,
        environment: &ResolvedEnvironment,
    ) -> crate::layout::LayoutNode {
        crate::layout::LayoutNode::Widget(
            self.declared_text_metrics()
                .resolve(environment, self.text_scale_participation())
                .layout_node(self.common.id),
        )
    }

    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        SelectableWidget::handle_input(self, bounds, input).map(WidgetOutput::typed)
    }

    fn capabilities(&self) -> WidgetCapabilities<'_> {
        WidgetCapabilities::new().semantics(self)
    }

    fn action_capabilities(&mut self) -> crate::widgets::WidgetActionCapabilities<'_> {
        crate::widgets::WidgetActionCapabilities::none().with_semantic_actions(self)
    }

    fn capabilities_v2(&self) -> crate::widgets::WidgetCapabilitiesV2<'_> {
        crate::widgets::WidgetCapabilitiesV2::new()
            .with_pointer_motion(self)
            .with_semantic_actions(self)
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        paint::push_selectable_widget_paint(primitives, self, bounds, theme);
    }

    fn append_paint_with_context(&self, context: &mut WidgetPaintContext<'_>) {
        paint::push_selectable_widget_paint_with_context(context, self);
    }
}
