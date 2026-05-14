use super::{ViewLowering, ViewNode};
use crate::runtime::SurfaceChild;

impl ViewLowering<'_> {
    pub(super) fn lower_child<Message>(
        &mut self,
        child: ViewNode<Message>,
        scope: u64,
        parent_horizontal: bool,
    ) -> SurfaceChild<Message>
    where
        Message: 'static,
    {
        let slot = child.slot.to_slot_params(parent_horizontal);
        SurfaceChild::new(slot, self.lower_node(child, scope))
    }

    pub(super) fn lower_slot_children<Message>(
        &mut self,
        children: Vec<ViewNode<Message>>,
        scope: u64,
        parent_horizontal: bool,
    ) -> Vec<SurfaceChild<Message>>
    where
        Message: 'static,
    {
        children
            .into_iter()
            .map(|child| self.lower_child(child, scope, parent_horizontal))
            .collect()
    }

    pub(super) fn lower_fill_child<Message>(
        &mut self,
        child: ViewNode<Message>,
        scope: u64,
    ) -> SurfaceChild<Message>
    where
        Message: 'static,
    {
        SurfaceChild::fill(self.lower_node(child, scope))
    }

    pub(super) fn lower_fill_children<Message>(
        &mut self,
        children: Vec<ViewNode<Message>>,
        scope: u64,
    ) -> Vec<SurfaceChild<Message>>
    where
        Message: 'static,
    {
        children
            .into_iter()
            .map(|child| self.lower_fill_child(child, scope))
            .collect()
    }
}
