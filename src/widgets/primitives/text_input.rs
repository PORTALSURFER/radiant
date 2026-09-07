//! Reusable single-line text-input primitive.

use crate::gui::types::{Point, Rect};
use crate::layout::LayoutOutput;
use crate::runtime::{PaintPrimitive, ResolvedEnvironment};
use crate::theme::ThemeTokens;

use super::WidgetCommon;
use super::text::TextAlign;
use crate::widgets::contract::{
    FocusBehavior, FocusedKeyDisposition, Widget, WidgetCapabilities, WidgetId, WidgetPaintContext,
    WidgetPointerMotion, WidgetPointerMotionRevision, WidgetSemantics, WidgetSizing,
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

fn scalar_byte(text: &str, scalar: usize) -> Option<usize> {
    text.char_indices()
        .map(|(byte, _)| byte)
        .chain(std::iter::once(text.len()))
        .nth(scalar)
}

fn scalar_index(text: &str, byte: usize) -> Option<usize> {
    text.char_indices()
        .map(|(candidate, _)| candidate)
        .chain(std::iter::once(text.len()))
        .position(|candidate| candidate == byte)
}

/// Public single-line text-input primitive.
#[derive(Clone, Debug)]
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
    text_edit_authority: std::rc::Rc<crate::widgets::TextEditAuthorityOwner>,
    privacy: crate::widgets::TextPrivacy,
    privacy_mapping: Option<std::rc::Rc<crate::widgets::interaction::SecretTextMapping>>,
}

impl PartialEq for TextInputWidget {
    fn eq(&self, other: &Self) -> bool {
        self.common == other.common
            && self.props == other.props
            && self.state == other.state
            && self.align == other.align
            && self.composition == other.composition
            && self.native_pointer_caret == other.native_pointer_caret
            && self.native_pointer_caret_acceptance == other.native_pointer_caret_acceptance
            && self.native_caret_affinity == other.native_caret_affinity
            && self.privacy == other.privacy
    }
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
            text_edit_authority: std::rc::Rc::new(crate::widgets::TextEditAuthorityOwner::new()),
            privacy: crate::widgets::TextPrivacy::Public,
            privacy_mapping: None,
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

    /// Mask secret text and set explicit clipboard/automation permissions.
    pub fn with_privacy(mut self, privacy: crate::widgets::TextPrivacy) -> Self {
        if self.privacy != privacy {
            self.privacy = privacy;
            self.refresh_text_privacy_mapping();
            self.invalidate_text_edit_authority();
        }
        self
    }

    /// Current text privacy policy.
    pub const fn privacy(&self) -> crate::widgets::TextPrivacy {
        self.privacy
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
        self.handle_input_with_authority(bounds, input, &ResolvedEnvironment::default())
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

    pub(crate) fn append_paint_with_context_hidden_composition(
        &self,
        context: &mut WidgetPaintContext<'_>,
        hidden_composition: bool,
    ) {
        paint::push_text_input_widget_paint_with_context_hidden_composition(
            context,
            self,
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

    pub(crate) fn native_pointer_source_matches(&self, source: &str) -> bool {
        self.display_text() == source
    }

    pub(crate) fn set_native_pointer_display_caret(
        &mut self,
        display_caret: usize,
        affinity: NativeCaretAffinity,
    ) -> bool {
        let Some(caret) = self.source_scalar_for_display_scalar(display_caret) else {
            return false;
        };
        self.set_native_pointer_caret(caret, affinity);
        true
    }

    pub(super) fn pointer_caret_for_position(
        &self,
        bounds: Rect,
        position: Point,
        environment: &ResolvedEnvironment,
    ) -> usize {
        let display = self.display_text();
        let display_caret = editing_ops::caret_for_pointer_x_with_environment(
            bounds,
            position.x,
            &display,
            self.declared_text_metrics(),
            self.align,
            environment,
        );
        self.source_scalar_for_display_scalar(display_caret)
            .unwrap_or_default()
    }

    pub(crate) fn refresh_text_privacy_mapping(&mut self) {
        self.privacy_mapping = match self.privacy {
            crate::widgets::TextPrivacy::Public => None,
            crate::widgets::TextPrivacy::Secret(_) => {
                crate::widgets::interaction::SecretTextMapping::new(
                    &self.state.value,
                    crate::widgets::interaction::SecretTextUnit::ExtendedGrapheme,
                )
                .ok()
                .map(std::rc::Rc::new)
            }
        };
    }

    pub(crate) fn display_state(&self) -> TextInputState {
        match self.privacy {
            crate::widgets::TextPrivacy::Public => self.state.clone(),
            crate::widgets::TextPrivacy::Secret(_) => {
                let Some(mapping) = &self.privacy_mapping else {
                    return TextInputState::from_value(String::new());
                };
                TextInputState {
                    value: mapping.masked().to_owned(),
                    caret: self
                        .source_scalar_to_display_scalar(self.state.caret)
                        .unwrap_or_default(),
                    selection_anchor: self
                        .source_scalar_to_display_scalar(self.state.selection_anchor)
                        .unwrap_or_default(),
                }
            }
        }
    }

    fn display_text(&self) -> std::sync::Arc<str> {
        match self.privacy {
            crate::widgets::TextPrivacy::Public => std::sync::Arc::from(self.state.value.as_str()),
            crate::widgets::TextPrivacy::Secret(_) => self.privacy_mapping.as_ref().map_or_else(
                || std::sync::Arc::from(""),
                |mapping| std::sync::Arc::from(mapping.masked()),
            ),
        }
    }

    fn source_scalar_to_display_scalar(&self, source_scalar: usize) -> Option<usize> {
        if matches!(self.privacy, crate::widgets::TextPrivacy::Public) {
            return (source_scalar <= self.state.char_len()).then_some(source_scalar);
        }
        let source_byte = scalar_byte(&self.state.value, source_scalar)?;
        let mapping = self.privacy_mapping.as_ref()?;
        let display_byte = (0..=source_scalar)
            .rev()
            .filter_map(|scalar| scalar_byte(&self.state.value, scalar))
            .find_map(|byte| mapping.source_to_display_byte(byte))?;
        Some(self.display_text()[..display_byte].chars().count())
    }

    fn source_scalar_for_display_scalar(&self, display_scalar: usize) -> Option<usize> {
        if matches!(self.privacy, crate::widgets::TextPrivacy::Public) {
            return (display_scalar <= self.state.char_len()).then_some(display_scalar);
        }
        let display = self.display_text();
        let display_byte = scalar_byte(&display, display_scalar)?;
        let source_byte = self
            .privacy_mapping
            .as_ref()?
            .display_to_source_byte(display_byte)?;
        scalar_index(&self.state.value, source_byte)
    }

    pub(crate) fn capture_text_edit_authority(&self) -> Option<crate::widgets::TextEditAuthority> {
        self.text_edit_authority.authority()
    }

    pub(crate) fn is_current_text_edit_authority(
        &self,
        authority: &crate::widgets::TextEditAuthority,
    ) -> bool {
        self.text_edit_authority.is_current(authority)
    }

    pub(crate) fn invalidate_text_edit_authority(&self) {
        let _ = self.text_edit_authority.advance();
    }

    pub(crate) fn preserve_text_edit_authority_from(&mut self, previous: &Self) {
        self.text_edit_authority = std::rc::Rc::clone(&previous.text_edit_authority);
    }

    fn handle_input_with_authority(
        &mut self,
        bounds: Rect,
        input: WidgetInput,
        environment: &ResolvedEnvironment,
    ) -> Option<TextInputMessage> {
        input::handle_text_input_with_environment(self, bounds, input, environment)
    }

    fn can_preserve_text_edit_authority_with(&self, successor: Option<&dyn Widget>) -> bool {
        let Some(successor) = successor.and_then(|widget| widget.as_any().downcast_ref::<Self>())
        else {
            return false;
        };
        if self.common.id != successor.common.id
            || self.common.state.disabled != successor.common.state.disabled
            || self.common.state.read_only != successor.common.state.read_only
            || self.props.character_limit != successor.props.character_limit
            || self.props.submit_on_enter != successor.props.submit_on_enter
            || self.privacy != successor.privacy
        {
            return false;
        }
        match (self.props.revision, successor.props.revision) {
            (Some(previous), Some(current)) => current <= previous,
            (None, None) => successor.state.value == self.committed_value_for_sync(),
            (Some(_), None) | (None, Some(_)) => false,
        }
    }
}

impl WidgetSemantics for TextInputWidget {
    fn automation_metadata(&self) -> std::collections::BTreeMap<String, String> {
        match self.privacy {
            crate::widgets::TextPrivacy::Public => Default::default(),
            crate::widgets::TextPrivacy::Secret(policy) => std::collections::BTreeMap::from([
                ("text.privacy".into(), "secret".into()),
                (
                    "text.copy_allowed".into(),
                    policy.copy_allowed().to_string(),
                ),
                (
                    "text.automation_allowed".into(),
                    policy.automation_allowed().to_string(),
                ),
            ]),
        }
    }
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
        match self.privacy {
            crate::widgets::TextPrivacy::Public => Some(self.state.value.clone()),
            crate::widgets::TextPrivacy::Secret(policy) if policy.automation_allowed() => {
                Some(self.state.value.clone())
            }
            crate::widgets::TextPrivacy::Secret(_) => None,
        }
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

impl crate::widgets::WidgetSemanticActions for TextInputWidget {
    fn revision(&self) -> crate::widgets::WidgetSemanticActionRevision {
        crate::widgets::WidgetSemanticActionRevision::exact(self.props.character_limit)
    }
    fn supports(&self, action: &crate::widgets::SemanticAction) -> bool {
        !matches!(self.privacy, crate::widgets::TextPrivacy::Secret(policy) if !policy.automation_allowed())
            && matches!(action, crate::widgets::SemanticAction::SetText(value) if value.len() <= 65_536)
    }
    fn dispatch(
        &mut self,
        action: crate::widgets::SemanticAction,
        _source: crate::widgets::SemanticActionSource,
    ) -> crate::widgets::WidgetSemanticActionResult {
        use crate::widgets::WidgetSemanticActionResult;
        if !self.supports(&action)
            || self.common.state.disabled
            || self.common.state.read_only
            || self.composition.is_some()
        {
            return WidgetSemanticActionResult::Unsupported;
        }
        let crate::widgets::SemanticAction::SetText(value) = action else {
            return WidgetSemanticActionResult::Unsupported;
        };
        let mut value = editing_ops::sanitize_single_line_text(&value);
        if let Some(limit) = self.props.character_limit {
            value = value.chars().take(limit).collect();
        }
        if self.state.value == value {
            return WidgetSemanticActionResult::Accepted(None);
        }
        self.state = TextInputState::from_value(value.clone());
        self.refresh_text_privacy_mapping();
        self.native_pointer_caret = None;
        self.native_pointer_caret_acceptance = None;
        self.native_caret_affinity = NativeCaretAffinity::Downstream;
        self.invalidate_text_edit_authority();
        WidgetSemanticActionResult::Accepted(Some(WidgetOutput::typed(TextInputMessage::Changed {
            value,
        })))
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
        self.handle_input_with_authority(bounds, input, environment)
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
            previous_widget.invalidate_text_edit_authority();
            return;
        }

        let policy_changed = self.common.state.disabled != previous_widget.common.state.disabled
            || self.common.state.read_only != previous_widget.common.state.read_only
            || self.props.character_limit != previous_widget.props.character_limit
            || self.props.submit_on_enter != previous_widget.props.submit_on_enter
            || self.privacy != previous_widget.privacy;
        let preserved = match (previous_widget.props.revision, self.props.revision) {
            (Some(previous_revision), Some(current_revision))
                if current_revision <= previous_revision =>
            {
                self.state = previous_widget.state.clone();
                self.composition = previous_widget.composition.clone();
                true
            }
            (Some(_), Some(_)) | (Some(_), None) | (None, Some(_)) => false,
            (None, None) if self.state.value == previous_widget.committed_value_for_sync() => {
                self.state = previous_widget.state.clone();
                self.composition = previous_widget.composition.clone();
                true
            }
            (None, None) => false,
        };
        if preserved && !policy_changed {
            self.preserve_text_edit_authority_from(previous_widget);
            self.privacy_mapping = previous_widget.privacy_mapping.clone();
        } else {
            previous_widget.invalidate_text_edit_authority();
            self.refresh_text_privacy_mapping();
        }
    }

    fn prepare_replacement(&mut self, successor: Option<&dyn Widget>) -> Option<WidgetOutput> {
        if !self.can_preserve_text_edit_authority_with(successor) {
            self.invalidate_text_edit_authority();
        }
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

    fn action_capabilities(&mut self) -> crate::widgets::WidgetActionCapabilities<'_> {
        crate::widgets::WidgetActionCapabilities::none().with_semantic_actions(self)
    }

    fn capabilities_v2(&self) -> crate::widgets::WidgetCapabilitiesV2<'_> {
        crate::widgets::WidgetCapabilitiesV2::new()
            .with_pointer_motion(self)
            .with_semantic_actions(self)
    }

    fn selected_text_slice(&self) -> Option<&str> {
        (!matches!(self.privacy, crate::widgets::TextPrivacy::Secret(policy) if !policy.copy_allowed()))
            .then(|| self.selected_text_slice())
            .flatten()
    }

    fn text_clipboard_receipt(
        &self,
        operation: crate::runtime::TextClipboardOperation,
    ) -> Option<crate::runtime::TextClipboardReceipt> {
        if !self.common.state.focused
            || self.common.state.disabled
            || self.composition.is_some()
            || (operation != crate::runtime::TextClipboardOperation::Copy
                && self.common.state.read_only)
            || self.state.value.len() > 1024 * 1024
        {
            return None;
        }
        let selected = self.selected_text_slice();
        if matches!(
            operation,
            crate::runtime::TextClipboardOperation::Copy
                | crate::runtime::TextClipboardOperation::Cut
        ) && selected.is_none_or(|text| text.len() > 16 * 1024)
        {
            return None;
        }
        crate::runtime::TextClipboardReceipt::new(
            self.common.id,
            operation,
            self.privacy,
            self.capture_text_edit_authority()?,
            crate::runtime::TextClipboardSnapshot::SingleLine(self.state.clone()),
            selected,
        )
    }

    fn accepts_text_clipboard_receipt(
        &self,
        receipt: &crate::runtime::TextClipboardReceipt,
    ) -> bool {
        let crate::runtime::TextClipboardSnapshot::SingleLine(snapshot) = &receipt.snapshot else {
            return false;
        };
        self.common.id == receipt.widget
            && self.common.state.focused
            && !self.common.state.disabled
            && (receipt.operation == crate::runtime::TextClipboardOperation::Copy
                || !self.common.state.read_only)
            && self.composition.is_none()
            && self.privacy == receipt.privacy
            && self.state == *snapshot
            && self.is_current_text_edit_authority(&receipt.authority)
    }

    fn native_text_input_delegate_mut(&mut self) -> Option<&mut TextInputWidget> {
        Some(self)
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
        self.append_paint_with_context_hidden_composition(
            context,
            self.composition_hides_native_adornments(),
        );
    }
}
