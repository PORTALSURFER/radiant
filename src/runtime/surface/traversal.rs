#[cfg(test)]
use super::SourceTraversalIndex;
use super::UiSurface;

mod index;
mod stats;

pub(in crate::runtime) use index::{
    SurfaceContainerTraversalRecord, SurfaceLayoutInteractionRecord,
    SurfaceSplitPaneFocusOrderCandidate, SurfaceSplitPaneRatioActionCandidate,
    SurfaceTraversalIndex, SurfaceWidgetTraversalRecord, WheelHitTarget,
};
pub(in crate::runtime) use stats::SurfaceTraversalStats;

impl<Message> UiSurface<Message> {
    #[cfg(test)]
    pub(in crate::runtime) fn runtime_traversal_index(&self) -> SurfaceTraversalIndex<Message> {
        let stats = self.root.runtime_traversal_stats();
        let mut index = SurfaceTraversalIndex::with_stats(stats);
        self.root.project_runtime_index(
            &mut Vec::with_capacity(stats.max_scroll_depth),
            &mut Vec::with_capacity(stats.max_depth),
            &mut index,
        );
        index
    }

    #[cfg(test)]
    pub(in crate::runtime) fn runtime_source_traversal_index(&self) -> SourceTraversalIndex {
        let stats = self.root.runtime_traversal_stats();
        let mut traversal = SurfaceTraversalIndex::with_stats(stats);
        let mut source = SourceTraversalIndex::with_stats(stats);
        let environment = self.resolved_environment();
        self.runtime_projection_into_with_source(&mut traversal, stats, &mut source, &environment);
        source
    }
}

#[cfg(test)]
#[path = "traversal/tests.rs"]
mod tests;
