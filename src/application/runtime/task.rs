//! Small keyed-task helpers for application-owned background work.

mod cancellation;
mod completion;
mod keyed_latest;
mod latest;
mod resource_tasks;

pub use cancellation::CancellationToken;
pub use completion::{KeyedTaskCompletion, TaskCompletion, TaskTicket};
pub use keyed_latest::KeyedLatestTasks;
pub use latest::LatestTask;
pub use resource_tasks::{ResourceTaskTicket, ResourceTasks};
