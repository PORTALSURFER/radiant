use crate::{
    application::{
        CancellationToken, KeyedLatestTasks, KeyedTaskCompletion, LatestTask, TaskCompletion,
        TaskTicket,
    },
    runtime::{Command, ResourceCompletion, ResourceCompletionParts, ResourceSlot, TaskPriority},
};

use super::UpdateContext;
use std::time::Duration;

impl<Message> UpdateContext<Message> {
    /// Run work on a runtime-managed business thread and map the output into a
    /// host message.
    ///
    /// Use this for slow work so the UI thread remains responsive. The result
    /// returns through the normal message path after the work completes.
    pub fn spawn<Output>(
        &mut self,
        name: &'static str,
        work: impl FnOnce() -> Output + Send + 'static,
        map: impl FnOnce(Output) -> Message + Send + 'static,
    ) where
        Output: Send + 'static,
    {
        self.spawn_with_priority(name, TaskPriority::Background, work, map);
    }

    /// Run work on a runtime-managed business thread with a scheduling hint.
    pub fn spawn_with_priority<Output>(
        &mut self,
        name: &'static str,
        priority: TaskPriority,
        work: impl FnOnce() -> Output + Send + 'static,
        map: impl FnOnce(Output) -> Message + Send + 'static,
    ) where
        Output: Send + 'static,
    {
        self.command(Command::perform_with_priority(name, priority, work, map));
    }

    /// Run cancellable work on a runtime-managed business thread.
    ///
    /// Hosts keep a clone of the token when they need to cancel the operation.
    /// The closure should check [`CancellationToken::is_cancelled`] at natural
    /// boundaries and return promptly when cancellation is requested.
    pub fn spawn_cancellable<Output>(
        &mut self,
        name: &'static str,
        token: CancellationToken,
        work: impl FnOnce(CancellationToken) -> Output + Send + 'static,
        map: impl FnOnce(Output) -> Message + Send + 'static,
    ) where
        Output: Send + 'static,
    {
        self.spawn_cancellable_with_priority(name, TaskPriority::Background, token, work, map);
    }

    /// Run cancellable work with a scheduling hint on a runtime-managed
    /// business thread.
    pub fn spawn_cancellable_with_priority<Output>(
        &mut self,
        name: &'static str,
        priority: TaskPriority,
        token: CancellationToken,
        work: impl FnOnce(CancellationToken) -> Output + Send + 'static,
        map: impl FnOnce(Output) -> Message + Send + 'static,
    ) where
        Output: Send + 'static,
    {
        self.spawn_with_priority(name, priority, move || work(token), map);
    }

    /// Start the latest task for one host-owned resource and run work on a
    /// runtime-managed business thread.
    ///
    /// The returned message receives a [`TaskCompletion`] tagged with the ticket
    /// created before the work started. Hosts can use [`LatestTask::finish`] to
    /// accept only the current completion and reject stale results.
    pub fn spawn_latest<Output>(
        &mut self,
        latest: &mut LatestTask,
        name: &'static str,
        work: impl FnOnce() -> Output + Send + 'static,
        map: impl FnOnce(TaskCompletion<Output>) -> Message + Send + 'static,
    ) where
        Output: Send + 'static,
    {
        let ticket = latest.begin();
        self.spawn(
            name,
            move || TaskCompletion {
                ticket,
                output: work(),
            },
            map,
        );
    }

    /// Start the latest task for one host-owned resource and run cancellable
    /// work on a runtime-managed business thread.
    ///
    /// The returned token should be stored by the host when it needs to request
    /// cooperative cancellation before the task completes.
    pub fn spawn_cancellable_latest_with_priority<Output>(
        &mut self,
        latest: &mut LatestTask,
        name: &'static str,
        priority: TaskPriority,
        work: impl FnOnce(TaskTicket, CancellationToken) -> Output + Send + 'static,
        map: impl FnOnce(TaskCompletion<Output>) -> Message + Send + 'static,
    ) -> CancellationToken
    where
        Output: Send + 'static,
    {
        let ticket = latest.begin();
        let token = CancellationToken::new();
        let worker_token = token.clone();
        self.spawn_cancellable_with_priority(
            name,
            priority,
            worker_token,
            move |token| TaskCompletion {
                ticket,
                output: work(ticket, token),
            },
            map,
        );
        token
    }

    /// Schedule a delayed message for the latest request for one host-owned
    /// resource.
    ///
    /// This is useful for debounced work such as selection previews, search
    /// requests, or inspector loads. The returned message receives the ticket
    /// created before the delay was scheduled. Hosts should accept it with
    /// [`LatestTask::finish`] or inspect it with [`LatestTask::is_active`] so
    /// older delayed messages cannot start stale work.
    pub fn after_latest(
        &mut self,
        latest: &mut LatestTask,
        delay: Duration,
        map: impl FnOnce(TaskTicket) -> Message,
    ) {
        let ticket = latest.begin();
        self.after(delay, map(ticket));
    }

    /// Start the latest task for one key in a keyed task registry and run work
    /// on a runtime-managed business thread.
    ///
    /// The returned message receives a keyed completion tagged with the key and
    /// ticket created before the work started. Hosts can use
    /// [`crate::application::KeyedLatestTasks::finish`] to accept only the
    /// current completion for that key and reject stale results.
    pub fn spawn_latest_for<Key, Output>(
        &mut self,
        latest: &mut KeyedLatestTasks<Key>,
        key: Key,
        name: &'static str,
        work: impl FnOnce() -> Output + Send + 'static,
        map: impl FnOnce(KeyedTaskCompletion<Key, Output>) -> Message + Send + 'static,
    ) where
        Key: Clone + Eq + std::hash::Hash + Send + 'static,
        Output: Send + 'static,
    {
        let ticket = latest.begin(key.clone());
        self.spawn(
            name,
            move || KeyedTaskCompletion {
                key,
                ticket,
                output: work(),
            },
            map,
        );
    }

    /// Start a resource load and run fallible work on a runtime-managed
    /// business thread.
    ///
    /// The returned message receives a [`ResourceCompletion`] tagged with the
    /// request created before the work started. Hosts should apply it with
    /// [`ResourceSlot::apply_for`] so older completions cannot overwrite newer
    /// requests for the same resource key.
    pub fn spawn_resource<Output>(
        &mut self,
        slot: &mut ResourceSlot<Output>,
        name: &'static str,
        work: impl FnOnce() -> Result<Output, String> + Send + 'static,
        map: impl FnOnce(ResourceCompletion<Output>) -> Message + Send + 'static,
    ) where
        Output: Send + 'static,
    {
        let request = slot.begin_load();
        self.spawn(
            name,
            move || {
                let load = match work() {
                    Ok(value) => request.ready(value),
                    Err(error) => request.failed(error),
                };
                ResourceCompletion::from_parts(ResourceCompletionParts { request, load })
            },
            map,
        );
    }
}
