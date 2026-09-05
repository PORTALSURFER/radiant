//! Reusable single-line text-input primitive.

use crate::gui::types::Rect;
use crate::layout::LayoutOutput;
use crate::runtime::{PaintPrimitive, ResolvedEnvironment};
use crate::theme::ThemeTokens;

use super::WidgetCommon;
use super::text::TextAlign;
use crate::widgets::contract::{
    FocusBehavior, FocusedKeyDisposition, Widget, WidgetCapabilities, WidgetId,
    WidgetPaintContext, WidgetPointerMotion, WidgetPointerMotionRevision, WidgetSemantics,
    WidgetSizing,
};
use crate::widgets::interaction::{
    CompositionRange, CompositionSample, CompositionStartContext, TextInputMessage, WidgetInput,
    WidgetKey, WidgetOutput,
};
use crate::widgets::{DeclaredTextMetrics, TextScaleParticipation};

mod builders;
mod composition;
mod editing;
mod editing_ops;
mod input;
mod model;
mod paint;

pub(super) const COMPACT_INPUT_HEIGHT: f32 = 28.0;

#[cfg(test)]
mod tests;

pub use model::{TextInputChrome, TextInputEditResult, TextInputProps, TextInputState};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum NativeCaretAffinity {
    Upstream,
    Downstream,
}

/// Public single-line text-input primitive.
#[derive(Clone, Debug, PartialEq)]
pub struct TextInputWidget {
    /// Shared widget contract.
    pub common: WidgetCommon,
    /// Immutable user-facing text-input configuration.
    pub props: TextInputProps,
    /// Mutable input state owned by the widget.
    pub state: TextInputState,
    /// Logical alignment used by the input text and native caret geometry.
    pub align: TextAlign,
    /// Transient IME composition state owned by this widget.
    pub(crate) composition: Option<composition::TextInputComposition>,
    native_pointer_caret: Option<(usize, NativeCaretAffinity)>,
    native_pointer_caret_acceptance: Option<NativeCaretAffinity>,
    native_caret_affinity: NativeCaretAffinity,
}

/// Named construction fields for [`TextInputWidget`].
#[derive(Clone, Debug, PartialEq)]
pub struct TextInputWidgetParts {
    /// Stable widget identity used by layout, events, and state synchronization.
    pub id: WidgetId,
    /// Initial text value.
    pub value: String,
    /// Intrinsic text-input sizing contract.
    pub sizing: WidgetSizing,
}

impl TextInputWidget {
    /// Build a single-line text-input descriptor from named identity, value, and sizing fields.
    pub fn from_parts(parts: TextInputWidgetParts) -> Self {
        let mut common = WidgetCommon::new(parts.id, parts.sizing);
        common.focus = FocusBehavior::Keyboard;
        Self {
            common,
            props: TextInputProps {
                placeholder: None,
                completion_suffix: None,
                submit_on_enter: true,
                character_limit: None,
                chrome: TextInputChrome::Full,
                revision: None,
            },
            state: TextInputState::from_value(parts.value),
            align: TextAlign::Start,
            composition: None,
            native_pointer_caret: None,
            native_pointer_caret_acceptance: None,
            native_caret_affinity: NativeCaretAffinity::Downstream,
        }
    }

    /// Build a single-line text-input descriptor with edit semantics.
    pub fn new(id: WidgetId, value: impl Into<String>, sizing: WidgetSizing) -> Self {
        Self::from_parts(TextInputWidgetParts {
            id,
            value: value.into(),
            sizing,
        })
    }

    /// Set logical text alignment inside the input content rectangle.
    pub fn with_align(mut self, align: TextAlign) -> Self {
        self.align = align;
        self
    }

    pub(crate) fn declared_text_metrics(&self) -> DeclaredTextMetrics {
        let compact = self.common.sizing.preferred.y <= COMPACT_INPUT_HEIGHT;
        DeclaredTextMetrics::new(
            self.common.sizing,
            crate::runtime::input_font_size_for_height(self.common.sizing.preferred.y),
            crate::layout::Vector2::new(
                if compact { 8.0 } else { 16.0 },
                if compact { 2.0 } else { 4.0 },
            ),
        )
    }

    /// Route one backend-neutral interaction into the single-line text input.
    pub fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<TextInputMessage> {
        input::handle_text_input(self, bounds, input)
    }

    pub(super) fn accepts_editing_input(&self) -> bool {
        self.common.state.focused && !self.common.state.disabled && !self.common.state.read_only
    }

    pub(crate) fn native_composition_start_context(&self) -> Option<CompositionStartContext> {
        if !self.accepts_editing_input() {
            return None;
        }
        let scalar_len = self.state.char_len();
        let (start, end) = self.state.selection_range();
        let selection = CompositionRange::new(start, end, scalar_len).ok()?;
        CompositionStartContext::new(selection, selection).ok()
    }

    pub(crate) fn append_paint_with_hidden_composition(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        theme: &ThemeTokens,
        hidden_composition: bool,
    ) {
        paint::push_text_input_widget_paint_with_hidden_composition(
            primitives,
            self,
            bounds,
            theme,
            hidden_composition,
        );
    }

    pub(crate) fn set_native_pointer_caret(&mut self, caret: usize, affinity: NativeCaretAffinity) {
        self.native_pointer_caret = Some((caret, affinity));
        self.native_pointer_caret_acceptance = None;
        self.native_caret_affinity = affinity;
    }

    pub(crate) fn take_native_pointer_caret(&mut self) -> Option<(usize, NativeCaretAffinity)> {
        self.native_pointer_caret.take()
    }

    pub(crate) fn accept_native_pointer_caret(&mut self, affinity: NativeCaretAffinity) {
        self.native_pointer_caret_acceptance = Some(affinity);
    }

    pub(crate) fn take_native_pointer_caret_acceptance(&mut self) -> Option<NativeCaretAffinity> {
        self.native_pointer_caret_acceptance.take()
    }

    pub(crate) fn clear_native_pointer_caret(&mut self) {
        self.native_pointer_caret = None;
        self.native_pointer_caret_acceptance = None;
    }

    pub(crate) fn reset_native_pointer_affinity(&mut self) {
        self.native_caret_affinity = NativeCaretAffinity::Downstream;
    }
}

impl WidgetSemantics for TextInputWidget {
    fn automation_role(&self) -> crate::gui::automation::AutomationRole {
        crate::gui::automation::AutomationRole::TextInput
    }

    fn automation_label(&self) -> Option<String> {
        self.props
            .placeholder
            .as_ref()
            .map(|placeholder| placeholder.as_str().to_owned())
    }

    fn automation_value_text(&self) -> Option<String> {
        Some(self.state.value.clone())
    }
}

impl WidgetPointerMotion for TextInputWidget {
    fn revision(&self) -> WidgetPointerMotionRevision {
        WidgetPointerMotionRevision::exact(false)
    }

    fn accepts_pointer_move(&self) -> bool {
        false
    }
}

impl Widget for TextInputWidget {
    fn focused_key_disposition(&self, key: WidgetKey) -> FocusedKeyDisposition {
        match key {
            WidgetKey::Home | WidgetKey::End => FocusedKeyDisposition::Consumed,
            WidgetKey::PageUp | WidgetKey::PageDown
                if self.composition.is_some() || self.common.state.pressed =>
            {
                FocusedKeyDisposition::Consumed
            }
            WidgetKey::PageUp | WidgetKey::PageDown => FocusedKeyDisposition::Unhandled,
            _ => FocusedKeyDisposition::Consumed,
        }
    }

    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn text_scale_participation(&self) -> TextScaleParticipation {
        TextScaleParticipation::Scaled
    }

    fn layout_node_with_environment(
        &self,
        environment: &ResolvedEnvironment,
    ) -> crate::layout::LayoutNode {
        let sizing = DeclaredTextMetrics::new(
            self.common.sizing,
            crate::runtime::input_font_size_for_height(self.common.sizing.preferred.y),
            crate::layout::Vector2::new(0.0, 0.0),
        )
        .resolve(environment, self.text_scale_participation());
        crate::layout::LayoutNode::Widget(sizing.layout_node(self.common.id))
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        TextInputWidget::handle_input(self, bounds, input).map(WidgetOutput::typed)
    }

    fn handle_input_with_environment(
        &mut self,
        bounds: Rect,
        input: WidgetInput,
        environment: &ResolvedEnvironment,
    ) -> Option<WidgetOutput> {
        input::handle_text_input_with_environment(self, bounds, input, environment)
            .map(WidgetOutput::typed)
    }

    fn accepts_composition_input(&self) -> bool {
        // Runtime focus authority is checked separately. Keep this capability
        // true during refresh reconciliation, before focused widget state is
        // restored on the replacement surface.
        !self.common.state.disabled && !self.common.state.read_only
    }

    fn composition_start_context(&self) -> Option<CompositionStartContext> {
        self.native_composition_start_context()
    }

    fn handle_composition_sample(&mut self, sample: CompositionSample) -> Option<WidgetOutput> {
        composition::handle_sample(self, sample).map(WidgetOutput::typed)
    }

    fn handle_hidden_composition_update(
        &mut self,
        preedit: String,
        _timestamp: Option<crate::gui::input::InputTimestamp>,
    ) -> Option<WidgetOutput> {
        composition::handle_hidden_update(self, preedit).map(WidgetOutput::typed)
    }

    fn retains_managed_composition(&self) -> bool {
        self.composition.is_some()
    }

    fn synchronize_from_previous(&mut self, previous: &dyn Widget) {
        let Some(previous_widget) = previous.as_any().downcast_ref::<TextInputWidget>() else {
            return;
        };
        if self.common.id != previous_widget.common.id {
            return;
        }

        match (previous_widget.props.revision, self.props.revision) {
            (Some(previous_revision), Some(current_revision))
                if current_revision <= previous_revision =>
            {
                self.state = previous_widget.state.clone();
                self.composition = previous_widget.composition.clone();
            }
            (Some(_), Some(_)) | (Some(_), None) | (None, Some(_)) => {}
            (None, None) if self.state.value == previous_widget.committed_value_for_sync() => {
                self.state = previous_widget.state.clone();
                self.composition = previous_widget.composition.clone();
            }
            (None, None) => {}
        }
    }

    fn prepare_replacement(&mut self, successor: Option<&dyn Widget>) -> Option<WidgetOutput> {
        if self.composition.is_some() && !self.can_preserve_composition_with(successor) {
            self.cancel_composition();
        }
        None
    }

    fn accepts_text_input(&self) -> bool {
        self.accepts_editing_input()
    }

    fn capabilities(&self) -> WidgetCapabilities<'_> {
        WidgetCapabilities::new().semantics(self)
    }

    fn capabilities_v2(&self) -> crate::widgets::WidgetCapabilitiesV2<'_> {
        crate::widgets::WidgetCapabilitiesV2::new().with_pointer_motion(self)
    }

    fn selected_text_slice(&self) -> Option<&str> {
        self.selected_text_slice()
    }

    fn selected_text(&self) -> Option<String> {
        self.selected_text()
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        paint::push_text_input_widget_paint(primitives, self, bounds, theme);
    }

    fn append_paint_with_context(&self, context: &mut WidgetPaintContext<'_>) {
        paint::push_text_input_widget_paint_with_context(context, self);
    }
}
