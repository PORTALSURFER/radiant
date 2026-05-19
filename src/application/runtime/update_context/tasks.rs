use crate::{
    application::{
        CancellationToken, KeyedLatestTasks, KeyedTaskCompletion, LatestTask, TaskCompletion,
    },
    runtime::{Command, ResourceCompletion, ResourceCompletionParts, ResourceSlot},
};

use super::UpdateContext;

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
        self.command(Command::perform(name, work, map));
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
        self.spawn(name, move || work(token), map);
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
