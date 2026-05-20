use super::*;
use crate::layout::OverflowInfo;

#[test]
fn scroll_affordance_clamps_thumb_to_cramped_track() {
    let mut layout = LayoutOutput::default();
    layout.rects.insert(
        1,
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 20.0)),
    );
    layout.rects.insert(
        2,
        Rect::from_min_size(Point::new(0.0, -20.0), Vector2::new(120.0, 80.0)),
    );
    layout.overflow_flags.insert(
        1,
        OverflowInfo {
            x: false,
            y: true,
            policy: OverflowPolicy::Scroll,
        },
    );

    let affordance = resolve_scroll_affordance(1, 2, &layout)
        .expect("cramped overflowing scroll area should still resolve a thumb");

    assert_eq!(affordance.track.height(), 8.0);
    assert_eq!(affordance.thumb.height(), affordance.track.height());
    assert_eq!(affordance.thumb.min.y, affordance.track.min.y);
    assert_eq!(affordance.thumb.max.y, affordance.track.max.y);
}

#[test]
fn scroll_affordance_omits_degenerate_track() {
    let mut layout = LayoutOutput::default();
    layout.rects.insert(
        1,
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 12.0)),
    );
    layout.rects.insert(
        2,
        Rect::from_min_size(Point::new(0.0, -20.0), Vector2::new(120.0, 80.0)),
    );
    layout.overflow_flags.insert(
        1,
        OverflowInfo {
            x: false,
            y: true,
            policy: OverflowPolicy::Scroll,
        },
    );

    assert_eq!(resolve_scroll_affordance(1, 2, &layout), None);
}

#[test]
fn scroll_affordance_rejects_nonfinite_layout_rects() {
    let mut layout = LayoutOutput::default();
    layout.rects.insert(
        1,
        Rect::from_min_max(Point::new(0.0, 0.0), Point::new(f32::NAN, 80.0)),
    );
    layout.rects.insert(
        2,
        Rect::from_min_size(Point::new(0.0, -20.0), Vector2::new(120.0, 160.0)),
    );
    layout.overflow_flags.insert(
        1,
        OverflowInfo {
            x: false,
            y: true,
            policy: OverflowPolicy::Scroll,
        },
    );

    assert_eq!(resolve_scroll_affordance(1, 2, &layout), None);

    layout.rects.insert(
        1,
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 80.0)),
    );
    layout.rects.insert(
        2,
        Rect::from_min_max(Point::new(0.0, f32::INFINITY), Point::new(120.0, 160.0)),
    );

    assert_eq!(resolve_scroll_affordance(1, 2, &layout), None);
}
