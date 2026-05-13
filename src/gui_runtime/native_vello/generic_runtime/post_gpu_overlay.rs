//! Immediate shape replay for paint ordered above native GPU surfaces.

mod geometry;
mod pipeline;
mod target;

use geometry::{OverlayVertex, overlay_vertex_bytes, replayable_vertices_into};
use pipeline::PostGpuOverlayPipeline;
pub(in crate::gui_runtime::native_vello::generic_runtime) use target::PostGpuOverlayRenderTarget;

#[derive(Default)]
pub(super) struct PostGpuOverlayRenderer {
    pipeline: Option<PostGpuOverlayPipeline>,
    vertices: Vec<OverlayVertex>,
}

impl PostGpuOverlayRenderer {
    pub(super) fn render(
        &mut self,
        target: &mut PostGpuOverlayRenderTarget<'_>,
        primitives: &[crate::runtime::PaintPrimitive],
    ) {
        let Some(suffix) = geometry::replayable_suffix(primitives) else {
            return;
        };
        self.render_primitives(target, suffix);
    }

    pub(super) fn render_primitives(
        &mut self,
        target: &mut PostGpuOverlayRenderTarget<'_>,
        primitives: &[crate::runtime::PaintPrimitive],
    ) {
        replayable_vertices_into(primitives, target.size, &mut self.vertices);
        if self.vertices.is_empty() {
            return;
        }
        let pipeline = self
            .pipeline
            .get_or_insert_with(|| PostGpuOverlayPipeline::new(target.device, target.format));
        if pipeline.format() != target.format {
            *pipeline = PostGpuOverlayPipeline::new(target.device, target.format);
        }
        let vertex_buffer =
            pipeline.create_vertex_buffer(target.device, overlay_vertex_bytes(&self.vertices));
        pipeline.render(target, &vertex_buffer, self.vertices.len() as u32);
    }
}
