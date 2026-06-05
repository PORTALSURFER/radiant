use crate::layout::{Point, Rect};

pub(in crate::gui_runtime::native_vello::generic_runtime) fn visible_rects_after_occlusion(
    rect: Rect,
    occlusions: impl IntoIterator<Item = Rect>,
) -> Vec<Rect> {
    let mut visible = Vec::new();
    let mut scratch = Vec::new();
    visible_rects_after_occlusion_into(rect, occlusions, &mut visible, &mut scratch);
    visible
}

pub(in crate::gui_runtime::native_vello::generic_runtime) fn visible_rects_after_occlusion_into(
    rect: Rect,
    occlusions: impl IntoIterator<Item = Rect>,
    visible: &mut Vec<Rect>,
    scratch: &mut Vec<Rect>,
) {
    visible.clear();
    scratch.clear();
    visible.push(rect);
    for occlusion in occlusions {
        scratch.clear();
        for rect in visible.drain(..) {
            subtract_rect(rect, occlusion, scratch);
        }
        std::mem::swap(visible, scratch);
        if visible.is_empty() {
            break;
        }
    }
}

fn subtract_rect(rect: Rect, occlusion: Rect, output: &mut Vec<Rect>) {
    let Some(cut) = intersect_rect(rect, occlusion) else {
        output.push(rect);
        return;
    };

    push_positive_rect(
        output,
        Rect::from_min_max(rect.min, Point::new(rect.max.x, cut.min.y)),
    );
    push_positive_rect(
        output,
        Rect::from_min_max(Point::new(rect.min.x, cut.max.y), rect.max),
    );
    push_positive_rect(
        output,
        Rect::from_min_max(
            Point::new(rect.min.x, cut.min.y),
            Point::new(cut.min.x, cut.max.y),
        ),
    );
    push_positive_rect(
        output,
        Rect::from_min_max(
            Point::new(cut.max.x, cut.min.y),
            Point::new(rect.max.x, cut.max.y),
        ),
    );
}

fn push_positive_rect(output: &mut Vec<Rect>, rect: Rect) {
    if rect.width() > 0.0 && rect.height() > 0.0 {
        output.push(rect);
    }
}

pub(in crate::gui_runtime::native_vello::generic_runtime) fn intersect_rect(
    a: Rect,
    b: Rect,
) -> Option<Rect> {
    let min = Point::new(a.min.x.max(b.min.x), a.min.y.max(b.min.y));
    let max = Point::new(a.max.x.min(b.max.x), a.max.y.min(b.max.y));
    (max.x > min.x && max.y > min.y).then(|| Rect::from_min_max(min, max))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::types::Vector2;

    #[test]
    fn visible_rects_after_occlusion_remove_middle_rect() {
        let rect = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(100.0, 80.0));
        let occlusion = Rect::from_min_size(Point::new(20.0, 15.0), Vector2::new(50.0, 30.0));

        let visible = visible_rects_after_occlusion(rect, [occlusion]);

        assert_eq!(visible.len(), 4);
        assert!(visible.iter().all(|region| region.width() > 0.0));
        assert!(visible.iter().all(|region| region.height() > 0.0));
        assert!(!visible.contains(&occlusion));
    }

    #[test]
    fn intersect_rect_omits_edge_touching_rects() {
        let left = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(10.0, 10.0));
        let touching = Rect::from_min_size(Point::new(10.0, 0.0), Vector2::new(10.0, 10.0));

        assert_eq!(intersect_rect(left, touching), None);
    }

    #[test]
    fn visible_rects_after_occlusion_into_reuses_output_storage() {
        let rect = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(100.0, 80.0));
        let occlusion = Rect::from_min_size(Point::new(20.0, 15.0), Vector2::new(50.0, 30.0));
        let mut visible = Vec::with_capacity(8);
        let mut scratch = Vec::with_capacity(8);
        let visible_capacity = visible.capacity();
        let scratch_capacity = scratch.capacity();

        visible_rects_after_occlusion_into(rect, [occlusion], &mut visible, &mut scratch);

        assert_eq!(visible.len(), 4);
        assert_eq!(visible.capacity(), visible_capacity);
        assert_eq!(scratch.capacity(), scratch_capacity);
    }
}
