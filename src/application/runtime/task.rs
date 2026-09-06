//! Small keyed-task helpers for application-owned background work.

mod cancellation;
mod completion;
mod keyed_latest;
mod latest;
pub(crate) mod resource_interests;
pub(crate) mod resource_operations;
mod resource_tasks;
mod shared_resource_tasks;
pub use shared_resource_tasks::{
    SharedResourceCompletion, SharedResourceTaskError, SharedResourceTaskMode,
};

pub use cancellation::CancellationToken;
pub use completion::{KeyedTaskCompletion, TaskCompletion, TaskTicket};
pub use keyed_latest::KeyedLatestTasks;
pub use latest::LatestTask;
pub(crate) use latest::LatestTaskTransactionSettlement;
pub(crate) use latest::{LatestTaskTransaction, LatestTimerTransaction};
pub use resource_tasks::{ResourceTaskTicket, ResourceTasks};
pub use shared_resource_tasks::{
    ResourceInterest, ResourceInterestError, ResourceInterestKind, SharedResourceTasks,
};
