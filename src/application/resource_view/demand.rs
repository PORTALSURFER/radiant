//! Private declarative transport for one resource-view interest request.

use crate::{
    application::{ResourceInterestKind, SharedResourceTasks},
    runtime::ResourceKey,
};

/// Immutable demand declaration carried from a resource view to its lowered
/// container. Controller reconciliation owns lease admission and retirement.
#[derive(Clone)]
pub(crate) struct ResourceViewDemand {
    pub(crate) tasks: SharedResourceTasks,
    pub(crate) key: ResourceKey,
    pub(crate) kind: ResourceInterestKind,
    pub(crate) interest_id: u64,
}

impl ResourceViewDemand {
    pub(crate) fn same_demand(&self, other: &Self) -> bool {
        self.key == other.key
            && self.kind == other.kind
            && self.interest_id == other.interest_id
            && self.tasks.same_broker(&other.tasks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demand_identity_includes_the_shared_broker() {
        let key = ResourceKey::scoped("resource-view-demand", "same-key");
        let first = SharedResourceTasks::new();
        let second = SharedResourceTasks::new();
        let left = ResourceViewDemand {
            tasks: first.clone(),
            key: key.clone(),
            kind: ResourceInterestKind::Visible,
            interest_id: 3,
        };
        let same = ResourceViewDemand {
            tasks: first,
            key: key.clone(),
            kind: ResourceInterestKind::Visible,
            interest_id: 3,
        };
        let foreign = ResourceViewDemand {
            tasks: second,
            key,
            kind: ResourceInterestKind::Visible,
            interest_id: 3,
        };
        assert!(left.same_demand(&same));
        assert!(!left.same_demand(&foreign));
    }
}
