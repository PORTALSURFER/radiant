use super::Constraints;

#[test]
fn constraints_normalize_invalid_ranges() {
    let normalized = Constraints::new(-10.0, 4.0, 8.0, 2.0);
    assert_eq!(normalized.min_w, 0.0);
    assert_eq!(normalized.max_w, 4.0);
    assert_eq!(normalized.min_h, 8.0);
    assert_eq!(normalized.max_h, 8.0);
}

#[test]
fn direct_constraints_do_not_retain_nonfinite_requested_maxima() {
    for maximum in [f32::NAN, f32::NEG_INFINITY] {
        let normalized = Constraints {
            min_w: 4.0,
            max_w: maximum,
            min_h: 6.0,
            max_h: maximum,
        }
        .normalized();

        assert_eq!(normalized.max_w, normalized.min_w);
        assert_eq!(normalized.max_h, normalized.min_h);
        assert!(normalized.max_w.is_finite());
        assert!(normalized.max_h.is_finite());
    }
}

#[test]
fn inset_saturates_overflowing_negative_finite_insets() {
    let inset = Constraints {
        min_w: 0.0,
        max_w: 100.0,
        min_h: 0.0,
        max_h: 80.0,
    }
    .inset(-f32::MAX, -f32::MAX);

    assert_eq!(inset.max_w, f32::MAX);
    assert_eq!(inset.max_h, f32::MAX);
    assert!(inset.max_w.is_finite());
    assert!(inset.max_h.is_finite());
}

#[test]
fn inset_preserves_explicit_unbounded_max_for_overflowing_negative_insets() {
    let inset = Constraints {
        min_w: 0.0,
        max_w: f32::INFINITY,
        min_h: 0.0,
        max_h: f32::INFINITY,
    }
    .inset(-f32::MAX, -f32::MAX);

    assert_eq!(inset.max_w, f32::INFINITY);
    assert_eq!(inset.max_h, f32::INFINITY);
}

#[test]
fn direct_range_clamping_normalizes_invalid_bounds() {
    for maximum in [f32::NAN, f32::NEG_INFINITY, -1.0, 2.0] {
        let constraints = Constraints {
            min_w: 4.0,
            max_w: maximum,
            min_h: 6.0,
            max_h: maximum,
        };
        assert_eq!(constraints.clamp_w(50.0), 4.0);
        assert_eq!(constraints.clamp_h(50.0), 6.0);
    }
    let constraints = Constraints {
        min_w: f32::NAN,
        max_w: f32::INFINITY,
        min_h: f32::NEG_INFINITY,
        max_h: 30.0,
    };
    assert_eq!(constraints.clamp_w(50.0), 50.0);
    assert_eq!(constraints.clamp_h(50.0), 30.0);
}

#[test]
fn invalid_fill_slot_limits_do_not_panic_during_construction_or_layout() {
    use super::super::{
        engine::LayoutEngine,
        model::{ContainerPolicy, CrossAlign, SlotParams},
        tree::{LayoutNode, SlotChild},
    };
    use crate::gui::types::{Point, Rect, Vector2};
    for maximum in [f32::NAN, f32::NEG_INFINITY] {
        let root = LayoutNode::container(
            1,
            ContainerPolicy {
                align_cross: CrossAlign::Start,
                ..Default::default()
            },
            vec![SlotChild::new(
                SlotParams {
                    constraints: Constraints {
                        min_w: 0.0,
                        max_w: maximum,
                        min_h: 0.0,
                        max_h: maximum,
                    },
                    ..SlotParams::fill()
                },
                LayoutNode::widget(2, Vector2::new(40.0, 20.0)),
            )],
        );
        let output = LayoutEngine::default().layout(
            &root,
            Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(100.0, 100.0)),
        );
        assert_eq!(output.rects[&2].width(), 0.0);
        assert_eq!(output.rects[&2].height(), 0.0);
        assert!(
            output.diagnostics.iter().any(|item| item.node_id == 2
                && item.message == "max width was non-finite and was clamped")
        );
        assert!(
            output.diagnostics.iter().any(|item| item.node_id == 2
                && item.message == "max height was non-finite and was clamped")
        );
    }
}
