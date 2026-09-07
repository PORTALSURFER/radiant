//! Pure resource branch construction with accepted-projection demand metadata.

pub(crate) mod demand;

use crate::application::{
    ApplicationProjectionContext, IntoView, ResourceInterestKind, ResourcePhase, ResourceSnapshot,
    SharedResourceTasks, ViewNode, ViewProjection, column,
};
use demand::ResourceViewDemand;
use std::{rc::Rc, sync::Arc};

/// Equivalent prebuilt branch nodes for applications that do not need factories.
pub struct ResourceViewBranches<Message> {
    /// Presentation before a request starts.
    pub idle: Option<ViewNode<Message>>,
    /// Presentation while the first value is pending.
    pub pending: Option<ViewNode<Message>>,
    /// Presentation of accepted ready content.
    pub ready: Option<ViewNode<Message>>,
    /// Presentation while a retained value refreshes.
    pub refreshing: Option<ViewNode<Message>>,
    /// Presentation of the typed failure.
    pub failed: Option<ViewNode<Message>>,
    /// Presentation after cancellation without retained content.
    pub cancelled: Option<ViewNode<Message>>,
}

impl<Message> Default for ResourceViewBranches<Message> {
    fn default() -> Self {
        Self {
            idle: None,
            pending: None,
            ready: None,
            refreshing: None,
            failed: None,
            cancelled: None,
        }
    }
}

/// Immediate resource presentation builder. Branch factories never outlive construction.
///
/// Convert with `into_view` when nesting in an ordinary container. Retry/cancel
/// controls in a branch emit normal application messages; projection starts no work.
pub struct ResourceView<T, E, Message> {
    snapshot: ResourceSnapshot<T, E>,
    selected: Option<ViewNode<Message>>,
    demand: Option<Rc<ResourceViewDemand>>,
}

/// Select a branch from an owned application resource snapshot.
///
/// ```
/// use radiant::application::{Resource, resource, text};
///
/// let state = Resource::<u32, String>::new("preview");
/// let view = resource(state.snapshot())
///     .idle(text::<()>("Choose a preview"))
///     .ready(|value| text(format!("Value: {value}")))
///     .failed(|error| text(error.to_string()))
///     .into_view();
/// drop(view); // Construction starts no resource work.
/// ```
pub fn resource<T, E, Message>(snapshot: ResourceSnapshot<T, E>) -> ResourceView<T, E, Message> {
    ResourceView {
        snapshot,
        selected: None,
        demand: None,
    }
}

impl<T, E, Message> ResourceView<T, E, Message> {
    /// Select one prebuilt node without retaining any branch factories.
    pub fn from_branches(
        snapshot: ResourceSnapshot<T, E>,
        branches: ResourceViewBranches<Message>,
    ) -> Self {
        let selected = match snapshot.phase() {
            ResourcePhase::Idle => branches.idle,
            ResourcePhase::Pending => branches.pending,
            ResourcePhase::Ready => branches.ready,
            ResourcePhase::Refreshing => branches.refreshing,
            ResourcePhase::Failed => branches.failed,
            ResourcePhase::Cancelled => branches.cancelled,
        };
        Self {
            snapshot,
            selected,
            demand: None,
        }
    }

    /// Present this node before any request has started.
    pub fn idle(mut self, view: ViewNode<Message>) -> Self {
        if self.snapshot.phase() == ResourcePhase::Idle && self.selected.is_none() {
            self.selected = Some(view);
        }
        self
    }

    /// Present this node while the first value is pending.
    pub fn pending(mut self, view: ViewNode<Message>) -> Self {
        if self.snapshot.phase() == ResourcePhase::Pending && self.selected.is_none() {
            self.selected = Some(view);
        }
        self
    }

    /// Invoke the ready factory immediately with a shared owned value.
    pub fn ready(mut self, build: impl FnOnce(Arc<T>) -> ViewNode<Message>) -> Self {
        if self.snapshot.phase() == ResourcePhase::Ready
            && self.selected.is_none()
            && let Some(value) = self.snapshot.value()
        {
            self.selected = Some(build(Arc::clone(value)));
        }
        self
    }

    /// Invoke the refresh factory immediately with the retained ready value.
    pub fn refreshing(mut self, build: impl FnOnce(Arc<T>) -> ViewNode<Message>) -> Self {
        if self.snapshot.phase() == ResourcePhase::Refreshing
            && self.selected.is_none()
            && let Some(value) = self.snapshot.value()
        {
            self.selected = Some(build(Arc::clone(value)));
        }
        self
    }

    /// Invoke the failure factory immediately with the application's typed error.
    pub fn failed(mut self, build: impl FnOnce(Arc<E>) -> ViewNode<Message>) -> Self {
        if self.snapshot.phase() == ResourcePhase::Failed
            && self.selected.is_none()
            && let Some(error) = self.snapshot.error()
        {
            self.selected = Some(build(Arc::clone(error)));
        }
        self
    }

    /// Present a failure alongside an eligible retained ready value.
    pub fn failed_with_ready(
        mut self,
        build: impl FnOnce(Arc<E>, Option<Arc<T>>) -> ViewNode<Message>,
    ) -> Self {
        if self.snapshot.phase() == ResourcePhase::Failed
            && self.selected.is_none()
            && let Some(error) = self.snapshot.error()
        {
            self.selected = Some(build(Arc::clone(error), self.snapshot.value().cloned()));
        }
        self
    }

    /// Present cancellation when no retained ready value is available.
    pub fn cancelled(mut self, view: ViewNode<Message>) -> Self {
        if self.snapshot.phase() == ResourcePhase::Cancelled && self.selected.is_none() {
            self.selected = Some(view);
        }
        self
    }

    /// Contribute demand only while this consumer's projection is accepted.
    ///
    /// The containing structural node identifies the consumer; `interest_id`
    /// distinguishes explicit consumer policy within that node. This stores
    /// metadata only. The runtime acquires and retires the accepted view lease.
    pub fn interest(
        mut self,
        tasks: &SharedResourceTasks,
        interest_id: u64,
        kind: ResourceInterestKind,
    ) -> Self {
        self.demand = Some(Rc::new(ResourceViewDemand {
            tasks: tasks.clone(),
            key: self.snapshot.key().clone(),
            kind,
            interest_id,
        }));
        self
    }

    /// Finish with ordinary structural continuity and the selected branch's semantics.
    ///
    /// An omitted branch is an empty container. The wrapper stays structurally
    /// stable across phase changes and does not synthesize labels or actions.
    pub fn into_view(self) -> ViewNode<Message> {
        let mut wrapper = column(self.selected);
        wrapper.demands = self.demand.map(|resource| {
            Rc::new(crate::application::view_node::DeclarativeDemands {
                resource: Some(resource),
                notice: None,
            })
        });
        wrapper
    }
}

impl<T, E, Message: 'static> IntoView<Message> for ResourceView<T, E, Message> {
    fn into_projection(self) -> ViewProjection<Message> {
        self.into_view().into_projection()
    }

    fn into_application_projection(
        self,
        context: &mut ApplicationProjectionContext<'_>,
    ) -> ViewProjection<Message> {
        self.into_view().into_application_projection(context)
    }
}

#[cfg(test)]
mod tests;
