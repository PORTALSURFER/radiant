use super::TaskTicket;

#[cfg(test)]
#[path = "latest/tests.rs"]
mod tests;

/// Tracks the latest in-flight task for one host-owned resource.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LatestTask {
    next_id: u64,
    active: Option<TaskTicket>,
}

impl Default for LatestTask {
    fn default() -> Self {
        Self::new()
    }
}

impl LatestTask {
    /// Build an idle task tracker.
    pub const fn new() -> Self {
        Self {
            next_id: 1,
            active: None,
        }
    }

    /// Start a new latest task and return its ticket.
    pub fn begin(&mut self) -> TaskTicket {
        let ticket = TaskTicket::new(self.next_id);
        self.next_id = self.next_id.saturating_add(1);
        self.active = Some(ticket);
        ticket
    }

    /// Return the currently active latest task, if any.
    pub const fn active(&self) -> Option<TaskTicket> {
        self.active
    }

    /// Return whether this ticket is still the active latest task.
    pub fn is_active(&self, ticket: TaskTicket) -> bool {
        self.active == Some(ticket)
    }

    /// Clear this task if `ticket` is still active.
    pub fn finish(&mut self, ticket: TaskTicket) -> bool {
        if self.is_active(ticket) {
            self.active = None;
            true
        } else {
            false
        }
    }

    /// Clear any active task.
    pub fn cancel(&mut self) {
        self.active = None;
    }
}
