use super::super::ViewNode;
use crate::runtime::ScrollUpdate;

impl<Message> ViewNode<Message> {
    /// Emit a host message when this scroll container's runtime offset changes.
    ///
    /// This is intended for declarative scroll-driven views such as fixed-row
    /// virtual lists whose app state owns the logical window while Radiant owns
    /// the runtime scroll container and pixel offset.
    pub fn on_scroll_update(
        mut self,
        message: impl Fn(ScrollUpdate) -> Message + Send + Sync + 'static,
    ) -> Self {
        self.scroll_message = Some(std::sync::Arc::new(message));
        self
    }
}
