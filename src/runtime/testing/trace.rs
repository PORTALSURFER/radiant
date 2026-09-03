//! Bounded, versioned deterministic-host trace capture and replay.

#![allow(
    clippy::collapsible_match,
    clippy::large_enum_variant,
    clippy::needless_return,
    clippy::result_large_err,
    clippy::wrong_self_convention
)]

use super::{
    DeterministicHost, DeterministicHostConfig, NORMALIZED_SNAPSHOT_SCHEMA_VERSION,
    NormalizedSnapshot, PlatformRequestId, WorkerTaskId,
};
use crate::{
    gui::types::Vector2,
    runtime::{PlatformResult, WindowColorScheme, WindowEnvironment},
    theme::DpiScale,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::BTreeSet, fmt, time::Duration};

/// Media type used by the v1 trace encoder.
pub const DETERMINISTIC_TRACE_FORMAT: &str = "radiant.deterministic-trace";
/// Current deterministic trace format version.
pub const DETERMINISTIC_TRACE_VERSION: u32 = 1;

const DEFAULT_MAX_BYTES: usize = 1024 * 1024;
const DEFAULT_MAX_OPERATIONS: usize = 4096;
const DEFAULT_MAX_SNAPSHOTS: usize = 4096;
const DEFAULT_MAX_JSON_DEPTH: usize = 64;
const DEFAULT_MAX_VIRTUAL_TIME_NANOS: u128 = 24 * 60 * 60 * 1_000_000_000;

/// Stable caller-owned identity attached to a trace.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeterministicTraceIdentity {
    /// Stable application or test identity.
    pub application: String,
    /// Stable scenario identity.
    pub scenario: String,
}

/// Decode, capture, and replay budgets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeterministicTraceLimits {
    /// Maximum encoded trace size.
    pub max_bytes: usize,
    /// Maximum number of operations.
    pub max_operations: usize,
    /// Maximum number of snapshots/publication boundaries.
    pub max_snapshots: usize,
    /// Maximum JSON nesting depth for caller-owned values.
    pub max_json_depth: usize,
    /// Maximum cumulative virtual time in the trace.
    pub max_virtual_time_nanos: u128,
}

impl Default for DeterministicTraceLimits {
    fn default() -> Self {
        Self {
            max_bytes: DEFAULT_MAX_BYTES,
            max_operations: DEFAULT_MAX_OPERATIONS,
            max_snapshots: DEFAULT_MAX_SNAPSHOTS,
            max_json_depth: DEFAULT_MAX_JSON_DEPTH,
            max_virtual_time_nanos: DEFAULT_MAX_VIRTUAL_TIME_NANOS,
        }
    }
}

/// A bounded first-divergence diagnostic.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TraceDivergence {
    /// Operation index at which the mismatch was observed.
    pub step: usize,
    /// Boundary kind (`initial` or `publication`).
    pub boundary: String,
    /// Publication number, when applicable.
    pub publication: Option<usize>,
    /// JSON path to the first differing value.
    pub json_path: String,
    /// Expected and actual compact values.
    pub expected: Value,
    /// Actual compact value.
    pub actual: Value,
}

/// Summary returned by replay.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeterministicReplayReport {
    /// Number of operations admitted.
    pub operations: usize,
    /// Number of publication boundaries compared.
    pub publications: usize,
    /// First mismatch, if any.
    pub divergence: Option<TraceDivergence>,
}

/// Errors produced before or during trace replay.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DeterministicTraceError {
    /// The trace exceeded a configured bound.
    BudgetExceeded(&'static str),
    /// The trace envelope or a value was malformed.
    Malformed(String),
    /// The JSON input ended before a complete trace could be decoded.
    Truncated(String),
    /// The trace format was not recognized.
    WrongFormat(String),
    /// The trace version was not supported.
    WrongVersion(u32),
    /// An operation violates monotonic ordering or host state.
    InvalidOrder(String),
    /// Replay input could not be applied.
    Replay(String),
    /// The first published output differed.
    Diverged(TraceDivergence),
}

impl fmt::Display for DeterministicTraceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BudgetExceeded(s) => write!(f, "deterministic trace budget exceeded: {s}"),
            Self::Malformed(s) => write!(f, "malformed deterministic trace: {s}"),
            Self::Truncated(s) => write!(f, "truncated deterministic trace: {s}"),
            Self::WrongFormat(s) => write!(f, "wrong deterministic trace format: {s}"),
            Self::WrongVersion(v) => write!(f, "unsupported deterministic trace version: {v}"),
            Self::InvalidOrder(s) => write!(f, "invalid deterministic trace order: {s}"),
            Self::Replay(s) => write!(f, "deterministic trace replay failed: {s}"),
            Self::Diverged(d) => write!(f, "deterministic trace diverged at {}", d.json_path),
        }
    }
}
impl std::error::Error for DeterministicTraceError {}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TraceEnvelope {
    format: String,
    version: u32,
    host: HostConfigSchema,
    identity: DeterministicTraceIdentity,
    initial_state: Value,
    initial_view_identity: Value,
    operations: Vec<TraceOperation>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
enum TraceOperation {
    Input { value: Value },
    AdvanceVirtualTime { nanos: u128 },
    CompleteWorker { id: u64 },
    CompletePlatform { id: u64, result: Value },
    Publication { snapshot: NormalizedSnapshot },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct HostConfigSchema {
    viewport: [f32; 2],
    display_scale: f32,
    color_scheme: Option<String>,
    contrast: bool,
    reduced_motion: bool,
    max_pending_workers: usize,
    max_pending_platform_requests: usize,
    max_pending_timers: usize,
    max_pending_queue_items: usize,
    step_budget: usize,
}

impl HostConfigSchema {
    fn from_config(config: DeterministicHostConfig) -> Self {
        let env = config.environment();
        Self {
            viewport: [config.viewport().x, config.viewport().y],
            display_scale: env.display_scale().factor(),
            color_scheme: match env.color_scheme() {
                Some(super::WindowColorScheme::Light) => Some("light".into()),
                Some(super::WindowColorScheme::Dark) => Some("dark".into()),
                None => None,
            },
            contrast: env.contrast(),
            reduced_motion: env.reduced_motion(),
            max_pending_workers: config.max_pending_workers(),
            max_pending_platform_requests: config.max_pending_platform_requests(),
            max_pending_timers: config.max_pending_timers(),
            max_pending_queue_items: config.max_pending_queue_items(),
            step_budget: config.step_budget(),
        }
    }

    fn to_config(self) -> Result<DeterministicHostConfig, DeterministicTraceError> {
        let scheme = match self.color_scheme.as_deref() {
            None => None,
            Some("light") => Some(WindowColorScheme::Light),
            Some("dark") => Some(WindowColorScheme::Dark),
            Some(_) => {
                return Err(DeterministicTraceError::Malformed(
                    "unknown color scheme".into(),
                ));
            }
        };
        let config = DeterministicHostConfig::new(Vector2::new(self.viewport[0], self.viewport[1]))
            .with_environment(WindowEnvironment::new(
                DpiScale::new(self.display_scale as f64),
                scheme,
                self.contrast,
                self.reduced_motion,
            ))
            .with_max_pending_workers(self.max_pending_workers)
            .with_max_pending_platform_requests(self.max_pending_platform_requests)
            .with_max_pending_timers(self.max_pending_timers)
            .with_max_pending_queue_items(self.max_pending_queue_items)
            .with_step_budget(self.step_budget);
        config
            .validate()
            .map_err(|e| DeterministicTraceError::Malformed(e.to_string()))?;
        Ok(config)
    }
}

/// A validated, immutable deterministic trace.
#[derive(Clone, Debug, PartialEq)]
pub struct DeterministicTrace {
    envelope: TraceEnvelope,
    limits: DeterministicTraceLimits,
}

impl DeterministicTrace {
    /// Decode and validate canonical JSON bytes under `limits`.
    pub fn from_json_bytes(
        bytes: &[u8],
        limits: DeterministicTraceLimits,
    ) -> Result<Self, DeterministicTraceError> {
        if bytes.len() > limits.max_bytes {
            return Err(DeterministicTraceError::BudgetExceeded("bytes"));
        }
        let envelope: TraceEnvelope = serde_json::from_slice(bytes).map_err(|e| {
            if e.classify() == serde_json::error::Category::Eof {
                DeterministicTraceError::Truncated(e.to_string())
            } else {
                DeterministicTraceError::Malformed(e.to_string())
            }
        })?;
        validate_envelope(&envelope, limits)?;
        let canonical = serde_json::to_vec(&envelope)
            .map_err(|e| DeterministicTraceError::Malformed(e.to_string()))?;
        if canonical != bytes {
            return Err(DeterministicTraceError::Malformed(
                "trace is not canonical JSON".into(),
            ));
        }
        Ok(Self { envelope, limits })
    }

    /// Serialize the validated trace using stable compact JSON.
    pub fn to_json_bytes(&self) -> Result<Vec<u8>, DeterministicTraceError> {
        serde_json::to_vec(&self.envelope)
            .map_err(|e| DeterministicTraceError::Malformed(e.to_string()))
    }
    /// Return the trace identity.
    pub fn identity(&self) -> &DeterministicTraceIdentity {
        &self.envelope.identity
    }
    /// Return the host configuration recorded in the trace.
    pub fn host_config(&self) -> DeterministicTraceConfigView {
        DeterministicTraceConfigView(self.envelope.host.clone())
    }
    /// Return the number of recorded operations.
    pub fn operation_count(&self) -> usize {
        self.envelope.operations.len()
    }

    /// Replay through a caller-provided host factory and typed decoders.
    /// The factory runs only after complete trace preflight.
    pub fn replay<Bridge, Message, Factory, Input, Platform, FactoryError, ActionError>(
        &self,
        factory: Factory,
        mut input: Input,
        mut platform: Platform,
    ) -> Result<DeterministicReplayReport, DeterministicTraceError>
    where
        Bridge: super::RuntimeBridge<Message>,
        Factory: FnOnce(
            DeterministicHostConfig,
            &Value,
            &Value,
        ) -> Result<DeterministicHost<Bridge, Message>, FactoryError>,
        Input: FnMut(&mut DeterministicHost<Bridge, Message>, &Value) -> Result<(), ActionError>,
        Platform: FnMut(&Value) -> Result<PlatformResult, ActionError>,
        FactoryError: fmt::Display,
        ActionError: fmt::Display,
    {
        validate_envelope(&self.envelope, self.limits)?;
        let mut host = factory(
            self.envelope.host.clone().to_config()?,
            &self.envelope.initial_state,
            &self.envelope.initial_view_identity,
        )
        .map_err(|e| DeterministicTraceError::Replay(e.to_string()))?;
        let mut publications = 0;
        for (step, operation) in self.envelope.operations.iter().enumerate() {
            match operation {
                TraceOperation::Input { value } => input(&mut host, value)
                    .map_err(|e| DeterministicTraceError::Replay(e.to_string()))?,
                TraceOperation::AdvanceVirtualTime { nanos } => host
                    .advance_time(Duration::from_nanos((*nanos).try_into().map_err(|_| {
                        DeterministicTraceError::InvalidOrder("duration overflow".into())
                    })?))
                    .map_err(|e| DeterministicTraceError::Replay(e.to_string()))?,
                TraceOperation::CompleteWorker { id } => host
                    .complete_worker(WorkerTaskId(*id))
                    .map_err(|e| DeterministicTraceError::Replay(e.to_string()))?,
                TraceOperation::CompletePlatform { id, result } => {
                    let result = platform(result)
                        .map_err(|e| DeterministicTraceError::Replay(e.to_string()))?;
                    host.complete_platform_request(PlatformRequestId(*id), result)
                        .map_err(|e| DeterministicTraceError::Replay(e.to_string()))?;
                }
                TraceOperation::Publication { snapshot } => {
                    if publications == 0 {
                        if let Some(divergence) =
                            first_initial_divergence(snapshot, host.published_snapshot(), step)
                        {
                            return Err(DeterministicTraceError::Diverged(divergence));
                        }
                    } else {
                        host.turn()
                            .map_err(|e| DeterministicTraceError::Replay(e.to_string()))?;
                        if let Some(divergence) = first_divergence(
                            snapshot,
                            host.published_snapshot(),
                            step,
                            publications,
                        ) {
                            return Err(DeterministicTraceError::Diverged(divergence));
                        }
                    }
                    publications += 1;
                }
            }
        }
        Ok(DeterministicReplayReport {
            operations: self.envelope.operations.len(),
            publications,
            divergence: None,
        })
    }
}

/// Read-only exact configuration values in a trace.
#[derive(Clone, Debug, PartialEq)]
pub struct DeterministicTraceConfigView(HostConfigSchema);
impl DeterministicTraceConfigView {
    /// Return the viewport.
    pub fn viewport(&self) -> [f32; 2] {
        self.0.viewport
    }
    /// Return the configured display scale.
    pub fn display_scale(&self) -> f32 {
        self.0.display_scale
    }
}

/// Mutable capture builder for a deterministic trace.
#[derive(Clone, Debug)]
pub struct DeterministicTraceCapture {
    envelope: TraceEnvelope,
    limits: DeterministicTraceLimits,
}

impl DeterministicTraceCapture {
    /// Start a capture with exact host configuration and initial caller state.
    pub fn new(
        identity: DeterministicTraceIdentity,
        config: DeterministicHostConfig,
        initial_state: Value,
        initial_view_identity: Value,
        limits: DeterministicTraceLimits,
    ) -> Result<Self, DeterministicTraceError> {
        config
            .validate()
            .map_err(|e| DeterministicTraceError::Malformed(e.to_string()))?;
        validate_identity(&identity, limits.max_bytes)?;
        validate_bounded_value(&initial_state, limits, "value")?;
        validate_bounded_value(&initial_view_identity, limits, "identity")?;
        let envelope = TraceEnvelope {
            format: DETERMINISTIC_TRACE_FORMAT.into(),
            version: DETERMINISTIC_TRACE_VERSION,
            host: HostConfigSchema::from_config(config),
            identity,
            initial_state,
            initial_view_identity,
            operations: Vec::new(),
        };
        validate_encoded_envelope(&envelope, limits)?;
        Ok(Self { envelope, limits })
    }
    fn push(&mut self, operation: TraceOperation) -> Result<(), DeterministicTraceError> {
        if self.envelope.operations.len() >= self.limits.max_operations {
            return Err(DeterministicTraceError::BudgetExceeded("operations"));
        }
        let mut candidate = self.envelope.clone();
        candidate.operations.push(operation);
        validate_encoded_envelope(&candidate, self.limits)?;
        self.envelope = candidate;
        Ok(())
    }
    /// Record a normalized input payload decoded by the caller at replay time.
    pub fn capture_input(&mut self, value: Value) -> Result<(), DeterministicTraceError> {
        validate_bounded_value(&value, self.limits, "value")?;
        self.push(TraceOperation::Input { value })
    }
    /// Record a non-negative virtual-time advance.
    pub fn advance_virtual_time(&mut self, delta: Duration) -> Result<(), DeterministicTraceError> {
        let next_time = virtual_time_after(
            current_virtual_time_nanos(&self.envelope),
            delta.as_nanos(),
            self.limits,
        )?;
        self.push(TraceOperation::AdvanceVirtualTime {
            nanos: delta.as_nanos(),
        })?;
        debug_assert_eq!(current_virtual_time_nanos(&self.envelope), next_time);
        Ok(())
    }
    /// Record a worker completion through the host API.
    pub fn complete_worker(&mut self, id: WorkerTaskId) -> Result<(), DeterministicTraceError> {
        self.push(TraceOperation::CompleteWorker { id: id.get() })
    }
    /// Record a platform completion payload through the host API.
    pub fn complete_platform(
        &mut self,
        id: PlatformRequestId,
        result: Value,
    ) -> Result<(), DeterministicTraceError> {
        validate_bounded_value(&result, self.limits, "value")?;
        self.push(TraceOperation::CompletePlatform {
            id: id.get(),
            result,
        })
    }
    /// Record one explicit atomic publication boundary.
    pub fn publication(
        &mut self,
        snapshot: NormalizedSnapshot,
    ) -> Result<(), DeterministicTraceError> {
        if self.publication_count() >= self.limits.max_snapshots {
            return Err(DeterministicTraceError::BudgetExceeded("snapshots"));
        }
        let value = serde_json::to_value(&snapshot)
            .map_err(|e| DeterministicTraceError::Malformed(e.to_string()))?;
        validate_bounded_value(&value, self.limits, "snapshot")?;
        self.push(TraceOperation::Publication { snapshot })
    }
    /// Finish and validate the capture.
    pub fn finish(self) -> Result<DeterministicTrace, DeterministicTraceError> {
        validate_envelope(&self.envelope, self.limits)?;
        Ok(DeterministicTrace {
            envelope: self.envelope,
            limits: self.limits,
        })
    }
    fn publication_count(&self) -> usize {
        self.envelope
            .operations
            .iter()
            .filter(|o| matches!(o, TraceOperation::Publication { .. }))
            .count()
    }
}

fn validate_envelope(
    e: &TraceEnvelope,
    limits: DeterministicTraceLimits,
) -> Result<(), DeterministicTraceError> {
    if e.format != DETERMINISTIC_TRACE_FORMAT {
        return Err(DeterministicTraceError::WrongFormat(e.format.clone()));
    }
    if e.version != DETERMINISTIC_TRACE_VERSION {
        return Err(DeterministicTraceError::WrongVersion(e.version));
    }
    if e.host.viewport.iter().any(|v| !v.is_finite() || *v <= 0.0)
        || !e.host.display_scale.is_finite()
        || e.host.display_scale <= 0.0
    {
        return Err(DeterministicTraceError::Malformed(
            "invalid host geometry or scale".into(),
        ));
    }
    e.host.clone().to_config()?;
    if e.operations.len() > limits.max_operations {
        return Err(DeterministicTraceError::BudgetExceeded("operations"));
    }
    if e.operations
        .iter()
        .filter(|o| matches!(o, TraceOperation::Publication { .. }))
        .count()
        > limits.max_snapshots
    {
        return Err(DeterministicTraceError::BudgetExceeded("snapshots"));
    }
    validate_identity(&e.identity, limits.max_bytes)?;
    validate_bounded_value(&e.initial_state, limits, "value")?;
    validate_bounded_value(&e.initial_view_identity, limits, "identity")?;
    let Some(TraceOperation::Publication { snapshot }) = e.operations.first() else {
        return Err(DeterministicTraceError::InvalidOrder(
            "trace must begin with an initial publication".into(),
        ));
    };
    if snapshot.turn != 0 || snapshot.virtual_time_nanos != 0 {
        return Err(DeterministicTraceError::InvalidOrder(
            "initial publication must represent turn 0 and virtual time 0".into(),
        ));
    }
    let mut last_time = 0u128;
    let mut publications = 0usize;
    let mut worker_ids = BTreeSet::new();
    let mut platform_ids = BTreeSet::new();
    for op in &e.operations {
        match op {
            TraceOperation::AdvanceVirtualTime { nanos } => {
                last_time = virtual_time_after(last_time, *nanos, limits)?;
            }
            TraceOperation::Input { value } => validate_bounded_value(value, limits, "value")?,
            TraceOperation::CompletePlatform { id, result } => {
                validate_bounded_value(result, limits, "value")?;
                if *id == 0 {
                    return Err(DeterministicTraceError::InvalidOrder(
                        "zero completion id".into(),
                    ));
                }
                if !platform_ids.insert(*id) {
                    return Err(DeterministicTraceError::InvalidOrder(
                        "duplicate platform completion id".into(),
                    ));
                }
            }
            TraceOperation::CompleteWorker { id } if *id == 0 => {
                return Err(DeterministicTraceError::InvalidOrder(
                    "zero completion id".into(),
                ));
            }
            TraceOperation::CompleteWorker { id } => {
                if !worker_ids.insert(*id) {
                    return Err(DeterministicTraceError::InvalidOrder(
                        "duplicate worker completion id".into(),
                    ));
                }
            }
            TraceOperation::Publication { snapshot } => {
                let value = serde_json::to_value(snapshot)
                    .map_err(|e| DeterministicTraceError::Malformed(e.to_string()))?;
                validate_bounded_value(&value, limits, "snapshot")?;
                if snapshot.schema_version != NORMALIZED_SNAPSHOT_SCHEMA_VERSION {
                    return Err(DeterministicTraceError::InvalidOrder(
                        "invalid snapshot version".into(),
                    ));
                }
                if publications != 0 {
                    let expected_turn = u64::try_from(publications).map_err(|_| {
                        DeterministicTraceError::InvalidOrder(
                            "snapshot publication turn overflow".into(),
                        )
                    })?;
                    if snapshot.turn != expected_turn || snapshot.virtual_time_nanos != last_time {
                        return Err(DeterministicTraceError::InvalidOrder(
                            "invalid snapshot publication order or clock".into(),
                        ));
                    }
                }
                publications += 1;
            }
        }
    }
    if publications == 0 {
        return Err(DeterministicTraceError::InvalidOrder(
            "trace must contain a publication".into(),
        ));
    }
    validate_encoded_envelope(e, limits)
}

fn validate_encoded_envelope(
    envelope: &TraceEnvelope,
    limits: DeterministicTraceLimits,
) -> Result<(), DeterministicTraceError> {
    let encoded = serde_json::to_vec(envelope)
        .map_err(|e| DeterministicTraceError::Malformed(e.to_string()))?;
    if encoded.len() > limits.max_bytes {
        return Err(DeterministicTraceError::BudgetExceeded("bytes"));
    }
    Ok(())
}

fn current_virtual_time_nanos(envelope: &TraceEnvelope) -> u128 {
    envelope
        .operations
        .iter()
        .filter_map(|operation| match operation {
            TraceOperation::AdvanceVirtualTime { nanos } => Some(*nanos),
            _ => None,
        })
        .sum()
}

fn virtual_time_after(
    current: u128,
    advance: u128,
    limits: DeterministicTraceLimits,
) -> Result<u128, DeterministicTraceError> {
    if advance > u64::MAX as u128 {
        return Err(DeterministicTraceError::InvalidOrder(
            "virtual time overflow".into(),
        ));
    }
    let next = current
        .checked_add(advance)
        .ok_or_else(|| DeterministicTraceError::InvalidOrder("virtual time overflow".into()))?;
    if next > Duration::MAX.as_nanos() {
        return Err(DeterministicTraceError::InvalidOrder(
            "virtual time overflow".into(),
        ));
    }
    if next > limits.max_virtual_time_nanos {
        return Err(DeterministicTraceError::BudgetExceeded("virtual time"));
    }
    Ok(next)
}

fn validate_value(value: &Value, max_depth: usize) -> Result<(), DeterministicTraceError> {
    fn walk(value: &Value, depth: usize, max: usize) -> bool {
        if depth > max {
            return false;
        } else {
            match value {
                Value::Array(a) => a.iter().all(|v| walk(v, depth + 1, max)),
                Value::Object(o) => o.values().all(|v| walk(v, depth + 1, max)),
                _ => true,
            }
        }
    }
    if walk(value, 0, max_depth) {
        Ok(())
    } else {
        Err(DeterministicTraceError::BudgetExceeded("JSON depth"))
    }
}

fn validate_bounded_value(
    value: &Value,
    limits: DeterministicTraceLimits,
    budget: &'static str,
) -> Result<(), DeterministicTraceError> {
    validate_value(value, limits.max_json_depth)?;
    let encoded =
        serde_json::to_vec(value).map_err(|e| DeterministicTraceError::Malformed(e.to_string()))?;
    if encoded.len() > limits.max_bytes {
        return Err(DeterministicTraceError::BudgetExceeded(budget));
    }
    Ok(())
}

fn validate_identity(
    identity: &DeterministicTraceIdentity,
    max_bytes: usize,
) -> Result<(), DeterministicTraceError> {
    let encoded = serde_json::to_vec(identity)
        .map_err(|e| DeterministicTraceError::Malformed(e.to_string()))?;
    if encoded.len() > max_bytes {
        Err(DeterministicTraceError::BudgetExceeded("identity"))
    } else {
        Ok(())
    }
}

/// Find the first differing JSON leaf, bounded to a stable path.
pub fn first_divergence(
    expected: &NormalizedSnapshot,
    actual: &NormalizedSnapshot,
    step: usize,
    publication: usize,
) -> Option<TraceDivergence> {
    first_divergence_at_boundary(expected, actual, step, "publication", Some(publication))
}

fn first_initial_divergence(
    expected: &NormalizedSnapshot,
    actual: &NormalizedSnapshot,
    step: usize,
) -> Option<TraceDivergence> {
    first_divergence_at_boundary(expected, actual, step, "initial", None)
}

fn first_divergence_at_boundary(
    expected: &NormalizedSnapshot,
    actual: &NormalizedSnapshot,
    step: usize,
    boundary: &str,
    publication: Option<usize>,
) -> Option<TraceDivergence> {
    let left = serde_json::to_value(expected).ok()?;
    let right = serde_json::to_value(actual).ok()?;
    fn find(a: &Value, b: &Value, path: &mut String) -> Option<(String, Value, Value)> {
        if a == b {
            return None;
        }
        match (a, b) {
            (Value::Object(x), Value::Object(y)) => {
                for (k, av) in x {
                    path.push('/');
                    path.push_str(k);
                    if let Some(v) = y.get(k).and_then(|bv| find(av, bv, path)) {
                        return Some(v);
                    }
                    path.truncate(path.rfind('/').unwrap_or(0));
                }
                Some((path.clone(), a.clone(), b.clone()))
            }
            (Value::Array(x), Value::Array(y)) => {
                let n = x.len().min(y.len());
                for i in 0..n {
                    let old = path.len();
                    path.push('/');
                    path.push_str(&i.to_string());
                    if let Some(v) = find(&x[i], &y[i], path) {
                        return Some(v);
                    }
                    path.truncate(old);
                }
                Some((path.clone(), a.clone(), b.clone()))
            }
            _ => Some((path.clone(), a.clone(), b.clone())),
        }
    }
    let (json_path, expected, actual) = find(&left, &right, &mut String::new())?;
    Some(TraceDivergence {
        step,
        boundary: boundary.into(),
        publication,
        json_path,
        expected,
        actual,
    })
}

// Keep the host type in this module's public surface discoverable without making
// the schema depend on application message or command serialization.
impl<Bridge, Message> DeterministicHost<Bridge, Message>
where
    Bridge: super::RuntimeBridge<Message>,
{
    /// Record the exact snapshot currently published by this host.
    pub fn capture_publication(
        &self,
        capture: &mut DeterministicTraceCapture,
    ) -> Result<(), DeterministicTraceError> {
        capture.publication(self.published_snapshot().clone())
    }

    /// Create a capture seeded with this host's exact initial published output.
    pub fn begin_trace_capture(
        &self,
        identity: DeterministicTraceIdentity,
        limits: DeterministicTraceLimits,
    ) -> Result<DeterministicTraceCapture, DeterministicTraceError> {
        DeterministicTraceCapture::new(identity, self.config(), Value::Null, Value::Null, limits)
            .and_then(|mut c| {
                c.publication(self.published_snapshot().clone())?;
                Ok(c)
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::types::Vector2;
    use crate::runtime::{RuntimeBridge, SurfaceNode, UiSurface};
    use std::sync::Arc;

    #[derive(Default)]
    struct MinimalBridge;

    impl RuntimeBridge<()> for MinimalBridge {
        #[allow(clippy::arc_with_non_send_sync)]
        fn project_surface(&mut self) -> Arc<UiSurface<()>> {
            Arc::new(UiSurface::new(SurfaceNode::column(1, 0.0, Vec::new())))
        }
    }

    fn capture() -> DeterministicTraceCapture {
        DeterministicTraceCapture::new(
            DeterministicTraceIdentity {
                application: "test-app".into(),
                scenario: "round-trip".into(),
            },
            DeterministicHostConfig::new(Vector2::new(100.0, 80.0)),
            serde_json::json!({"state": 1}),
            serde_json::json!({"view": "root"}),
            DeterministicTraceLimits::default(),
        )
        .expect("capture")
    }

    #[test]
    fn canonical_round_trip_and_preflight_rejections() {
        let initial_snapshot = DeterministicHost::new(
            MinimalBridge,
            DeterministicHostConfig::new(Vector2::new(100.0, 80.0)),
        )
        .expect("host")
        .published_snapshot()
        .clone();
        let mut capture = capture();
        capture
            .publication(initial_snapshot)
            .expect("initial publication");
        let trace = capture.finish().expect("finish");
        let bytes = trace.to_json_bytes().expect("encode");
        let decoded =
            DeterministicTrace::from_json_bytes(&bytes, DeterministicTraceLimits::default())
                .expect("decode");
        assert_eq!(decoded.to_json_bytes().expect("re-encode"), bytes);
        assert!(matches!(
            DeterministicTrace::from_json_bytes(
                &bytes[..bytes.len() - 1],
                DeterministicTraceLimits::default()
            ),
            Err(DeterministicTraceError::Truncated(_))
        ));
        assert!(matches!(
            DeterministicTrace::from_json_bytes(
                &bytes,
                DeterministicTraceLimits {
                    max_bytes: 1,
                    ..Default::default()
                }
            ),
            Err(DeterministicTraceError::BudgetExceeded("bytes"))
        ));
        let mut unknown = bytes[..bytes.len() - 1].to_vec();
        unknown.extend_from_slice(b",\"unknown\":true}");
        assert!(matches!(
            DeterministicTrace::from_json_bytes(&unknown, DeterministicTraceLimits::default()),
            Err(DeterministicTraceError::Malformed(_))
        ));
    }
}
