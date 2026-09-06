use std::{
    cell::Cell,
    collections::VecDeque,
    rc::{Rc, Weak},
    sync::Arc,
    time::Duration,
};

/// Stable application-chosen identity of a notice.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NoticeId(pub u64);

impl From<u64> for NoticeId {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

/// Presentation severity, independent of product error taxonomies.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum NoticeSeverity {
    Info,
    Success,
    Warning,
    Error,
    Critical,
}

/// Validated application-owned notice data.
#[derive(Clone, Debug)]
pub struct Notice {
    pub(crate) id: NoticeId,
    pub(crate) severity: NoticeSeverity,
    pub(crate) message: Arc<str>,
    pub(crate) timeout: Option<Duration>,
    pub(crate) action: Option<(Arc<str>, u64)>,
}

/// Rejected notice input or bounded queue admission.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoticeError {
    TextTooLong,
    InvalidTimeout,
    Capacity,
    RevisionExhausted,
}

impl std::fmt::Display for NoticeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::TextTooLong => "notice text exceeds its bound",
            Self::InvalidTimeout => "notice timeout must be positive and at most one day",
            Self::Capacity => "notice queue is full",
            Self::RevisionExhausted => "notice revision space is exhausted",
        })
    }
}
impl std::error::Error for NoticeError {}

impl Notice {
    /// Construct a notice with a five-second timeout, except critical notices.
    pub fn new(
        id: impl Into<NoticeId>,
        severity: NoticeSeverity,
        message: impl Into<Arc<str>>,
    ) -> Result<Self, NoticeError> {
        let message = message.into();
        if message.len() > 4096 {
            return Err(NoticeError::TextTooLong);
        }
        Ok(Self {
            id: id.into(),
            severity,
            message,
            timeout: (severity != NoticeSeverity::Critical).then_some(Duration::from_secs(5)),
            action: None,
        })
    }
    /// Use an explicit timeout or keep the notice until manual dismissal.
    pub fn timeout(mut self, timeout: Option<Duration>) -> Result<Self, NoticeError> {
        if timeout.is_some_and(|value| value.is_zero() || value > Duration::from_secs(86400)) {
            return Err(NoticeError::InvalidTimeout);
        }
        self.timeout = timeout;
        Ok(self)
    }
    /// Attach one semantic application command. The label is at most 128 bytes.
    pub fn action(mut self, label: impl Into<Arc<str>>, command: u64) -> Result<Self, NoticeError> {
        let label = label.into();
        if label.len() > 128 {
            return Err(NoticeError::TextTooLong);
        }
        self.action = Some((label, command));
        Ok(self)
    }
    pub const fn id(&self) -> NoticeId {
        self.id
    }
    pub fn message(&self) -> &str {
        &self.message
    }
    pub const fn severity(&self) -> NoticeSeverity {
        self.severity
    }
}

/// Mutable invalidation bit shared by every immutable projection of one entry.
#[derive(Debug)]
struct EntryLease {
    live: Cell<bool>,
}
impl EntryLease {
    fn new() -> Self {
        Self {
            live: Cell::new(true),
        }
    }
    fn invalidate(&self) {
        self.live.set(false);
    }
    fn is_live(&self) -> bool {
        self.live.get()
    }
}

#[derive(Clone)]
pub(crate) struct Entry {
    pub notice: Notice,
    pub revision: u64,
    lease: Rc<EntryLease>,
}
impl Entry {
    fn new(notice: Notice, revision: u64) -> Self {
        Self {
            notice,
            revision,
            lease: Rc::new(EntryLease::new()),
        }
    }
    fn invalidate(&self) {
        self.lease.invalidate();
    }
    fn replace_revision(&mut self, revision: u64) {
        self.invalidate();
        self.revision = revision;
        self.lease = Rc::new(EntryLease::new());
    }
    pub(crate) fn token(&self, queue: &Weak<()>) -> NoticeToken {
        NoticeToken {
            queue: queue.clone(),
            lease: Rc::downgrade(&self.lease),
            id: self.notice.id,
            revision: self.revision,
        }
    }
}

/// Application-owned bounded notice data. Snapshot projection never starts a timer.
pub struct NoticeQueue {
    identity: Rc<()>,
    entries: VecDeque<Entry>,
    next_revision: Option<u64>,
}
impl Default for NoticeQueue {
    fn default() -> Self {
        Self::new()
    }
}
impl NoticeQueue {
    /// Create an empty window-independent data queue with a 64-notice bound.
    pub fn new() -> Self {
        Self {
            identity: Rc::new(()),
            entries: VecDeque::new(),
            next_revision: Some(1),
        }
    }
    fn revision(&mut self) -> Result<u64, NoticeError> {
        let next = self.next_revision.ok_or(NoticeError::RevisionExhausted)?;
        self.next_revision = next.checked_add(1);
        Ok(next)
    }
    /// Insert or replace the same identity in its current queue position.
    /// Replacement invalidates prior actions and restarts visible timeout admission.
    pub fn push(&mut self, notice: Notice) -> Result<(), NoticeError> {
        let position = self
            .entries
            .iter()
            .position(|entry| entry.notice.id == notice.id);
        if position.is_none() && self.entries.len() >= 64 {
            return Err(NoticeError::Capacity);
        }
        let entry = Entry::new(notice, self.revision()?);
        if let Some(index) = position {
            self.entries[index].invalidate();
            self.entries[index] = entry;
        } else {
            self.entries.push_back(entry);
        }
        Ok(())
    }
    /// Remove a notice by explicit application decision.
    pub fn remove(&mut self, id: impl Into<NoticeId>) -> bool {
        let id = id.into();
        let mut removed = false;
        self.entries.retain(|entry| {
            let keep = entry.notice.id != id;
            if !keep {
                entry.invalidate();
                removed = true;
            }
            keep
        });
        removed
    }
    /// Apply one current typed dismissal. Duplicate or stale events are inert.
    pub fn dismiss(&mut self, event: &NoticeDismissal) -> bool {
        self.matches(&event.token) && self.remove(event.token.id)
    }
    /// Consume a current action once and return its application command ID.
    /// A fresh lease fences duplicate clicks before the next projection.
    pub fn take_action(&mut self, event: &NoticeAction) -> Option<u64> {
        if !self.matches(&event.token) {
            return None;
        }
        let index = self
            .entries
            .iter()
            .position(|entry| entry.notice.id == event.token.id)?;
        let command = self.entries[index].notice.action.as_ref()?.1;
        let revision = self.revision().ok()?;
        self.entries[index].replace_revision(revision);
        Some(command)
    }
    fn matches(&self, token: &NoticeToken) -> bool {
        token.is_live()
            && Weak::ptr_eq(&token.queue, &Rc::downgrade(&self.identity))
            && self.entries.iter().any(|entry| {
                entry.notice.id == token.id
                    && entry.revision == token.revision
                    && Weak::ptr_eq(&token.lease, &Rc::downgrade(&entry.lease))
            })
    }
    /// Immutable bounded data for `notifications(...)`.
    pub fn snapshot(&self) -> NoticeSnapshot {
        NoticeSnapshot {
            queue: Rc::downgrade(&self.identity),
            entries: self.entries.iter().cloned().collect(),
        }
    }
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Immutable owned notice snapshot. It does not keep its data queue alive.
#[derive(Clone)]
pub struct NoticeSnapshot {
    queue: Weak<()>,
    entries: Vec<Entry>,
}
impl NoticeSnapshot {
    /// Entries in stable application insertion order for framework projection.
    pub(crate) fn entries(&self) -> &[Entry] {
        &self.entries
    }
    /// Produce an exact entry token for a currently projected entry.
    pub(crate) fn token_for(&self, entry: &Entry) -> NoticeToken {
        entry.token(&self.queue)
    }

    /// Consume a snapshot into independently owned entry/token pairs for projection.
    pub(crate) fn into_entries_with_tokens(self) -> Vec<(Entry, NoticeToken)> {
        let queue = self.queue;
        self.entries
            .into_iter()
            .map(|entry| {
                let token = entry.token(&queue);
                (entry, token)
            })
            .collect()
    }
}

#[derive(Clone, Debug)]
pub(crate) struct NoticeToken {
    queue: Weak<()>,
    lease: Weak<EntryLease>,
    id: NoticeId,
    revision: u64,
}
impl NoticeToken {
    /// True only while both its queue and exact immutable entry lease remain current.
    pub(crate) fn is_live(&self) -> bool {
        self.queue.upgrade().is_some() && self.lease.upgrade().is_some_and(|lease| lease.is_live())
    }
    /// Exact source-entry comparison for retained framework state.
    pub(crate) fn same(&self, other: &Self) -> bool {
        self.id == other.id
            && self.revision == other.revision
            && Weak::ptr_eq(&self.queue, &other.queue)
            && Weak::ptr_eq(&self.lease, &other.lease)
    }
}

/// Cause of a typed dismissal event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoticeDismissalReason {
    User,
    Timeout,
}

/// Version-fenced dismissal delivered through the application's ordinary update.
#[derive(Clone, Debug)]
pub struct NoticeDismissal {
    pub(crate) token: NoticeToken,
    pub(crate) reason: NoticeDismissalReason,
}
impl NoticeDismissal {
    pub const fn id(&self) -> NoticeId {
        self.token.id
    }
    pub const fn reason(&self) -> NoticeDismissalReason {
        self.reason
    }
    pub(crate) fn is_live(&self) -> bool {
        self.token.is_live()
    }
}

/// Version-fenced activation of a notice's semantic command.
#[derive(Clone, Debug)]
pub struct NoticeAction {
    pub(crate) token: NoticeToken,
}
impl NoticeAction {
    pub const fn id(&self) -> NoticeId {
        self.token.id
    }
    pub(crate) fn is_live(&self) -> bool {
        self.token.is_live()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn notice(id: u64) -> Notice {
        Notice::new(id, NoticeSeverity::Info, format!("notice {id}")).unwrap()
    }
    fn action(snapshot: &NoticeSnapshot, index: usize) -> NoticeAction {
        NoticeAction {
            token: snapshot.token_for(&snapshot.entries()[index]),
        }
    }
    fn dismissal(snapshot: &NoticeSnapshot, index: usize) -> NoticeDismissal {
        NoticeDismissal {
            token: snapshot.token_for(&snapshot.entries()[index]),
            reason: NoticeDismissalReason::User,
        }
    }

    #[test]
    fn coalescing_replaces_in_place_and_invalidates_the_prior_projection() {
        let mut queue = NoticeQueue::new();
        queue.push(notice(1)).unwrap();
        queue.push(notice(2)).unwrap();
        let stale = queue.snapshot();
        let stale_action = action(&stale, 0);
        queue
            .push(
                Notice::new(1, NoticeSeverity::Warning, "replacement")
                    .unwrap()
                    .action("Retry", 7)
                    .unwrap(),
            )
            .unwrap();
        assert_eq!(queue.len(), 2);
        assert_eq!(
            queue.snapshot().entries()[0].notice.message(),
            "replacement"
        );
        assert!(!stale_action.is_live());
        assert_eq!(queue.take_action(&stale_action), None);
    }
    #[test]
    fn removal_invalidates_tokens_before_another_projection() {
        let mut queue = NoticeQueue::new();
        queue.push(notice(1)).unwrap();
        let snapshot = queue.snapshot();
        let event = dismissal(&snapshot, 0);
        assert!(queue.remove(1));
        assert!(!event.is_live());
        assert!(!queue.dismiss(&event));
    }
    #[test]
    fn action_is_single_use_before_a_fresh_projection() {
        let mut queue = NoticeQueue::new();
        queue.push(notice(1).action("Retry", 42).unwrap()).unwrap();
        let snapshot = queue.snapshot();
        let event = action(&snapshot, 0);
        assert_eq!(queue.take_action(&event), Some(42));
        assert!(!event.is_live());
        assert_eq!(queue.take_action(&event), None);
        let fresh = queue.snapshot();
        let fresh_event = action(&fresh, 0);
        assert!(fresh_event.is_live());
        assert_eq!(queue.take_action(&fresh_event), Some(42));
    }
    #[test]
    fn cross_queue_tokens_are_inert() {
        let mut first = NoticeQueue::new();
        let mut second = NoticeQueue::new();
        first.push(notice(1).action("Retry", 11).unwrap()).unwrap();
        second.push(notice(1).action("Retry", 22).unwrap()).unwrap();
        let snapshot = first.snapshot();
        let event = action(&snapshot, 0);
        assert_eq!(second.take_action(&event), None);
        assert!(event.is_live());
        assert_eq!(first.take_action(&event), Some(11));
    }
    #[test]
    fn snapshots_do_not_retain_the_queue_lifetime() {
        let (snapshot, event) = {
            let mut queue = NoticeQueue::new();
            queue.push(notice(1)).unwrap();
            let snapshot = queue.snapshot();
            let event = dismissal(&snapshot, 0);
            assert!(event.is_live());
            (snapshot, event)
        };
        assert_eq!(snapshot.entries().len(), 1);
        assert!(!event.is_live());
    }
    #[test]
    fn queue_capacity_and_timeout_bounds_are_enforced() {
        let mut queue = NoticeQueue::new();
        for id in 0..64 {
            queue.push(notice(id)).unwrap();
        }
        assert_eq!(queue.push(notice(64)), Err(NoticeError::Capacity));
        assert_eq!(queue.len(), 64);
        assert!(matches!(
            notice(65).timeout(Some(Duration::ZERO)),
            Err(NoticeError::InvalidTimeout)
        ));
        assert!(matches!(
            notice(65).timeout(Some(Duration::from_secs(86401))),
            Err(NoticeError::InvalidTimeout)
        ));
        assert!(notice(65).timeout(Some(Duration::from_secs(86400))).is_ok());
    }
    #[test]
    fn critical_notices_default_to_persistence() {
        let critical = Notice::new(1, NoticeSeverity::Critical, "critical").unwrap();
        assert_eq!(critical.timeout, None);
        assert_eq!(notice(2).timeout, Some(Duration::from_secs(5)));
    }
}
