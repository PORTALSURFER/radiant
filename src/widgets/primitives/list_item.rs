//! Reusable list-row and list-item primitive.

use crate::gui::types::{Rect, Vector2};
use crate::layout::LayoutOutput;
use crate::runtime::{PaintPrimitive, PaintText, ResolvedEnvironment, text_font_size_for_height};
use crate::theme::ThemeTokens;

use super::support::WidgetCommon;
use crate::widgets::contract::{
    FocusBehavior, Widget, WidgetCapabilities, WidgetId, WidgetPaintContext, WidgetPointerMotion,
    WidgetPointerMotionRevision, WidgetSemantics, WidgetSizing,
};
use crate::widgets::interaction::{ListItemMessage, WidgetInput, WidgetOutput};

mod builders;
mod input;
mod paint;

/// Public list-row or list-item primitive.
#[derive(Clone, Debug, PartialEq)]
pub struct ListItemWidget {
    /// Shared widget contract.
    pub common: WidgetCommon,
    /// Primary row label.
    pub label: PaintText,
    /// Optional secondary text.
    pub detail: Option<PaintText>,
}

/// Named construction fields for [`ListItemWidget`].
#[derive(Clone, Debug, PartialEq)]
pub struct ListItemWidgetParts {
    /// Stable widget identity used by layout, events, and state synchronization.
    pub id: WidgetId,
    /// Primary row label.
    pub label: PaintText,
    /// Intrinsic row sizing contract.
    pub sizing: WidgetSizing,
}

impl ListItemWidget {
    pub(super) fn declared_text_metrics(&self) -> crate::widgets::DeclaredTextMetrics {
        crate::widgets::DeclaredTextMetrics::new(
            self.common.sizing,
            text_font_size_for_height(self.common.sizing.preferred.y),
            Vector2::new(8.0, 3.0),
        )
    }

    /// Build a list-item descriptor from named identity, content, and sizing fields.
    pub fn from_parts(parts: ListItemWidgetParts) -> Self {
        let mut common = WidgetCommon::new(parts.id, parts.sizing);
        common.focus = FocusBehavior::Keyboard;
        Self {
            common,
            label: parts.label,
            detail: None,
        }
    }

    /// Build a list-item descriptor that can be focused, selected, and invoked.
    pub fn new(id: WidgetId, label: impl Into<PaintText>, sizing: WidgetSizing) -> Self {
        Self::from_parts(ListItemWidgetParts {
            id,
            label: label.into(),
            sizing,
        })
    }

    /// Route one backend-neutral interaction into the list item.
    pub fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<ListItemMessage> {
        input::handle_list_item_input(self, bounds, input)
    }
}

impl WidgetSemantics for ListItemWidget {
    fn automation_role(&self) -> crate::gui::automation::AutomationRole {
        crate::gui::automation::AutomationRole::Row
    }

    fn automation_label(&self) -> Option<String> {
        Some(self.label.as_str().to_owned())
    }

    fn automation_value_text(&self) -> Option<String> {
        self.detail
            .as_ref()
            .map(|detail| detail.as_str().to_owned())
    }
}

impl WidgetPointerMotion for ListItemWidget {
    fn revision(&self) -> WidgetPointerMotionRevision {
        WidgetPointerMotionRevision::exact(false)
    }

    fn accepts_pointer_move(&self) -> bool {
        false
    }
}

impl crate::widgets::WidgetSemanticActions for ListItemWidget {
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
        crate::widgets::WidgetSemanticActionResult::Accepted(Some(WidgetOutput::typed(
            ListItemMessage::Invoked,
        )))
    }
}

impl Widget for ListItemWidget {
    fn focused_key_disposition(
        &self,
        key: crate::widgets::WidgetKey,
    ) -> crate::widgets::FocusedKeyDisposition {
        if crate::widgets::interaction::is_scroll_fallback_key(key) {
            crate::widgets::FocusedKeyDisposition::Unhandled
        } else {
            crate::widgets::FocusedKeyDisposition::Consumed
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
        ListItemWidget::handle_input(self, bounds, input).map(WidgetOutput::typed)
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
        paint::push_list_item_widget_paint(primitives, self, bounds, theme);
    }

    fn append_paint_with_context(&self, context: &mut WidgetPaintContext<'_>) {
        paint::push_list_item_widget_paint_with_context(context, self);
    }
}
