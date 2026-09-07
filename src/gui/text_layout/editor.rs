//! Exact declaration and geometry receipts shared by editor hosts and widgets.
use super::paragraph::ParagraphGeometry;
use crate::{
    application::{LocaleId, WritingDirection},
    gui::types::Rect,
    widgets::WidgetId,
};
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

/// Immutable geometry inputs for one accepted editor declaration.
#[derive(Clone, Debug, PartialEq)]
pub struct TextEditorLayoutRequest {
    /// Stable widget identity.
    pub widget_id: WidgetId,
    /// Application-owned document identity.
    pub owner: u64,
    /// Exact edit, selection, or composition revision.
    pub revision: u64,
    /// Displayed text, including active preedit.
    pub text: Arc<str>,
    /// Visible text viewport in logical window coordinates.
    pub rect: Rect,
    /// Resolved font size, including application text scaling.
    pub font_size: f32,
    /// Resolved line advance in logical pixels.
    pub line_height: f32,
    /// Whether lines wrap to the viewport width.
    pub wrap: bool,
    /// Effective shaping direction.
    pub direction: WritingDirection,
    /// Effective shaping language.
    pub locale: Option<LocaleId>,
}
impl TextEditorLayoutRequest {
    /// Whether the declaration can produce finite, bounded geometry.
    pub fn is_valid(&self) -> bool {
        self.rect.is_finite()
            && self.rect.width() > 0.0
            && self.rect.height() > 0.0
            && self.font_size.is_finite()
            && self.font_size > 0.0
            && self.line_height.is_finite()
            && self.line_height > 0.0
            && self.text.len() <= super::paragraph::MAX_PARAGRAPH_SOURCE_BYTES
    }
}
static NEXT_RECEIPT: AtomicU64 = AtomicU64::new(1);
/// A host's publication of the shared paragraph result for an exact declaration.
///
/// Hosts publish after accepting their render plan and install this same receipt
/// before input. A later publication supersedes earlier font/context geometry.
#[derive(Clone, Debug)]
pub struct TextEditorGeometryReceipt {
    request: TextEditorLayoutRequest,
    geometry: Arc<ParagraphGeometry>,
    authority: u64,
}
impl TextEditorGeometryReceipt {
    /// Publish geometry from the host's accepted plan. Retain the returned receipt
    /// for paint, hit testing, and IME until that plan is replaced.
    pub fn new(request: TextEditorLayoutRequest, geometry: Arc<ParagraphGeometry>) -> Option<Self> {
        if !request.is_valid()
            || geometry.source() != request.text.as_ref()
            || geometry.line_height() != request.line_height
            || geometry.wrap_width()
                != if request.wrap {
                    request.rect.width()
                } else {
                    f32::MAX
                }
        {
            return None;
        }
        let authority = NEXT_RECEIPT
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| v.checked_add(1))
            .ok()?;
        Some(Self {
            request,
            geometry,
            authority,
        })
    }
    /// Exact declaration covered by the result.
    pub fn request(&self) -> &TextEditorLayoutRequest {
        &self.request
    }
    /// Shared immutable geometry used by every consumer.
    pub fn geometry(&self) -> &Arc<ParagraphGeometry> {
        &self.geometry
    }
    /// Monotonic publication identity, independent of document revisions.
    pub fn authority(&self) -> u64 {
        self.authority
    }
}

/// Resolve the shared editor viewport used by widget input, native paint, and IME.
/// Geometry admission and caret reveal must use this same projection even when
/// the paint plan was constructed before the host installed its receipt.
pub fn resolve_editor_scroll(
    geometry: &ParagraphGeometry,
    request: &TextEditorLayoutRequest,
    selection: crate::widgets::TextEditorSelection,
    mut scroll: crate::gui::types::Vector2,
    reveal: bool,
) -> crate::gui::types::Vector2 {
    let maximum_x = (geometry.width() - request.rect.width()).max(0.0);
    let maximum_y = (geometry.height() - request.rect.height()).max(0.0);
    scroll.x = if scroll.x.is_finite() {
        scroll.x.clamp(0.0, maximum_x)
    } else {
        0.0
    };
    scroll.y = if scroll.y.is_finite() {
        scroll.y.clamp(0.0, maximum_y)
    } else {
        0.0
    };
    if reveal
        && let Some(point) = geometry.caret(super::paragraph::ParagraphCaret {
            byte: selection.caret,
            affinity: selection.affinity,
        })
    {
        scroll.x = scroll
            .x
            .min(point.x)
            .max(point.x - request.rect.width() + 1.0)
            .clamp(0.0, maximum_x);
        scroll.y = scroll
            .y
            .min(point.y)
            .max(point.y + request.line_height - request.rect.height())
            .clamp(0.0, maximum_y);
    }
    scroll
}
