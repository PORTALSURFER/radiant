//! One semantic and interaction owner for a menu command and its text columns.

use crate::{
    application::{TextContent, WritingDirection},
    gui::{automation::AutomationRole, types::Rect},
    layout::{LayoutNode, LayoutOutput, Vector2},
    runtime::{
        PaintPrimitive, PaintTextRun, ResolvedEnvironment, inset_rect, optical_centered_baseline,
        push_text_run,
    },
    theme::ThemeTokens,
    widgets::{
        ButtonWidget, DeclaredTextMetrics, FocusedKeyDisposition, TextAlign, TextColorRole,
        TextScaleParticipation, TextWrap, Widget, WidgetCapabilities, WidgetCapabilitiesV2,
        WidgetCommon, WidgetInput, WidgetKey, WidgetOutput, WidgetPaintContext, WidgetSemantics,
        WidgetSemanticsRevision, WidgetSizing,
    },
};

use super::projection::{
    MENU_LABEL_HOTKEY_GAP, MENU_ROW_TEXT_PADDING_X, menu_command_hotkey_hint_color,
    menu_command_label_color,
};

#[derive(Clone)]
pub(super) struct MenuCommandWidget {
    button: ButtonWidget,
    label: TextContent,
    hint: Option<TextContent>,
    hint_width: f32,
}

impl MenuCommandWidget {
    pub(super) fn new(label: TextContent, hint: Option<TextContent>, hint_width: f32) -> Self {
        Self {
            button: ButtonWidget::new(
                0,
                "",
                WidgetSizing::fixed(Vector2::new(0.0, super::actions::MENU_ITEM_HEIGHT)),
            ),
            label,
            hint,
            hint_width,
        }
    }

    fn metrics(&self) -> DeclaredTextMetrics {
        DeclaredTextMetrics::new(
            self.button.common.sizing,
            13.0,
            Vector2::new(MENU_ROW_TEXT_PADDING_X, 0.0),
        )
    }

    fn append_labels(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        theme: &ThemeTokens,
        environment: &ResolvedEnvironment,
    ) {
        if !self.button.common.paint.paints_state_layers {
            return;
        }
        let metrics = self
            .metrics()
            .resolve(environment, TextScaleParticipation::Scaled);
        let content = inset_rect(bounds, metrics.insets.x, 0.0);
        let mut label_rect = content;
        let mut hint_rect = content;
        if self.hint.is_some() {
            let width =
                (self.hint_width * environment.text_scale().factor()).min(content.width().max(0.0));
            let gap = MENU_LABEL_HOTKEY_GAP * environment.text_scale().factor();
            match environment.writing_direction() {
                WritingDirection::Ltr => {
                    hint_rect.min.x = content.max.x - width;
                    label_rect.max.x = (hint_rect.min.x - gap).max(content.min.x);
                }
                WritingDirection::Rtl => {
                    hint_rect.max.x = content.min.x + width;
                    label_rect.min.x = (hint_rect.max.x + gap).min(content.max.x);
                }
            }
        }
        for (text, rect, align, role) in [
            (
                Some(&self.label),
                label_rect,
                TextAlign::Start,
                menu_command_label_color(self.button.common.style),
            ),
            (
                self.hint.as_ref(),
                hint_rect,
                TextAlign::End,
                menu_command_hotkey_hint_color(self.button.common.style),
            ),
        ] {
            let Some(text) = text else { continue };
            let color = match role {
                TextColorRole::Primary => theme.text_primary,
                TextColorRole::Muted => theme.text_muted,
                TextColorRole::OnAccent => theme.bg_primary,
                TextColorRole::Custom(color) => color,
            };
            push_text_run(
                primitives,
                PaintTextRun {
                    widget_id: self.button.common.id,
                    text: text.clone().into_paint_text(),
                    rect,
                    font_size: metrics.font_size,
                    baseline: optical_centered_baseline(rect, metrics.font_size),
                    color,
                    align: align.resolve(environment.writing_direction()),
                    wrap: TextWrap::None,
                },
            );
        }
    }
}

impl WidgetSemantics for MenuCommandWidget {
    fn revision(&self) -> WidgetSemanticsRevision {
        WidgetSemanticsRevision::exact((
            self.label.clone(),
            self.hint.clone(),
            self.button.common.focus,
            self.button.common.state.selected,
            self.button.common.state.disabled,
            self.button.common.state.read_only,
        ))
    }

    fn automation_role(&self) -> AutomationRole {
        AutomationRole::Button
    }
    fn automation_label(&self) -> Option<String> {
        Some(self.label.as_str().to_owned())
    }
    fn automation_description(&self) -> Option<String> {
        self.hint.as_ref().map(|hint| hint.as_str().to_owned())
    }
}

impl Widget for MenuCommandWidget {
    fn common(&self) -> &WidgetCommon {
        &self.button.common
    }
    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.button.common
    }
    fn focused_key_disposition(&self, key: WidgetKey) -> FocusedKeyDisposition {
        self.button.focused_key_disposition(key)
    }
    fn text_scale_participation(&self) -> TextScaleParticipation {
        TextScaleParticipation::Scaled
    }
    fn layout_node_with_environment(&self, environment: &ResolvedEnvironment) -> LayoutNode {
        LayoutNode::Widget(
            self.metrics()
                .resolve(environment, TextScaleParticipation::Scaled)
                .layout_node(self.button.common.id),
        )
    }
    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        Widget::handle_input(&mut self.button, bounds, input)
    }
    fn handle_pointer_capture_cancelled(&mut self, bounds: Rect) -> Option<WidgetOutput> {
        self.button.handle_pointer_capture_cancelled(bounds)
    }
    fn needs_state_synchronization(&self) -> bool {
        true
    }
    fn synchronize_from_previous(&mut self, previous: &dyn Widget) {
        if let Some(previous) = previous.as_any().downcast_ref::<Self>() {
            self.button.synchronize_from_previous(&previous.button);
        }
    }
    fn capabilities(&self) -> WidgetCapabilities<'_> {
        WidgetCapabilities::new().semantics(self)
    }
    fn capabilities_v2(&self) -> WidgetCapabilitiesV2<'_> {
        self.button.capabilities_v2()
    }
    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        self.button.append_paint(primitives, bounds, layout, theme);
        self.append_labels(primitives, bounds, theme, &Default::default());
    }
    fn append_paint_with_context(&self, context: &mut WidgetPaintContext<'_>) {
        self.button.append_paint_with_context(context);
        let bounds = context.bounds();
        let theme = context.theme();
        let environment = context.environment();
        self.append_labels(context.primitives(), bounds, theme, environment);
    }
}
