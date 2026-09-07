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
            text: self.snapshot.display_text(),
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
        let edit = self.snapshot.edit(delta, selection).ok()?;
        let next = self.snapshot.after(&edit).ok()?;
        if let Some(document) = &self.owned_document {
            document.try_borrow_mut().ok()?.apply(&edit).ok()?;
        }
        self.snapshot = next;
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
                self.selection(),
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
                self.selection(),
                self.scroll,
                true,
            );
            self.reveal_pending = false;
        }
    }
}
impl Widget for TextEditorWidget {
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
        }
        self.geometry = None;
        self.reveal_pending = true;
    }
    fn selected_text_slice(&self) -> Option<&str> {
        if self.snapshot.is_composing() {
            return None;
        }
        let range = self.selection().range();
        (!range.is_empty()).then(|| &self.text()[range])
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
    fn automation_role(&self) -> crate::gui::automation::AutomationRole {
        crate::gui::automation::AutomationRole::TextInput
    }
    fn automation_value_text(&self) -> Option<String> {
        Some(self.snapshot.display_text().to_string())
    }
}
