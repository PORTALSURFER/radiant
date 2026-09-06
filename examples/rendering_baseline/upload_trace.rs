//! Opt-in capture of the render-profile upload counters for baseline artifacts.

use std::sync::{Arc, Mutex};
use tracing::{
    Event, Metadata, Subscriber,
    field::{Field, Visit},
    span::{Attributes, Id, Record},
    subscriber::Interest,
};

const TRACE_ENV: &str = "RADIANT_BASELINE_UPLOAD_TRACE";
const RENDER_PROFILE_ENV: &str = "RADIANT_NATIVE_RENDER_PROFILE";
const RENDER_PROFILE_TARGET: &str =
    "radiant::gui_runtime::native_vello::generic_runtime::render_profile";
const CAPACITY: usize = 4096;
const FIELDS: [&str; 8] = [
    "window_identity",
    "frame_sequence",
    "gpu_surface_render_canvas_upload_immutable_payload_operations",
    "gpu_surface_render_canvas_upload_immutable_payload_bytes",
    "gpu_surface_render_canvas_upload_volatile_payload_operations",
    "gpu_surface_render_canvas_upload_volatile_payload_bytes",
    "gpu_surface_render_canvas_upload_renderer_parameter_operations",
    "gpu_surface_render_canvas_upload_renderer_parameter_bytes",
];

pub(super) struct UploadTrace {
    buffer: Option<Arc<Mutex<TraceBuffer>>>,
}

impl UploadTrace {
    pub(super) fn from_environment() -> Result<Self, String> {
        match std::env::var(TRACE_ENV).ok().as_deref() {
            None | Some("") => Ok(Self { buffer: None }),
            Some("1") if std::env::var(RENDER_PROFILE_ENV).ok().as_deref() == Some("1") => {
                Ok(Self {
                    buffer: Some(Arc::new(Mutex::new(TraceBuffer::default()))),
                })
            }
            Some("1") => Err(format!(
                "{TRACE_ENV}=1 requires {RENDER_PROFILE_ENV}=1; set both before launching the recorder"
            )),
            Some(value) => Err(format!("{TRACE_ENV} must be unset or 1, got {value:?}")),
        }
    }

    pub(super) fn run<T>(&self, run: impl FnOnce() -> T) -> T {
        let Some(buffer) = &self.buffer else {
            return run();
        };
        tracing::subscriber::with_default(
            tracing::Dispatch::new(UploadTraceSubscriber {
                buffer: Arc::clone(buffer),
            }),
            run,
        )
    }

    pub(super) fn append_artifacts(&self, observations: &mut Vec<serde_json::Value>) {
        let Some(buffer) = &self.buffer else {
            return;
        };
        let buffer = buffer
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        observations.push(serde_json::json!({
            "type": "native_upload_trace_config",
            "enabled": true,
            "upload_trace_environment": TRACE_ENV,
            "render_profile_environment": RENDER_PROFILE_ENV,
            "render_profile_enabled": true,
            "event_target": RENDER_PROFILE_TARGET,
            "fields": FIELDS,
            "capacity": CAPACITY,
            "overhead": "opt-in tracing capture; do not compare its timings with runs captured without it",
        }));
        observations.extend(buffer.events.iter().map(UploadTraceEvent::artifact));
        observations.push(serde_json::json!({
            "type": "native_upload_trace_summary",
            "captured_event_count": buffer.events.len(),
            "dropped_event_count": buffer.dropped,
            "truncated": buffer.dropped > 0,
        }));
    }
}

#[derive(Default)]
struct TraceBuffer {
    events: Vec<UploadTraceEvent>,
    dropped: u64,
}

impl TraceBuffer {
    fn push(&mut self, event: UploadTraceEvent) {
        if self.events.len() < CAPACITY {
            self.events.push(event);
        } else {
            self.dropped = self.dropped.saturating_add(1);
        }
    }
}

#[derive(Default)]
struct UploadTraceEvent {
    window_identity: Option<u64>,
    frame_sequence: Option<u64>,
    immutable_payload_operations: Option<u64>,
    immutable_payload_bytes: Option<u64>,
    volatile_payload_operations: Option<u64>,
    volatile_payload_bytes: Option<u64>,
    renderer_parameter_operations: Option<u64>,
    renderer_parameter_bytes: Option<u64>,
}

impl UploadTraceEvent {
    fn artifact(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "native_upload_trace",
            "window_identity": self.window_identity,
            "frame_sequence": self.frame_sequence,
            "immutable_payload_operations": self.immutable_payload_operations,
            "immutable_payload_bytes": self.immutable_payload_bytes,
            "volatile_payload_operations": self.volatile_payload_operations,
            "volatile_payload_bytes": self.volatile_payload_bytes,
            "renderer_parameter_operations": self.renderer_parameter_operations,
            "renderer_parameter_bytes": self.renderer_parameter_bytes,
        })
    }

    fn record(&mut self, field: &Field, value: u64) {
        match field.name() {
            "window_identity" => self.window_identity = Some(value),
            "frame_sequence" => self.frame_sequence = Some(value),
            "gpu_surface_render_canvas_upload_immutable_payload_operations" => {
                self.immutable_payload_operations = Some(value)
            }
            "gpu_surface_render_canvas_upload_immutable_payload_bytes" => {
                self.immutable_payload_bytes = Some(value)
            }
            "gpu_surface_render_canvas_upload_volatile_payload_operations" => {
                self.volatile_payload_operations = Some(value)
            }
            "gpu_surface_render_canvas_upload_volatile_payload_bytes" => {
                self.volatile_payload_bytes = Some(value)
            }
            "gpu_surface_render_canvas_upload_renderer_parameter_operations" => {
                self.renderer_parameter_operations = Some(value)
            }
            "gpu_surface_render_canvas_upload_renderer_parameter_bytes" => {
                self.renderer_parameter_bytes = Some(value)
            }
            _ => {}
        }
    }
}

#[derive(Clone)]
struct UploadTraceSubscriber {
    buffer: Arc<Mutex<TraceBuffer>>,
}

impl Subscriber for UploadTraceSubscriber {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        is_upload_event(metadata)
    }

    fn new_span(&self, _: &Attributes<'_>) -> Id {
        Id::from_u64(1)
    }

    fn record(&self, _: &Id, _: &Record<'_>) {}

    fn record_follows_from(&self, _: &Id, _: &Id) {}

    fn event(&self, event: &Event<'_>) {
        if !is_upload_event(event.metadata()) {
            return;
        }
        let mut captured = UploadTraceEvent::default();
        event.record(&mut captured);
        self.buffer
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(captured);
    }

    fn enter(&self, _: &Id) {}

    fn exit(&self, _: &Id) {}

    fn register_callsite(&self, metadata: &'static Metadata<'static>) -> Interest {
        if is_upload_event(metadata) {
            Interest::always()
        } else {
            Interest::never()
        }
    }

    fn max_level_hint(&self) -> Option<tracing::level_filters::LevelFilter> {
        Some(tracing::level_filters::LevelFilter::INFO)
    }

    fn clone_span(&self, id: &Id) -> Id {
        id.clone()
    }

    fn try_close(&self, _: Id) -> bool {
        false
    }
}

impl Visit for UploadTraceEvent {
    fn record_u64(&mut self, field: &Field, value: u64) {
        self.record(field, value);
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        if let Ok(value) = u64::try_from(value) {
            self.record(field, value);
        }
    }

    fn record_debug(&mut self, _: &Field, _: &dyn std::fmt::Debug) {
        // Optional profile fields remain null when the runtime reports None.
    }
}

fn is_upload_event(metadata: &Metadata<'_>) -> bool {
    metadata.is_event()
        && metadata.target() == RENDER_PROFILE_TARGET
        && FIELDS
            .iter()
            .all(|field| metadata.fields().field(field).is_some())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_keeps_numeric_values_and_optional_fields() {
        // Exercise the visitor's field mapping through the production subscriber.
        let buffer = Arc::new(Mutex::new(TraceBuffer::default()));
        let subscriber = UploadTraceSubscriber {
            buffer: Arc::clone(&buffer),
        };
        tracing::subscriber::with_default(tracing::Dispatch::new(subscriber), || {
            tracing::info!(
                target: RENDER_PROFILE_TARGET,
                window_identity = Option::<u64>::None,
                frame_sequence = 11_u64,
                gpu_surface_render_canvas_upload_immutable_payload_operations = 2_u64,
                gpu_surface_render_canvas_upload_immutable_payload_bytes = 512_u64,
                gpu_surface_render_canvas_upload_volatile_payload_operations = 3_u64,
                gpu_surface_render_canvas_upload_volatile_payload_bytes = 96_u64,
                gpu_surface_render_canvas_upload_renderer_parameter_operations = 1_u64,
                gpu_surface_render_canvas_upload_renderer_parameter_bytes = 64_u64,
                "render profile"
            );
        });
        let buffer = buffer.lock().unwrap();
        assert_eq!(buffer.events.len(), 1);
        let artifact = buffer.events[0].artifact();
        assert!(artifact["window_identity"].is_null());
        assert_eq!(artifact["immutable_payload_bytes"], 512);
        assert_eq!(artifact["renderer_parameter_operations"], 1);
        assert!(artifact["volatile_payload_bytes"].is_number());
    }

    #[test]
    fn subscriber_rejects_incomplete_profile_events() {
        let buffer = Arc::new(Mutex::new(TraceBuffer::default()));
        let subscriber = UploadTraceSubscriber {
            buffer: Arc::clone(&buffer),
        };
        tracing::subscriber::with_default(tracing::Dispatch::new(subscriber), || {
            tracing::info!(
                target: RENDER_PROFILE_TARGET,
                window_identity = 7_u64,
                frame_sequence = 11_u64,
                "unrelated profile"
            );
        });
        assert!(buffer.lock().unwrap().events.is_empty());
    }

    #[test]
    fn buffer_records_its_truncation_count() {
        let mut buffer = TraceBuffer::default();
        for _ in 0..=CAPACITY {
            buffer.push(UploadTraceEvent::default());
        }
        assert_eq!(buffer.events.len(), CAPACITY);
        assert_eq!(buffer.dropped, 1);
    }
}
