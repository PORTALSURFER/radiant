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

fn additional_reserve_for_capacity(
    current_len: usize,
    current_capacity: usize,
    desired_capacity: usize,
) -> usize {
    if desired_capacity > current_capacity {
        desired_capacity.saturating_sub(current_len)
    } else {
        0
    }
}

pub(in crate::runtime) fn reserve_vec_capacity<T>(values: &mut Vec<T>, desired_capacity: usize) {
    let additional =
        additional_reserve_for_capacity(values.len(), values.capacity(), desired_capacity);
    if additional > 0 {
        values.reserve(additional);
    }
}

pub(in crate::runtime) fn reserve_map_capacity<K, V>(
    values: &mut HashMap<K, V>,
    desired_capacity: usize,
) where
    K: Eq + Hash,
{
    let additional =
        additional_reserve_for_capacity(values.len(), values.capacity(), desired_capacity);
    if additional > 0 {
        values.reserve(additional);
    }
}

pub(in crate::runtime) fn reserve_set_capacity<T>(values: &mut HashSet<T>, desired_capacity: usize)
where
    T: Eq + Hash,
{
    let additional =
        additional_reserve_for_capacity(values.len(), values.capacity(), desired_capacity);
    if additional > 0 {
        values.reserve(additional);
    }
}
