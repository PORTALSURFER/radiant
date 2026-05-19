use super::super::SurfaceTraversalStats;
use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
};

pub(in crate::runtime) fn widget_clip_capacity(stats: SurfaceTraversalStats) -> usize {
    if stats.scroll_containers == 0 {
        0
    } else {
        stats.widgets
    }
}

pub(in crate::runtime) fn reserve_vec_capacity<T>(values: &mut Vec<T>, desired_capacity: usize) {
    if desired_capacity > values.capacity() {
        values.reserve(desired_capacity);
    }
}

pub(in crate::runtime) fn reserve_map_capacity<K, V>(
    values: &mut HashMap<K, V>,
    desired_capacity: usize,
) where
    K: Eq + Hash,
{
    if desired_capacity > values.capacity() {
        values.reserve(desired_capacity);
    }
}

pub(in crate::runtime) fn reserve_set_capacity<T>(values: &mut HashSet<T>, desired_capacity: usize)
where
    T: Eq + Hash,
{
    if desired_capacity > values.capacity() {
        values.reserve(desired_capacity);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn widget_clip_capacity_is_zero_without_scroll_containers() {
        assert_eq!(
            widget_clip_capacity(SurfaceTraversalStats {
                widgets: 8,
                scroll_containers: 0,
                ..SurfaceTraversalStats::default()
            }),
            0
        );
    }

    #[test]
    fn widget_clip_capacity_tracks_widgets_when_scroll_containers_exist() {
        assert_eq!(
            widget_clip_capacity(SurfaceTraversalStats {
                widgets: 8,
                scroll_containers: 1,
                ..SurfaceTraversalStats::default()
            }),
            8
        );
    }
}
