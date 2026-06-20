use super::UiSurface;

mod index;
mod stats;

pub(in crate::runtime) use index::{
    SurfaceContainerTraversalRecord, SurfaceTraversalIndex, SurfaceWidgetTraversalRecord,
    WheelHitTarget,
};
pub(in crate::runtime) use stats::SurfaceTraversalStats;

impl<Message> UiSurface<Message> {
    #[cfg(test)]
    pub(in crate::runtime) fn runtime_traversal_index(&self) -> SurfaceTraversalIndex {
        let stats = self.root.runtime_traversal_stats();
        let mut index = SurfaceTraversalIndex::with_stats(stats);
        self.root.project_runtime_index(
            &mut Vec::with_capacity(stats.max_scroll_depth),
            &mut Vec::with_capacity(stats.max_depth),
            &mut index,
        );
        index
    }
}

#[cfg(test)]
#[path = "traversal/tests.rs"]
mod tests;
