//! Bounded editor geometry retention and exact accepted-plan receipts.
use super::{
    NativeEditorParagraph, NativeTextInputSnapshotFence, NativeTextRenderer, editor,
    model::TextPresentation,
};
use crate::{
    gui::text_layout::editor::{TextEditorGeometryReceipt, TextEditorLayoutRequest},
    runtime::{PaintPrimitive, PaintTextEditor, SurfacePaintPlan},
};
use std::{collections::VecDeque, sync::Arc};
const MAX_EDITOR_CACHE_ENTRIES: usize = 8;
const MAX_EDITOR_BYTES: usize = 32 * 1024 * 1024;
const MAX_EDITOR_PLAN_ENTRIES: usize = 64;
#[derive(Clone, PartialEq)]
struct EditorCacheKey {
    source: Arc<str>,
    font_size: u32,
    width: u32,
    line_height: u32,
    font_generation: u64,
    presentation: TextPresentation,
}
struct CacheEntry {
    key: EditorCacheKey,
    paragraph: Arc<NativeEditorParagraph>,
    bytes: usize,
}
struct PlanEntry {
    receipt: TextEditorGeometryReceipt,
    paragraph: Arc<NativeEditorParagraph>,
}
#[derive(Default)]
pub(super) struct EditorRetention {
    cache: VecDeque<CacheEntry>,
    bytes: usize,
    plan: Vec<PlanEntry>,
    fence: Option<NativeTextInputSnapshotFence>,
}
impl EditorRetention {
    pub(super) fn invalidate(&mut self) {
        self.plan.clear();
        self.fence = None;
    }
}
impl NativeTextRenderer {
    pub(in crate::gui_runtime::native_vello) fn editor_paragraph_for_request(
        &mut self,
        request: &TextEditorLayoutRequest,
    ) -> Option<Arc<NativeEditorParagraph>> {
        if !request.is_valid() {
            return None;
        }
        let width = if request.wrap {
            request.rect.width()
        } else {
            f32::MAX
        };
        let mut key = EditorCacheKey {
            source: request.text.clone(),
            font_size: request.font_size.to_bits(),
            width: width.to_bits(),
            line_height: request.line_height.to_bits(),
            font_generation: self.font_stack.generation(),
            presentation: TextPresentation {
                locale: request.locale.clone(),
                direction: Some(request.direction),
            },
        };
        if let Some(index) = self
            .editor_retention
            .cache
            .iter()
            .position(|entry| entry.key == key)
        {
            let entry = self.editor_retention.cache.remove(index)?;
            let result = entry.paragraph.clone();
            self.editor_retention.cache.push_back(entry);
            return Some(result);
        }
        let paragraph = editor::layout_editor_paragraph(
            &mut self.font_stack,
            &key.presentation,
            request.text.clone(),
            request.font_size,
            width,
            request.line_height,
        )?;
        key.font_generation = self.font_stack.generation();
        let bytes = paragraph.estimated_bytes();
        if bytes > MAX_EDITOR_BYTES {
            return None;
        }
        while self.editor_retention.cache.len() >= MAX_EDITOR_CACHE_ENTRIES
            || self.editor_retention.bytes.saturating_add(bytes) > MAX_EDITOR_BYTES
        {
            let old = self.editor_retention.cache.pop_front()?;
            self.editor_retention.bytes = self.editor_retention.bytes.saturating_sub(old.bytes);
        }
        self.editor_retention.bytes += bytes;
        self.editor_retention.cache.push_back(CacheEntry {
            key,
            paragraph: paragraph.clone(),
            bytes,
        });
        Some(paragraph)
    }
    pub(in crate::gui_runtime::native_vello) fn seed_editor_plan(
        &mut self,
        plan: &SurfacePaintPlan,
        fence: NativeTextInputSnapshotFence,
    ) {
        self.editor_retention.invalidate();
        self.editor_retention.fence = Some(fence);
        let mut bytes = 0usize;
        for primitive in &plan.primitives {
            let PaintPrimitive::TextEditor(input) = primitive else {
                continue;
            };
            if self.editor_retention.plan.len() >= MAX_EDITOR_PLAN_ENTRIES {
                break;
            }
            let Some(paragraph) = self.editor_paragraph_for_request(&input.request) else {
                continue;
            };
            let size = paragraph.estimated_bytes();
            if bytes.saturating_add(size) > MAX_EDITOR_BYTES {
                continue;
            }
            let Some(receipt) =
                TextEditorGeometryReceipt::new(input.request.clone(), paragraph.geometry().clone())
            else {
                continue;
            };
            bytes += size;
            self.editor_retention
                .plan
                .push(PlanEntry { receipt, paragraph });
        }
    }
    pub(in crate::gui_runtime::native_vello) fn editor_plan_receipts(
        &self,
        fence: NativeTextInputSnapshotFence,
    ) -> Vec<TextEditorGeometryReceipt> {
        if self.editor_retention.fence != Some(fence) {
            return Vec::new();
        }
        self.editor_retention
            .plan
            .iter()
            .map(|entry| entry.receipt.clone())
            .collect()
    }
    pub(in crate::gui_runtime::native_vello) fn editor_caret_for_plan(
        &self,
        input: &PaintTextEditor,
        fence: NativeTextInputSnapshotFence,
    ) -> Option<crate::gui::types::Rect> {
        if self.editor_retention.fence != Some(fence) {
            return None;
        }
        let entry = self
            .editor_retention
            .plan
            .iter()
            .find(|entry| entry.receipt.request() == &input.request)?;
        super::editor_paint::editor_caret_rect(input, &entry.paragraph)
    }
}
