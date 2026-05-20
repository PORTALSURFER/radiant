//! Scroll-view paint clipping and affordance helpers.

use super::SurfacePaintContext;
use crate::{
    gui::types::Rect,
    runtime::{
        SurfaceContainer,
        paint::{
            SurfacePaintPlan, push_clip_end, push_clip_start, push_scroll_affordance,
            scroll_content_clip_rect,
        },
    },
};

impl<Message> SurfaceContainer<Message> {
    pub(super) fn begin_scroll_clip(
        &self,
        context: &SurfacePaintContext<'_>,
        plan: &mut SurfacePaintPlan,
    ) -> Option<Rect> {
        let bounds = context.layout.rects.get(&self.id).copied()?;
        let clip_rect = scroll_content_clip_rect(self.id, context.layout, bounds);
        push_clip_start(&mut plan.primitives, self.id, clip_rect);
        Some(clip_rect)
    }

    pub(super) fn end_scroll_clip(&self, plan: &mut SurfacePaintPlan) {
        push_clip_end(&mut plan.primitives, self.id);
    }

    pub(super) fn append_scroll_affordance(
        &self,
        context: &SurfacePaintContext<'_>,
        plan: &mut SurfacePaintPlan,
    ) {
        let Some(content_id) = self.children.first().map(|child| child.child.id()) else {
            return;
        };
        push_scroll_affordance(
            &mut plan.primitives,
            self.id,
            content_id,
            context.layout,
            context.theme,
            context.active_scroll_affordance == Some(self.id),
        );
    }
}
