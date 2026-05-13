const DEFAULT_STATUS_LINE_LIMIT: usize = 5;

/// Bounded one-line status message history for status bars and compact logs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StatusLineLog {
    entries: Vec<StatusLineEntry>,
    limit: usize,
}

/// One status-line event from a named UI, worker, or system source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StatusLineEntry {
    source: String,
    message: String,
}

impl StatusLineLog {
    /// Build a status-line log retaining at most `limit` entries.
    pub fn new(limit: usize) -> Self {
        Self {
            entries: vec![StatusLineEntry::new("system", "Ready")],
            limit: limit.max(1),
        }
    }

    /// Publish one compact status message from a named source.
    pub fn publish(&mut self, source: impl Into<String>, message: impl Into<String>) {
        self.entries
            .push(StatusLineEntry::new(source.into(), message.into()));
        let overflow = self.entries.len().saturating_sub(self.limit);
        if overflow > 0 {
            self.entries.drain(0..overflow);
        }
    }

    /// Return the latest status line formatted as `source: message`.
    pub fn latest(&self) -> String {
        self.entries
            .last()
            .map(StatusLineEntry::line)
            .unwrap_or_else(|| "system: Ready".to_string())
    }

    /// Return recent lines newest-first.
    pub fn recent_lines(&self) -> Vec<String> {
        self.entries
            .iter()
            .rev()
            .map(StatusLineEntry::line)
            .collect()
    }

    /// Return the number of retained entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Return whether this log has no retained entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Default for StatusLineLog {
    fn default() -> Self {
        Self::new(DEFAULT_STATUS_LINE_LIMIT)
    }
}

impl StatusLineEntry {
    /// Build a status-line entry from source and message text.
    pub fn new(source: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            message: message.into(),
        }
    }

    /// Return the status producer label.
    pub fn source(&self) -> &str {
        self.source.as_str()
    }

    /// Return the one-line status message.
    pub fn message(&self) -> &str {
        self.message.as_str()
    }

    /// Format this entry as `source: message` for compact status bars.
    pub fn line(&self) -> String {
        format!("{}: {}", self.source, self.message)
    }
}

#[cfg(test)]
mod tests {
    use super::{StatusLineEntry, StatusLineLog};

    #[test]
    fn status_line_log_keeps_latest_bounded_message() {
        let mut log = StatusLineLog::new(3);

        log.publish("button", "pressed");
        log.publish("worker", "started");
        log.publish("animation", "stopped");

        assert_eq!(log.len(), 3);
        assert_eq!(log.latest(), "animation: stopped");
        assert_eq!(
            log.recent_lines(),
            vec![
                "animation: stopped".to_string(),
                "worker: started".to_string(),
                "button: pressed".to_string()
            ]
        );
    }

    #[test]
    fn status_line_entry_exposes_source_message_and_line() {
        let entry = StatusLineEntry::new("worker", "finished");

        assert_eq!(entry.source(), "worker");
        assert_eq!(entry.message(), "finished");
        assert_eq!(entry.line(), "worker: finished");
    }
}
