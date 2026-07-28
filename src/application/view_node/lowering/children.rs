use super::{ViewLowering, ViewNode};
use crate::application::ids::StructuralRole;
use crate::runtime::SurfaceChild;

impl<Message: 'static> ViewLowering<'_, Message> {
    pub(super) fn lower_child(
        &mut self,
        child: ViewNode<Message>,
        scope: u64,
        parent_horizontal: bool,
        role: StructuralRole,
    ) -> SurfaceChild<Message> {
        let slot = child.slot.to_slot_params(parent_horizontal);
        SurfaceChild::new(slot, self.lower_node(child, scope, role))
    }

    pub(super) fn lower_slot_children(
        &mut self,
        children: Vec<ViewNode<Message>>,
        scope: u64,
        parent_horizontal: bool,
    ) -> Vec<SurfaceChild<Message>> {
        children
            .into_iter()
            .enumerate()
            .map(|(index, child)| {
                self.lower_child(
                    child,
                    scope,
                    parent_horizontal,
                    StructuralRole::ContainerChild(index),
                )
            })
            .collect()
    }

    pub(super) fn lower_fill_child(
        &mut self,
        child: ViewNode<Message>,
        scope: u64,
        role: StructuralRole,
    ) -> SurfaceChild<Message> {
        SurfaceChild::fill(self.lower_node(child, scope, role))
    }

    pub(super) fn lower_fill_children(
        &mut self,
        children: Vec<ViewNode<Message>>,
        scope: u64,
    ) -> Vec<SurfaceChild<Message>> {
        children
            .into_iter()
            .enumerate()
            .map(|(index, child)| {
                self.lower_fill_child(child, scope, StructuralRole::ContainerChild(index))
            })
            .collect()
    }
}
