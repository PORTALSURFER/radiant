//! Immediate shape replay for paint ordered above native GPU surfaces.

mod buffer;
mod geometry;
mod pipeline;
mod target;
mod vertex;

use crate::gui::types::Rect as UiRect;
use buffer::OverlayVertexBuffer;
use geometry::{append_replayable_vertices, replayable_vertices_in_regions_into};
use pipeline::PostGpuOverlayPipeline;
pub(in crate::gui_runtime::native_vello::generic_runtime) use target::PostGpuOverlayRenderTarget;
use vertex::{OverlayVertex, overlay_vertex_bytes};

#[derive(Default)]
pub(super) struct PostGpuOverlayRenderer {
    pipeline: Option<PostGpuOverlayPipeline>,
    vertices: Vec<OverlayVertex>,
    gpu_regions: Vec<UiRect>,
    vertex_buffer: OverlayVertexBuffer,
}

impl PostGpuOverlayRenderer {
    pub(super) fn render_layers(
        &mut self,
        target: &mut PostGpuOverlayRenderTarget<'_>,
        primitives: &[crate::runtime::PaintPrimitive],
        overlay_primitives: &[crate::runtime::PaintPrimitive],
    ) {
        let suffix = geometry::replayable_suffix(primitives);
        geometry::gpu_surface_overlay_regions_into(primitives, &mut self.gpu_regions);
        if overlay_primitives.is_empty() {
            if let Some(suffix) = suffix {
                replayable_vertices_in_regions_into(
                    suffix,
                    target.size,
                    &self.gpu_regions,
                    &mut self.vertices,
                );
            } else {
                self.vertices.clear();
            }
        } else {
            self.vertices.clear();
            if let Some(suffix) = suffix {
                geometry::append_replayable_vertices_in_regions(
                    suffix,
                    target.size,
                    &self.gpu_regions,
                    &mut self.vertices,
                );
            }
            append_replayable_vertices(overlay_primitives, target.size, &mut self.vertices);
        }
        self.render_vertices(target);
    }

    fn render_vertices(&mut self, target: &mut PostGpuOverlayRenderTarget<'_>) {
        if self.vertices.is_empty() {
            return;
        }
        let vertex_bytes = overlay_vertex_bytes(&self.vertices);
        let vertex_buffer = self.vertex_buffer.upload(target, vertex_bytes);
        let pipeline = self
            .pipeline
            .get_or_insert_with(|| PostGpuOverlayPipeline::new(target.device, target.format));
        if pipeline.format() != target.format {
            *pipeline = PostGpuOverlayPipeline::new(target.device, target.format);
        }
        pipeline.render(target, vertex_buffer, self.vertices.len() as u32);
    }
}
