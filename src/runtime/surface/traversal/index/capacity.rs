use super::super::SurfaceTraversalStats;
use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
};

#[cfg(test)]
#[path = "capacity/tests.rs"]
mod tests;

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
