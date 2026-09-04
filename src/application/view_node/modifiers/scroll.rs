use super::super::ViewNode;
use crate::{
    layout::{Controlled, ScrollPolicy, ScrollRequest, Vector2},
    runtime::ScrollUpdate,
};

impl<Message> ViewNode<Message> {
    /// Configure the backend-neutral policy for this scroll container.
    pub fn scroll_policy(mut self, policy: ScrollPolicy) -> Self {
        self.scroll_policy = Some(policy);
        self
    }

    /// Seed the runtime-owned offset on a new mount.
    pub fn initial_offset(mut self, offset: Vector2) -> Self {
        self.initial_offset = Some(offset);
        self
    }

    /// Provide a strictly generation-ordered controlled offset.
    pub fn controlled_offset(mut self, offset: Controlled<Vector2>) -> Self {
        self.controlled_offset = Some(offset);
        self
    }

    /// Issue a one-shot generation-bearing reveal request.
    pub fn scroll_request(mut self, request: ScrollRequest) -> Self {
        self.scroll_request = Some(request);
        self
    }

    /// Emit one host message after an accepted offset settles.
    pub fn on_offset_settled(mut self, message: impl Fn(Vector2) -> Message + 'static) -> Self
    where
        Message: 'static,
    {
        self.offset_settled = Some(std::rc::Rc::new(message));
        self
    }

    /// Emit a host message when this scroll container's runtime offset changes.
    ///
    /// This is intended for declarative scroll-driven views such as fixed-row
    /// virtual lists whose app state owns the logical window while Radiant owns
    /// the runtime scroll container and pixel offset.
    pub fn on_scroll_update(mut self, message: impl Fn(ScrollUpdate) -> Message + 'static) -> Self {
        self.scroll_message = Some(std::rc::Rc::new(move |update| Some(message(update))));
        self
    }

    /// Optionally emit a host message when this scroll container's runtime offset changes.
    ///
    /// Use this for high-frequency scroll surfaces that can update local runtime
    /// offset without host reprojection until the logical scroll window changes.
    pub fn on_scroll_update_opt(
        mut self,
        message: impl Fn(ScrollUpdate) -> Option<Message> + 'static,
    ) -> Self {
        self.scroll_message = Some(std::rc::Rc::new(message));
        self
    }
}
