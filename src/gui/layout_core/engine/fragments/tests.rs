use super::*;
use crate::gui::layout_core::{engine::LayoutEngine, tree::SlotChild};
use crate::gui::types::{Point, Vector2};

fn slot() -> SlotParams {
    use crate::gui::layout_core::{
        constraints::Constraints,
        model::{SizeModeCross, SizeModeMain},
    };
    SlotParams {
        size_main: SizeModeMain::Intrinsic,
        size_cross: SizeModeCross::Intrinsic,
        constraints: Constraints::new(0.0, 300.0, 0.0, 300.0),
        ..SlotParams::fill()
    }
}

fn tree(changed: bool) -> LayoutNode {
    tree_columns(changed, 3)
}

fn tree_columns(changed: bool, columns: u64) -> LayoutNode {
    LayoutNode::container(
        1,
        ContainerPolicy {
            kind: ContainerKind::Row,
            ..Default::default()
        },
        (0..columns)
            .map(|column| {
                SlotChild::new(
                    slot(),
                    LayoutNode::container(
                        10 + column,
                        ContainerPolicy::default(),
                        (0..20)
                            .map(|index| {
                                SlotChild::new(
                                    slot(),
                                    LayoutNode::widget(
                                        100 + column * 100 + index,
                                        Vector2::new(
                                            40.0,
                                            if column == 0 && index == 0 && changed {
                                                11.0
                                            } else {
                                                10.0
                                            },
                                        ),
                                    ),
                                )
                            })
                            .collect(),
                    ),
                )
            })
            .collect(),
    )
}

#[test]
fn engine_remains_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<LayoutEngine>();
}

#[test]
fn unchanged_geometry_matches_uncached_output_with_fewer_layout_calls() {
    let mut cached = LayoutEngine::with_static_geometry_fragments();
    let mut fresh = LayoutEngine::default();
    let rect = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(300.0, 300.0));
    let mut before = LayoutOutput::default();
    cached.layout_with_state_into(
        &tree(false),
        rect,
        &Default::default(),
        Default::default(),
        &mut before,
    );
    fresh.layout_with_state_into(
        &tree(false),
        rect,
        &Default::default(),
        Default::default(),
        &mut before,
    );
    let mut actual = LayoutOutput::default();
    let mut expected = LayoutOutput::default();
    cached.layout_with_state_into(
        &tree(true),
        rect,
        &Default::default(),
        Default::default(),
        &mut actual,
    );
    fresh.layout_with_state_into(
        &tree(true),
        rect,
        &Default::default(),
        Default::default(),
        &mut expected,
    );
    assert_eq!(actual.rects, expected.rects);
    assert_eq!(actual.diagnostics, expected.diagnostics);
    assert_eq!(actual.overflow_flags, expected.overflow_flags);
    assert_eq!(
        actual.stats.materialized_nodes,
        expected.stats.materialized_nodes
    );
    assert!(actual.stats.laid_out_nodes < expected.stats.laid_out_nodes);
}

fn viewport() -> Rect {
    Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(300.0, 300.0))
}

fn run(engine: &mut LayoutEngine, root: &LayoutNode) -> LayoutOutput {
    let mut output = LayoutOutput::default();
    engine.layout_with_state_into(
        root,
        viewport(),
        &Default::default(),
        Default::default(),
        &mut output,
    );
    output
}

#[test]
fn exact_inputs_invalidate_without_relying_on_versions() {
    let mut engine = LayoutEngine::with_static_geometry_fragments();
    let root = tree(false);
    run(&mut engine, &root);
    let fragment = engine
        .fragments
        .entries
        .get(&10)
        .expect("plain column retained");
    let LayoutNode::Container(parent) = &root else {
        unreachable!()
    };
    let column = &parent.children[0].child;
    let rect = fragment.nodes[0].1;
    assert!(fragment.matches(column, rect, WritingDirection::Ltr));
    let mut changed = column.clone();
    let LayoutNode::Container(container) = &mut changed else {
        unreachable!()
    };
    let LayoutNode::Widget(widget) = &mut container.children[0].child else {
        unreachable!()
    };
    widget.intrinsic.y += 1.0;
    assert_eq!(changed.state_version(), column.state_version());
    assert!(!fragment.matches(&changed, rect, WritingDirection::Ltr));
    let mut changed = column.clone();
    let LayoutNode::Container(container) = &mut changed else {
        unreachable!()
    };
    container.children[0].slot.margin.left = 1.0;
    assert!(!fragment.matches(&changed, rect, WritingDirection::Ltr));
    let mut changed = column.clone();
    let LayoutNode::Container(container) = &mut changed else {
        unreachable!()
    };
    container.policy.spacing = 1.0;
    assert!(!fragment.matches(&changed, rect, WritingDirection::Ltr));
    assert!(!fragment.matches(column, rect, WritingDirection::Rtl));
    let moved = Rect::from_min_size(
        Point::new(1.0, 0.0),
        Vector2::new(rect.width(), rect.height()),
    );
    assert!(!fragment.matches(column, moved, WritingDirection::Ltr));
}

#[test]
fn dirty_debug_duplicate_ids_and_removal_retire_reuse() {
    let root = tree(false);
    let mut engine = LayoutEngine::with_static_geometry_fragments();
    run(&mut engine, &root);
    assert_eq!(engine.fragments.entries.len(), 3);
    engine.mark_layout_dirty(100);
    assert_eq!(run(&mut engine, &root).stats.laid_out_nodes, 64);
    assert!(engine.fragments.entries.is_empty());
    run(&mut engine, &root);
    let mut output = LayoutOutput::default();
    engine.layout_with_state_into(
        &root,
        viewport(),
        &Default::default(),
        LayoutDebugOptions::bounds_only(),
        &mut output,
    );
    assert_eq!(output.stats.laid_out_nodes, 64);
    assert!(!output.debug_primitives.is_empty());
    assert!(engine.fragments.entries.is_empty());
    run(&mut engine, &root);
    let mut duplicate = root.clone();
    let LayoutNode::Container(parent) = &mut duplicate else {
        unreachable!()
    };
    parent.children.push(parent.children[0].clone());
    run(&mut engine, &duplicate);
    assert!(engine.fragments.entries.is_empty());
    run(&mut engine, &root);
    run(
        &mut engine,
        &LayoutNode::widget(900, Vector2::new(10.0, 10.0)),
    );
    assert!(engine.fragments.entries.is_empty());
    assert_eq!(engine.fragments.retained_nodes, 0);
}

#[test]
fn discarded_candidate_does_not_publish_geometry() {
    use crate::gui::layout_core::engine::{LayoutAuthorityEvidence, LayoutInputEvidence};
    let root = tree(false);
    let mut engine = LayoutEngine::with_static_geometry_fragments();
    let before = run(&mut engine, &root);
    let retained = Arc::clone(engine.fragments.entries.get(&10).unwrap());
    let evidence = LayoutInputEvidence::new(
        Some(LayoutAuthorityEvidence::new(1, 1)),
        Some(LayoutAuthorityEvidence::new(1, 1)),
        None,
        viewport(),
        Default::default(),
    );
    let prepared = engine.prepare_layout_with_state(
        &tree(true),
        viewport(),
        &Default::default(),
        Default::default(),
        evidence,
    );
    assert!(Arc::ptr_eq(
        &retained,
        engine.fragments.entries.get(&10).unwrap()
    ));
    prepared.discard();
    let actual = run(&mut engine, &root);
    assert_eq!(actual.rects, before.rects);
    assert_eq!(actual.stats.laid_out_nodes, 1);
}

#[test]
fn prepared_commit_publishes_only_current_geometry() {
    use crate::gui::layout_core::engine::{LayoutAuthorityEvidence, LayoutInputEvidence};
    let mut engine = LayoutEngine::with_static_geometry_fragments();
    let before = run(&mut engine, &tree(false));
    let evidence = LayoutInputEvidence::new(
        Some(LayoutAuthorityEvidence::new(1, 1)),
        Some(LayoutAuthorityEvidence::new(1, 1)),
        None,
        viewport(),
        Default::default(),
    );
    let prepared = engine.prepare_layout_with_state(
        &tree(true),
        viewport(),
        &Default::default(),
        Default::default(),
        evidence,
    );
    assert!(prepared.is_usable());
    assert_ne!(prepared.output().unwrap().rects[&100], before.rects[&100]);
    let mut published = before.clone();
    prepared
        .commit(&mut engine, &mut published, evidence)
        .unwrap();
    assert_ne!(published.rects[&100], before.rects[&100]);
    assert_eq!(run(&mut engine, &tree(true)).stats.laid_out_nodes, 1);
    let stale = engine.prepare_layout_with_state(
        &tree(false),
        viewport(),
        &Default::default(),
        Default::default(),
        evidence,
    );
    engine.mark_layout_dirty(100);
    let retained = Arc::clone(engine.fragments.entries.get(&10).unwrap());
    let prior = published.clone();
    assert!(stale.commit(&mut engine, &mut published, evidence).is_err());
    assert_eq!(published, prior);
    assert!(Arc::ptr_eq(
        &retained,
        engine.fragments.entries.get(&10).unwrap()
    ));
}

#[test]
fn invalid_or_nonplain_output_is_not_retained() {
    let root = tree(false);
    let mut engine = LayoutEngine::default();
    let output = run(&mut engine, &root);
    let LayoutNode::Container(parent) = &root else {
        unreachable!()
    };
    let column = &parent.children[0].child;
    let rect = output.rects[&10];
    assert!(Fragment::capture(column, rect, WritingDirection::Ltr, &output).is_some());
    let mut invalid = output.clone();
    invalid.record_omitted_node(100);
    assert!(Fragment::capture(column, rect, WritingDirection::Ltr, &invalid).is_none());
    let mut invalid = output.clone();
    invalid.overflowed.insert(100);
    assert!(Fragment::capture(column, rect, WritingDirection::Ltr, &invalid).is_none());
    let mut invalid = output.clone();
    invalid.viewport_bounds.insert(100, rect);
    assert!(Fragment::capture(column, rect, WritingDirection::Ltr, &invalid).is_none());
    let mut nonplain = column.clone();
    let LayoutNode::Container(container) = &mut nonplain else {
        unreachable!()
    };
    container.policy.kind = ContainerKind::Grid;
    assert!(Fragment::capture(&nonplain, rect, WritingDirection::Ltr, &output).is_none());
    assert!(Fragment::capture(&root, viewport(), WritingDirection::Ltr, &output).is_none());
    let mut oversized = column.clone();
    let LayoutNode::Container(container) = &mut oversized else {
        unreachable!()
    };
    container
        .children
        .resize(MAX_FRAGMENT_NODES, container.children[0].clone());
    assert!(Fragment::capture(&oversized, rect, WritingDirection::Ltr, &output).is_none());
}

#[test]
fn retained_fragment_capacity_is_bounded_and_unused_entries_retire() {
    let root = tree_columns(false, 70);
    let mut engine = LayoutEngine::with_static_geometry_fragments();
    let mut output = LayoutOutput::default();
    engine.layout_with_state_into(
        &root,
        Rect::from_min_size(Point::default(), Vector2::new(3000.0, 300.0)),
        &Default::default(),
        Default::default(),
        &mut output,
    );
    assert_eq!(engine.fragments.entries.len(), MAX_FRAGMENTS);
    assert!(engine.fragments.retained_nodes <= MAX_RETAINED_NODES);
    assert!(engine.fragments.retained_events <= MAX_RETAINED_EVENTS);
    run(&mut engine, &tree(false));
    assert_eq!(engine.fragments.entries.len(), 3);
    assert_eq!(engine.fragments.retained_nodes, 63);
}
