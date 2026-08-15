//! Public API coverage for `radiant::layout`.

use radiant::layout::{
    Constraints, ConstraintsParts, ContainerKind, ContainerNodeParts, ContainerPolicy,
    ContainerStateDeclaration, ContainerStateId, CrossAlign, Insets, LayoutContainerStateContext,
    LayoutEngine, LayoutEventContext, LayoutInput, LayoutInteraction, LayoutNode, LayoutState,
    NodeId, Point, Rect, SizeModeCross, SizeModeMain, SlotChild, SlotChildParts, SlotParams,
    SplitPaneAxis, SplitPanePolicy, Vector2, WidgetNodeParts, layout_tree,
};
use std::{cell::Cell, rc::Rc};

#[test]
fn public_layout_module_supports_generic_tree_construction() {
    let root = LayoutNode::container(
        1,
        ContainerPolicy {
            kind: ContainerKind::Column,
            padding: Insets::all(4.0),
            ..ContainerPolicy::default()
        },
        vec![
            SlotChild::new(
                SlotParams {
                    size_main: SizeModeMain::Fixed(20.0),
                    size_cross: SizeModeCross::Fill,
                    constraints: Constraints::from_parts(ConstraintsParts {
                        min_w: 0.0,
                        max_w: 200.0,
                        min_h: 20.0,
                        max_h: 20.0,
                    }),
                    margin: Insets::default(),
                    align_cross_override: Some(CrossAlign::Stretch),
                    allow_fixed_compress: false,
                },
                LayoutNode::widget(2, Vector2::new(40.0, 20.0)),
            ),
            SlotChild::new(
                SlotParams::fill(),
                LayoutNode::widget(3, Vector2::new(60.0, 30.0)),
            ),
        ],
    );

    let root_rect = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 90.0));
    let one_shot = layout_tree(&root, root_rect);
    assert_eq!(
        one_shot.rect_for(99, root_rect.empty_at_min()),
        root_rect.empty_at_min()
    );
    assert_eq!(
        one_shot.rect_for_clamped(2, root_rect.empty_at_min(), root_rect),
        Rect::from_min_max(Point::new(4.0, 4.0), Point::new(116.0, 24.0))
    );

    let mut engine = LayoutEngine::default();
    let stateful = engine.layout_with_state(
        &root,
        root_rect,
        &LayoutState::default(),
        Default::default(),
    );

    assert_eq!(one_shot.rects, stateful.rects);
    assert!(one_shot.rects.contains_key(&2));
    assert!(one_shot.rects.contains_key(&3));
}

#[test]
fn public_layout_tree_nodes_support_named_parts_construction() {
    let root = LayoutNode::container_from_parts(ContainerNodeParts {
        id: 10,
        policy: ContainerPolicy {
            kind: ContainerKind::Row,
            spacing: 6.0,
            ..ContainerPolicy::default()
        },
        children: vec![
            SlotChild::from_parts(SlotChildParts {
                slot: SlotParams::fill(),
                child: LayoutNode::widget_from_parts(WidgetNodeParts {
                    id: 11,
                    intrinsic: Vector2::new(24.0, 12.0),
                }),
            }),
            SlotChild::from_parts(SlotChildParts {
                slot: SlotParams::fill(),
                child: LayoutNode::widget_from_parts(WidgetNodeParts {
                    id: 12,
                    intrinsic: Vector2::new(36.0, 12.0),
                }),
            }),
        ],
    });

    assert_eq!(root.id(), 10);
    let output = layout_tree(
        &root,
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 30.0)),
    );

    assert!(output.rects.contains_key(&11));
    assert!(output.rects.contains_key(&12));
}

#[test]
fn public_layout_module_exposes_exact_static_split_geometry() {
    let root = LayoutNode::container(
        20,
        ContainerPolicy {
            kind: ContainerKind::SplitPane,
            split_pane: SplitPanePolicy {
                axis: SplitPaneAxis::Horizontal,
                initial_ratio: 0.25,
                divider_extent: 8.0,
                first_min_extent: 24.0,
                second_min_extent: 32.0,
            },
            ..ContainerPolicy::default()
        },
        vec![
            SlotChild::new(
                SlotParams::fill(),
                LayoutNode::widget(21, Vector2::new(8.0, 8.0)),
            ),
            SlotChild::new(
                SlotParams::fill(),
                LayoutNode::widget(22, Vector2::new(8.0, 8.0)),
            ),
        ],
    );
    let output = layout_tree(
        &root,
        Rect::from_min_size(Point::new(4.0, 6.0), Vector2::new(120.0, 40.0)),
    );

    assert_eq!(output.rects[&21].width(), 28.0);
    assert_eq!(output.rects[&21].max.x, 32.0);
    assert_eq!(output.rects[&22].min.x, 40.0);
    assert_eq!(output.rects[&22].width(), 84.0);
    assert!(output.diagnostics.iter().all(|diagnostic| diagnostic.code
        != radiant::layout::LayoutDiagnosticCode::SplitPaneChildCountMismatch));
}

struct LocalStateInteraction {
    initialized: Rc<Cell<u32>>,
}

impl LayoutInteraction<()> for LocalStateInteraction {
    fn state(&self, container_id: NodeId) -> Option<ContainerStateDeclaration> {
        let initialized = Rc::clone(&self.initialized);
        Some(ContainerStateDeclaration::new::<Rc<Cell<u32>>, _>(
            container_id,
            7,
            move || {
                initialized.set(initialized.get() + 1);
                Rc::new(Cell::new(0))
            },
        ))
    }

    fn handle_layout_input_with_state(
        &self,
        _input: LayoutInput,
        _context: &mut LayoutEventContext<()>,
        state: &mut LayoutContainerStateContext<'_>,
    ) {
        let value = state
            .state_mut::<Rc<Cell<u32>>>()
            .expect("the runtime supplies the declared concrete state type");
        value.set(value.get() + 1);
    }
}

#[test]
fn public_layout_state_api_keeps_type_identity_opaque_and_typed() {
    let declaration =
        ContainerStateDeclaration::new::<Rc<Cell<u32>>, _>(41, 7, || Rc::new(Cell::new(0)));
    let id = declaration.id();
    assert_eq!(id.container_id(), 41);
    assert_eq!(id.schema_version(), 7);
    assert!(id.is::<Rc<Cell<u32>>>());
    assert!(!id.is::<Rc<Cell<u64>>>());
    assert_eq!(ContainerStateId::new::<Rc<Cell<u32>>>(41, 7), id);

    let interaction = LocalStateInteraction {
        initialized: Rc::new(Cell::new(0)),
    };
    assert!(interaction.state(41).is_some());
}
