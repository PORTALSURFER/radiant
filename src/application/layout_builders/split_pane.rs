//! Static two-pane split layout builder.

use crate::{
    application::{ApplicationProjectionContext, IntoView, ViewNode, ViewNodeKind},
    gui::layout_core::{Controlled, SplitPaneRuntimeMode},
    gui::panel::{SplitPaneAxis, SplitPaneCollapsePolicy},
    layout::{ContainerKind, ContainerPolicy, SplitPanePolicy},
};
use std::rc::Rc;

/// Declarative builder for one static, exactly-two-child split pane.
pub struct SplitPaneBuilder<Message> {
    first: ViewNode<Message>,
    second: ViewNode<Message>,
    policy: SplitPanePolicy,
    runtime_ratio: Option<SplitPaneRuntimeMode>,
    collapse_policy: Option<SplitPaneCollapsePolicy>,
    ratio_settled: Option<Rc<dyn Fn(f32) -> Message>>,
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

    /// Select which pane a runtime-owned divider double activation collapses.
    ///
    /// The option is inert for the static and controlled-ratio forms. Runtime
    /// collapse resolves the selected pane through the shared split geometry
    /// contract and restores its last finite expanded ratio.
    pub fn collapse_policy(mut self, policy: SplitPaneCollapsePolicy) -> Self {
        self.collapse_policy = Some(policy);
        self
    }

    /// Let the mounted split-pane state own the live ratio, seeded once from
    /// [`Self::initial_ratio`].
    pub fn runtime_owned_ratio(mut self) -> Self {
        self.runtime_ratio = Some(SplitPaneRuntimeMode::RuntimeOwned {
            collapse_policy: None,
        });
        self
    }

    /// Accept a controlled ratio on mount and only from strictly newer
    /// generations thereafter.
    pub fn controlled_ratio(mut self, controlled: Controlled<f32>) -> Self {
        self.runtime_ratio = Some(SplitPaneRuntimeMode::Controlled(controlled));
        self
    }

    /// Emit one host message when a meaningful runtime-owned divider drag
    /// commits its final normalized ratio.
    pub fn on_ratio_settled(mut self, map: impl Fn(f32) -> Message + 'static) -> Self
    where
        Message: 'static,
    {
        self.ratio_settled = Some(Rc::new(map));
        self
    }

    /// Lower this builder into an ordinary declarative view node.
    pub fn into_view(self) -> ViewNode<Message> {
        let runtime_ratio = match self.runtime_ratio {
            Some(SplitPaneRuntimeMode::RuntimeOwned { .. }) => {
                Some(SplitPaneRuntimeMode::RuntimeOwned {
                    collapse_policy: self.collapse_policy,
                })
            }
            runtime_ratio => runtime_ratio,
        };
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
        .with_split_pane_runtime_mode(runtime_ratio)
        .with_split_pane_ratio_settled(self.ratio_settled)
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
        collapse_policy: None,
        ratio_settled: None,
    }
}

impl<Message: 'static> IntoView<Message> for SplitPaneBuilder<Message> {
    fn into_projection(self) -> crate::application::ViewProjection<Message> {
        self.into_view().into_projection()
    }

    fn into_application_projection(
        self,
        context: &mut ApplicationProjectionContext<'_>,
    ) -> crate::application::ViewProjection<Message> {
        self.into_view().into_application_projection(context)
    }
}
