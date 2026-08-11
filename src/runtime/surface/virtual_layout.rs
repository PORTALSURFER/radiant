//! Crate-private shell registration evidence for the virtual-layout consumer.
//!
//! This is deliberately not part of the public surface API. It is carried by
//! a projected container only so `SurfaceRuntime` can discover one immutable
//! policy/data snapshot before it performs the shell pass.

use crate::{
    application::View,
    gui::layout_core::{VirtualLayoutBatchProjector, VirtualLayoutSemanticProvider},
    gui::types::Rect,
    layout::{
        VirtualLayoutBudget, VirtualLayoutCoordinateSpace, VirtualLayoutItem, VirtualLayoutItemKey,
        VirtualLayoutOverscan, VirtualLayoutPolicy, VirtualLayoutPolicyIdentity,
    },
    runtime::{SurfaceChild, SurfaceNode, UiSurface},
};
use std::rc::Rc;

pub(crate) const MAX_VIRTUAL_LAYOUT_REGISTRATIONS: usize = 64;

type VirtualLayoutShellFactory<Message> = Rc<dyn Fn() -> View<Message>>;
type VirtualLayoutItemFactory<Message> = Rc<dyn Fn(&VirtualLayoutItem) -> View<Message>>;
type VirtualLayoutKindFactory = Rc<dyn Fn(&VirtualLayoutItem) -> VirtualLayoutPolicyIdentity>;
type VirtualLayoutShellLowerer<Message> = Rc<dyn Fn() -> Option<SurfaceNode<Message>>>;
type VirtualLayoutProjectorFactory<Message> = Rc<dyn Fn() -> VirtualLayoutBatchProjector<Message>>;

/// Exact revision evidence supplied with one projected registration.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct VirtualLayoutRegistrationRevisions {
    pub(crate) viewport: u64,
    pub(crate) data: u64,
    pub(crate) policy: u64,
    pub(crate) measurement: u64,
    pub(crate) semantic: u64,
}

/// One immutable, crate-private virtual-layout shell registration.
pub(crate) struct VirtualLayoutRegistration<Message> {
    pub(crate) container_id: crate::layout::NodeId,
    pub(crate) policy_identity: VirtualLayoutPolicyIdentity,
    pub(crate) policy: Rc<dyn VirtualLayoutPolicy>,
    pub(crate) coordinate_space: VirtualLayoutCoordinateSpace,
    pub(crate) overscan: VirtualLayoutOverscan,
    pub(crate) budget: VirtualLayoutBudget,
    required_key: Option<VirtualLayoutItemKey>,
    pub(crate) revisions: VirtualLayoutRegistrationRevisions,
    pub(crate) shell: VirtualLayoutShellFactory<Message>,
    pub(crate) item: VirtualLayoutItemFactory<Message>,
    pub(crate) kind: VirtualLayoutKindFactory,
    semantic_provider: Option<Rc<dyn VirtualLayoutSemanticProvider>>,
    shell_lowerer: VirtualLayoutShellLowerer<Message>,
    projector_factory: VirtualLayoutProjectorFactory<Message>,
}

impl<Message> Clone for VirtualLayoutRegistration<Message> {
    fn clone(&self) -> Self {
        Self {
            container_id: self.container_id,
            policy_identity: self.policy_identity.clone(),
            policy: Rc::clone(&self.policy),
            coordinate_space: self.coordinate_space.clone(),
            overscan: self.overscan,
            budget: self.budget,
            required_key: self.required_key.clone(),
            revisions: self.revisions,
            shell: Rc::clone(&self.shell),
            item: Rc::clone(&self.item),
            kind: Rc::clone(&self.kind),
            semantic_provider: self.semantic_provider.as_ref().map(Rc::clone),
            shell_lowerer: Rc::clone(&self.shell_lowerer),
            projector_factory: Rc::clone(&self.projector_factory),
        }
    }
}

impl<Message> VirtualLayoutRegistration<Message> {
    /// Construct private registration evidence for runtime-owned tests and
    /// future in-crate product adapters.
    #[allow(dead_code)]
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        container_id: crate::layout::NodeId,
        policy_identity: VirtualLayoutPolicyIdentity,
        policy: Rc<dyn VirtualLayoutPolicy>,
        coordinate_space: VirtualLayoutCoordinateSpace,
        overscan: VirtualLayoutOverscan,
        budget: VirtualLayoutBudget,
        revisions: VirtualLayoutRegistrationRevisions,
        shell: VirtualLayoutShellFactory<Message>,
        item: VirtualLayoutItemFactory<Message>,
        kind: VirtualLayoutKindFactory,
    ) -> Self
    where
        Message: 'static,
    {
        let shell_for_lowering = Rc::clone(&shell);
        let shell_container_id = container_id;
        let shell_lowerer = Rc::new(move || {
            crate::application::lower_virtual_layout_shell(shell_for_lowering(), shell_container_id)
                .ok()
        });
        let projector_factory = VirtualLayoutBatchProjector::factory(
            Rc::clone(&shell),
            Rc::clone(&item),
            Rc::clone(&kind),
        );
        Self {
            container_id,
            policy_identity,
            policy,
            coordinate_space,
            overscan,
            budget,
            revisions,
            shell,
            item,
            kind,
            semantic_provider: None,
            required_key: None,
            shell_lowerer,
            projector_factory,
        }
    }

    pub(crate) fn lowered_shell(&self) -> Option<SurfaceNode<Message>> {
        (self.shell_lowerer)()
    }

    pub(crate) fn projector(&self) -> VirtualLayoutBatchProjector<Message> {
        (self.projector_factory)()
    }

    /// Request one private exact item key in the next bounded policy query.
    #[allow(dead_code)]
    pub(crate) fn with_required_key(mut self, key: VirtualLayoutItemKey) -> Self {
        self.required_key = Some(key);
        self
    }

    pub(crate) fn required_key(&self) -> Option<&VirtualLayoutItemKey> {
        self.required_key.as_ref()
    }

    #[allow(dead_code)]
    pub(crate) fn with_semantic_provider(
        mut self,
        provider: Rc<dyn VirtualLayoutSemanticProvider>,
    ) -> Self {
        self.semantic_provider = Some(provider);
        self
    }

    #[allow(dead_code)]
    pub(crate) fn semantic_provider(&self) -> Option<&dyn VirtualLayoutSemanticProvider> {
        self.semantic_provider.as_deref()
    }

    pub(crate) fn semantic_provider_is_same(&self, other: &Self) -> bool {
        match (&self.semantic_provider, &other.semantic_provider) {
            (None, None) => true,
            (Some(previous), Some(next)) => Rc::ptr_eq(previous, next),
            _ => false,
        }
    }

    pub(crate) const fn semantic_revision(&self) -> u64 {
        self.revisions.semantic
    }

    pub(crate) const fn data_revision(&self) -> u64 {
        self.revisions.data
    }

    pub(crate) const fn policy_revision(&self) -> u64 {
        self.revisions.policy
    }

    pub(crate) const fn measurement_revision(&self) -> u64 {
        self.revisions.measurement
    }

    pub(crate) fn same_scope(&self, other: &Self) -> bool {
        self.container_id == other.container_id && self.policy_identity == other.policy_identity
    }

    pub(crate) fn query_parts(
        &self,
        viewport: Rect,
        mount_generation: u64,
    ) -> crate::layout::VirtualLayoutQueryInputParts {
        crate::layout::VirtualLayoutQueryInputParts {
            container_id: self.container_id,
            policy_identity: self.policy_identity.clone(),
            mount_generation,
            query_sequence: 0,
            viewport,
            coordinate_space: self.coordinate_space.clone(),
            overscan: self.overscan,
            budget: self.budget,
            viewport_revision: self.revisions.viewport,
            data_revision: self.revisions.data,
            policy_revision: self.revisions.policy,
            measurement_revision: self.revisions.measurement,
            semantic_revision: self.revisions.semantic,
        }
    }
}

impl<Message> UiSurface<Message> {
    pub(in crate::runtime) fn replace_virtual_layout_shell(
        &mut self,
        container_id: crate::layout::NodeId,
        shell: SurfaceNode<Message>,
        registration: VirtualLayoutRegistration<Message>,
    ) -> bool {
        replace_node(
            &mut self.root,
            container_id,
            shell.with_virtual_layout_registration(registration),
        )
    }

    pub(in crate::runtime) fn install_virtual_layout_subtree(
        &mut self,
        container_id: crate::layout::NodeId,
        shell: &SurfaceNode<Message>,
        registration: &VirtualLayoutRegistration<Message>,
        items: &[SurfaceNode<Message>],
    ) -> bool {
        let node = shell
            .clone()
            .with_virtual_layout_registration(registration.clone())
            .with_virtual_layout_items(items);
        replace_node(&mut self.root, container_id, node)
    }
}

impl<Message> SurfaceNode<Message> {
    fn with_virtual_layout_items(mut self, items: &[SurfaceNode<Message>]) -> Self {
        if let Self::Container(container) = &mut self {
            container
                .children
                .extend(items.iter().cloned().map(SurfaceChild::fill));
        }
        self
    }
}

fn replace_node<Message>(
    node: &mut SurfaceNode<Message>,
    container_id: crate::layout::NodeId,
    replacement: SurfaceNode<Message>,
) -> bool {
    if node.id() == container_id {
        *node = replacement;
        return true;
    }
    match node {
        SurfaceNode::Scene(scene) => {
            if replace_node(&mut scene.base, container_id, replacement.clone()) {
                return true;
            }
            for layer in &mut scene.layers {
                if let Some(input) = &mut layer.input
                    && replace_node(input, container_id, replacement.clone())
                {
                    return true;
                }
                if replace_node(&mut layer.node, container_id, replacement.clone()) {
                    return true;
                }
            }
            false
        }
        SurfaceNode::Container(container) => container
            .children
            .iter_mut()
            .any(|child| replace_node(&mut child.child, container_id, replacement.clone())),
        SurfaceNode::FloatingLayer(layer) => layer
            .container
            .children
            .iter_mut()
            .any(|child| replace_node(&mut child.child, container_id, replacement.clone())),
        SurfaceNode::Widget(_) | SurfaceNode::Overlay(_) => false,
    }
}
