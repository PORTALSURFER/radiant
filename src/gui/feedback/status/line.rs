use std::collections::VecDeque;

const DEFAULT_STATUS_LINE_LIMIT: usize = 5;

#[cfg(test)]
#[path = "line/tests.rs"]
mod tests;

/// Bounded one-line status message history for status bars and compact logs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StatusLineLog {
    entries: VecDeque<StatusLineEntry>,
    limit: usize,
}

/// One status-line event from a named UI, worker, or system source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StatusLineEntry {
    source: String,
    message: String,
    line: String,
}

/// Named fields for constructing a status-line entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StatusLineEntryParts {
    /// Status producer label.
    pub source: String,
    /// One-line status message.
    pub message: String,
}

impl StatusLineLog {
    /// Build a status-line log retaining at most `limit` entries.
    pub fn new(limit: usize) -> Self {
        let limit = limit.max(1);
        let mut entries = VecDeque::with_capacity(limit);
        entries.push_back(StatusLineEntry::new("system", "Ready"));
        Self { entries, limit }
    }

    /// Publish one compact status message from a named source.
    pub fn publish(&mut self, source: impl Into<String>, message: impl Into<String>) {
        if self.entries.len() == self.limit {
            self.entries.pop_front();
        }
        self.entries
            .push_back(StatusLineEntry::new(source.into(), message.into()));
    }

    /// Return the latest status line formatted as `source: message`.
    pub fn latest(&self) -> String {
        self.latest_line().to_owned()
    }

    /// Return the latest status line without allocating.
    pub fn latest_line(&self) -> &str {
        self.entries
            .back()
            .map(StatusLineEntry::line)
            .unwrap_or("system: Ready")
    }

    /// Iterate retained entries from oldest to newest without formatting them.
    pub fn entries(&self) -> impl DoubleEndedIterator<Item = &StatusLineEntry> {
        self.entries.iter()
    }

    /// Iterate retained entries from newest to oldest without formatting them.
    pub fn recent_entries(&self) -> impl Iterator<Item = &StatusLineEntry> {
        self.entries.iter().rev()
    }

    /// Return recent lines newest-first.
    pub fn recent_lines(&self) -> Vec<String> {
        let mut lines = Vec::new();
        self.recent_lines_into(&mut lines);
        lines
    }

    /// Write recent lines newest-first into caller-owned storage.
    ///
    /// The output is cleared before use and retains its allocation for repeated
    /// status-bar projection without changing the convenience
    /// [`Self::recent_lines`] API.
    pub fn recent_lines_into(&self, lines: &mut Vec<String>) {
        lines.clear();
        if self.entries.len() > lines.capacity() {
            lines.reserve(self.entries.len());
        }
        lines.extend(self.recent_entries().map(|entry| entry.line().to_string()));
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
    /// Build a status-line entry from named parts.
    ///
    /// Multiline input is normalized to one trimmed display line so background
    /// workers and actions cannot break compact status-bar layout.
    pub fn from_parts(parts: StatusLineEntryParts) -> Self {
        let source = one_line(parts.source);
        let message = one_line(parts.message);
        let line = format!("{source}: {message}");
        Self {
            source,
            message,
            line,
        }
    }

    /// Build a status-line entry from source and message text.
    ///
    /// Multiline input is normalized to one trimmed display line so background
    /// workers and actions cannot break compact status-bar layout.
    pub fn new(source: impl Into<String>, message: impl Into<String>) -> Self {
        Self::from_parts(StatusLineEntryParts {
            source: source.into(),
            message: message.into(),
        })
    }

    /// Return the status producer label.
    pub fn source(&self) -> &str {
        self.source.as_str()
    }

    /// Return the one-line status message.
    pub fn message(&self) -> &str {
        self.message.as_str()
    }

    /// Return the formatted `source: message` line for compact status bars.
    pub fn line(&self) -> &str {
        self.line.as_str()
    }
}

fn one_line(text: String) -> String {
    let mut line = String::with_capacity(text.len());
    for segment in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(segment);
    }
    line
}
