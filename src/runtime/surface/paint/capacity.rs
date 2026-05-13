use crate::layout::LayoutOutput;

const MAX_INITIAL_PAINT_PRIMITIVE_CAPACITY: usize = 1024;

pub(in crate::runtime) fn estimated_paint_primitive_capacity(layout: &LayoutOutput) -> usize {
    layout
        .rects
        .len()
        .saturating_mul(3)
        .min(MAX_INITIAL_PAINT_PRIMITIVE_CAPACITY)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estimated_paint_primitive_capacity_scales_for_small_layouts() {
        let mut layout = LayoutOutput::default();
        for node_id in 0..16 {
            layout.rects.insert(node_id, Default::default());
        }

        assert_eq!(estimated_paint_primitive_capacity(&layout), 48);
    }

    #[test]
    fn estimated_paint_primitive_capacity_caps_large_initial_reserves() {
        let mut layout = LayoutOutput::default();
        for node_id in 0..10_000 {
            layout.rects.insert(node_id, Default::default());
        }

        assert_eq!(estimated_paint_primitive_capacity(&layout), 1024);
    }
}
