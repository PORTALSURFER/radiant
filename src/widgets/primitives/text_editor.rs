//! Bounded multiline editor document and widget contracts.
use crate::widgets::interaction::CompositionStartContext;
mod document;
mod editing;
mod input;
mod paint;
#[cfg(test)]
mod tests;
use crate::{
    gui::{
        text_layout::editor::{TextEditorGeometryReceipt, TextEditorLayoutRequest},
        types::{Rect, Vector2},
    },
    layout::LayoutOutput,
    runtime::{PaintPrimitive, ResolvedEnvironment},
    theme::ThemeTokens,
    widgets::{
        CompositionSample, FocusBehavior, FocusedKeyDisposition, TextScaleParticipation, Widget,
        WidgetCommon, WidgetId, WidgetInput, WidgetKey, WidgetOutput, WidgetPaintContext,
        WidgetSizing,
    },
};
pub use document::{
    MAX_TEXT_EDITOR_BYTES, MAX_TEXT_EDITOR_GRAPHEMES, TextEditorCompositionDelta, TextEditorDelta,
    TextEditorDocument, TextEditorEdit, TextEditorError, TextEditorRevision, TextEditorSelection,
    TextEditorSnapshot,
};
use std::{cell::RefCell, rc::Rc};

/// Public multiline editor using exact document deltas and host-shaped geometry.
#[derive(Clone)]
pub struct TextEditorWidget {
    /// Shared sizing, focus, and interaction contract.
    pub common: WidgetCommon,
    snapshot: TextEditorSnapshot,
    owned_document: Option<Rc<RefCell<TextEditorDocument>>>,
    /// Font size before application text scaling.
    pub font_size: f32,
    /// Wrap text to the assigned viewport width.
    pub wrap: bool,
    geometry: Option<TextEditorGeometryReceipt>,
    last_geometry_authority: u64,
    scroll: Vector2,
    preferred_x: Option<f32>,
    reveal_pending: bool,
    hide_adornments: bool,
    privacy: crate::widgets::TextPrivacy,
    privacy_mapping: Option<Rc<crate::widgets::interaction::SecretTextMapping>>,
    clipboard_authority: Rc<crate::widgets::TextEditAuthorityOwner>,
    groups: crate::widgets::interaction::TextEditGroups,
}
/// Construction inputs for an application-owned editor document.
#[derive(Clone)]
pub struct TextEditorWidgetParts {
    /// Stable widget identity.
    pub id: WidgetId,
    /// Exact document authority projected by the application.
    pub snapshot: TextEditorSnapshot,
    /// Assigned intrinsic sizing.
    pub sizing: WidgetSizing,
}
impl TextEditorWidget {
    /// Create a controlled editor. Equal or older reprojections preserve local edits.
    pub fn from_parts(parts: TextEditorWidgetParts) -> Self {
        let mut common = WidgetCommon::new(parts.id, parts.sizing);
        common.focus = FocusBehavior::Keyboard;
        Self {
            common,
            snapshot: parts.snapshot,
            owned_document: None,
            font_size: 14.0,
            wrap: true,
            geometry: None,
            last_geometry_authority: 0,
            scroll: Vector2::new(0.0, 0.0),
            preferred_x: None,
            reveal_pending: true,
            hide_adornments: false,
            privacy: crate::widgets::TextPrivacy::Public,
            privacy_mapping: None,
            clipboard_authority: Rc::new(crate::widgets::TextEditAuthorityOwner::new()),
            groups: Default::default(),
        }
    }
    /// Create an editor that retains its own bounded document until removal.
    pub fn uncontrolled(
        id: WidgetId,
        text: impl Into<std::sync::Arc<str>>,
        sizing: WidgetSizing,
    ) -> Result<Self, TextEditorError> {
        let document = Rc::new(RefCell::new(TextEditorDocument::new(text)?));
        let snapshot = document.borrow().snapshot();
        let mut widget = Self::from_parts(TextEditorWidgetParts {
            id,
            snapshot,
            sizing,
        });
        widget.owned_document = Some(document);
        Ok(widget)
    }
    /// Set bounded secret presentation and explicit copy/automation policy.
    pub fn with_privacy(mut self, privacy: crate::widgets::TextPrivacy) -> Self {
        if self.privacy != privacy {
            let _ = self.clipboard_authority.advance();
            self.privacy = privacy;
            self.refresh_privacy_mapping();
            self.geometry = None;
        }
        self
    }
    /// Current text privacy policy.
    pub const fn privacy(&self) -> crate::widgets::TextPrivacy {
        self.privacy
    }
    fn refresh_privacy_mapping(&mut self) {
        self.privacy_mapping = match self.privacy {
            crate::widgets::TextPrivacy::Public => None,
            crate::widgets::TextPrivacy::Secret(_) => {
                crate::widgets::interaction::SecretTextMapping::new(
                    &self.snapshot.display_text(),
                    crate::widgets::interaction::SecretTextUnit::ExtendedGrapheme,
                )
                .ok()
                .map(Rc::new)
            }
        };
    }
    fn display_text(&self) -> std::sync::Arc<str> {
        match self.privacy {
            crate::widgets::TextPrivacy::Public => self.snapshot.display_text(),
            crate::widgets::TextPrivacy::Secret(_) => self
                .privacy_mapping
                .as_ref()
                .map_or_else(|| std::sync::Arc::from(""), |mapping| mapping.masked_arc()),
        }
    }
    fn display_byte(&self, byte: usize) -> Option<usize> {
        match self.privacy {
            crate::widgets::TextPrivacy::Public => Some(byte),
            crate::widgets::TextPrivacy::Secret(_) => {
                self.privacy_mapping.as_ref()?.source_to_display_byte(byte)
            }
        }
    }
    fn source_caret(
        &self,
        mut caret: crate::gui::text_layout::paragraph::ParagraphCaret,
    ) -> Option<crate::gui::text_layout::paragraph::ParagraphCaret> {
        if matches!(self.privacy, crate::widgets::TextPrivacy::Secret(_)) {
            caret.byte = self
                .privacy_mapping
                .as_ref()?
                .display_to_source_byte(caret.byte)?;
        }
        Some(caret)
    }
    fn display_selection(&self) -> Option<TextEditorSelection> {
        let selection = self.selection();
        Some(TextEditorSelection {
            anchor: self.display_byte(selection.anchor)?,
            caret: self.display_byte(selection.caret)?,
            affinity: selection.affinity,
        })
    }
    /// Current committed text, excluding transient preedit.
    pub fn text(&self) -> &str {
        self.snapshot.text()
    }
    /// Current logical selection in displayed text.
    pub fn selection(&self) -> TextEditorSelection {
        self.snapshot.selection()
    }
    /// Current editor-local scroll offset.
    pub fn scroll_offset(&self) -> Vector2 {
        self.scroll
    }
    /// Build the exact geometry request for this accepted layout/environment.
    pub fn layout_request(
        &self,
        bounds: Rect,
        environment: &ResolvedEnvironment,
    ) -> TextEditorLayoutRequest {
        let font_size = self.font_size * environment.text_scale().factor();
        TextEditorLayoutRequest {
            widget_id: self.common.id,
            owner: self.snapshot.source_id,
            revision: self.snapshot.revision().value(),
            text: self.display_text(),
            rect: crate::runtime::inset_rect(bounds, 6.0, 6.0),
            font_size,
            line_height: font_size * 1.4,
            wrap: self.wrap,
            direction: environment.writing_direction(),
            locale: environment.locale().cloned(),
        }
    }
    /// Accept the host's shared geometry only for the current exact declaration.
    pub fn install_geometry(
        &mut self,
        receipt: TextEditorGeometryReceipt,
        bounds: Rect,
        environment: &ResolvedEnvironment,
    ) -> bool {
        if receipt.authority() < self.last_geometry_authority
            || receipt.request() != &self.layout_request(bounds, environment)
        {
            return false;
        }
        self.last_geometry_authority = receipt.authority();
        self.geometry = Some(receipt);
        self.clamp_scroll();
        if self.reveal_pending {
            self.reveal_selection();
        }
        true
    }
    fn emit(
        &mut self,
        delta: TextEditorDelta,
        selection: TextEditorSelection,
    ) -> Option<WidgetOutput> {
        self.emit_grouped(delta, selection, None, None)
    }
    fn emit_grouped(
        &mut self,
        delta: TextEditorDelta,
        selection: TextEditorSelection,
        kind: Option<crate::widgets::interaction::TextEditKind>,
        boundary: Option<crate::widgets::interaction::TextEditBoundary>,
    ) -> Option<WidgetOutput> {
        use crate::widgets::interaction::{TextEditBoundary, TextEditKind};
        let mut edit = self.snapshot.edit(delta, selection).ok()?;
        let next = self.snapshot.after(&edit).ok()?;
        if let Some(document) = &self.owned_document {
            document.try_borrow_mut().ok()?.apply(&edit).ok()?;
        }
        let grouping = match edit.delta() {
            TextEditorDelta::Selection => self
                .groups
                .boundary(boundary.unwrap_or(TextEditBoundary::Selection)),
            TextEditorDelta::Composition(
                TextEditorCompositionDelta::Start { .. }
                | TextEditorCompositionDelta::Update { .. },
            ) => self.groups.edit(TextEditKind::Composition),
            TextEditorDelta::Composition(TextEditorCompositionDelta::Commit { .. }) => {
                self.groups.finish_composition(false)
            }
            TextEditorDelta::Composition(TextEditorCompositionDelta::Cancel) => {
                self.groups.finish_composition(true)
            }
            _ => self.groups.edit(kind.unwrap_or(TextEditKind::Typing)),
        };
        edit.set_grouping(grouping);
        self.snapshot = next;
        let _ = self.clipboard_authority.advance();
        self.refresh_privacy_mapping();
        self.geometry = None;
        self.reveal_pending = true;
        Some(WidgetOutput::typed(edit))
    }
    fn editing_enabled(&self) -> bool {
        self.common.state.focused && !self.common.state.disabled && !self.common.state.read_only
    }
    fn current_geometry(&self) -> Option<&TextEditorGeometryReceipt> {
        self.geometry.as_ref().filter(|receipt| {
            receipt.request().revision == self.snapshot.revision().value()
                && receipt.request().owner == self.snapshot.source_id
        })
    }
    fn clamp_scroll(&mut self) {
        if let Some(receipt) = self.current_geometry() {
            self.scroll = crate::gui::text_layout::editor::resolve_editor_scroll(
                receipt.geometry(),
                receipt.request(),
                self.display_selection().unwrap_or_default(),
                self.scroll,
                false,
            );
        }
    }
    fn reveal_selection(&mut self) {
        if let Some(receipt) = self.current_geometry() {
            self.scroll = crate::gui::text_layout::editor::resolve_editor_scroll(
                receipt.geometry(),
                receipt.request(),
                self.display_selection().unwrap_or_default(),
                self.scroll,
                true,
            );
            self.reveal_pending = false;
        }
    }
}
impl Widget for TextEditorWidget {
    fn owns_text_clipboard_shortcut(&self) -> bool {
        true
    }
    fn install_text_editor_geometry(
        &mut self,
        receipt: TextEditorGeometryReceipt,
        bounds: Rect,
        environment: &ResolvedEnvironment,
    ) -> bool {
        self.install_geometry(receipt, bounds, environment)
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
    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        self.handle_editor_input(bounds, input, &ResolvedEnvironment::default())
    }
    fn handle_input_with_environment(
        &mut self,
        bounds: Rect,
        input: WidgetInput,
        environment: &ResolvedEnvironment,
    ) -> Option<WidgetOutput> {
        self.handle_editor_input(bounds, input, environment)
    }
    fn capabilities(&self) -> crate::widgets::WidgetCapabilities<'_> {
        crate::widgets::WidgetCapabilities::new().semantics(self)
    }
    fn cursor_for_point(
        &self,
        bounds: Rect,
        point: crate::gui::types::Point,
    ) -> Option<crate::widgets::WidgetCursor> {
        bounds
            .contains(point)
            .then_some(crate::widgets::WidgetCursor::Text)
    }
    fn handle_wheel_sample_with_environment(
        &mut self,
        bounds: Rect,
        position: crate::gui::types::Point,
        sample: crate::widgets::WheelSample,
        environment: &ResolvedEnvironment,
    ) -> Option<WidgetOutput> {
        sample
            .to_widget_input(position)
            .and_then(|input| self.handle_editor_input(bounds, input, environment))
    }
    fn accepts_wheel_input(&self) -> bool {
        !self.common.state.disabled
    }
    fn accepts_text_input(&self) -> bool {
        self.editing_enabled()
    }
    fn focused_key_disposition(&self, _key: WidgetKey) -> FocusedKeyDisposition {
        FocusedKeyDisposition::Consumed
    }
    fn accepts_composition_input(&self) -> bool {
        !self.common.state.disabled && !self.common.state.read_only
    }
    fn composition_start_context(&self) -> Option<CompositionStartContext> {
        self.editor_composition_context()
    }
    fn handle_composition_sample(&mut self, sample: CompositionSample) -> Option<WidgetOutput> {
        self.editor_composition(sample)
    }
    fn handle_hidden_composition_update(
        &mut self,
        preedit: String,
        _timestamp: Option<crate::gui::input::InputTimestamp>,
    ) -> Option<WidgetOutput> {
        self.editor_hidden_preedit(preedit)
    }
    fn retains_managed_composition(&self) -> bool {
        self.snapshot.is_composing()
    }
    fn synchronize_from_previous(&mut self, previous: &dyn Widget) {
        let Some(previous) = previous.as_any().downcast_ref::<Self>() else {
            return;
        };
        if self.common.id != previous.common.id {
            return;
        }
        let uncontrolled = self.owned_document.is_some() && previous.owned_document.is_some();
        if !uncontrolled && !self.snapshot.same_owner(&previous.snapshot) {
            return;
        }
        self.last_geometry_authority = previous.last_geometry_authority;
        if uncontrolled || self.snapshot.revision() <= previous.snapshot.revision() {
            self.snapshot = previous.snapshot.clone();
            self.owned_document = previous.owned_document.clone();
            self.scroll = previous.scroll;
            self.preferred_x = previous.preferred_x;
            self.hide_adornments = previous.hide_adornments;
            if self.privacy == previous.privacy
                && self.common.state.disabled == previous.common.state.disabled
                && self.common.state.read_only == previous.common.state.read_only
            {
                self.clipboard_authority = Rc::clone(&previous.clipboard_authority);
                self.groups = previous.groups.clone();
            } else {
                let _ = previous.clipboard_authority.advance();
            }
        } else {
            let _ = previous.clipboard_authority.advance();
        }
        self.refresh_privacy_mapping();
        self.geometry = None;
        self.reveal_pending = true;
    }
    fn selected_text_slice(&self) -> Option<&str> {
        if self.snapshot.is_composing()
            || matches!(self.privacy, crate::widgets::TextPrivacy::Secret(policy) if !policy.copy_allowed())
        {
            return None;
        }
        let range = self.selection().range();
        (!range.is_empty()).then(|| &self.text()[range])
    }
    fn text_clipboard_receipt(
        &self,
        operation: crate::runtime::TextClipboardOperation,
    ) -> Option<crate::runtime::TextClipboardReceipt> {
        if !self.common.state.focused
            || self.common.state.disabled
            || self.snapshot.is_composing()
            || !self.snapshot.document_is_current()
            || (operation != crate::runtime::TextClipboardOperation::Copy
                && self.common.state.read_only)
        {
            return None;
        }
        crate::runtime::TextClipboardReceipt::new(
            self.common.id,
            operation,
            self.privacy,
            self.clipboard_authority.authority()?,
            crate::runtime::TextClipboardSnapshot::Multiline(self.snapshot.clone()),
            self.selected_text_slice(),
        )
    }
    fn accepts_text_clipboard_receipt(
        &self,
        receipt: &crate::runtime::TextClipboardReceipt,
    ) -> bool {
        let crate::runtime::TextClipboardSnapshot::Multiline(snapshot) = &receipt.snapshot else {
            return false;
        };
        self.common.id == receipt.widget
            && self.common.state.focused
            && !self.common.state.disabled
            && (receipt.operation == crate::runtime::TextClipboardOperation::Copy
                || !self.common.state.read_only)
            && self.privacy == receipt.privacy
            && !self.snapshot.is_composing()
            && self.clipboard_authority.is_current(&receipt.authority)
            && self.snapshot.same_owner(snapshot)
            && self.snapshot.revision() == snapshot.revision()
            && self.snapshot.document_is_current()
    }
    fn prepare_replacement(&mut self, successor: Option<&dyn Widget>) -> Option<WidgetOutput> {
        let compatible = successor
            .and_then(|widget| widget.as_any().downcast_ref::<Self>())
            .is_some_and(|next| {
                next.common.id == self.common.id
                    && next.privacy == self.privacy
                    && next.common.state.disabled == self.common.state.disabled
                    && next.common.state.read_only == self.common.state.read_only
                    && ((next.owned_document.is_some() && self.owned_document.is_some())
                        || (next.snapshot.same_owner(&self.snapshot)
                            && next.snapshot.revision() <= self.snapshot.revision()))
            });
        if !compatible {
            let _ = self.clipboard_authority.advance();
        }
        None
    }
    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        self.paint_editor(primitives, bounds, theme, &ResolvedEnvironment::default());
    }
    fn append_paint_with_context(&self, context: &mut WidgetPaintContext<'_>) {
        let bounds = context.bounds();
        let environment = context.environment().clone();
        let theme = context.theme();
        self.paint_editor(context.primitives(), bounds, theme, &environment);
    }
}

impl crate::widgets::WidgetSemantics for TextEditorWidget {
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
    fn automation_value_text(&self) -> Option<String> {
        match self.privacy {
            crate::widgets::TextPrivacy::Public => Some(self.snapshot.display_text().to_string()),
            crate::widgets::TextPrivacy::Secret(policy) if policy.automation_allowed() => {
                Some(self.snapshot.display_text().to_string())
            }
            crate::widgets::TextPrivacy::Secret(_) => None,
        }
    }
}
