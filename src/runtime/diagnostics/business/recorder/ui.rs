use std::time::Duration;

use super::{RuntimeDiagnosticsRecorder, lock_diagnostics_state};
use crate::runtime::{SLOW_UPDATE_HANDLER_GUIDANCE, UiUpdateHandlerDiagnostic};

impl RuntimeDiagnosticsRecorder {
    pub(crate) fn record_update_handler(
        &self,
        duration: Duration,
        threshold: Duration,
        handler: &'static str,
        message: &'static str,
    ) -> Option<UiUpdateHandlerDiagnostic> {
        let mut state = lock_diagnostics_state(&self.state);
        state.snapshot.ui.update_handlers += 1;
        state.snapshot.ui.longest_update_handler =
            state.snapshot.ui.longest_update_handler.max(duration);
        if duration < threshold {
            return None;
        }
        let diagnostic = UiUpdateHandlerDiagnostic {
            duration,
            threshold,
            handler,
            message,
            guidance: SLOW_UPDATE_HANDLER_GUIDANCE,
        };
        state.snapshot.ui.slow_update_handlers += 1;
        state.snapshot.ui.last_slow_update_handler = Some(diagnostic.clone());
        tracing::warn!(
            update_duration_ms = duration.as_secs_f64() * 1000.0,
            threshold_ms = threshold.as_secs_f64() * 1000.0,
            update_handler = handler,
            update_message = message,
            guidance = SLOW_UPDATE_HANDLER_GUIDANCE,
            "radiant update handler exceeded configured responsiveness threshold"
        );
        Some(diagnostic)
    }
}
