//! Qualified deterministic infrastructure for exercising a [`SurfaceRuntime`]
//! without a native window, GPU, or operating-system scheduler.
//!
//! [`DeterministicHost`] keeps the existing production runtime controller at
//! the center of the test boundary. It supplies a fixed
//! [`WindowEnvironment`], a logical viewport, virtual time, and explicit
//! controls for worker and platform completions. Completion callbacks never
//! run while the host is admitting the completion; they become observable only
//! after a later [`DeterministicHost::turn`].
//!
//! The host is deliberately qualified rather than part of the normal runtime
//! prelude. It is test infrastructure, not a second production scheduler or a
//! native presentation backend.

use crate::{
    gui::{
        input::InputTimestamp,
        types::{Point, Rect, Vector2},
    },
    layout::{LayoutDiagnosticCode, MainAlign, OverflowPolicy},
    runtime::{
        Command, CommandOutcome, Event, GuiAutomationSnapshot, GuiAutomationTargetSnapshot,
        PlatformRequest, PlatformResponse, PlatformResult, RepaintScope, RuntimeBridge,
        RuntimeDiagnostics, RuntimeLifecyclePhase, RuntimePlatformResultHost,
        RuntimePlatformResultSink, RuntimeQueueHost, RuntimeQueueItem, RuntimeTaskHost,
        SurfaceIdentityDiagnostics, SurfaceIdentityPath, SurfaceIdentityReplacement,
        SurfaceLayoutStateDiagnostics, SurfaceLayoutStateReplacement, SurfaceRefreshCounters,
        SurfaceRefreshDiagnostics, SurfaceRuntime, TaskPriority, UiSurface,
        UiUpdateHandlerDiagnosticsPolicy, WindowColorScheme, WindowEnvironment,
    },
    theme::DpiScale,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{BTreeSet, VecDeque},
    fmt,
    sync::Arc,
    time::{Duration, Instant},
};

/// Version of the normalized deterministic-host snapshot schema.
pub const NORMALIZED_SNAPSHOT_SCHEMA_VERSION: u32 = 1;

const DEFAULT_VIEWPORT: Vector2 = Vector2 { x: 800.0, y: 600.0 };
const DEFAULT_PENDING_WORKERS: usize = 64;
const DEFAULT_PENDING_PLATFORM_REQUESTS: usize = 64;
const DEFAULT_PENDING_TIMERS: usize = 64;
const DEFAULT_PENDING_QUEUE_ITEMS: usize = 64;
const DEFAULT_STEP_BUDGET: usize = 128;

/// A lane with an explicit deterministic-host capacity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeterministicLane {
    /// Explicitly controlled worker tasks.
    Workers,
    /// Explicitly controlled platform requests.
    Platform,
    /// Virtual timer registrations.
    Timers,
    /// Messages, timer wakes, and other host queue items.
    Queue,
}

impl fmt::Display for DeterministicLane {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Workers => "workers",
            Self::Platform => "platform",
            Self::Timers => "timers",
            Self::Queue => "queue",
        })
    }
}

/// Validation failures for [`DeterministicHostConfig`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeterministicHostConfigError {
    /// The configured logical viewport is not finite and strictly positive.
    InvalidViewport,
    /// The configured display scale is not finite and strictly positive.
    InvalidDisplayScale,
    /// One of the bounded host lanes was configured with zero capacity.
    ZeroCapacity {
        /// Lane whose capacity was invalid.
        lane: DeterministicLane,
    },
    /// The queue cannot admit every registered timer if they become due together.
    TimerQueueCapacityMismatch {
        /// Maximum number of timer registrations that may be pending.
        timers: usize,
        /// Maximum number of pending queue items.
        queue: usize,
    },
    /// The configured `run_until_idle` budget was zero.
    ZeroStepBudget,
}

impl fmt::Display for DeterministicHostConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidViewport => formatter.write_str("viewport must be finite and positive"),
            Self::InvalidDisplayScale => {
                formatter.write_str("display scale must be finite and positive")
            }
            Self::ZeroCapacity { lane } => write!(formatter, "{lane} capacity must be positive"),
            Self::TimerQueueCapacityMismatch { timers, queue } => write!(
                formatter,
                "timer capacity ({timers}) must not exceed queue capacity ({queue})"
            ),
            Self::ZeroStepBudget => formatter.write_str("step budget must be positive"),
        }
    }
}

impl std::error::Error for DeterministicHostConfigError {}

/// Fixed environment and bounded-work configuration for a deterministic host.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DeterministicHostConfig {
    viewport: Vector2,
    environment: WindowEnvironment,
    max_pending_workers: usize,
    max_pending_platform_requests: usize,
    max_pending_timers: usize,
    max_pending_queue_items: usize,
    step_budget: usize,
}

impl DeterministicHostConfig {
    /// Build a configuration with a fixed logical viewport and shipped default
    /// [`WindowEnvironment`].
    pub fn new(viewport: Vector2) -> Self {
        Self {
            viewport,
            ..Self::default()
        }
    }

    /// Replace the fixed window environment used before the first projection.
    pub fn with_environment(mut self, environment: WindowEnvironment) -> Self {
        self.environment = environment;
        self
    }

    /// Set the maximum number of worker tasks awaiting explicit completion.
    pub fn with_max_pending_workers(mut self, limit: usize) -> Self {
        self.max_pending_workers = limit;
        self
    }

    /// Set the maximum number of platform requests awaiting explicit completion.
    pub fn with_max_pending_platform_requests(mut self, limit: usize) -> Self {
        self.max_pending_platform_requests = limit;
        self
    }

    /// Set the maximum number of timer registrations awaiting virtual time.
    pub fn with_max_pending_timers(mut self, limit: usize) -> Self {
        self.max_pending_timers = limit;
        self
    }

    /// Set the maximum number of directly queued host items.
    pub fn with_max_pending_queue_items(mut self, limit: usize) -> Self {
        self.max_pending_queue_items = limit;
        self
    }

    /// Set the maximum number of turns allowed by [`DeterministicHost::run_until_idle`].
    pub fn with_step_budget(mut self, budget: usize) -> Self {
        self.step_budget = budget;
        self
    }

    /// Return the fixed logical viewport.
    pub const fn viewport(self) -> Vector2 {
        self.viewport
    }

    /// Return the fixed shipped window environment.
    pub const fn environment(self) -> WindowEnvironment {
        self.environment
    }

    /// Return the worker-task capacity.
    pub const fn max_pending_workers(self) -> usize {
        self.max_pending_workers
    }

    /// Return the platform-request capacity.
    pub const fn max_pending_platform_requests(self) -> usize {
        self.max_pending_platform_requests
    }

    /// Return the timer-registration capacity.
    pub const fn max_pending_timers(self) -> usize {
        self.max_pending_timers
    }

    /// Return the directly queued-item capacity.
    pub const fn max_pending_queue_items(self) -> usize {
        self.max_pending_queue_items
    }

    /// Return the bounded `run_until_idle` step budget.
    pub const fn step_budget(self) -> usize {
        self.step_budget
    }

    /// Validate the fixed geometry, environment, and work bounds.
    pub fn validate(self) -> Result<(), DeterministicHostConfigError> {
        if !self.viewport.x.is_finite()
            || !self.viewport.y.is_finite()
            || self.viewport.x <= 0.0
            || self.viewport.y <= 0.0
        {
            return Err(DeterministicHostConfigError::InvalidViewport);
        }
        let scale = self.environment.display_scale().factor();
        if !scale.is_finite() || scale <= 0.0 {
            return Err(DeterministicHostConfigError::InvalidDisplayScale);
        }
        for (lane, limit) in [
            (DeterministicLane::Workers, self.max_pending_workers),
            (
                DeterministicLane::Platform,
                self.max_pending_platform_requests,
            ),
            (DeterministicLane::Timers, self.max_pending_timers),
            (DeterministicLane::Queue, self.max_pending_queue_items),
        ] {
            if limit == 0 {
                return Err(DeterministicHostConfigError::ZeroCapacity { lane });
            }
        }
        if self.step_budget == 0 {
            return Err(DeterministicHostConfigError::ZeroStepBudget);
        }
        if self.max_pending_timers > self.max_pending_queue_items {
            return Err(DeterministicHostConfigError::TimerQueueCapacityMismatch {
                timers: self.max_pending_timers,
                queue: self.max_pending_queue_items,
            });
        }
        Ok(())
    }
}

impl Default for DeterministicHostConfig {
    fn default() -> Self {
        Self {
            viewport: DEFAULT_VIEWPORT,
            environment: WindowEnvironment::default(),
            max_pending_workers: DEFAULT_PENDING_WORKERS,
            max_pending_platform_requests: DEFAULT_PENDING_PLATFORM_REQUESTS,
            max_pending_timers: DEFAULT_PENDING_TIMERS,
            max_pending_queue_items: DEFAULT_PENDING_QUEUE_ITEMS,
            step_budget: DEFAULT_STEP_BUDGET,
        }
    }
}

/// Errors returned by deterministic-host admission, completion, and snapshot operations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DeterministicHostError {
    /// The fixed host configuration failed validation.
    InvalidConfiguration(DeterministicHostConfigError),
    /// An event contained a non-finite or otherwise unsupported value.
    InvalidEvent(&'static str),
    /// The runtime no longer accepts UI work.
    RuntimeNotAcceptingWork,
    /// A bounded host lane could not admit another item.
    Capacity {
        /// Lane that reached its configured bound.
        lane: DeterministicLane,
        /// Configured capacity for that lane.
        limit: usize,
    },
    /// A monotonic host identifier could not be allocated.
    IdentifierOverflow,
    /// Virtual time or its `Instant` anchor could not advance.
    TimeOverflow,
    /// A worker id was not awaiting completion.
    UnknownWorker(WorkerTaskId),
    /// A worker completion was submitted more than once.
    DuplicateWorkerCompletion(WorkerTaskId),
    /// A platform request id was not awaiting completion.
    UnknownPlatformRequest(PlatformRequestId),
    /// A platform completion was submitted more than once.
    DuplicatePlatformCompletion(PlatformRequestId),
    /// A platform response did not match its request kind.
    IncompatiblePlatformResponse {
        /// Request whose response was rejected.
        request: PlatformRequestId,
    },
    /// `run_until_idle` reached its configured turn budget.
    StepBudgetExceeded {
        /// Budget that was exhausted.
        budget: usize,
    },
    /// A normalized output contained a non-finite numeric value.
    NonFiniteOutput(&'static str),
    /// Snapshot serialization failed.
    SnapshotSerialization(String),
}

impl fmt::Display for DeterministicHostError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfiguration(error) => {
                write!(formatter, "invalid host configuration: {error}")
            }
            Self::InvalidEvent(reason) => {
                write!(formatter, "invalid deterministic event: {reason}")
            }
            Self::RuntimeNotAcceptingWork => formatter.write_str("runtime does not accept work"),
            Self::Capacity { lane, limit } => {
                write!(formatter, "{lane} capacity exhausted at {limit}")
            }
            Self::IdentifierOverflow => {
                formatter.write_str("deterministic host identifier overflow")
            }
            Self::TimeOverflow => formatter.write_str("deterministic virtual time overflow"),
            Self::UnknownWorker(id) => write!(formatter, "unknown worker task id {}", id.get()),
            Self::DuplicateWorkerCompletion(id) => {
                write!(formatter, "duplicate worker completion for id {}", id.get())
            }
            Self::UnknownPlatformRequest(id) => {
                write!(formatter, "unknown platform request id {}", id.get())
            }
            Self::DuplicatePlatformCompletion(id) => {
                write!(
                    formatter,
                    "duplicate platform completion for id {}",
                    id.get()
                )
            }
            Self::IncompatiblePlatformResponse { request } => write!(
                formatter,
                "incompatible platform response for request id {}",
                request.get()
            ),
            Self::StepBudgetExceeded { budget } => {
                write!(
                    formatter,
                    "deterministic host step budget exhausted at {budget}"
                )
            }
            Self::NonFiniteOutput(field) => {
                write!(formatter, "non-finite normalized output in {field}")
            }
            Self::SnapshotSerialization(error) => {
                write!(formatter, "serialize normalized snapshot: {error}")
            }
        }
    }
}

impl std::error::Error for DeterministicHostError {}

impl From<DeterministicHostConfigError> for DeterministicHostError {
    fn from(error: DeterministicHostConfigError) -> Self {
        Self::InvalidConfiguration(error)
    }
}

/// Stable id assigned to one explicitly controlled worker task.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct WorkerTaskId(u64);

impl WorkerTaskId {
    /// Return the numeric id for diagnostics and test assertions.
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Stable id assigned to one explicitly controlled platform request.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PlatformRequestId(u64);

impl PlatformRequestId {
    /// Return the numeric id for diagnostics and test assertions.
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Metadata for a worker task awaiting an explicit completion action.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PendingWorkerTask {
    /// Host-assigned task id.
    pub id: WorkerTaskId,
    /// Stable application task label supplied to the runtime scheduler.
    pub name: &'static str,
    /// Runtime scheduling priority supplied to the host.
    pub priority: TaskPriority,
}

/// Metadata for a platform request awaiting an explicit result action.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingPlatformRequest {
    /// Host-assigned request id.
    pub id: PlatformRequestId,
    /// Platform-neutral request retained by the deterministic host.
    pub request: PlatformRequest,
}

#[derive(Clone, Copy)]
struct AdapterLimits {
    max_pending_workers: usize,
    max_pending_platform_requests: usize,
    max_pending_timers: usize,
    max_pending_queue_items: usize,
}

impl From<DeterministicHostConfig> for AdapterLimits {
    fn from(config: DeterministicHostConfig) -> Self {
        Self {
            max_pending_workers: config.max_pending_workers,
            max_pending_platform_requests: config.max_pending_platform_requests,
            max_pending_timers: config.max_pending_timers,
            max_pending_queue_items: config.max_pending_queue_items,
        }
    }
}

#[derive(Clone, Copy)]
struct PendingTimer {
    due: Duration,
    sequence: u64,
    wake: crate::runtime::RuntimeTimerWake,
}

struct PendingWorker {
    info: PendingWorkerTask,
    work: Option<Box<dyn FnOnce() + Send + 'static>>,
}

struct PendingPlatform {
    info: PendingPlatformRequest,
    sink: Option<RuntimePlatformResultSink>,
}

enum AdapterFailure {
    Capacity {
        lane: DeterministicLane,
        limit: usize,
    },
    IdentifierOverflow,
    TimeOverflow,
}

/// RuntimeBridge adapter that replaces host schedulers with explicit test controls.
///
/// The adapter delegates projection, reduction, and environment observation to
/// `Bridge`. It only supplies the task, queue, and result-only platform
/// capabilities needed by [`DeterministicHost`].
pub struct DeterministicBridge<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    inner: Bridge,
    limits: AdapterLimits,
    now: Duration,
    next_worker_id: u64,
    next_platform_id: u64,
    next_timer_sequence: u64,
    workers: VecDeque<PendingWorker>,
    platform_requests: VecDeque<PendingPlatform>,
    timers: Vec<PendingTimer>,
    queue_items: VecDeque<RuntimeQueueItem<Message>>,
    commands: VecDeque<Command<Message>>,
    completed_workers: BTreeSet<WorkerTaskId>,
    completed_platform_requests: BTreeSet<PlatformRequestId>,
    failure: Option<AdapterFailure>,
}

impl<Bridge, Message> DeterministicBridge<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    fn new(inner: Bridge, limits: AdapterLimits) -> Self {
        Self {
            inner,
            limits,
            now: Duration::ZERO,
            next_worker_id: 1,
            next_platform_id: 1,
            next_timer_sequence: 1,
            workers: VecDeque::new(),
            platform_requests: VecDeque::new(),
            timers: Vec::new(),
            queue_items: VecDeque::new(),
            commands: VecDeque::new(),
            completed_workers: BTreeSet::new(),
            completed_platform_requests: BTreeSet::new(),
            failure: None,
        }
    }

    /// Borrow the wrapped application bridge.
    pub fn inner(&self) -> &Bridge {
        &self.inner
    }

    /// Mutably borrow the wrapped application bridge.
    pub fn inner_mut(&mut self) -> &mut Bridge {
        &mut self.inner
    }

    /// Consume the adapter and return the wrapped application bridge.
    pub fn into_inner(self) -> Bridge {
        self.inner
    }

    fn set_now(&mut self, now: Duration) {
        self.now = now;
    }

    fn allocate_id(next: &mut u64) -> Result<u64, AdapterFailure> {
        let id = *next;
        *next = next
            .checked_add(1)
            .ok_or(AdapterFailure::IdentifierOverflow)?;
        Ok(id)
    }

    fn record_failure(&mut self, failure: AdapterFailure) {
        if self.failure.is_none() {
            self.failure = Some(failure);
        }
    }

    fn take_failure(&mut self) -> Option<DeterministicHostError> {
        self.failure.take().map(|failure| match failure {
            AdapterFailure::Capacity { lane, limit } => {
                DeterministicHostError::Capacity { lane, limit }
            }
            AdapterFailure::IdentifierOverflow => DeterministicHostError::IdentifierOverflow,
            AdapterFailure::TimeOverflow => DeterministicHostError::TimeOverflow,
        })
    }

    fn pending_worker_tasks(&self) -> Vec<PendingWorkerTask> {
        self.workers.iter().map(|worker| worker.info).collect()
    }

    fn pending_platform_requests(&self) -> Vec<PendingPlatformRequest> {
        self.platform_requests
            .iter()
            .map(|request| request.info.clone())
            .collect()
    }

    fn pending_timer_count(&self) -> usize {
        self.timers.len()
    }

    fn queue_item_count(&self) -> usize {
        self.queue_items.len().saturating_add(self.commands.len())
    }

    fn queue_reservation_count(&self) -> usize {
        self.queue_item_count().saturating_add(self.timers.len())
    }

    fn can_admit_queue_reservation(&self, additional: usize) -> bool {
        self.queue_reservation_count().saturating_add(additional)
            <= self.limits.max_pending_queue_items
    }

    fn enqueue_message(&mut self, message: Message) -> Result<(), DeterministicHostError> {
        if !self.can_admit_queue_reservation(1) {
            return Err(DeterministicHostError::Capacity {
                lane: DeterministicLane::Queue,
                limit: self.limits.max_pending_queue_items,
            });
        }
        self.queue_items
            .push_back(RuntimeQueueItem::Message(message));
        Ok(())
    }

    fn enqueue_command(&mut self, command: Command<Message>) -> Result<(), DeterministicHostError> {
        if !self.can_admit_queue_reservation(1) {
            return Err(DeterministicHostError::Capacity {
                lane: DeterministicLane::Queue,
                limit: self.limits.max_pending_queue_items,
            });
        }
        self.commands.push_back(command);
        Ok(())
    }

    fn release_due_timers(&mut self) -> Result<(), DeterministicHostError> {
        let due_count = self
            .timers
            .iter()
            .filter(|timer| timer.due <= self.now)
            .count();
        let capacity = self.queue_item_count().saturating_add(due_count);
        if capacity > self.limits.max_pending_queue_items {
            return Err(DeterministicHostError::Capacity {
                lane: DeterministicLane::Queue,
                limit: self.limits.max_pending_queue_items,
            });
        }
        self.timers.sort_by_key(|timer| (timer.due, timer.sequence));
        let mut due = Vec::new();
        let mut future = Vec::with_capacity(self.timers.len().saturating_sub(due_count));
        for timer in self.timers.drain(..) {
            if timer.due <= self.now {
                due.push(timer);
            } else {
                future.push(timer);
            }
        }
        self.timers = future;
        self.queue_items.extend(
            due.into_iter()
                .map(|timer| RuntimeQueueItem::Timer(timer.wake)),
        );
        Ok(())
    }

    fn complete_worker(&mut self, id: WorkerTaskId) -> Result<(), DeterministicHostError> {
        let Some(index) = self.workers.iter().position(|worker| worker.info.id == id) else {
            return if self.completed_workers.contains(&id) {
                Err(DeterministicHostError::DuplicateWorkerCompletion(id))
            } else {
                Err(DeterministicHostError::UnknownWorker(id))
            };
        };
        let mut worker = self
            .workers
            .remove(index)
            .ok_or(DeterministicHostError::UnknownWorker(id))?;
        self.completed_workers.insert(id);
        if let Some(work) = worker.work.take() {
            work();
        }
        Ok(())
    }

    fn complete_platform_request(
        &mut self,
        id: PlatformRequestId,
        result: PlatformResult,
    ) -> Result<(), DeterministicHostError> {
        let Some(index) = self
            .platform_requests
            .iter()
            .position(|request| request.info.id == id)
        else {
            return if self.completed_platform_requests.contains(&id) {
                Err(DeterministicHostError::DuplicatePlatformCompletion(id))
            } else {
                Err(DeterministicHostError::UnknownPlatformRequest(id))
            };
        };
        let request = &self.platform_requests[index].info.request;
        if !platform_result_matches(request, &result) {
            return Err(DeterministicHostError::IncompatiblePlatformResponse { request: id });
        }
        let mut pending = self
            .platform_requests
            .remove(index)
            .ok_or(DeterministicHostError::UnknownPlatformRequest(id))?;
        self.completed_platform_requests.insert(id);
        if let Some(sink) = pending.sink.take() {
            sink.send(result);
        }
        Ok(())
    }
}

impl<Bridge, Message> RuntimeBridge<Message> for DeterministicBridge<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    fn project_surface(&mut self) -> Arc<UiSurface<Message>> {
        self.inner.project_surface()
    }

    fn pull_surface(&mut self) -> UiSurface<Message> {
        self.inner.pull_surface()
    }

    fn reduce_message(&mut self, message: Message) {
        self.inner.reduce_message(message);
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        self.inner.update(message)
    }

    fn update_with_runtime(
        &mut self,
        message: Message,
        snapshot: crate::runtime::RuntimeUpdateSnapshot,
    ) -> Command<Message> {
        self.inner.update_with_runtime(message, snapshot)
    }

    fn window_environment_changed(&mut self, environment: WindowEnvironment) {
        self.inner.window_environment_changed(environment);
    }

    fn set_window_environment(&mut self, environment: WindowEnvironment) {
        self.inner.set_window_environment(environment);
    }

    fn host_capabilities(&self) -> crate::runtime::RuntimeHostCapabilities<Self, Message> {
        crate::runtime::RuntimeHostCapabilities::new()
            .with_tasks()
            .with_queues()
            .with_platform_results()
    }
}

impl<Bridge, Message> RuntimeTaskHost<Message> for DeterministicBridge<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    fn schedule_timer(&mut self, delay: Duration, wake: crate::runtime::RuntimeTimerWake) -> bool {
        if self.timers.len() >= self.limits.max_pending_timers {
            self.record_failure(AdapterFailure::Capacity {
                lane: DeterministicLane::Timers,
                limit: self.limits.max_pending_timers,
            });
            return false;
        }
        if !self.can_admit_queue_reservation(1) {
            self.record_failure(AdapterFailure::Capacity {
                lane: DeterministicLane::Queue,
                limit: self.limits.max_pending_queue_items,
            });
            return false;
        }
        let Some(due) = self.now.checked_add(delay) else {
            self.record_failure(AdapterFailure::TimeOverflow);
            return false;
        };
        let Ok(sequence) = Self::allocate_id(&mut self.next_timer_sequence) else {
            self.record_failure(AdapterFailure::IdentifierOverflow);
            return false;
        };
        self.timers.push(PendingTimer {
            due,
            sequence,
            wake,
        });
        true
    }

    fn spawn_worker_task(
        &mut self,
        name: &'static str,
        priority: TaskPriority,
        _is_cancelled: Option<Box<dyn Fn() -> bool + Send + Sync + 'static>>,
        work: Box<dyn FnOnce() + Send + 'static>,
    ) -> bool {
        if self.workers.len() >= self.limits.max_pending_workers {
            self.record_failure(AdapterFailure::Capacity {
                lane: DeterministicLane::Workers,
                limit: self.limits.max_pending_workers,
            });
            return false;
        }
        let Ok(id) = Self::allocate_id(&mut self.next_worker_id) else {
            self.record_failure(AdapterFailure::IdentifierOverflow);
            return false;
        };
        self.workers.push_back(PendingWorker {
            info: PendingWorkerTask {
                id: WorkerTaskId(id),
                name,
                priority,
            },
            work: Some(work),
        });
        true
    }
}

impl<Bridge, Message> RuntimePlatformResultHost for DeterministicBridge<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    fn request_platform_result(
        &mut self,
        request: PlatformRequest,
        sink: RuntimePlatformResultSink,
    ) -> Result<(), crate::runtime::PlatformResultServiceFallback> {
        if self.platform_requests.len() >= self.limits.max_pending_platform_requests {
            self.record_failure(AdapterFailure::Capacity {
                lane: DeterministicLane::Platform,
                limit: self.limits.max_pending_platform_requests,
            });
            return Err(Box::new((request, sink)));
        }
        let Ok(id) = Self::allocate_id(&mut self.next_platform_id) else {
            self.record_failure(AdapterFailure::IdentifierOverflow);
            return Err(Box::new((request, sink)));
        };
        self.platform_requests.push_back(PendingPlatform {
            info: PendingPlatformRequest {
                id: PlatformRequestId(id),
                request,
            },
            sink: Some(sink),
        });
        Ok(())
    }
}

impl<Bridge, Message> RuntimeQueueHost<Message> for DeterministicBridge<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    fn take_runtime_commands(&mut self) -> Vec<Command<Message>> {
        self.commands.drain(..).collect()
    }

    fn take_runtime_messages(&mut self) -> Vec<Message> {
        let mut messages = Vec::new();
        let mut retained = VecDeque::with_capacity(self.queue_items.len());
        for item in self.queue_items.drain(..) {
            match item {
                RuntimeQueueItem::Message(message) => messages.push(message),
                other => retained.push_back(other),
            }
        }
        self.queue_items = retained;
        messages
    }

    fn take_runtime_timer_wakes(&mut self) -> Vec<crate::runtime::RuntimeTimerWake> {
        let mut wakes = Vec::new();
        let mut retained = VecDeque::with_capacity(self.queue_items.len());
        for item in self.queue_items.drain(..) {
            match item {
                RuntimeQueueItem::Timer(wake) => wakes.push(wake),
                other => retained.push_back(other),
            }
        }
        self.queue_items = retained;
        wakes
    }

    fn drain_runtime_queue_item_batch_into(
        &mut self,
        items: &mut Vec<RuntimeQueueItem<Message>>,
        max_items: usize,
    ) -> bool {
        let count = max_items.max(1).min(self.queue_items.len());
        items.extend(self.queue_items.drain(..count));
        !self.queue_items.is_empty()
    }
}

/// Deterministic host for production `SurfaceRuntime` dispatch and normalized snapshots.
pub struct DeterministicHost<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    runtime: SurfaceRuntime<DeterministicBridge<Bridge, Message>, Message>,
    config: DeterministicHostConfig,
    virtual_time: Duration,
    instant_origin: Instant,
    turn: u64,
    pending_outcome: CommandOutcome,
    last_outcome: CommandOutcome,
    application_observation: Option<Value>,
    published_snapshot: NormalizedSnapshot,
}

impl<Bridge, Message> DeterministicHost<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    /// Build a deterministic host and validate all fixed bounds before projection.
    pub fn new(
        bridge: Bridge,
        config: DeterministicHostConfig,
    ) -> Result<Self, DeterministicHostError> {
        config.validate()?;
        let environment = config.environment();
        let instant_origin = Instant::now();
        let mut runtime = SurfaceRuntime::new_with_environment(
            DeterministicBridge::new(bridge, config.into()),
            config.viewport(),
            environment,
        );
        runtime.set_timed_repaint_clock(Some(instant_origin));
        runtime.set_update_handler_diagnostics_policy(UiUpdateHandlerDiagnosticsPolicy::disabled());
        let published_snapshot = build_normalized_snapshot(
            &runtime,
            Duration::ZERO,
            0,
            CommandOutcome::default(),
            None,
        )?;
        Ok(Self {
            runtime,
            config,
            virtual_time: Duration::ZERO,
            instant_origin,
            turn: 0,
            pending_outcome: CommandOutcome::default(),
            last_outcome: CommandOutcome::default(),
            application_observation: None,
            published_snapshot,
        })
    }

    /// Build a deterministic host with default limits and the supplied viewport.
    pub fn with_default_config(
        bridge: Bridge,
        viewport: Vector2,
    ) -> Result<Self, DeterministicHostError> {
        Self::new(bridge, DeterministicHostConfig::new(viewport))
    }

    /// Borrow the production runtime controller.
    pub fn runtime(&self) -> &SurfaceRuntime<DeterministicBridge<Bridge, Message>, Message> {
        &self.runtime
    }

    /// Borrow the wrapped application bridge.
    pub fn bridge(&self) -> &Bridge {
        self.runtime.bridge().inner()
    }

    /// Mutably borrow the wrapped application bridge.
    pub fn bridge_mut(&mut self) -> &mut Bridge {
        self.runtime.bridge_mut().inner_mut()
    }

    /// Return the validated host configuration.
    pub const fn config(&self) -> DeterministicHostConfig {
        self.config
    }

    /// Return the current virtual time.
    pub const fn virtual_time(&self) -> Duration {
        self.virtual_time
    }

    /// Return the number of completed runtime turns.
    pub const fn turn_count(&self) -> u64 {
        self.turn
    }

    /// Return the currently published normalized snapshot.
    ///
    /// Completion actions do not change this value. Call [`Self::turn`] to
    /// atomically publish the next candidate snapshot.
    pub fn published_snapshot(&self) -> &NormalizedSnapshot {
        &self.published_snapshot
    }

    /// Return pending worker tasks in deterministic admission order.
    pub fn pending_worker_tasks(&self) -> Vec<PendingWorkerTask> {
        self.runtime.bridge().pending_worker_tasks()
    }

    /// Return pending platform requests in deterministic admission order.
    pub fn pending_platform_requests(&self) -> Vec<PendingPlatformRequest> {
        self.runtime.bridge().pending_platform_requests()
    }

    /// Return the number of timer registrations awaiting virtual time.
    pub fn pending_timer_count(&self) -> usize {
        self.runtime.bridge().pending_timer_count()
    }

    /// Return the number of explicitly queued commands, messages, or timer wakes.
    pub fn pending_queue_item_count(&self) -> usize {
        self.runtime.bridge().queue_item_count()
    }

    /// Retain a caller-supplied JSON observation in future normalized snapshots.
    pub fn set_application_observation(&mut self, observation: Value) {
        self.application_observation = Some(observation);
    }

    /// Remove the caller-supplied JSON observation from future snapshots.
    pub fn clear_application_observation(&mut self) {
        self.application_observation = None;
    }

    /// Return the currently configured caller-supplied JSON observation.
    pub fn application_observation(&self) -> Option<&Value> {
        self.application_observation.as_ref()
    }

    /// Dispatch one normalized event through production `SurfaceRuntime` routing.
    pub fn dispatch_event(
        &mut self,
        event: Event,
    ) -> Result<Option<crate::widgets::WidgetId>, DeterministicHostError> {
        self.ensure_runtime_accepts_work()?;
        validate_event(event)?;
        let now = self.prepare_runtime_operation()?;
        let event = with_virtual_timestamp(event, InputTimestamp::from_instant(now));
        let target = self.runtime.dispatch_event(event);
        self.pending_outcome
            .merge(self.runtime.take_pending_input_command_outcome());
        self.finish_adapter_operation(target)
    }

    /// Dispatch one host message through production reduction and refresh handling.
    pub fn dispatch_message(
        &mut self,
        message: Message,
    ) -> Result<CommandOutcome, DeterministicHostError> {
        self.ensure_runtime_accepts_work()?;
        self.prepare_runtime_operation()?;
        let outcome = self.runtime.dispatch_message(message);
        self.pending_outcome.merge(outcome);
        self.finish_adapter_operation(outcome)
    }

    /// Execute one runtime command through production command handling.
    pub fn execute_command(
        &mut self,
        command: Command<Message>,
    ) -> Result<CommandOutcome, DeterministicHostError> {
        self.ensure_runtime_accepts_work()?;
        self.prepare_runtime_operation()?;
        let outcome = self.runtime.execute_command(command);
        self.pending_outcome.merge(outcome);
        self.finish_adapter_operation(outcome)
    }

    /// Queue a message for a later production runtime turn.
    pub fn enqueue_message(&mut self, message: Message) -> Result<(), DeterministicHostError> {
        self.ensure_runtime_accepts_work()?;
        self.prepare_runtime_operation()?;
        let result = self.runtime.bridge_mut().enqueue_message(message);
        self.finish_adapter_operation(result?)
    }

    /// Queue a command for a later production runtime turn.
    pub fn enqueue_command(
        &mut self,
        command: Command<Message>,
    ) -> Result<(), DeterministicHostError> {
        self.ensure_runtime_accepts_work()?;
        self.prepare_runtime_operation()?;
        let result = self.runtime.bridge_mut().enqueue_command(command);
        self.finish_adapter_operation(result?)
    }

    /// Advance virtual time without running a runtime turn.
    pub fn advance_time(&mut self, delta: Duration) -> Result<(), DeterministicHostError> {
        self.ensure_runtime_accepts_work()?;
        let next = self
            .virtual_time
            .checked_add(delta)
            .ok_or(DeterministicHostError::TimeOverflow)?;
        let instant = self
            .instant_origin
            .checked_add(next)
            .ok_or(DeterministicHostError::TimeOverflow)?;
        self.virtual_time = next;
        self.runtime.set_timed_repaint_clock(Some(instant));
        self.runtime.bridge_mut().set_now(next);
        if self.runtime.advance_timed_repaints(instant) {
            self.pending_outcome.repaint_requested = true;
        }
        let result = self.runtime.bridge_mut().release_due_timers();
        self.finish_adapter_operation(result?)
    }

    /// Explicitly execute one worker task; its mapped message waits for a later turn.
    pub fn complete_worker(&mut self, id: WorkerTaskId) -> Result<(), DeterministicHostError> {
        self.ensure_runtime_accepts_work()?;
        self.prepare_runtime_operation()?;
        let result = self.runtime.bridge_mut().complete_worker(id);
        self.finish_adapter_operation(result?)
    }

    /// Explicitly complete one platform request; its mapper waits for a later turn.
    pub fn complete_platform_request(
        &mut self,
        id: PlatformRequestId,
        result: PlatformResult,
    ) -> Result<(), DeterministicHostError> {
        self.ensure_runtime_accepts_work()?;
        self.prepare_runtime_operation()?;
        let result = self
            .runtime
            .bridge_mut()
            .complete_platform_request(id, result);
        self.finish_adapter_operation(result?)
    }

    /// Run exactly one bounded production runtime drain and atomically publish its snapshot.
    pub fn turn(&mut self) -> Result<NormalizedSnapshot, DeterministicHostError> {
        self.ensure_runtime_accepts_work()?;
        self.prepare_runtime_operation()?;
        let outcome = self.runtime.drain_runtime_messages();
        // A command admitted during the production drain may attempt to
        // schedule work after the host-side admission methods have returned.
        // Surface that failure before consuming the outcome or publishing a
        // snapshot, so bounded-lane exhaustion cannot look like success.
        self.finish_adapter_operation(())?;
        self.pending_outcome.merge(outcome);
        self.last_outcome = std::mem::take(&mut self.pending_outcome);
        self.turn = self
            .turn
            .checked_add(1)
            .ok_or(DeterministicHostError::IdentifierOverflow)?;
        self.publish_snapshot()
    }

    /// Drain production work until no current-turn work remains, bounded by configuration.
    pub fn run_until_idle(&mut self) -> Result<NormalizedSnapshot, DeterministicHostError> {
        for _ in 0..self.config.step_budget() {
            if !self.has_pending_turn_work() {
                return Ok(self.published_snapshot().clone());
            }
            let snapshot = self.turn()?;
            if !self.has_pending_turn_work() {
                return Ok(snapshot);
            }
        }
        Err(DeterministicHostError::StepBudgetExceeded {
            budget: self.config.step_budget(),
        })
    }

    /// Refresh the current production surface without publishing until a later turn.
    pub fn refresh(&mut self) -> Result<(), DeterministicHostError> {
        self.ensure_runtime_accepts_work()?;
        self.prepare_runtime_operation()?;
        self.runtime.refresh();
        self.finish_adapter_operation(())
    }

    /// Build a normalized snapshot of current runtime state without publishing it.
    pub fn snapshot(&self) -> Result<NormalizedSnapshot, DeterministicHostError> {
        self.build_snapshot()
    }

    /// Return the current production backend-neutral raw paint plan.
    ///
    /// The normalized snapshot carries its deterministic paint summary. This
    /// accessor is for focused tests that need to inspect the production plan
    /// itself without introducing a renderer or a native presentation host.
    pub fn paint_plan(&self) -> crate::runtime::SurfacePaintPlan {
        self.runtime
            .paint_plan(&crate::theme::ThemeTokens::default())
    }

    /// Serialize the currently published snapshot as stable compact JSON bytes.
    pub fn snapshot_bytes(&self) -> Result<Vec<u8>, DeterministicHostError> {
        self.published_snapshot().to_json_bytes()
    }

    fn publish_snapshot(&mut self) -> Result<NormalizedSnapshot, DeterministicHostError> {
        let candidate = self.build_snapshot()?;
        self.published_snapshot = candidate.clone();
        Ok(candidate)
    }

    fn build_snapshot(&self) -> Result<NormalizedSnapshot, DeterministicHostError> {
        build_normalized_snapshot(
            &self.runtime,
            self.virtual_time,
            self.turn,
            self.last_outcome,
            self.application_observation.as_ref(),
        )
    }

    fn has_pending_turn_work(&self) -> bool {
        let diagnostics = self.runtime.runtime_diagnostics();
        self.runtime.bridge().queue_item_count() != 0
            || diagnostics.queue.current_pending_messages != 0
            || diagnostics.queue.current_pending_controller_completions != 0
            || self.last_outcome.runtime_work_remaining
    }

    fn current_instant(&self) -> Result<Instant, DeterministicHostError> {
        self.instant_origin
            .checked_add(self.virtual_time)
            .ok_or(DeterministicHostError::TimeOverflow)
    }

    fn prepare_runtime_operation(&mut self) -> Result<Instant, DeterministicHostError> {
        let now = self.current_instant()?;
        self.runtime.set_timed_repaint_clock(Some(now));
        Ok(now)
    }

    fn ensure_runtime_accepts_work(&self) -> Result<(), DeterministicHostError> {
        if matches!(
            self.runtime.runtime_diagnostics().lifecycle.phase,
            RuntimeLifecyclePhase::Closing | RuntimeLifecyclePhase::Stopped
        ) {
            Err(DeterministicHostError::RuntimeNotAcceptingWork)
        } else {
            Ok(())
        }
    }

    fn finish_adapter_operation<T>(&mut self, value: T) -> Result<T, DeterministicHostError> {
        if let Some(error) = self.runtime.bridge_mut().take_failure() {
            return Err(error);
        }
        Ok(value)
    }
}

fn with_virtual_timestamp(event: Event, timestamp: InputTimestamp) -> Event {
    match event {
        Event::Resize { .. } | Event::TraverseFocus(_) | Event::ClearFocus => event,
        Event::PointerMove {
            position,
            modifiers,
            sequence_range,
            ..
        } => Event::PointerMove {
            position,
            modifiers,
            timestamp: Some(timestamp),
            sequence_range,
        },
        Event::PointerModifiersChanged { modifiers, .. } => Event::PointerModifiersChanged {
            modifiers,
            timestamp: Some(timestamp),
        },
        Event::PointerPress {
            position,
            button,
            modifiers,
            ..
        } => Event::PointerPress {
            position,
            button,
            modifiers,
            timestamp: Some(timestamp),
        },
        Event::PointerDoubleClick {
            position,
            button,
            modifiers,
            ..
        } => Event::PointerDoubleClick {
            position,
            button,
            modifiers,
            timestamp: Some(timestamp),
        },
        Event::PointerRelease {
            position,
            button,
            modifiers,
            ..
        } => Event::PointerRelease {
            position,
            button,
            modifiers,
            timestamp: Some(timestamp),
        },
        Event::KeyPress {
            key,
            modifiers,
            repeat,
            ..
        } => Event::KeyPress {
            key,
            modifiers,
            repeat,
            timestamp: Some(timestamp),
        },
        Event::KeyRelease { key, modifiers, .. } => Event::KeyRelease {
            key,
            modifiers,
            timestamp: Some(timestamp),
        },
        Event::Character { character, .. } => Event::Character {
            character,
            timestamp: Some(timestamp),
        },
        Event::Scroll {
            position,
            delta,
            modifiers,
            sequence_range,
            ..
        } => Event::Scroll {
            position,
            delta,
            modifiers,
            timestamp: Some(timestamp),
            sequence_range,
        },
    }
}

fn build_normalized_snapshot<Bridge, Message>(
    runtime: &SurfaceRuntime<DeterministicBridge<Bridge, Message>, Message>,
    virtual_time: Duration,
    turn: u64,
    last_outcome: CommandOutcome,
    application_observation: Option<&Value>,
) -> Result<NormalizedSnapshot, DeterministicHostError>
where
    Bridge: RuntimeBridge<Message>,
{
    let diagnostics = runtime.runtime_diagnostics();
    let layout = normalized_layout(runtime.layout())?;
    let paint_plan = runtime.paint_plan(&crate::theme::ThemeTokens::default());
    let refresh = normalized_refresh(
        runtime.last_refresh_diagnostics(),
        runtime.refresh_counters(),
    )?;
    let command = normalized_command_outcome(last_outcome)?;
    let viewport = normalized_rect(runtime.context().viewport, "viewport")?;
    let environment = normalized_environment(runtime.window_environment())?;
    let focus = normalized_focus(runtime)?;
    let pending = normalized_pending_work(
        runtime.bridge(),
        diagnostics.queue.current_pending_controller_completions,
        last_outcome.runtime_work_remaining,
    );
    Ok(NormalizedSnapshot {
        schema_version: NORMALIZED_SNAPSHOT_SCHEMA_VERSION,
        turn,
        virtual_time_nanos: virtual_time.as_nanos(),
        environment,
        viewport,
        layout,
        automation: runtime.automation_snapshot(),
        automation_targets: runtime.automation_target_snapshot(),
        focus,
        paint: normalized_paint(&paint_plan),
        refresh,
        command,
        diagnostics: normalized_diagnostics(&diagnostics),
        pending,
        repaint_requested: runtime.repaint_requested(),
        application_observation: application_observation.cloned(),
    })
}

fn validate_event(event: Event) -> Result<(), DeterministicHostError> {
    match event {
        Event::Resize { viewport }
            if !finite_vector(viewport) || viewport.x <= 0.0 || viewport.y <= 0.0 =>
        {
            Err(DeterministicHostError::InvalidEvent(
                "resize viewport must be finite and positive",
            ))
        }
        Event::PointerMove { position, .. }
        | Event::PointerPress { position, .. }
        | Event::PointerDoubleClick { position, .. }
        | Event::PointerRelease { position, .. }
        | Event::Scroll { position, .. }
            if !position.is_finite() =>
        {
            Err(DeterministicHostError::InvalidEvent(
                "pointer position must be finite",
            ))
        }
        Event::Scroll { delta, .. } if !finite_vector(delta) => Err(
            DeterministicHostError::InvalidEvent("scroll delta must be finite"),
        ),
        _ => Ok(()),
    }
}

fn finite_vector(vector: Vector2) -> bool {
    vector.x.is_finite() && vector.y.is_finite()
}

fn platform_result_matches(request: &PlatformRequest, result: &PlatformResult) -> bool {
    let Ok(response) = result else {
        return true;
    };
    match request {
        PlatformRequest::PickFolder(_)
        | PlatformRequest::PickFile(_)
        | PlatformRequest::SaveFile(_) => {
            matches!(
                response,
                PlatformResponse::Path(_) | PlatformResponse::Canceled
            )
        }
        PlatformRequest::OpenPath(_)
        | PlatformRequest::RevealPath(_)
        | PlatformRequest::OpenUrl(_)
        | PlatformRequest::CopyText(_)
        | PlatformRequest::CopyFilePaths(_) => matches!(response, PlatformResponse::Completed),
        PlatformRequest::ReadText => matches!(response, PlatformResponse::Text(_)),
        PlatformRequest::ReadFilePaths => matches!(response, PlatformResponse::FilePaths(_)),
        PlatformRequest::Confirm(_) => matches!(response, PlatformResponse::Confirmation(_)),
    }
}

fn repaint_scope_name(scope: RepaintScope) -> String {
    match scope {
        RepaintScope::Surface => "surface",
        RepaintScope::Layout => "layout",
        RepaintScope::Projection => "projection",
        RepaintScope::PaintOnly => "paint_only",
    }
    .to_owned()
}

fn priority_name(priority: TaskPriority) -> String {
    match priority {
        TaskPriority::Interactive => "interactive",
        TaskPriority::Background => "background",
        TaskPriority::BlockingIo => "blocking_io",
        TaskPriority::Idle => "idle",
    }
    .to_owned()
}

fn lifecycle_phase_name(phase: RuntimeLifecyclePhase) -> String {
    match phase {
        RuntimeLifecyclePhase::Unknown => "unknown",
        RuntimeLifecyclePhase::Starting => "starting",
        RuntimeLifecyclePhase::Running => "running",
        RuntimeLifecyclePhase::Recovering => "recovering",
        RuntimeLifecyclePhase::Closing => "closing",
        RuntimeLifecyclePhase::Stopped => "stopped",
    }
    .to_owned()
}

fn overflow_policy_name(policy: OverflowPolicy) -> String {
    match policy {
        OverflowPolicy::Clip => "clip",
        OverflowPolicy::Scroll => "scroll",
        OverflowPolicy::Wrap => "wrap",
        OverflowPolicy::Shrink => "shrink",
    }
    .to_owned()
}

fn main_align_name(alignment: MainAlign) -> String {
    match alignment {
        MainAlign::Start => "start",
        MainAlign::Center => "center",
        MainAlign::End => "end",
        MainAlign::SpaceBetween => "space_between",
        MainAlign::SpaceAround => "space_around",
        MainAlign::SpaceEvenly => "space_evenly",
    }
    .to_owned()
}

fn layout_diagnostic_code_name(code: LayoutDiagnosticCode) -> String {
    match code {
        LayoutDiagnosticCode::NegativeSizeClamped => "negative_size_clamped",
        LayoutDiagnosticCode::ConstraintContradiction => "constraint_contradiction",
        LayoutDiagnosticCode::OverflowPolicyDefaulted => "overflow_policy_defaulted",
        LayoutDiagnosticCode::OverflowOccurred => "overflow_occurred",
        LayoutDiagnosticCode::InvalidScrollOffsetClamped => "invalid_scroll_offset_clamped",
        LayoutDiagnosticCode::VirtualizationPolicyIgnored => "virtualization_policy_ignored",
        LayoutDiagnosticCode::VirtualizationWindowClamped => "virtualization_window_clamped",
        LayoutDiagnosticCode::VirtualizationAlignmentFallback => {
            "virtualization_alignment_fallback"
        }
        LayoutDiagnosticCode::VirtualizationSpanResolutionFallback => {
            "virtualization_span_resolution_fallback"
        }
        LayoutDiagnosticCode::SplitPaneChildCountMismatch => "split_pane_child_count_mismatch",
        LayoutDiagnosticCode::SplitPaneMinimumsUnsatisfied => "split_pane_minimums_unsatisfied",
        LayoutDiagnosticCode::CustomLayoutHintNonFinite => "custom_layout_hint_non_finite",
        LayoutDiagnosticCode::CustomLayoutHintNegative => "custom_layout_hint_negative",
        LayoutDiagnosticCode::CustomLayoutHintContradictory => "custom_layout_hint_contradictory",
        LayoutDiagnosticCode::CustomLayoutInvalidChildIndex => "custom_layout_invalid_child_index",
        LayoutDiagnosticCode::CustomLayoutInvalidPlacement => "custom_layout_invalid_placement",
        LayoutDiagnosticCode::CustomLayoutDuplicatePlacement => "custom_layout_duplicate_placement",
        LayoutDiagnosticCode::CustomLayoutChildUnresolved => "custom_layout_child_unresolved",
    }
    .to_owned()
}

fn normalized_float(value: f32, field: &'static str) -> Result<f32, DeterministicHostError> {
    if !value.is_finite() {
        return Err(DeterministicHostError::NonFiniteOutput(field));
    }
    Ok(if value == 0.0 { 0.0 } else { value })
}

fn normalized_rect(
    rect: Rect,
    field: &'static str,
) -> Result<NormalizedRect, DeterministicHostError> {
    Ok(NormalizedRect {
        x: normalized_float(rect.min.x, field)?,
        y: normalized_float(rect.min.y, field)?,
        width: normalized_float(rect.width(), field)?,
        height: normalized_float(rect.height(), field)?,
    })
}

fn normalized_vector(
    vector: Vector2,
    field: &'static str,
) -> Result<NormalizedVector2, DeterministicHostError> {
    Ok(NormalizedVector2 {
        x: normalized_float(vector.x, field)?,
        y: normalized_float(vector.y, field)?,
    })
}

fn normalized_point(
    point: Point,
    field: &'static str,
) -> Result<NormalizedPoint, DeterministicHostError> {
    Ok(NormalizedPoint {
        x: normalized_float(point.x, field)?,
        y: normalized_float(point.y, field)?,
    })
}

fn normalized_environment(
    environment: WindowEnvironment,
) -> Result<NormalizedEnvironment, DeterministicHostError> {
    let color_scheme = environment.color_scheme().map(|scheme| match scheme {
        WindowColorScheme::Light => "light".to_owned(),
        WindowColorScheme::Dark => "dark".to_owned(),
    });
    Ok(NormalizedEnvironment {
        display_scale: normalized_float(
            environment.display_scale().factor(),
            "environment.display_scale",
        )?,
        color_scheme,
        contrast: environment.contrast(),
        reduced_motion: environment.reduced_motion(),
    })
}

fn normalized_layout(
    layout: &crate::layout::LayoutOutput,
) -> Result<NormalizedLayout, DeterministicHostError> {
    let rects = layout
        .rects
        .iter()
        .map(|(&node_id, &rect)| {
            Ok(NormalizedNodeRect {
                node_id,
                rect: normalized_rect(rect, "layout.rects")?,
            })
        })
        .collect::<Result<Vec<_>, DeterministicHostError>>()?;
    let overflow = layout
        .overflow_flags
        .iter()
        .map(|(&node_id, info)| NormalizedOverflow {
            node_id,
            x: info.x,
            y: info.y,
            policy: overflow_policy_name(info.policy),
        })
        .collect();
    let diagnostics = layout
        .diagnostics
        .iter()
        .map(|diagnostic| NormalizedLayoutDiagnostic {
            node_id: diagnostic.node_id,
            code: layout_diagnostic_code_name(diagnostic.code),
            message: diagnostic.message.to_string(),
        })
        .collect();
    let viewport_bounds = layout
        .viewport_bounds
        .iter()
        .map(|(&node_id, &rect)| {
            Ok(NormalizedNodeRect {
                node_id,
                rect: normalized_rect(rect, "layout.viewport_bounds")?,
            })
        })
        .collect::<Result<Vec<_>, DeterministicHostError>>()?;
    let virtual_windows = layout
        .virtual_windows
        .iter()
        .map(|(&node_id, window)| {
            Ok(NormalizedVirtualWindow {
                node_id,
                total_children: window.total_children,
                first_index: window.first_index,
                last_index_exclusive: window.last_index_exclusive,
                culled_before: window.culled_before,
                culled_after: window.culled_after,
                viewport_main_start: normalized_float(
                    window.viewport_main_start,
                    "layout.virtual_windows",
                )?,
                viewport_main_end: normalized_float(
                    window.viewport_main_end,
                    "layout.virtual_windows",
                )?,
                window_main_start: normalized_float(
                    window.window_main_start,
                    "layout.virtual_windows",
                )?,
                window_main_end: normalized_float(
                    window.window_main_end,
                    "layout.virtual_windows",
                )?,
                resolved_total_main: normalized_float(
                    window.resolved_total_main,
                    "layout.virtual_windows",
                )?,
                alignment_mode: main_align_name(window.alignment_mode),
            })
        })
        .collect::<Result<Vec<_>, DeterministicHostError>>()?;
    Ok(NormalizedLayout {
        rects,
        overflowed: layout.overflowed.iter().copied().collect(),
        overflow,
        diagnostics,
        viewport_bounds,
        virtual_windows,
        stats: NormalizedLayoutStats {
            measured_nodes: layout.stats.measured_nodes,
            laid_out_nodes: layout.stats.laid_out_nodes,
            materialized_nodes: layout.stats.materialized_nodes,
        },
    })
}

fn normalized_focus<Bridge, Message>(
    runtime: &SurfaceRuntime<Bridge, Message>,
) -> Result<NormalizedFocus, DeterministicHostError>
where
    Bridge: RuntimeBridge<Message>,
{
    Ok(NormalizedFocus {
        focused_widget: runtime.focused_widget(),
        pointer_capture: runtime.pointer_capture(),
        layout_pointer_capture: runtime
            .layout_pointer_capture()
            .map(|identity| identity.container_id),
        hovered_widget: runtime.hovered_widget(),
        hovered_container: runtime.hovered_container(),
        hovered_scroll_affordance: runtime.hovered_scroll_affordance(),
        current_pointer_position: runtime
            .current_pointer_position()
            .map(|point| normalized_point(point, "focus.current_pointer_position"))
            .transpose()?,
    })
}

fn normalized_paint(plan: &crate::runtime::SurfacePaintPlan) -> NormalizedPaint {
    let stats = plan.stats();
    NormalizedPaint {
        clear_color: [
            plan.clear_color.r,
            plan.clear_color.g,
            plan.clear_color.b,
            plan.clear_color.a,
        ],
        total: stats.total,
        fills: stats.fills,
        svg_documents: stats.svg_documents,
        strokes: stats.strokes,
        text: stats.text,
        clips: stats.clips,
        images: stats.images,
        overlay_panels: stats.overlay_panels,
        custom_surfaces: stats.custom_surfaces,
        gpu_surfaces: stats.gpu_surfaces,
    }
}

fn normalized_command_outcome(
    outcome: CommandOutcome,
) -> Result<NormalizedCommandOutcome, DeterministicHostError> {
    Ok(NormalizedCommandOutcome {
        messages_dispatched: outcome.messages_dispatched,
        repaint_requested: outcome.repaint_requested,
        paint_only_requested: outcome.paint_only_requested,
        surface_repaint_requested: outcome.surface_repaint_requested,
        surface_refresh_requested: outcome.surface_refresh_requested,
        surface_refresh_scope: outcome.surface_refresh_scope.map(repaint_scope_name),
        surface_refresh_applied: outcome.surface_refresh_applied,
        exit_requested: outcome.exit_requested,
        runtime_work_remaining: outcome.runtime_work_remaining,
        dpi_scale_override: outcome
            .dpi_scale_override
            .map(DpiScale::factor)
            .map(|scale| normalized_float(scale, "command.dpi_scale_override"))
            .transpose()?,
        window_logical_size: outcome
            .window_logical_size
            .map(|size| normalized_vector(size, "command.window_logical_size"))
            .transpose()?,
    })
}

fn normalized_refresh(
    diagnostics: SurfaceRefreshDiagnostics,
    counters: SurfaceRefreshCounters,
) -> Result<NormalizedRefresh, DeterministicHostError> {
    Ok(NormalizedRefresh {
        invalidation: diagnostics.invalidation.name().to_owned(),
        scope: diagnostics
            .invalidation
            .repaint_scope()
            .map(repaint_scope_name),
        identity: normalized_identity(diagnostics.identity),
        layout_state: normalized_layout_state(diagnostics.layout_state),
        counters: NormalizedRefreshCounters {
            application_projection: counters.application_projection,
            runtime_projection: counters.runtime_projection,
            widget_state_sync: counters.widget_state_sync,
            layout: counters.layout,
            base_paint_plan_rebuilds: counters.base_paint_plan_rebuilds,
        },
    })
}

fn normalized_identity(diagnostics: SurfaceIdentityDiagnostics) -> NormalizedIdentityDiagnostics {
    NormalizedIdentityDiagnostics {
        replacement_count: diagnostics.replacement_count,
        replacements: diagnostics
            .replacements
            .into_iter()
            .flatten()
            .map(normalized_identity_replacement)
            .collect(),
    }
}

fn normalized_identity_replacement(
    replacement: SurfaceIdentityReplacement,
) -> NormalizedIdentityReplacement {
    NormalizedIdentityReplacement {
        widget_id: replacement.widget_id,
        previous_kind: replacement.previous_kind.to_owned(),
        current_kind: replacement.current_kind.to_owned(),
        previous_path: normalized_path(replacement.previous_path),
        current_path: normalized_path(replacement.current_path),
        discarded_focus: replacement.discarded_ownership.focus,
        discarded_pointer_capture: replacement.discarded_ownership.pointer_capture,
        discarded_hover: replacement.discarded_ownership.hover,
        discarded_widget_state: replacement.discarded_ownership.widget_state,
    }
}

fn normalized_path(path: SurfaceIdentityPath) -> NormalizedPath {
    NormalizedPath {
        components: path.components[..usize::from(path.len).min(path.components.len())].to_vec(),
        truncated: path.truncated,
    }
}

fn normalized_layout_state(
    diagnostics: SurfaceLayoutStateDiagnostics,
) -> NormalizedLayoutStateDiagnostics {
    NormalizedLayoutStateDiagnostics {
        replacements: diagnostics
            .replacements
            .into_iter()
            .flatten()
            .map(normalized_layout_state_replacement)
            .collect(),
        replacement_count: diagnostics.replacement_count,
        dropped_count: diagnostics.dropped_count,
        initialized_count: diagnostics.initialized_count,
        capacity_exceeded_count: diagnostics.capacity_exceeded_count,
        foreign_declaration_count: diagnostics.foreign_declaration_count,
        generation_exhaustion_count: diagnostics.generation_exhaustion_count,
    }
}

fn normalized_layout_state_replacement(
    replacement: SurfaceLayoutStateReplacement,
) -> NormalizedLayoutStateReplacement {
    NormalizedLayoutStateReplacement {
        container_id: replacement.container_id,
        previous_schema_version: replacement.previous.schema_version(),
        current_schema_version: replacement.current.schema_version(),
    }
}

fn normalized_diagnostics(diagnostics: &RuntimeDiagnostics) -> NormalizedDiagnostics {
    NormalizedDiagnostics {
        queue: NormalizedQueueDiagnostics {
            current_pending_messages: diagnostics.queue.current_pending_messages,
            max_pending_messages: diagnostics.queue.max_pending_messages,
            current_pending_stream_slots: diagnostics.queue.current_pending_stream_slots,
            max_pending_stream_slots: diagnostics.queue.max_pending_stream_slots,
            current_pending_controller_completions: diagnostics
                .queue
                .current_pending_controller_completions,
            max_pending_controller_completions: diagnostics
                .queue
                .max_pending_controller_completions,
            controller_completion_deferrals: diagnostics.queue.controller_completion_deferrals,
            stream_events_coalesced: diagnostics.queue.stream_events_coalesced,
            stream_events_stale: diagnostics.queue.stream_events_stale,
            stream_events_dropped: diagnostics.queue.stream_events_dropped,
            shared_ingress_rejected: diagnostics.queue.shared_ingress_rejected,
            shared_ingress_coalesced: diagnostics.queue.shared_ingress_coalesced,
        },
        business: NormalizedBusinessDiagnostics {
            queued: diagnostics.business.queued,
            started: diagnostics.business.started,
            completed: diagnostics.business.completed,
            cancelled: diagnostics.business.cancelled,
            failed: diagnostics.business.failed,
            rejected: diagnostics.business.rejected,
            running: diagnostics.business.running,
            checkpoints: diagnostics.business.checkpoints,
            stream_events: diagnostics.business.stream_events,
            missing_checkpoint_warnings: diagnostics.business.missing_checkpoint_warnings,
            missing_stream_event_warnings: diagnostics.business.missing_stream_event_warnings,
            recent: diagnostics
                .business
                .recent
                .iter()
                .map(|event| NormalizedBusinessEvent {
                    name: event.name.to_owned(),
                    priority: priority_name(event.priority),
                    state: business_state_name(event.state),
                })
                .collect(),
        },
        ui: NormalizedUiDiagnostics {
            update_handlers: diagnostics.ui.update_handlers,
            slow_update_handlers: diagnostics.ui.slow_update_handlers,
            last_slow_handler: diagnostics
                .ui
                .last_slow_update_handler
                .as_ref()
                .map(|event| NormalizedSlowHandler {
                    handler: event.handler.to_owned(),
                    message: event.message.to_owned(),
                }),
        },
        lifecycle: NormalizedLifecycleDiagnostics {
            available: diagnostics.lifecycle.available,
            phase: lifecycle_phase_name(diagnostics.lifecycle.phase),
            transition_count: diagnostics.lifecycle.transition_count,
            history: diagnostics
                .lifecycle
                .history
                .iter()
                .map(|transition| NormalizedLifecycleTransition {
                    sequence: transition.sequence,
                    from: lifecycle_phase_name(transition.from),
                    to: lifecycle_phase_name(transition.to),
                })
                .collect(),
        },
    }
}

fn business_state_name(state: crate::runtime::BusinessTaskDiagnosticState) -> String {
    match state {
        crate::runtime::BusinessTaskDiagnosticState::Queued => "queued",
        crate::runtime::BusinessTaskDiagnosticState::Started => "started",
        crate::runtime::BusinessTaskDiagnosticState::Completed => "completed",
        crate::runtime::BusinessTaskDiagnosticState::Cancelled => "cancelled",
        crate::runtime::BusinessTaskDiagnosticState::Panicked => "panicked",
        crate::runtime::BusinessTaskDiagnosticState::Rejected => "rejected",
        crate::runtime::BusinessTaskDiagnosticState::Checkpoint => "checkpoint",
        crate::runtime::BusinessTaskDiagnosticState::StreamEvent => "stream_event",
        crate::runtime::BusinessTaskDiagnosticState::MissingCheckpoint => "missing_checkpoint",
        crate::runtime::BusinessTaskDiagnosticState::MissingStreamEvent => "missing_stream_event",
    }
    .to_owned()
}

fn normalized_pending_work<Bridge, Message>(
    bridge: &DeterministicBridge<Bridge, Message>,
    controller_completions: usize,
    runtime_work_remaining: bool,
) -> NormalizedPendingWork
where
    Bridge: RuntimeBridge<Message>,
{
    NormalizedPendingWork {
        workers: bridge
            .pending_worker_tasks()
            .into_iter()
            .map(|task| NormalizedPendingWorker {
                id: task.id,
                name: task.name.to_owned(),
                priority: priority_name(task.priority),
            })
            .collect(),
        platform_requests: bridge
            .pending_platform_requests()
            .into_iter()
            .map(|request| NormalizedPendingPlatform {
                id: request.id,
                kind: platform_request_kind(&request.request),
            })
            .collect(),
        scheduled_timers: bridge.pending_timer_count(),
        queued_items: bridge.queue_item_count(),
        controller_completions,
        runtime_work_remaining,
    }
}

fn platform_request_kind(request: &PlatformRequest) -> String {
    match request {
        PlatformRequest::PickFolder(_) => "pick_folder",
        PlatformRequest::PickFile(_) => "pick_file",
        PlatformRequest::SaveFile(_) => "save_file",
        PlatformRequest::OpenPath(_) => "open_path",
        PlatformRequest::RevealPath(_) => "reveal_path",
        PlatformRequest::OpenUrl(_) => "open_url",
        PlatformRequest::CopyText(_) => "copy_text",
        PlatformRequest::CopyFilePaths(_) => "copy_file_paths",
        PlatformRequest::ReadText => "read_text",
        PlatformRequest::ReadFilePaths => "read_file_paths",
        PlatformRequest::Confirm(_) => "confirm",
    }
    .to_owned()
}

/// Versioned normalized output published by [`DeterministicHost`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NormalizedSnapshot {
    /// Snapshot schema version.
    pub schema_version: u32,
    /// Number of completed runtime turns represented by this snapshot.
    pub turn: u64,
    /// Virtual time in nanoseconds from host startup.
    pub virtual_time_nanos: u128,
    /// Fixed shipped window environment.
    pub environment: NormalizedEnvironment,
    /// Fixed logical viewport rectangle.
    pub viewport: NormalizedRect,
    /// Backend-neutral layout output.
    pub layout: NormalizedLayout,
    /// Production automation/semantics snapshot.
    pub automation: GuiAutomationSnapshot,
    /// Production flattened automation targets.
    pub automation_targets: GuiAutomationTargetSnapshot,
    /// Production focus and pointer ownership state.
    pub focus: NormalizedFocus,
    /// Backend-neutral paint summary.
    pub paint: NormalizedPaint,
    /// Non-timing refresh diagnostics and counters.
    pub refresh: NormalizedRefresh,
    /// Last normalized command outcome.
    pub command: NormalizedCommandOutcome,
    /// Non-timing runtime diagnostics.
    pub diagnostics: NormalizedDiagnostics,
    /// Explicitly controlled pending-work state.
    pub pending: NormalizedPendingWork,
    /// Whether production runtime state currently requests repaint.
    pub repaint_requested: bool,
    /// Optional caller-supplied application observation.
    pub application_observation: Option<Value>,
}

impl NormalizedSnapshot {
    /// Serialize this snapshot as stable compact JSON bytes.
    pub fn to_json_bytes(&self) -> Result<Vec<u8>, DeterministicHostError> {
        serde_json::to_vec(self)
            .map_err(|error| DeterministicHostError::SnapshotSerialization(error.to_string()))
    }
}

/// Normalized environment values in a deterministic snapshot.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NormalizedEnvironment {
    /// Effective display scale.
    pub display_scale: f32,
    /// Stable color-scheme label, when known.
    pub color_scheme: Option<String>,
    /// Higher-contrast preference.
    pub contrast: bool,
    /// Reduced-motion preference.
    pub reduced_motion: bool,
}

/// Normalized rectangle with explicit origin and extent.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct NormalizedRect {
    /// Horizontal origin.
    pub x: f32,
    /// Vertical origin.
    pub y: f32,
    /// Width.
    pub width: f32,
    /// Height.
    pub height: f32,
}

/// Normalized two-dimensional value.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct NormalizedVector2 {
    /// Horizontal component.
    pub x: f32,
    /// Vertical component.
    pub y: f32,
}

/// Normalized point value.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct NormalizedPoint {
    /// Horizontal component.
    pub x: f32,
    /// Vertical component.
    pub y: f32,
}

/// Normalized layout output and layout diagnostics.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NormalizedLayout {
    /// Node rectangles in ascending node-id order.
    pub rects: Vec<NormalizedNodeRect>,
    /// Node ids that overflowed available space.
    pub overflowed: Vec<u64>,
    /// Per-node overflow metadata in ascending node-id order.
    pub overflow: Vec<NormalizedOverflow>,
    /// Stable layout diagnostics in production order.
    pub diagnostics: Vec<NormalizedLayoutDiagnostic>,
    /// Scroll viewport bounds in ascending node-id order.
    pub viewport_bounds: Vec<NormalizedNodeRect>,
    /// Virtualization windows in ascending node-id order.
    pub virtual_windows: Vec<NormalizedVirtualWindow>,
    /// Traversal counters.
    pub stats: NormalizedLayoutStats,
}

/// One normalized node rectangle.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NormalizedNodeRect {
    /// Node id.
    pub node_id: u64,
    /// Resolved rectangle.
    pub rect: NormalizedRect,
}

/// One normalized overflow record.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedOverflow {
    /// Node id.
    pub node_id: u64,
    /// Whether horizontal overflow occurred.
    pub x: bool,
    /// Whether vertical overflow occurred.
    pub y: bool,
    /// Stable overflow policy label.
    pub policy: String,
}

/// One normalized layout diagnostic.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedLayoutDiagnostic {
    /// Node id associated with the diagnostic.
    pub node_id: u64,
    /// Stable diagnostic-code label.
    pub code: String,
    /// Diagnostic text.
    pub message: String,
}

/// Normalized virtualization-window metadata.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NormalizedVirtualWindow {
    /// Scroll-container node id.
    pub node_id: u64,
    /// Total available children.
    pub total_children: usize,
    /// First materialized index.
    pub first_index: usize,
    /// Exclusive last materialized index.
    pub last_index_exclusive: usize,
    /// Children culled before the window.
    pub culled_before: usize,
    /// Children culled after the window.
    pub culled_after: usize,
    /// Viewport start on the virtualization axis.
    pub viewport_main_start: f32,
    /// Viewport end on the virtualization axis.
    pub viewport_main_end: f32,
    /// Window start on the virtualization axis.
    pub window_main_start: f32,
    /// Window end on the virtualization axis.
    pub window_main_end: f32,
    /// Resolved total main-axis extent.
    pub resolved_total_main: f32,
    /// Stable alignment label.
    pub alignment_mode: String,
}

/// Normalized layout traversal counters.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedLayoutStats {
    /// Nodes measured without a cache hit.
    pub measured_nodes: usize,
    /// Nodes visited by layout.
    pub laid_out_nodes: usize,
    /// Nodes materialized into output rectangles.
    pub materialized_nodes: usize,
}

/// Focus, pointer capture, and hover state in a normalized snapshot.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NormalizedFocus {
    /// Focused widget id.
    pub focused_widget: Option<u64>,
    /// Pointer-captured widget id.
    pub pointer_capture: Option<u64>,
    /// Container id owning layout-level pointer capture.
    pub layout_pointer_capture: Option<u64>,
    /// Hovered widget id.
    pub hovered_widget: Option<u64>,
    /// Hovered container id.
    pub hovered_container: Option<u64>,
    /// Hovered scroll affordance id.
    pub hovered_scroll_affordance: Option<u64>,
    /// Latest logical pointer position.
    pub current_pointer_position: Option<NormalizedPoint>,
}

/// Paint-plan summary that does not include backend or timing data.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedPaint {
    /// Clear color in RGBA8 order.
    pub clear_color: [u8; 4],
    /// Total primitive count.
    pub total: usize,
    /// Fill primitive count.
    pub fills: usize,
    /// SVG primitive count.
    pub svg_documents: usize,
    /// Stroke primitive count.
    pub strokes: usize,
    /// Text primitive count.
    pub text: usize,
    /// Clip primitive count.
    pub clips: usize,
    /// Image primitive count.
    pub images: usize,
    /// Overlay-panel primitive count.
    pub overlay_panels: usize,
    /// Custom-surface primitive count.
    pub custom_surfaces: usize,
    /// GPU-surface primitive count.
    pub gpu_surfaces: usize,
}

/// Non-timing refresh result and replacement diagnostics.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedRefresh {
    /// Stable typed invalidation label.
    pub invalidation: String,
    /// Stable repaint scope label, when applicable.
    pub scope: Option<String>,
    /// Widget-identity replacement diagnostics.
    pub identity: NormalizedIdentityDiagnostics,
    /// Layout-state replacement diagnostics.
    pub layout_state: NormalizedLayoutStateDiagnostics,
    /// Cumulative production refresh-stage counters.
    pub counters: NormalizedRefreshCounters,
}

/// Cumulative refresh-stage counters without timing data.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedRefreshCounters {
    /// Host application surface projections pulled by the runtime.
    pub application_projection: u64,
    /// Runtime projection/traversal rebuilds.
    pub runtime_projection: u64,
    /// Widget-state synchronization passes.
    pub widget_state_sync: u64,
    /// Layout passes.
    pub layout: u64,
    /// Base paint-plan rebuilds reported by a host renderer.
    pub base_paint_plan_rebuilds: u64,
}

/// Bounded widget-identity diagnostics.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedIdentityDiagnostics {
    /// Total replacements observed, including bounded-out records.
    pub replacement_count: u32,
    /// Retained replacements in deterministic order.
    pub replacements: Vec<NormalizedIdentityReplacement>,
}

/// One normalized incompatible widget replacement.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedIdentityReplacement {
    /// Shared widget id.
    pub widget_id: u64,
    /// Previous compatibility label.
    pub previous_kind: String,
    /// Replacement compatibility label.
    pub current_kind: String,
    /// Previous projected path.
    pub previous_path: NormalizedPath,
    /// Replacement projected path.
    pub current_path: NormalizedPath,
    /// Whether focus ownership was discarded.
    pub discarded_focus: bool,
    /// Whether pointer capture was discarded.
    pub discarded_pointer_capture: bool,
    /// Whether hover ownership was discarded.
    pub discarded_hover: bool,
    /// Whether widget-local state was discarded.
    pub discarded_widget_state: bool,
}

/// One bounded normalized widget path.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedPath {
    /// Retained path components.
    pub components: Vec<usize>,
    /// Whether the source path exceeded the diagnostic bound.
    pub truncated: bool,
}

/// Bounded layout-state diagnostics.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedLayoutStateDiagnostics {
    /// Retained state-identity replacements.
    pub replacements: Vec<NormalizedLayoutStateReplacement>,
    /// Total state replacements.
    pub replacement_count: u32,
    /// Dropped unmounted slots.
    pub dropped_count: u32,
    /// Initialized slots.
    pub initialized_count: u32,
    /// Capacity failures.
    pub capacity_exceeded_count: u32,
    /// Foreign declarations.
    pub foreign_declaration_count: u32,
    /// Mount-generation exhaustion count.
    pub generation_exhaustion_count: u32,
}

/// One normalized layout-state replacement.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedLayoutStateReplacement {
    /// Container id.
    pub container_id: u64,
    /// Previous caller-owned schema version.
    pub previous_schema_version: u16,
    /// Current caller-owned schema version.
    pub current_schema_version: u16,
}

/// Normalized command-dispatch outcome.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NormalizedCommandOutcome {
    /// Host messages reduced by the pass.
    pub messages_dispatched: usize,
    /// Whether repaint was requested.
    pub repaint_requested: bool,
    /// Whether paint-only work was requested.
    pub paint_only_requested: bool,
    /// Whether a surface repaint was requested.
    pub surface_repaint_requested: bool,
    /// Whether a surface refresh was requested.
    pub surface_refresh_requested: bool,
    /// Selected refresh scope.
    pub surface_refresh_scope: Option<String>,
    /// Whether requested refresh work was applied.
    pub surface_refresh_applied: bool,
    /// Whether runtime exit was requested.
    pub exit_requested: bool,
    /// Whether runtime work remains queued.
    pub runtime_work_remaining: bool,
    /// Requested DPI override.
    pub dpi_scale_override: Option<f32>,
    /// Requested logical window size.
    pub window_logical_size: Option<NormalizedVector2>,
}

/// Non-timing queue, business, UI, and lifecycle diagnostics.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedDiagnostics {
    /// Queue-pressure counters.
    pub queue: NormalizedQueueDiagnostics,
    /// Business-work counters and state-only recent events.
    pub business: NormalizedBusinessDiagnostics,
    /// UI update-handler counters.
    pub ui: NormalizedUiDiagnostics,
    /// Lifecycle phase and transition history.
    pub lifecycle: NormalizedLifecycleDiagnostics,
}

/// Normalized runtime queue diagnostics.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedQueueDiagnostics {
    /// Current pending message count.
    pub current_pending_messages: usize,
    /// Maximum pending message count.
    pub max_pending_messages: usize,
    /// Current stream-slot count.
    pub current_pending_stream_slots: usize,
    /// Maximum stream-slot count.
    pub max_pending_stream_slots: usize,
    /// Current controller-completion count.
    pub current_pending_controller_completions: usize,
    /// Maximum controller-completion count.
    pub max_pending_controller_completions: usize,
    /// Completion-budget deferrals.
    pub controller_completion_deferrals: usize,
    /// Coalesced stream events.
    pub stream_events_coalesced: usize,
    /// Stale stream events.
    pub stream_events_stale: usize,
    /// Dropped stream events.
    pub stream_events_dropped: usize,
    /// Shared-ingress rejections.
    pub shared_ingress_rejected: usize,
    /// Shared-ingress coalescing.
    pub shared_ingress_coalesced: usize,
}

/// Normalized business-work counters.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedBusinessDiagnostics {
    /// Queued task count.
    pub queued: usize,
    /// Started task count.
    pub started: usize,
    /// Completed task count.
    pub completed: usize,
    /// Cancelled task count.
    pub cancelled: usize,
    /// Failed task count.
    pub failed: usize,
    /// Rejected task count.
    pub rejected: usize,
    /// Running task count.
    pub running: usize,
    /// Cooperative checkpoint count.
    pub checkpoints: usize,
    /// Stream event count.
    pub stream_events: usize,
    /// Missing-checkpoint warning count.
    pub missing_checkpoint_warnings: usize,
    /// Missing-stream-event warning count.
    pub missing_stream_event_warnings: usize,
    /// Recent lifecycle events with timing removed.
    pub recent: Vec<NormalizedBusinessEvent>,
}

/// One state-only recent business event.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedBusinessEvent {
    /// Stable task name.
    pub name: String,
    /// Stable priority label.
    pub priority: String,
    /// Stable lifecycle-state label.
    pub state: String,
}

/// Normalized UI responsiveness diagnostics with timing removed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedUiDiagnostics {
    /// Number of update handlers observed.
    pub update_handlers: usize,
    /// Number of slow handlers observed.
    pub slow_update_handlers: usize,
    /// Most recent slow-handler identity.
    pub last_slow_handler: Option<NormalizedSlowHandler>,
}

/// Identity of a slow update handler without elapsed timing.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedSlowHandler {
    /// Bridge handler type name.
    pub handler: String,
    /// Message type name.
    pub message: String,
}

/// Normalized lifecycle diagnostics.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedLifecycleDiagnostics {
    /// Whether lifecycle evidence is available.
    pub available: bool,
    /// Current lifecycle phase.
    pub phase: String,
    /// Accepted transition count.
    pub transition_count: u64,
    /// Bounded lifecycle history.
    pub history: Vec<NormalizedLifecycleTransition>,
}

/// One normalized lifecycle transition.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedLifecycleTransition {
    /// Transition sequence.
    pub sequence: u64,
    /// Previous phase.
    pub from: String,
    /// Next phase.
    pub to: String,
}

/// Explicit pending-work controls represented in a normalized snapshot.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedPendingWork {
    /// Pending worker tasks.
    pub workers: Vec<NormalizedPendingWorker>,
    /// Pending platform requests.
    pub platform_requests: Vec<NormalizedPendingPlatform>,
    /// Timer registrations awaiting virtual time.
    pub scheduled_timers: usize,
    /// Queue items awaiting a runtime turn.
    pub queued_items: usize,
    /// Runtime-owned completions awaiting a runtime turn.
    pub controller_completions: usize,
    /// Whether production runtime work remains after the last turn.
    pub runtime_work_remaining: bool,
}

/// One normalized pending worker.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedPendingWorker {
    /// Worker id.
    pub id: WorkerTaskId,
    /// Stable task name.
    pub name: String,
    /// Stable priority label.
    pub priority: String,
}

/// One normalized pending platform request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedPendingPlatform {
    /// Platform request id.
    pub id: PlatformRequestId,
    /// Stable request-kind label.
    pub kind: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        gui::types::Point,
        layout::{ContainerPolicy, SlotParams},
        runtime::{
            DeclarativeOwnedRuntimeBridge, LayerKind, SurfaceChild, SurfaceLayer, SurfaceNode,
            WidgetMessageMapper,
        },
        widgets::{ButtonWidget, DragHandleWidget, WidgetSizing, WidgetStyle},
    };

    fn empty_surface<Message>() -> UiSurface<Message> {
        UiSurface::new(SurfaceNode::container(
            1,
            ContainerPolicy::default(),
            Vec::new(),
        ))
    }

    fn config() -> DeterministicHostConfig {
        DeterministicHostConfig::new(Vector2::new(160.0, 80.0))
    }

    #[test]
    fn config_rejects_invalid_viewport_and_zero_capacity() {
        let error = DeterministicHostConfig::new(Vector2::new(f32::NAN, 80.0))
            .validate()
            .expect_err("non-finite viewport must be rejected");
        assert_eq!(error, DeterministicHostConfigError::InvalidViewport);

        let error = config()
            .with_max_pending_workers(0)
            .validate()
            .expect_err("zero worker capacity must be rejected");
        assert_eq!(
            error,
            DeterministicHostConfigError::ZeroCapacity {
                lane: DeterministicLane::Workers,
            }
        );
    }

    #[test]
    fn config_rejects_unwakeupable_timer_capacity() {
        let config = config()
            .with_max_pending_timers(2)
            .with_max_pending_queue_items(1);
        assert_eq!(
            config
                .validate()
                .expect_err("every registered timer must fit the queue lane"),
            DeterministicHostConfigError::TimerQueueCapacityMismatch {
                timers: 2,
                queue: 1,
            }
        );

        let bridge = DeclarativeOwnedRuntimeBridge::new((), |_| empty_surface(), |_, _: ()| {});
        assert!(matches!(
            DeterministicHost::new(bridge, config),
            Err(DeterministicHostError::InvalidConfiguration(
                DeterministicHostConfigError::TimerQueueCapacityMismatch {
                    timers: 2,
                    queue: 1,
                }
            ))
        ));
    }

    #[test]
    fn step_budget_rejects_unbounded_turn_work() {
        let bridge = RecordingBridge {
            messages: Vec::new(),
        };
        assert!(matches!(
            DeterministicHost::new(bridge, config().with_step_budget(0)),
            Err(DeterministicHostError::InvalidConfiguration(
                DeterministicHostConfigError::ZeroStepBudget
            ))
        ));

        let bridge = RecordingBridge {
            messages: Vec::new(),
        };
        let mut host = DeterministicHost::new(
            bridge,
            config()
                .with_max_pending_queue_items(65)
                .with_step_budget(1),
        )
        .expect("host construction");
        for message in 0..65 {
            host.enqueue_message(message)
                .expect("bounded message admission");
        }

        assert_eq!(
            host.run_until_idle(),
            Err(DeterministicHostError::StepBudgetExceeded { budget: 1 })
        );
        assert_eq!(host.turn_count(), 1);
        assert_eq!(host.bridge().messages.len(), 64);
    }

    #[test]
    fn time_and_identifier_overflow_fail_without_publishing() {
        let mut time_host = recording_host();
        let before = time_host.published_snapshot().clone();
        time_host.virtual_time = Duration::new(u64::MAX, 999_999_999);
        assert_eq!(
            time_host.advance_time(Duration::from_nanos(1)),
            Err(DeterministicHostError::TimeOverflow)
        );
        assert_eq!(time_host.published_snapshot(), &before);

        let mut identifier_host = recording_host();
        let before = identifier_host.published_snapshot().clone();
        identifier_host.runtime.bridge_mut().next_timer_sequence = u64::MAX;
        assert_eq!(
            identifier_host.execute_command(Command::after(Duration::ZERO, 1)),
            Err(DeterministicHostError::IdentifierOverflow)
        );
        assert_eq!(identifier_host.published_snapshot(), &before);
        assert_eq!(identifier_host.pending_timer_count(), 0);
    }

    #[test]
    fn pending_timers_reserve_queue_capacity_from_queued_work() {
        let bounded_config = config()
            .with_max_pending_timers(1)
            .with_max_pending_queue_items(1);
        let bridge = RecordingBridge {
            messages: Vec::new(),
        };
        let mut timer_first =
            DeterministicHost::new(bridge, bounded_config).expect("host construction");
        timer_first
            .execute_command(Command::after(Duration::ZERO, 9))
            .expect("timer admission");
        assert!(matches!(
            timer_first.enqueue_message(1),
            Err(DeterministicHostError::Capacity {
                lane: DeterministicLane::Queue,
                limit: 1,
            })
        ));
        timer_first
            .advance_time(Duration::ZERO)
            .expect("timer release");
        timer_first
            .run_until_idle()
            .expect("timer delivery after the later turn");
        assert_eq!(timer_first.bridge().messages, vec![9]);

        let bridge = RecordingBridge {
            messages: Vec::new(),
        };
        let mut queue_first =
            DeterministicHost::new(bridge, bounded_config).expect("host construction");
        queue_first.enqueue_message(1).expect("queue admission");
        assert!(matches!(
            queue_first.execute_command(Command::after(Duration::ZERO, 9)),
            Err(DeterministicHostError::Capacity {
                lane: DeterministicLane::Queue,
                limit: 1,
            })
        ));
        queue_first
            .run_until_idle()
            .expect("queued message delivery");
        assert_eq!(queue_first.bridge().messages, vec![1]);
    }

    #[test]
    fn focus_and_overlay_use_production_dispatch_and_paint_paths() {
        let bridge = DeclarativeOwnedRuntimeBridge::new(
            (),
            |_| {
                UiSurface::new(SurfaceNode::scene(
                    1,
                    SurfaceNode::widget(
                        ButtonWidget::new(
                            10,
                            "Focus",
                            WidgetSizing::fixed(Vector2::new(80.0, 28.0)),
                        ),
                        WidgetMessageMapper::none(),
                    ),
                    vec![SurfaceLayer::new(
                        LayerKind::Floating,
                        SurfaceNode::overlay_panel(
                            20,
                            Rect::from_min_size(Point::new(4.0, 4.0), Vector2::new(40.0, 20.0)),
                            "Overlay",
                            WidgetStyle::default(),
                        ),
                    )],
                ))
            },
            |_, _: ()| {},
        );
        let mut host = DeterministicHost::new(bridge, config()).expect("host construction");

        assert_eq!(
            host.dispatch_event(Event::primary_press(Point::new(12.0, 12.0)))
                .expect("pointer dispatch"),
            Some(10)
        );
        let snapshot = host.turn().expect("publish focused snapshot");
        assert_eq!(snapshot.focus.focused_widget, Some(10));
        assert!(snapshot.paint.total > 0);
        assert_eq!(host.paint_plan().stats().total, snapshot.paint.total);
    }

    #[test]
    fn tooltip_reveals_at_exact_virtual_delay_and_repeated_hosts_match_bytes() {
        let make_host = || {
            let bridge = DeclarativeOwnedRuntimeBridge::new(
                (),
                |_| {
                    let mut button = ButtonWidget::new(
                        10,
                        "Hinted",
                        WidgetSizing::fixed(Vector2::new(80.0, 28.0)),
                    );
                    button.common.tooltip = Some(String::from("Exact tooltip"));
                    UiSurface::new(SurfaceNode::widget(button, WidgetMessageMapper::none()))
                },
                |_, _: ()| {},
            );
            DeterministicHost::new(bridge, config()).expect("host construction")
        };

        let mut first = make_host();
        first
            .dispatch_event(Event::pointer_move(Point::new(12.0, 12.0)))
            .expect("hover dispatch");
        first
            .advance_time(Duration::from_millis(499))
            .expect("pre-deadline advance");
        assert!(!first.paint_plan().contains_text("Exact tooltip"));
        first
            .advance_time(Duration::from_millis(1))
            .expect("exact tooltip deadline");
        assert!(first.paint_plan().contains_text("Exact tooltip"));
        first.turn().expect("publish tooltip snapshot");

        let mut second = make_host();
        second
            .dispatch_event(Event::pointer_move(Point::new(12.0, 12.0)))
            .expect("repeated hover dispatch");
        second
            .advance_time(Duration::from_millis(500))
            .expect("repeated exact tooltip deadline");
        second.turn().expect("publish repeated tooltip snapshot");

        assert_eq!(
            first.snapshot_bytes().expect("first tooltip bytes"),
            second.snapshot_bytes().expect("second tooltip bytes")
        );
    }

    #[test]
    fn hover_only_drag_handle_reveals_at_exact_virtual_delay_and_matches_bytes() {
        let make_host = || {
            let bridge = DeclarativeOwnedRuntimeBridge::new(
                (),
                |_| {
                    UiSurface::new(SurfaceNode::widget(
                        DragHandleWidget::new(10, WidgetSizing::fixed(Vector2::new(24.0, 24.0)))
                            .with_hover_chrome_only(),
                        WidgetMessageMapper::none(),
                    ))
                },
                |_, _: ()| {},
            );
            DeterministicHost::new(bridge, config()).expect("host construction")
        };

        let mut first = make_host();
        first
            .dispatch_event(Event::pointer_move(Point::new(12.0, 12.0)))
            .expect("hover dispatch");
        assert_eq!(first.paint_plan().stats().strokes, 0);
        first
            .advance_time(Duration::from_millis(99))
            .expect("pre-deadline advance");
        assert_eq!(first.paint_plan().stats().strokes, 0);
        first
            .advance_time(Duration::from_millis(1))
            .expect("exact handle deadline");
        assert!(first.paint_plan().stats().strokes > 0);
        first.turn().expect("publish handle snapshot");

        let mut second = make_host();
        second
            .dispatch_event(Event::pointer_move(Point::new(12.0, 12.0)))
            .expect("repeated hover dispatch");
        second
            .advance_time(Duration::from_millis(100))
            .expect("repeated exact handle deadline");
        second.turn().expect("publish repeated handle snapshot");

        assert_eq!(
            first.snapshot_bytes().expect("first handle bytes"),
            second.snapshot_bytes().expect("second handle bytes")
        );
    }

    #[test]
    fn focused_drag_handle_cancellation_uses_the_virtual_clock() {
        let bridge = DeclarativeOwnedRuntimeBridge::new(
            (),
            |_| {
                UiSurface::new(SurfaceNode::widget(
                    DragHandleWidget::new(10, WidgetSizing::fixed(Vector2::new(24.0, 24.0)))
                        .with_hover_chrome_only(),
                    WidgetMessageMapper::none(),
                ))
            },
            |_, _: ()| {},
        );
        let mut host = DeterministicHost::new(bridge, config()).expect("host construction");
        let point = Point::new(12.0, 12.0);

        host.dispatch_event(Event::pointer_move(point))
            .expect("hover dispatch");
        host.dispatch_event(Event::primary_press(point))
            .expect("press dispatch");
        host.dispatch_event(Event::clear_focus())
            .expect("focus clear dispatch");
        assert_eq!(host.paint_plan().stats().strokes, 0);

        host.advance_time(Duration::from_millis(100))
            .expect("exact cancellation hover deadline");
        assert!(host.paint_plan().stats().strokes > 0);
    }

    #[test]
    fn overlay_dismissal_reprojects_controlled_state() {
        #[derive(Clone, Copy)]
        enum Message {
            Dismiss,
        }

        let bridge = DeclarativeOwnedRuntimeBridge::new(
            true,
            |visible| {
                let base = SurfaceNode::container(
                    1,
                    ContainerPolicy::default(),
                    vec![SurfaceChild::new(
                        SlotParams::fill(),
                        SurfaceNode::container(2, ContainerPolicy::default(), Vec::new()),
                    )],
                );
                if *visible {
                    UiSurface::new(SurfaceNode::scene(
                        1,
                        base,
                        vec![SurfaceLayer::new(
                            LayerKind::Modal,
                            SurfaceNode::overlay_panel(
                                20,
                                Rect::from_min_size(Point::new(4.0, 4.0), Vector2::new(40.0, 20.0)),
                                "Modal",
                                WidgetStyle::default(),
                            ),
                        )],
                    ))
                } else {
                    UiSurface::new(base)
                }
            },
            |visible, message| {
                if matches!(message, Message::Dismiss) {
                    *visible = false;
                }
            },
        );
        let mut host = DeterministicHost::new(bridge, config()).expect("host construction");
        let overlay_paint_total = host.published_snapshot().paint.total;
        assert!(overlay_paint_total > 0);

        host.dispatch_message(Message::Dismiss)
            .expect("dismiss message");
        let snapshot = host.turn().expect("publish dismissed snapshot");
        assert!(snapshot.paint.total < overlay_paint_total);
        assert_eq!(host.bridge().state(), &false);
    }

    struct RecordingBridge {
        messages: Vec<u8>,
    }

    impl RuntimeBridge<u8> for RecordingBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<u8>> {
            crate::runtime::test_arc_surface(empty_surface())
        }

        fn update(&mut self, message: u8) -> Command<u8> {
            self.messages.push(message);
            Command::none()
        }
    }

    fn recording_host() -> DeterministicHost<RecordingBridge, u8> {
        DeterministicHost::new(
            RecordingBridge {
                messages: Vec::new(),
            },
            config(),
        )
        .expect("host construction")
    }

    fn latest_worker_command(
        effect_id: u64,
        transaction: crate::application::LatestTaskTransaction,
        ticket: crate::application::TaskTicket,
        message: u8,
    ) -> Command<u8> {
        Command::perform_worker_effect_with_identity_and_transaction(
            crate::runtime::EffectId(effect_id),
            "deterministic-test-worker",
            TaskPriority::Background,
            None,
            ticket.id(),
            Some(transaction),
            || 1_u8,
            move |_| message,
        )
    }

    #[test]
    fn stale_worker_completion_is_fenced_by_generation_and_waits_for_a_later_turn() {
        let mut host = recording_host();
        let mut latest = crate::application::LatestTask::new();
        let first_transaction = latest.begin_replacement();
        let first_ticket = first_transaction.replacement();
        host.execute_command(latest_worker_command(1, first_transaction, first_ticket, 1))
            .expect("first worker admission");
        let first_id = host
            .pending_worker_tasks()
            .first()
            .expect("first worker")
            .id;

        let second_transaction = latest.begin_replacement();
        let second_ticket = second_transaction.replacement();
        host.execute_command(latest_worker_command(
            2,
            second_transaction,
            second_ticket,
            2,
        ))
        .expect("second worker admission");
        let second_id = host
            .pending_worker_tasks()
            .get(1)
            .expect("second worker")
            .id;

        host.complete_worker(first_id)
            .expect("stale completion action");
        assert!(host.bridge().messages.is_empty());
        host.turn().expect("stale completion turn");
        assert!(host.bridge().messages.is_empty());

        host.complete_worker(second_id)
            .expect("current completion action");
        assert!(host.bridge().messages.is_empty());
        host.turn().expect("current completion turn");
        assert_eq!(host.bridge().messages, vec![2]);
    }

    #[test]
    fn publication_is_atomic_and_repeated_runs_are_byte_identical() {
        let mut host = recording_host();
        let before = host.published_snapshot().clone();

        let mut latest = crate::application::LatestTask::new();
        let transaction = latest.begin_replacement();
        let ticket = transaction.replacement();
        host.execute_command(latest_worker_command(3, transaction, ticket, 7))
            .expect("worker admission");
        let id = host.pending_worker_tasks()[0].id;
        host.complete_worker(id).expect("completion action");
        assert_eq!(host.published_snapshot(), &before);
        host.turn().expect("publish completion");
        assert_ne!(host.published_snapshot(), &before);

        let mut first = recording_host();
        let mut second = recording_host();
        for repeated in [&mut first, &mut second] {
            repeated
                .execute_command(Command::after(Duration::from_secs(1), 7))
                .expect("repeated timer admission");
            repeated
                .advance_time(Duration::from_secs(1))
                .expect("repeated timer release");
            repeated.turn().expect("repeated timer turn");
        }
        assert_eq!(
            first.snapshot_bytes().expect("first repeated bytes"),
            second.snapshot_bytes().expect("second repeated bytes")
        );
    }

    #[test]
    fn completion_error_matrix_is_explicit() {
        let mut host = recording_host();
        let mut latest = crate::application::LatestTask::new();
        let transaction = latest.begin_replacement();
        let ticket = transaction.replacement();
        host.execute_command(latest_worker_command(4, transaction, ticket, 4))
            .expect("worker admission");
        let worker_id = host.pending_worker_tasks()[0].id;
        assert!(matches!(
            host.complete_worker(WorkerTaskId(worker_id.get() + 100)),
            Err(DeterministicHostError::UnknownWorker(_))
        ));
        host.complete_worker(worker_id).expect("worker completion");
        assert!(matches!(
            host.complete_worker(worker_id),
            Err(DeterministicHostError::DuplicateWorkerCompletion(_))
        ));

        host.execute_command(Command::platform_request(PlatformRequest::ReadText, |_| 8))
            .expect("platform admission");
        let request_id = host.pending_platform_requests()[0].id;
        assert!(matches!(
            host.complete_platform_request(request_id, Ok(PlatformResponse::Completed)),
            Err(DeterministicHostError::IncompatiblePlatformResponse { .. })
        ));
        host.complete_platform_request(request_id, Ok(PlatformResponse::Text("ok".to_owned())))
            .expect("platform completion");
        assert!(host.bridge().messages.is_empty());
        host.turn().expect("platform completion turn");
        assert_eq!(host.bridge().messages, vec![8, 4]);
        assert!(matches!(
            host.complete_platform_request(
                request_id,
                Ok(PlatformResponse::Text("again".to_owned()))
            ),
            Err(DeterministicHostError::DuplicatePlatformCompletion(_))
        ));
    }

    #[test]
    fn virtual_time_releases_opaque_timer_wakes_for_a_later_turn() {
        let mut host = recording_host();
        host.execute_command(Command::after(Duration::from_secs(2), 9))
            .expect("timer admission");
        assert_eq!(host.pending_timer_count(), 1);

        host.advance_time(Duration::from_secs(1))
            .expect("partial virtual-time advance");
        assert_eq!(host.pending_timer_count(), 1);
        assert!(host.bridge().messages.is_empty());

        host.advance_time(Duration::from_secs(1))
            .expect("timer release");
        assert_eq!(host.pending_timer_count(), 0);
        assert_eq!(host.pending_queue_item_count(), 1);
        assert!(host.bridge().messages.is_empty());

        host.turn().expect("timer mapping turn");
        assert_eq!(host.bridge().messages, vec![9]);
    }

    #[test]
    fn turn_reports_late_capacity_without_publishing_a_partial_snapshot() {
        let mut host = DeterministicHost::new(
            RecordingBridge {
                messages: Vec::new(),
            },
            config().with_max_pending_timers(1),
        )
        .expect("host construction");
        let before = host.published_snapshot().clone();
        host.enqueue_command(Command::after(Duration::from_secs(1), 10))
            .expect("first timer command");
        host.enqueue_command(Command::after(Duration::from_secs(1), 11))
            .expect("second timer command");

        assert!(matches!(
            host.turn(),
            Err(DeterministicHostError::Capacity {
                lane: DeterministicLane::Timers,
                limit: 1,
            })
        ));
        assert_eq!(host.turn_count(), 0);
        assert_eq!(host.published_snapshot(), &before);
    }

    #[test]
    fn malformed_event_is_rejected_before_production_dispatch() {
        let bridge = DeclarativeOwnedRuntimeBridge::new((), |_| empty_surface(), |_, _: ()| {});
        let mut host = DeterministicHost::new(bridge, config()).expect("host construction");
        assert!(matches!(
            host.dispatch_event(Event::pointer_move(Point::new(f32::NAN, 1.0))),
            Err(DeterministicHostError::InvalidEvent(_))
        ));
    }
}
