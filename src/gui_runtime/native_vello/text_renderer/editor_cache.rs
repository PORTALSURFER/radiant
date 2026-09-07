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
    focused: bool,
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
        // Retain the focused editor before optional idle geometry.
        for focused in [true, false] {
            for primitive in &plan.primitives {
                let PaintPrimitive::TextEditor(input) = primitive else {
                    continue;
                };
                if self.editor_retention.plan.len() >= MAX_EDITOR_PLAN_ENTRIES {
                    break;
                }
                if input.focused == focused {
                    self.admit_editor_input(input, false);
                }
            }
        }
    }
    pub(in crate::gui_runtime::native_vello) fn retained_editor_paragraph(
        &self,
        request: &TextEditorLayoutRequest,
    ) -> Option<Arc<NativeEditorParagraph>> {
        self.editor_retention.fence?;
        self.editor_retention
            .plan
            .iter()
            .find(|entry| entry.receipt.request() == request)
            .map(|entry| entry.paragraph.clone())
    }
    pub(in crate::gui_runtime::native_vello) fn admit_editor_plan_request(
        &mut self,
        plan: &SurfacePaintPlan,
        request: &TextEditorLayoutRequest,
        fence: NativeTextInputSnapshotFence,
        promote: bool,
    ) -> Option<TextEditorGeometryReceipt> {
        if self.editor_retention.fence != Some(fence) {
            return None;
        }
        let input = plan
            .primitives
            .iter()
            .find_map(|primitive| match primitive {
                PaintPrimitive::TextEditor(input) if &input.request == request => Some(input),
                _ => None,
            })?;
        self.admit_editor_input(input, promote)
    }
    fn admit_editor_input(
        &mut self,
        input: &PaintTextEditor,
        promote: bool,
    ) -> Option<TextEditorGeometryReceipt> {
        let request = &input.request;
        if let Some(entry) = self
            .editor_retention
            .plan
            .iter()
            .find(|entry| entry.receipt.request() == request)
        {
            return Some(entry.receipt.clone());
        }
        if !promote && self.editor_retention.plan.len() >= MAX_EDITOR_PLAN_ENTRIES {
            return None;
        }
        let paragraph = self.editor_paragraph_for_request(request)?;
        let size = paragraph.estimated_bytes();
        loop {
            let bytes: usize = self
                .editor_retention
                .plan
                .iter()
                .map(|entry| entry.paragraph.estimated_bytes())
                .sum();
            if self.editor_retention.plan.len() < MAX_EDITOR_PLAN_ENTRIES
                && bytes.saturating_add(size) <= MAX_EDITOR_BYTES
            {
                break;
            }
            if !promote {
                return None;
            }
            let index = self
                .editor_retention
                .plan
                .iter()
                .position(|entry| !entry.focused)?;
            self.editor_retention.plan.remove(index);
        }
        let receipt =
            TextEditorGeometryReceipt::new(request.clone(), paragraph.geometry().clone())?;
        self.editor_retention.plan.push(PlanEntry {
            receipt: receipt.clone(),
            paragraph,
            focused: input.focused,
        });
        Some(receipt)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        application::WritingDirection,
        gui::types::{Rect, Rgba8, Vector2},
        widgets::TextEditorSelection,
    };
    fn renderer() -> NativeTextRenderer {
        NativeTextRenderer {
            font_stack: super::super::font::NativeFontStack::from_test_bytes(&[include_bytes!(
                "../../../../tests/fixtures/fonts/primary.ttf"
            )]),
            layout_cache: super::super::cache::TextLayoutCache::new(),
            editor_retention: EditorRetention::default(),
            native_caret_affinities: Default::default(),
            retained_text_input_snapshot: Default::default(),
        }
    }
    fn input(text: &str) -> PaintTextEditor {
        let color = Rgba8 {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        };
        PaintTextEditor {
            request: TextEditorLayoutRequest {
                widget_id: 17,
                owner: 2,
                revision: 3,
                text: Arc::from(text),
                rect: Rect::from_xy_size(6.0, 6.0, 36.0, 28.0),
                font_size: 20.0,
                line_height: 28.0,
                wrap: true,
                direction: WritingDirection::Ltr,
                locale: None,
            },
            selection: TextEditorSelection::caret(text.len()),
            scroll: Vector2::new(0.0, 0.0),
            color,
            selection_color: color,
            caret_color: color,
            focused: true,
            hide_adornments: false,
            reveal_caret: true,
        }
    }
    #[test]
    fn editor_plan_prioritizes_focus_and_promotes_pointer_target_with_exact_fence() {
        let mut renderer = renderer();
        let mut plan = SurfacePaintPlan::empty(&crate::theme::ThemeTokens::default());
        for id in 0..65 {
            let mut editor = input("A");
            editor.request.widget_id = id;
            editor.focused = id == 64;
            plan.primitives
                .push(PaintPrimitive::TextEditor(Box::new(editor)));
        }
        let fence = NativeTextInputSnapshotFence::new(1, 1);
        renderer.seed_editor_plan(&plan, fence);
        let PaintPrimitive::TextEditor(focused) = &plan.primitives[64] else {
            unreachable!()
        };
        assert!(renderer.editor_caret_for_plan(focused, fence).is_some());
        let PaintPrimitive::TextEditor(omitted) = &plan.primitives[63] else {
            unreachable!()
        };
        assert!(
            renderer
                .retained_editor_paragraph(&omitted.request)
                .is_none()
        );
        let receipt = renderer
            .admit_editor_plan_request(&plan, &omitted.request, fence, true)
            .unwrap();
        let retained = renderer
            .retained_editor_paragraph(&omitted.request)
            .unwrap();
        assert!(Arc::ptr_eq(receipt.geometry(), retained.geometry()));
        assert_eq!(
            renderer.editor_retention.plan.len(),
            MAX_EDITOR_PLAN_ENTRIES
        );
        assert!(renderer.editor_caret_for_plan(focused, fence).is_some());
        // Churn the small shaping cache: painting still uses the admitted geometry.
        for width in 40..60 {
            let mut request = omitted.request.clone();
            request.rect = crate::gui::types::Rect::from_xy_size(0.0, 0.0, width as f32, 28.0);
            renderer.editor_paragraph_for_request(&request).unwrap();
        }
        assert!(Arc::ptr_eq(
            &retained,
            &renderer
                .retained_editor_paragraph(&omitted.request)
                .unwrap()
        ));
        let mut stale = omitted.request.clone();
        stale.revision += 1;
        assert!(
            renderer
                .admit_editor_plan_request(&plan, &stale, fence, true)
                .is_none()
        );
        assert!(
            renderer
                .admit_editor_plan_request(
                    &plan,
                    &omitted.request,
                    NativeTextInputSnapshotFence::new(2, 1),
                    true
                )
                .is_none()
        );
    }
    #[test]
    fn native_editor_cache_reuses_geometry_across_selection_but_fences_metrics_and_fonts() {
        let mut renderer = renderer();
        let mut input = input("A B A B");
        let first = renderer
            .editor_paragraph_for_request(&input.request)
            .unwrap();
        input.request.revision += 1;
        let selected = renderer
            .editor_paragraph_for_request(&input.request)
            .unwrap();
        assert!(Arc::ptr_eq(&first, &selected));
        input.request.rect = Rect::from_xy_size(6.0, 6.0, 70.0, 28.0);
        let wide = renderer
            .editor_paragraph_for_request(&input.request)
            .unwrap();
        assert!(!Arc::ptr_eq(&wide, &first));
        renderer
            .font_stack
            .append_test_bytes(include_bytes!(
                "../../../../tests/fixtures/fonts/primary.ttf"
            ))
            .unwrap();
        let fonts = renderer
            .editor_paragraph_for_request(&input.request)
            .unwrap();
        assert!(!Arc::ptr_eq(&fonts, &wide));
        for width in 10..30 {
            input.request.rect = Rect::from_xy_size(6.0, 6.0, width as f32, 28.0);
            renderer
                .editor_paragraph_for_request(&input.request)
                .unwrap();
        }
        assert_eq!(
            renderer.editor_retention.cache.len(),
            MAX_EDITOR_CACHE_ENTRIES
        );
        assert!(renderer.editor_retention.bytes <= MAX_EDITOR_BYTES);
    }
    #[test]
    fn native_editor_paint_and_ime_use_current_plan_and_same_revealed_geometry() {
        let mut renderer = renderer();
        let mut input = input("A B\nA B\nA");
        let plan = SurfacePaintPlan {
            primitives: vec![PaintPrimitive::TextEditor(Box::new(input.clone()))],
            ..SurfacePaintPlan::empty(&crate::theme::ThemeTokens::default())
        };
        let fence = NativeTextInputSnapshotFence::new(1, 1);
        renderer.seed_editor_plan(&plan, fence);
        let receipt = renderer.editor_plan_receipts(fence).pop().unwrap();
        assert!(receipt.geometry().lines().len() >= 3);
        let caret = renderer.editor_caret_for_plan(&input, fence).unwrap();
        assert!(caret.min.y >= input.request.rect.min.y && caret.max.y <= input.request.rect.max.y);
        let mut visible = vello::Scene::new();
        assert!(renderer.encode_editor(&mut visible, &input));
        input.hide_adornments = true;
        let mut hidden = vello::Scene::new();
        assert!(renderer.encode_editor(&mut hidden, &input));
        assert!(visible.encoding().draw_tags.len() > hidden.encoding().draw_tags.len());
        input.hide_adornments = false;
        input.request.revision += 1;
        assert!(renderer.editor_caret_for_plan(&input, fence).is_none());
        let next = NativeTextInputSnapshotFence::new(2, 2);
        renderer.seed_editor_plan(
            &SurfacePaintPlan::empty(&crate::theme::ThemeTokens::default()),
            next,
        );
        assert!(renderer.editor_plan_receipts(fence).is_empty());
        assert!(renderer.editor_caret_for_plan(&input, next).is_none());
    }
}
