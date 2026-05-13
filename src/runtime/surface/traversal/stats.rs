use crate::{layout::ContainerKind, runtime::SurfaceNode};

#[derive(Clone, Copy, Debug, Default)]
pub(in crate::runtime) struct SurfaceTraversalStats {
    pub(in crate::runtime) widgets: usize,
    pub(in crate::runtime) stateful_widgets: usize,
    pub(in crate::runtime) scroll_containers: usize,
    pub(in crate::runtime) clipped_containers: usize,
    pub(in crate::runtime) styled_hoverable_containers: usize,
    pub(in crate::runtime) max_depth: usize,
    pub(in crate::runtime) max_scroll_depth: usize,
}

impl<Message> SurfaceNode<Message> {
    pub(in crate::runtime) fn runtime_traversal_stats(&self) -> SurfaceTraversalStats {
        let mut stats = SurfaceTraversalStats::default();
        self.collect_runtime_traversal_stats(0, 0, &mut stats);
        stats
    }

    fn collect_runtime_traversal_stats(
        &self,
        depth: usize,
        scroll_depth: usize,
        stats: &mut SurfaceTraversalStats,
    ) {
        stats.max_depth = stats.max_depth.max(depth);
        stats.max_scroll_depth = stats.max_scroll_depth.max(scroll_depth);
        match self {
            Self::Container(container) => {
                let is_scroll = container.policy.kind == ContainerKind::ScrollView;
                if scroll_depth > 0 {
                    stats.clipped_containers += 1;
                }
                if is_scroll {
                    stats.scroll_containers += 1;
                }
                if container.style.is_some() && container.hoverable {
                    stats.styled_hoverable_containers += 1;
                }
                let child_scroll_depth = scroll_depth + usize::from(is_scroll);
                for child in &container.children {
                    child.child.collect_runtime_traversal_stats(
                        depth + 1,
                        child_scroll_depth,
                        stats,
                    );
                }
            }
            Self::Widget(widget) => {
                stats.widgets += 1;
                if widget.needs_state_synchronization() {
                    stats.stateful_widgets += 1;
                }
            }
            Self::Overlay(_) => {}
        }
    }
}
