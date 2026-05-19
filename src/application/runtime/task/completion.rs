/// Monotonic ticket for a host background task.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TaskTicket {
    pub(super) id: u64,
}

impl TaskTicket {
    pub(super) const fn new(id: u64) -> Self {
        Self { id }
    }

    /// Numeric task id suitable for host messages, logs, or progress events.
    pub const fn id(self) -> u64 {
        self.id
    }
}

/// Completion payload tagged with the ticket that started the work.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskCompletion<Output> {
    /// Ticket assigned when the task was started.
    pub ticket: TaskTicket,
    /// Output returned by the background work.
    pub output: Output,
}

impl<Output> TaskCompletion<Output> {
    /// Numeric task id suitable for matching existing host message contracts.
    pub const fn task_id(&self) -> u64 {
        self.ticket.id()
    }
}

/// Completion payload tagged with the key and ticket that started the work.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyedTaskCompletion<Key, Output> {
    /// Host-defined key for the resource or operation.
    pub key: Key,
    /// Ticket assigned when the task was started for this key.
    pub ticket: TaskTicket,
    /// Output returned by the background work.
    pub output: Output,
}

impl<Key, Output> KeyedTaskCompletion<Key, Output> {
    /// Numeric task id suitable for matching existing host message contracts.
    pub const fn task_id(&self) -> u64 {
        self.ticket.id()
    }
}
