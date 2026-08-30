#![allow(dead_code)]

use super::{ViewNode, ViewNodeKind, lowering::ViewLowering};
use crate::{
    application::{
        ROOT_KEY_SCOPE,
        ids::{IdGenerator, StructuralRole},
        launch::SceneProjection,
    },
    layout::{ContainerKind, ContainerPolicy, NodeId},
    runtime::SurfaceNode,
};
use std::{
    collections::HashSet,
    panic::{AssertUnwindSafe, panic_any},
};

/// Typed failures from the private virtual-layout item admission boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum VirtualLayoutViewAdmissionError {
    /// The item supplied an explicit numeric identity.
    ExplicitIdentity,
    /// The item retained a pre-lowered runtime surface node.
    DirectSurfaceNode,
    /// The item supplied a scene, overlay, or other out-of-band effect.
    UnsupportedSceneEffects,
    /// The shell directly supplied another virtual-layout registration.
    NestedVirtualLayout,
    /// The admitted identity set contained an ambiguous key or id.
    IdentityCollision,
    /// Lowering or user widget construction unwound.
    LoweringPanicked,
}

/// Pure payloads produced by one complete retained virtual-layout batch.
///
/// The batch is lowered with one identity admission pass.  Keeping the
/// payload private prevents callers from treating this prerequisite as a
/// runtime registration API.
pub(crate) struct VirtualLayoutViewBatch<Message> {
    pub(crate) shell: SurfaceNode<Message>,
    pub(crate) items: Vec<SurfaceNode<Message>>,
}

/// Lower one virtual-layout item beneath its private slot wrapper.
///
/// This is a pure, crate-private prerequisite for the later batch adapter. The
/// wrapper owns the identity scope derived from the complete slot tuple; the
/// item itself cannot bring an explicit runtime id or scene effect into the
/// retained tree.
pub(crate) fn lower_virtual_layout_item<Message: 'static>(
    node: ViewNode<Message>,
    container_id: NodeId,
    mount_generation: u64,
    slot_index: usize,
    checked_generation: u64,
) -> Result<SurfaceNode<Message>, VirtualLayoutViewAdmissionError> {
    validate_item(&node)?;
    let node = guard_widget_views(node);

    let wrapper_id = slot_wrapper_id(
        container_id,
        mount_generation,
        slot_index,
        checked_generation,
    );
    let has_reserved_descendant_identity = node.has_reserved_identity_in_subtree();
    let wrapper = ViewNode::new(ViewNodeKind::Container {
        policy: ContainerPolicy {
            kind: ContainerKind::Stack,
            ..ContainerPolicy::default()
        },
        children: vec![node],
    })
    .with_reserved_descendant_identity(has_reserved_descendant_identity)
    .id(wrapper_id);

    let lowered = std::panic::catch_unwind(AssertUnwindSafe(|| {
        let mut keyed_candidates = HashSet::new();
        let mut continuity_keys = HashSet::new();
        let mut explicit_ids = HashSet::new();
        if wrapper
            .collect_keyed_collisions(ROOT_KEY_SCOPE, &mut keyed_candidates)
            .is_err()
            || wrapper
                .collect_explicit_identity_collisions(
                    ROOT_KEY_SCOPE,
                    &mut continuity_keys,
                    &mut explicit_ids,
                )
                .is_err()
        {
            return Err(VirtualLayoutViewAdmissionError::IdentityCollision);
        }

        let mut reserved = Vec::new();
        wrapper.collect_reserved_ids(ROOT_KEY_SCOPE, &mut reserved);
        let mut ids = IdGenerator::new(reserved);
        let mut scene = SceneProjection::default();
        let mut lowering = ViewLowering::new(&mut ids, &mut scene);
        Ok(lowering.lower_node(wrapper, ROOT_KEY_SCOPE, StructuralRole::Root))
    }));

    match lowered {
        Ok(result) => result,
        Err(payload) => match payload.downcast::<WidgetViewAdmissionPanic>() {
            Ok(panic) => Err(panic.0),
            Err(_) => Err(VirtualLayoutViewAdmissionError::LoweringPanicked),
        },
    }
}

/// Lower a complete shell plus active retained item batch atomically.
///
/// Every item is wrapped below the exact slot tuple supplied by the private
/// materialization boundary.  The synthetic admission root is used only for
/// shared identity/capacity preflight; each admitted child is then lowered
/// with one generator and scene projection so generated descendants cannot
/// collide across the shell and active items.
pub(crate) fn lower_virtual_layout_batch<Message: 'static>(
    mut shell: ViewNode<Message>,
    container_id: NodeId,
    items: Vec<(ViewNode<Message>, u64, usize, u64)>,
) -> Result<VirtualLayoutViewBatch<Message>, VirtualLayoutViewAdmissionError> {
    if matches!(&shell.kind, ViewNodeKind::VirtualLayout(_)) {
        return Err(VirtualLayoutViewAdmissionError::NestedVirtualLayout);
    }
    if shell.id.is_some_and(|id| id != container_id) {
        return Err(VirtualLayoutViewAdmissionError::IdentityCollision);
    }
    shell = shell.id(container_id);

    let mut wrapped_items = Vec::with_capacity(items.len());
    for (node, mount_generation, slot_index, checked_generation) in items {
        validate_item(&node)?;
        wrapped_items.push(make_item_wrapper(
            guard_widget_views(node),
            container_id,
            mount_generation,
            slot_index,
            checked_generation,
        ));
    }

    let mut admission_children = Vec::with_capacity(wrapped_items.len() + 1);
    admission_children.push(shell);
    admission_children.extend(wrapped_items);
    let admission_root = ViewNode::new(ViewNodeKind::Container {
        policy: ContainerPolicy {
            kind: ContainerKind::Stack,
            ..ContainerPolicy::default()
        },
        children: admission_children,
    })
    .with_reserved_descendant_identity(true);

    let lowered = std::panic::catch_unwind(AssertUnwindSafe(|| {
        let mut keyed_candidates = HashSet::new();
        let mut continuity_keys = HashSet::new();
        let mut explicit_ids = HashSet::new();
        if admission_root
            .collect_keyed_collisions(ROOT_KEY_SCOPE, &mut keyed_candidates)
            .is_err()
            || admission_root
                .collect_explicit_identity_collisions(
                    ROOT_KEY_SCOPE,
                    &mut continuity_keys,
                    &mut explicit_ids,
                )
                .is_err()
        {
            return Err(VirtualLayoutViewAdmissionError::IdentityCollision);
        }

        let mut reserved = Vec::new();
        admission_root.collect_reserved_ids(ROOT_KEY_SCOPE, &mut reserved);
        let mut ids = IdGenerator::new(reserved);
        let mut scene = SceneProjection::default();
        let mut lowering = ViewLowering::new(&mut ids, &mut scene);
        let ViewNode {
            kind: ViewNodeKind::Container { children, .. },
            ..
        } = admission_root
        else {
            return Err(VirtualLayoutViewAdmissionError::LoweringPanicked);
        };
        let mut children = children.into_iter();
        let shell = lowering.lower_node(
            children
                .next()
                .ok_or(VirtualLayoutViewAdmissionError::LoweringPanicked)?,
            ROOT_KEY_SCOPE,
            StructuralRole::Root,
        );
        let items = children
            .map(|node| lowering.lower_node(node, ROOT_KEY_SCOPE, StructuralRole::Root))
            .collect();
        Ok(VirtualLayoutViewBatch { shell, items })
    }));

    let lowered = match lowered {
        Ok(result) => result,
        Err(payload) => match payload.downcast::<WidgetViewAdmissionPanic>() {
            Ok(panic) => Err(panic.0),
            Err(_) => Err(VirtualLayoutViewAdmissionError::LoweringPanicked),
        },
    };
    if let Ok(batch) = &lowered
        && !matches!(&batch.shell, SurfaceNode::Container(_))
    {
        return Err(VirtualLayoutViewAdmissionError::UnsupportedSceneEffects);
    }
    lowered
}

/// Lower one complete virtual-layout shell before the first query pass.
pub(crate) fn lower_virtual_layout_shell<Message: 'static>(
    shell: ViewNode<Message>,
    container_id: NodeId,
) -> Result<SurfaceNode<Message>, VirtualLayoutViewAdmissionError> {
    lower_virtual_layout_batch(shell, container_id, Vec::new()).map(|batch| batch.shell)
}

/// Guard every public widget-view boundary in the admitted declarative item.
///
/// `WidgetView` is intentionally public and returns a runtime `SurfaceNode`,
/// so its output must be checked at the point where this private adapter admits
/// it. A valid widget view returns one context-identified widget leaf; retained
/// containers and out-of-band runtime forms remain owned by the normal view
/// lowering path.
fn guard_widget_views<Message: 'static>(mut node: ViewNode<Message>) -> ViewNode<Message> {
    node.kind = match node.kind {
        ViewNodeKind::Widget(widget) => {
            ViewNodeKind::Widget(Box::new(GuardedWidgetView { widget }))
        }
        ViewNodeKind::Container { policy, children } => ViewNodeKind::Container {
            policy,
            children: children.into_iter().map(guard_widget_views).collect(),
        },
        ViewNodeKind::CustomLayout { policy, children } => ViewNodeKind::CustomLayout {
            policy,
            children: children.into_iter().map(guard_widget_views).collect(),
        },
        ViewNodeKind::Scroll { child } => ViewNodeKind::Scroll {
            child: Box::new(guard_widget_views(*child)),
        },
        ViewNodeKind::VirtualScroll { child, overscan_px } => ViewNodeKind::VirtualScroll {
            child: Box::new(guard_widget_views(*child)),
            overscan_px,
        },
        kind => kind,
    };
    node
}

#[derive(Debug)]
struct WidgetViewAdmissionPanic(VirtualLayoutViewAdmissionError);

struct GuardedWidgetView<Message> {
    widget: Box<dyn crate::application::WidgetView<Message>>,
}

impl<Message> crate::application::WidgetView<Message> for GuardedWidgetView<Message> {
    fn default_sizing(&self) -> crate::widgets::WidgetSizing {
        self.widget.default_sizing()
    }

    fn into_surface_node(
        self: Box<Self>,
        context: crate::application::WidgetViewContext,
    ) -> SurfaceNode<Message> {
        let expected_id = context.id;
        let surface = self.widget.into_surface_node(context);
        match &surface {
            SurfaceNode::Widget(_) if surface.id() == expected_id => surface,
            SurfaceNode::Scene(_) | SurfaceNode::Overlay(_) | SurfaceNode::FloatingLayer(_) => {
                panic_any(WidgetViewAdmissionPanic(
                    VirtualLayoutViewAdmissionError::UnsupportedSceneEffects,
                ));
            }
            SurfaceNode::Container(_) => {
                panic_any(WidgetViewAdmissionPanic(
                    VirtualLayoutViewAdmissionError::DirectSurfaceNode,
                ));
            }
            SurfaceNode::Widget(_) => {
                panic_any(WidgetViewAdmissionPanic(
                    VirtualLayoutViewAdmissionError::IdentityCollision,
                ));
            }
        }
    }
}

fn validate_item<Message>(node: &ViewNode<Message>) -> Result<(), VirtualLayoutViewAdmissionError> {
    if node.id.is_some() {
        return Err(VirtualLayoutViewAdmissionError::ExplicitIdentity);
    }
    if !node.overlay_layers.is_empty() {
        return Err(VirtualLayoutViewAdmissionError::UnsupportedSceneEffects);
    }
    match &node.kind {
        ViewNodeKind::Runtime(_) => Err(VirtualLayoutViewAdmissionError::DirectSurfaceNode),
        ViewNodeKind::Scene { .. }
        | ViewNodeKind::OverlayPanel { .. }
        | ViewNodeKind::VirtualLayout(_)
        | ViewNodeKind::FloatingLayer { .. } => {
            Err(VirtualLayoutViewAdmissionError::UnsupportedSceneEffects)
        }
        ViewNodeKind::Container { children, .. } | ViewNodeKind::CustomLayout { children, .. } => {
            for child in children {
                validate_item(child)?;
            }
            Ok(())
        }
        ViewNodeKind::Scroll { child } | ViewNodeKind::VirtualScroll { child, .. } => {
            validate_item(child)
        }
        ViewNodeKind::Widget(_) => Ok(()),
    }
}

fn make_item_wrapper<Message>(
    node: ViewNode<Message>,
    container_id: NodeId,
    mount_generation: u64,
    slot_index: usize,
    checked_generation: u64,
) -> ViewNode<Message> {
    let wrapper_id = slot_wrapper_id(
        container_id,
        mount_generation,
        slot_index,
        checked_generation,
    );
    let has_reserved_descendant_identity = node.has_reserved_identity_in_subtree();
    ViewNode::new(ViewNodeKind::Container {
        policy: ContainerPolicy {
            kind: ContainerKind::Stack,
            ..ContainerPolicy::default()
        },
        children: vec![node],
    })
    .with_reserved_descendant_identity(has_reserved_descendant_identity)
    .id(wrapper_id)
}

fn slot_wrapper_id(
    container_id: NodeId,
    mount_generation: u64,
    slot_index: usize,
    checked_generation: u64,
) -> NodeId {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for value in [
        0x56_4c_53_4c_4f_54_01_u64,
        container_id,
        mount_generation,
        slot_index as u64,
        checked_generation,
    ] {
        for byte in value.to_le_bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    if hash == 0 { 1 } else { hash }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::{WidgetView, WidgetViewContext, column, empty, overlays, scene, text};
    use crate::gui::types::{Point, Rect, Vector2};
    use crate::layout::{ContainerPolicy, LayoutNode};
    use crate::runtime::SurfaceChild;
    use crate::widgets::{TextWidget, WidgetSizing, WidgetStyle};

    struct PanickingView;

    impl WidgetView<()> for PanickingView {
        fn default_sizing(&self) -> crate::widgets::WidgetSizing {
            crate::widgets::WidgetSizing::new(
                crate::gui::types::Vector2::new(0.0, 0.0),
                crate::gui::types::Vector2::new(0.0, 0.0),
            )
        }

        fn into_surface_node(self: Box<Self>, _context: WidgetViewContext) -> SurfaceNode<()> {
            panic!("test lowering panic")
        }
    }

    #[derive(Clone, Copy)]
    enum CustomWidgetOutput {
        Valid,
        Scene,
        Overlay,
        Floating,
        FixedWidget,
        FixedSubtree,
    }

    struct CustomWidgetView {
        output: CustomWidgetOutput,
    }

    impl CustomWidgetView {
        fn new(output: CustomWidgetOutput) -> Self {
            Self { output }
        }
    }

    impl WidgetView<()> for CustomWidgetView {
        fn default_sizing(&self) -> WidgetSizing {
            WidgetSizing::fixed(Vector2::new(24.0, 16.0))
        }

        fn into_surface_node(self: Box<Self>, context: WidgetViewContext) -> SurfaceNode<()> {
            let runtime_widget = |id| {
                SurfaceNode::static_widget(TextWidget::new(
                    id,
                    "custom",
                    WidgetSizing::fixed(Vector2::new(24.0, 16.0)),
                ))
            };

            match self.output {
                CustomWidgetOutput::Valid => {
                    let mut widget =
                        TextWidget::new(0, "custom", WidgetSizing::fixed(Vector2::new(24.0, 16.0)));
                    context.apply_to(&mut widget);
                    SurfaceNode::static_widget(widget)
                }
                CustomWidgetOutput::Scene => {
                    SurfaceNode::scene(context.id, runtime_widget(90), Vec::new())
                }
                CustomWidgetOutput::Overlay => SurfaceNode::overlay_marker(
                    90,
                    Rect::from_xy_size(0.0, 0.0, 4.0, 4.0),
                    WidgetStyle::default(),
                ),
                CustomWidgetOutput::Floating => SurfaceNode::floating_layer(
                    90,
                    Point::new(0.0, 0.0),
                    Vector2::new(24.0, 16.0),
                    runtime_widget(91),
                    false,
                ),
                CustomWidgetOutput::FixedWidget => runtime_widget(90),
                CustomWidgetOutput::FixedSubtree => SurfaceNode::container(
                    90,
                    ContainerPolicy::default(),
                    vec![SurfaceChild::fill(runtime_widget(91))],
                ),
            }
        }
    }

    fn custom_widget(output: CustomWidgetOutput) -> ViewNode<()> {
        ViewNode::new(ViewNodeKind::Widget(Box::new(CustomWidgetView::new(
            output,
        ))))
    }

    fn lower(node: ViewNode<()>, checked_generation: u64) -> SurfaceNode<()> {
        lower_virtual_layout_item(node, 41, 7, 3, checked_generation)
            .expect("test item should be admitted")
    }

    #[test]
    fn equal_slot_tuple_preserves_wrapper_and_descendant_identity() {
        let first = lower(column([text::<()>("first")]), 1);
        let second = lower(column([text::<()>("second")]), 1);
        assert_eq!(first.id(), second.id());

        let first_layout = crate::runtime::UiSurface::new(first).layout_node();
        let second_layout = crate::runtime::UiSurface::new(second).layout_node();
        let (LayoutNode::Container(first), LayoutNode::Container(second)) =
            (first_layout, second_layout)
        else {
            panic!("slot wrapper should lower to a container");
        };
        assert_eq!(first.children[0].child.id(), second.children[0].child.id());
    }

    #[test]
    fn every_slot_tuple_component_changes_wrapper_identity() {
        let base = lower(text::<()>("item"), 1).id();
        assert_ne!(
            base,
            lower_virtual_layout_item(text::<()>("item"), 42, 7, 3, 1)
                .expect("container change")
                .id()
        );
        assert_ne!(
            base,
            lower_virtual_layout_item(text::<()>("item"), 41, 8, 3, 1)
                .expect("mount change")
                .id()
        );
        assert_ne!(
            base,
            lower_virtual_layout_item(text::<()>("item"), 41, 7, 4, 1)
                .expect("slot change")
                .id()
        );
        assert_ne!(base, lower(text::<()>("item"), 2).id());
    }

    #[test]
    fn unsupported_item_forms_are_rejected_before_lowering() {
        assert!(matches!(
            lower_virtual_layout_item(text::<()>("item").id(90), 41, 7, 3, 1),
            Err(VirtualLayoutViewAdmissionError::ExplicitIdentity)
        ));
        assert!(matches!(
            lower_virtual_layout_item(
                ViewNode::from(crate::runtime::SurfaceNode::<()>::overlay_marker(
                    90,
                    crate::gui::types::Rect::from_xy_size(0.0, 0.0, 1.0, 1.0),
                    crate::widgets::WidgetStyle::default(),
                )),
                41,
                7,
                3,
                1,
            ),
            Err(VirtualLayoutViewAdmissionError::DirectSurfaceNode)
        ));
        assert!(matches!(
            lower_virtual_layout_item(scene(empty::<()>()).into_view(), 41, 7, 3, 1),
            Err(VirtualLayoutViewAdmissionError::UnsupportedSceneEffects)
        ));
        assert!(matches!(
            lower_virtual_layout_item(
                text::<()>("item").overlays(overlays().floating(text::<()>("overlay"))),
                41,
                7,
                3,
                1,
            ),
            Err(VirtualLayoutViewAdmissionError::UnsupportedSceneEffects)
        ));
    }

    #[test]
    fn custom_widget_scene_overlay_and_floating_outputs_are_rejected() {
        for output in [
            CustomWidgetOutput::Scene,
            CustomWidgetOutput::Overlay,
            CustomWidgetOutput::Floating,
        ] {
            assert!(matches!(
                lower_virtual_layout_item(custom_widget(output), 41, 7, 3, 1),
                Err(VirtualLayoutViewAdmissionError::UnsupportedSceneEffects)
            ));
        }
    }

    #[test]
    fn custom_widget_retained_identity_forms_are_rejected() {
        assert!(matches!(
            lower_virtual_layout_item(custom_widget(CustomWidgetOutput::FixedWidget), 41, 7, 3, 1),
            Err(VirtualLayoutViewAdmissionError::IdentityCollision)
        ));
        assert!(matches!(
            lower_virtual_layout_item(custom_widget(CustomWidgetOutput::FixedSubtree), 41, 7, 3, 1),
            Err(VirtualLayoutViewAdmissionError::DirectSurfaceNode)
        ));
    }

    #[test]
    fn context_identified_custom_widget_preserves_the_wrapper() {
        let surface = lower(custom_widget(CustomWidgetOutput::Valid), 1);
        let LayoutNode::Container(wrapper) = crate::runtime::UiSurface::new(surface).layout_node()
        else {
            panic!("slot wrapper should lower to a container");
        };
        assert_eq!(wrapper.children.len(), 1);
        assert!(matches!(wrapper.children[0].child, LayoutNode::Widget(_)));
    }

    #[test]
    fn duplicate_key_identity_is_rejected() {
        let item = column([text::<()>("one").key("same"), text::<()>("two").key("same")]);
        assert!(matches!(
            lower_virtual_layout_item(item, 41, 7, 3, 1),
            Err(VirtualLayoutViewAdmissionError::IdentityCollision)
        ));
    }

    #[test]
    fn lowering_panic_is_a_typed_rejection() {
        let item = ViewNode::new(ViewNodeKind::Widget(Box::new(PanickingView)));
        assert!(matches!(
            lower_virtual_layout_item(item, 41, 7, 3, 1),
            Err(VirtualLayoutViewAdmissionError::LoweringPanicked)
        ));
    }

    #[test]
    fn complete_batch_admits_shell_and_items_with_scoped_wrappers() {
        let batch = lower_virtual_layout_batch(
            column([text::<()>("shell")]),
            41,
            vec![(text::<()>("one"), 7, 0, 1), (text::<()>("two"), 7, 1, 1)],
        )
        .expect("complete batch should be admitted");
        assert_eq!(batch.shell.id(), 41);
        assert_eq!(batch.items.len(), 2);
        assert_ne!(batch.items[0].id(), batch.items[1].id());

        let changed_generation = lower_virtual_layout_batch(
            column([text::<()>("shell")]),
            41,
            vec![(text::<()>("one"), 7, 0, 2)],
        )
        .expect("changed slot generation should be admitted");
        let same_generation = lower_virtual_layout_batch(
            column([text::<()>("shell")]),
            41,
            vec![(text::<()>("one"), 7, 0, 1)],
        )
        .expect("same slot generation should be admitted");
        assert_ne!(
            changed_generation.items[0].id(),
            same_generation.items[0].id()
        );
    }

    #[test]
    fn batch_identity_admission_rejects_duplicate_wrappers_before_lowering() {
        let first = ViewNode::new(ViewNodeKind::Widget(Box::new(PanickingView)));
        let result = lower_virtual_layout_batch(
            column([text::<()>("shell")]),
            41,
            vec![(first, 7, 0, 1), (text::<()>("duplicate"), 7, 0, 1)],
        );
        assert!(matches!(
            result,
            Err(VirtualLayoutViewAdmissionError::IdentityCollision)
        ));

        let shell_collision = lower_virtual_layout_batch(
            column([text::<()>("shell").id(slot_wrapper_id(41, 7, 0, 1))]),
            41,
            vec![(text::<()>("item"), 7, 0, 1)],
        );
        assert!(matches!(
            shell_collision,
            Err(VirtualLayoutViewAdmissionError::IdentityCollision)
        ));
    }

    #[test]
    fn batch_identity_admission_rejects_unsupported_item_forms() {
        assert!(matches!(
            lower_virtual_layout_batch(
                column([text::<()>("shell")]),
                41,
                vec![(scene(empty::<()>()).into_view(), 7, 0, 1)],
            ),
            Err(VirtualLayoutViewAdmissionError::UnsupportedSceneEffects)
        ));
        assert!(matches!(
            lower_virtual_layout_batch(
                column([text::<()>("shell")]),
                41,
                vec![(text::<()>("item").id(90), 7, 0, 1)],
            ),
            Err(VirtualLayoutViewAdmissionError::ExplicitIdentity)
        ));
    }
}
