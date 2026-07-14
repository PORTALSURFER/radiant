use crate::gui::types::Rect as UiRect;
use crate::gui_runtime::native_vello::generic_runtime::runtime_helpers::{
    append_rect_outside_clip, intersect_rect,
};
use crate::runtime::PaintPrimitive;

const OPAQUE_SUFFIX_OCCLUSION_ALPHA: u8 = 240;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::gui_runtime::native_vello) enum SurfaceOcclusionPolicy {
    Exact,
    GpuCompositor,
}

impl SurfaceOcclusionPolicy {
    const fn minimum_alpha(self) -> u8 {
        match self {
            Self::Exact => u8::MAX,
            Self::GpuCompositor => OPAQUE_SUFFIX_OCCLUSION_ALPHA,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::gui_runtime::native_vello) struct SurfaceOcclusionPlanStats {
    pub(in crate::gui_runtime::native_vello) paint_primitives_visited: usize,
    pub(in crate::gui_runtime::native_vello) occluder_rects_indexed: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::gui_runtime::native_vello) struct SurfaceOcclusionQueryStats {
    pub(in crate::gui_runtime::native_vello) index_nodes_visited: usize,
    pub(in crate::gui_runtime::native_vello) occluder_candidates_visited: usize,
}

impl SurfaceOcclusionQueryStats {
    pub(in crate::gui_runtime::native_vello) fn add(&mut self, other: Self) {
        self.index_nodes_visited = self
            .index_nodes_visited
            .saturating_add(other.index_nodes_visited);
        self.occluder_candidates_visited = self
            .occluder_candidates_visited
            .saturating_add(other.occluder_candidates_visited);
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum SurfaceClipState {
    #[default]
    Unclipped,
    Clipped(UiRect),
    Empty,
}

impl SurfaceClipState {
    fn begin(self, clip: UiRect) -> Self {
        if !clip.has_finite_positive_area() {
            return Self::Empty;
        }
        match self {
            Self::Unclipped => Self::Clipped(clip),
            Self::Clipped(parent) => intersect_rect(parent, clip)
                .map(Self::Clipped)
                .unwrap_or(Self::Empty),
            Self::Empty => Self::Empty,
        }
    }

    fn clip_rect(self, rect: UiRect) -> Option<UiRect> {
        if !rect.has_finite_positive_area() {
            return None;
        }
        match self {
            Self::Unclipped => Some(rect),
            Self::Clipped(clip) => intersect_rect(rect, clip),
            Self::Empty => None,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct IndexedOccluder {
    primitive_index: usize,
    rect: UiRect,
    alpha: u8,
}

#[derive(Clone, Copy, Debug, Default)]
struct OcclusionIndexNode {
    bounds: Option<UiRect>,
    max_primitive_index: usize,
    max_alpha: u8,
}

impl OcclusionIndexNode {
    fn leaf(occluder: IndexedOccluder) -> Self {
        Self {
            bounds: Some(occluder.rect),
            max_primitive_index: occluder.primitive_index,
            max_alpha: occluder.alpha,
        }
    }

    fn combine(left: Self, right: Self) -> Self {
        let bounds = match (left.bounds, right.bounds) {
            (Some(left), Some(right)) => Some(left.union(right)),
            (Some(bounds), None) | (None, Some(bounds)) => Some(bounds),
            (None, None) => None,
        };
        Self {
            bounds,
            max_primitive_index: left.max_primitive_index.max(right.max_primitive_index),
            max_alpha: left.max_alpha.max(right.max_alpha),
        }
    }
}

/// Reusable clip snapshots and spatial suffix coverage for one paint plan.
#[derive(Default)]
pub(in crate::gui_runtime::native_vello) struct SurfaceOcclusionPlan {
    clip_states: Vec<SurfaceClipState>,
    clip_stack: Vec<SurfaceClipState>,
    occluders: Vec<IndexedOccluder>,
    index_nodes: Vec<OcclusionIndexNode>,
    leaf_base: usize,
    stats: SurfaceOcclusionPlanStats,
}

impl SurfaceOcclusionPlan {
    pub(in crate::gui_runtime::native_vello) fn preprocess(
        &mut self,
        primitives: &[PaintPrimitive],
    ) {
        self.clip_states.clear();
        self.clip_stack.clear();
        self.occluders.clear();
        self.clip_states.reserve(primitives.len().saturating_add(1));
        self.occluders.reserve(primitives.len());

        let mut clip_state = SurfaceClipState::Unclipped;
        for (primitive_index, primitive) in primitives.iter().enumerate() {
            self.clip_states.push(clip_state);
            match primitive {
                PaintPrimitive::ClipStart(clip) => {
                    clip_state = clip_state.begin(clip.rect);
                    self.clip_stack.push(clip_state);
                }
                PaintPrimitive::ClipEnd(_) => {
                    self.clip_stack.pop();
                    clip_state = self
                        .clip_stack
                        .last()
                        .copied()
                        .unwrap_or(SurfaceClipState::Unclipped);
                }
                PaintPrimitive::FillRect(fill) if fill.color.a >= OPAQUE_SUFFIX_OCCLUSION_ALPHA => {
                    self.push_occluder(primitive_index, fill.rect, fill.color.a, clip_state);
                }
                PaintPrimitive::FillRectBatch(fill)
                    if fill.color.a >= OPAQUE_SUFFIX_OCCLUSION_ALPHA =>
                {
                    for rect in fill.rects.iter().copied() {
                        self.push_occluder(primitive_index, rect, fill.color.a, clip_state);
                    }
                }
                PaintPrimitive::OverlayPanel(panel) => {
                    self.push_occluder(primitive_index, panel.rect, u8::MAX, clip_state);
                }
                _ => {}
            }
        }
        self.clip_states.push(clip_state);
        self.stats = SurfaceOcclusionPlanStats {
            paint_primitives_visited: primitives.len(),
            occluder_rects_indexed: self.occluders.len(),
        };
        self.rebuild_spatial_index();
    }

    pub(in crate::gui_runtime::native_vello) const fn stats(&self) -> SurfaceOcclusionPlanStats {
        self.stats
    }

    fn push_occluder(
        &mut self,
        primitive_index: usize,
        rect: UiRect,
        alpha: u8,
        clip_state: SurfaceClipState,
    ) {
        let Some(rect) = clip_state.clip_rect(rect) else {
            return;
        };
        self.occluders.push(IndexedOccluder {
            primitive_index,
            rect,
            alpha,
        });
    }

    fn rebuild_spatial_index(&mut self) {
        if self.occluders.is_empty() {
            self.leaf_base = 0;
            self.index_nodes.clear();
            return;
        }
        self.leaf_base = self.occluders.len().next_power_of_two();
        let node_count = self.leaf_base.saturating_mul(2);
        self.index_nodes
            .resize(node_count, OcclusionIndexNode::default());
        self.index_nodes.fill(OcclusionIndexNode::default());
        for (offset, occluder) in self.occluders.iter().copied().enumerate() {
            self.index_nodes[self.leaf_base + offset] = OcclusionIndexNode::leaf(occluder);
        }
        for index in (1..self.leaf_base).rev() {
            self.index_nodes[index] = OcclusionIndexNode::combine(
                self.index_nodes[index * 2],
                self.index_nodes[index * 2 + 1],
            );
        }
    }
}

#[derive(Default)]
pub(in crate::gui_runtime::native_vello) struct SurfaceOcclusionQueryScratch {
    node_stack: Vec<usize>,
}

#[cfg(test)]
impl SurfaceOcclusionQueryScratch {
    pub(in crate::gui_runtime::native_vello) fn capacity(&self) -> usize {
        self.node_stack.capacity()
    }
}

pub(in crate::gui_runtime::native_vello) fn planned_surface_occlusion_regions_into(
    surface_rect: UiRect,
    primitive_index: usize,
    plan: &SurfaceOcclusionPlan,
    policy: SurfaceOcclusionPolicy,
    regions: &mut Vec<UiRect>,
    scratch: &mut SurfaceOcclusionQueryScratch,
) -> SurfaceOcclusionQueryStats {
    regions.clear();
    let Some(clip_state) = plan.clip_states.get(primitive_index).copied() else {
        return SurfaceOcclusionQueryStats::default();
    };
    match clip_state {
        SurfaceClipState::Unclipped => {}
        SurfaceClipState::Clipped(clip) => append_rect_outside_clip(surface_rect, clip, regions),
        SurfaceClipState::Empty => regions.push(surface_rect),
    }

    let mut stats = SurfaceOcclusionQueryStats::default();
    scratch.node_stack.clear();
    if plan.leaf_base == 0 {
        return stats;
    }
    scratch.node_stack.push(1);
    while let Some(index) = scratch.node_stack.pop() {
        stats.index_nodes_visited = stats.index_nodes_visited.saturating_add(1);
        let node = plan.index_nodes[index];
        let Some(bounds) = node.bounds else {
            continue;
        };
        if node.max_primitive_index <= primitive_index
            || node.max_alpha < policy.minimum_alpha()
            || !bounds.overlaps(surface_rect)
        {
            continue;
        }
        if index >= plan.leaf_base {
            let occluder_index = index - plan.leaf_base;
            let Some(occluder) = plan.occluders.get(occluder_index).copied() else {
                continue;
            };
            stats.occluder_candidates_visited = stats.occluder_candidates_visited.saturating_add(1);
            if occluder.primitive_index > primitive_index
                && occluder.alpha >= policy.minimum_alpha()
                && let Some(region) = intersect_rect(surface_rect, occluder.rect)
            {
                regions.push(region);
            }
            continue;
        }
        // Push right first so the left subtree is visited first, preserving paint order.
        scratch.node_stack.push(index * 2 + 1);
        scratch.node_stack.push(index * 2);
    }
    stats
}

#[cfg(test)]
fn surface_occlusion_regions_into(
    surface_rect: UiRect,
    prefix: &[PaintPrimitive],
    suffix: &[PaintPrimitive],
    policy: SurfaceOcclusionPolicy,
    regions: &mut Vec<UiRect>,
    legacy_clip_stack: &mut Vec<Option<UiRect>>,
) {
    let primitive_index = prefix.len();
    let mut primitives = Vec::with_capacity(prefix.len() + suffix.len() + 1);
    primitives.extend_from_slice(prefix);
    primitives.push(PaintPrimitive::FillRect(crate::runtime::PaintFillRect {
        widget_id: 0,
        rect: surface_rect,
        color: crate::gui::types::Rgba8 {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        },
    }));
    primitives.extend_from_slice(suffix);
    let mut plan = SurfaceOcclusionPlan::default();
    plan.preprocess(&primitives);
    let mut scratch = SurfaceOcclusionQueryScratch::default();
    planned_surface_occlusion_regions_into(
        surface_rect,
        primitive_index,
        &plan,
        policy,
        regions,
        &mut scratch,
    );
    legacy_clip_stack.clear();
}

#[cfg(test)]
#[path = "surface_occlusion/tests.rs"]
mod tests;
