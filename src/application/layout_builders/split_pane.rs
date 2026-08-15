//! Static two-pane split layout builder.

use crate::{
    application::{IntoView, ViewNode, ViewNodeKind},
    gui::layout_core::{Controlled, SplitPaneRuntimeMode},
    gui::panel::SplitPaneAxis,
    layout::{ContainerKind, ContainerPolicy, SplitPanePolicy},
};

/// Declarative builder for one static, exactly-two-child split pane.
pub struct SplitPaneBuilder<Message> {
    first: ViewNode<Message>,
    second: ViewNode<Message>,
    policy: SplitPanePolicy,
    runtime_ratio: Option<SplitPaneRuntimeMode>,
}

impl<Message> SplitPaneBuilder<Message> {
    /// Set the axis along which the first and second panes are ordered.
    pub fn axis(mut self, axis: SplitPaneAxis) -> Self {
        self.policy.axis = axis;
        self
    }

    /// Set the requested normalized extent of the first pane.
    ///
    /// Finite values are clamped during layout; non-finite values use the
    /// balanced `0.5` fallback supplied by [`crate::gui::panel::SplitPaneLayout`].
    pub fn initial_ratio(mut self, ratio: f32) -> Self {
        self.policy.initial_ratio = ratio;
        self
    }

    /// Set the requested minimum extent of the first pane.
    ///
    /// Negative and non-finite values are normalized by the shared split
    /// geometry resolver.
    pub fn min_first(mut self, extent: f32) -> Self {
        self.policy.first_min_extent = extent;
        self
    }

    /// Set the requested minimum extent of the second pane.
    ///
    /// Negative and non-finite values are normalized by the shared split
    /// geometry resolver.
    pub fn min_second(mut self, extent: f32) -> Self {
        self.policy.second_min_extent = extent;
        self
    }

    /// Set the requested divider extent along the split axis.
    ///
    /// Negative and non-finite values are normalized by the shared split
    /// geometry resolver and the divider is bounded by the viewport.
    pub fn divider_extent(mut self, extent: f32) -> Self {
        self.policy.divider_extent = extent;
        self
    }

    /// Let the mounted split-pane state own the live ratio, seeded once from
    /// [`Self::initial_ratio`].
    pub fn runtime_owned_ratio(mut self) -> Self {
        self.runtime_ratio = Some(SplitPaneRuntimeMode::RuntimeOwned);
        self
    }

    /// Accept a controlled ratio on mount and only from strictly newer
    /// generations thereafter.
    pub fn controlled_ratio(mut self, controlled: Controlled<f32>) -> Self {
        self.runtime_ratio = Some(SplitPaneRuntimeMode::Controlled(controlled));
        self
    }

    /// Lower this builder into an ordinary declarative view node.
    pub fn into_view(self) -> ViewNode<Message> {
        let has_reserved_descendant_identity = self.first.has_reserved_identity_in_subtree()
            || self.second.has_reserved_identity_in_subtree();
        ViewNode::new(ViewNodeKind::Container {
            policy: ContainerPolicy {
                kind: ContainerKind::SplitPane,
                split_pane: self.policy,
                ..ContainerPolicy::default()
            },
            children: vec![self.first, self.second],
        })
        .with_split_pane_runtime_mode(self.runtime_ratio)
        .with_reserved_descendant_identity(has_reserved_descendant_identity)
    }
}

/// Build a static two-pane split with the first child on the leading side.
pub fn split_pane<Message>(
    first: ViewNode<Message>,
    second: ViewNode<Message>,
) -> SplitPaneBuilder<Message> {
    SplitPaneBuilder {
        first,
        second,
        policy: SplitPanePolicy::default(),
        runtime_ratio: None,
    }
}

impl<Message: 'static> IntoView<Message> for SplitPaneBuilder<Message> {
    fn into_projection(self) -> crate::application::ViewProjection<Message> {
        self.into_view().into_projection()
    }
}
