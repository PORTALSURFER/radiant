use super::super::ViewNode;
use crate::layout::NodeId;

impl<Message> ViewNode<Message> {
    /// Use an explicit stable id instead of the generated structural id.
    pub fn id(mut self, id: NodeId) -> Self {
        self.id = Some(id);
        self.key = None;
        self.has_reserved_identity = true;
        self
    }

    /// Use a scoped stable key instead of a numeric id.
    ///
    /// Child keys are scoped by their keyed or explicitly identified parent, so repeated rows can
    /// use names such as `"done"` or `"delete"` without colliding with sibling rows.
    pub fn key(mut self, key: impl ToString) -> Self {
        self.id = None;
        self.key = Some(key.to_string());
        self.has_reserved_identity = true;
        self
    }
}
