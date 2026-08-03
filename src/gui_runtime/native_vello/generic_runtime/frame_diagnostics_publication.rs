use crate::runtime::NativeFrameDiagnostics;

/// One-slot handoff for a successfully presented frame.
///
/// Presentation records the value, while the owning event boundary decides
/// when it is safe to deliver it to the public host observer. Keeping the
/// handoff here makes duplicate publication visible without allowing an
/// accidental second value to replace the first one in release builds.
#[derive(Default)]
pub(crate) struct NativeFrameDiagnosticsPublication {
    pending: Option<NativeFrameDiagnostics>,
    observation_finalized: bool,
    schedule_admission_required: bool,
    schedule_admission_recorded: bool,
}

impl NativeFrameDiagnosticsPublication {
    pub(crate) fn stage(&mut self, diagnostics: NativeFrameDiagnostics) {
        if self.pending.is_some() {
            debug_assert!(
                false,
                "native frame diagnostics staged more than once before take"
            );
            return;
        }
        self.pending = Some(diagnostics);
        self.observation_finalized = false;
        self.schedule_admission_recorded = false;
    }

    pub(crate) fn require_schedule_admission(&mut self) {
        self.schedule_admission_required = true;
    }

    pub(crate) fn mark_observation_finalized(&mut self) {
        self.observation_finalized = true;
    }

    pub(crate) fn mark_schedule_admission_recorded(&mut self) {
        if self.pending.is_some() {
            self.schedule_admission_recorded = true;
        } else {
            // A scheduled work item can be admitted without presenting a
            // frame. Do not let that empty admission gate the next direct
            // presentation.
            self.schedule_admission_required = false;
            self.schedule_admission_recorded = false;
        }
    }

    pub(crate) fn take_ready(&mut self) -> Option<NativeFrameDiagnostics> {
        if self.pending.is_some()
            && (!self.observation_finalized
                || (self.schedule_admission_required && !self.schedule_admission_recorded))
        {
            return None;
        }
        self.take()
    }

    pub(crate) fn take(&mut self) -> Option<NativeFrameDiagnostics> {
        let pending = self.pending.take();
        self.clear_state();
        pending
    }

    pub(crate) fn discard(&mut self) {
        self.pending = None;
        self.clear_state();
    }

    fn clear_state(&mut self) {
        self.observation_finalized = false;
        self.schedule_admission_required = false;
        self.schedule_admission_recorded = false;
    }
}

#[cfg(test)]
mod tests {
    use super::NativeFrameDiagnosticsPublication;
    use crate::runtime::{NativeFrameDiagnostics, NativeWindowDiagnosticIdentity};

    fn diagnostics(window_identity: u64, frame_sequence: u64) -> NativeFrameDiagnostics {
        NativeFrameDiagnostics {
            window_identity: Some(NativeWindowDiagnosticIdentity::from_runtime_value(
                window_identity,
            )),
            frame_sequence: Some(frame_sequence),
            ..NativeFrameDiagnostics::default()
        }
    }

    #[test]
    fn stage_and_take_ready_are_one_slot_and_do_not_retain_stale_values() {
        let mut publication = NativeFrameDiagnosticsPublication::default();
        let first = diagnostics(1, 7);
        let second = diagnostics(2, 11);

        publication.stage(first);
        publication.mark_observation_finalized();
        assert_eq!(publication.take_ready(), Some(first));
        assert_eq!(publication.take_ready(), None);

        publication.stage(second);
        publication.mark_observation_finalized();
        assert_eq!(publication.take_ready(), Some(second));
        assert_eq!(publication.take_ready(), None);
    }

    #[test]
    fn early_take_preserves_the_pending_value_until_observation_is_finalized() {
        let mut publication = NativeFrameDiagnosticsPublication::default();
        let diagnostics = diagnostics(1, 7);

        publication.stage(diagnostics);
        assert_eq!(publication.take_ready(), None);

        publication.mark_observation_finalized();
        assert_eq!(publication.take_ready(), Some(diagnostics));
    }

    #[test]
    fn scheduled_readiness_requires_observation_and_admission_marks() {
        let mut publication = NativeFrameDiagnosticsPublication::default();
        let diagnostics = diagnostics(1, 7);

        publication.stage(diagnostics);
        publication.require_schedule_admission();
        assert_eq!(publication.take_ready(), None);

        publication.mark_observation_finalized();
        publication.mark_schedule_admission_recorded();
        assert_eq!(publication.take_ready(), Some(diagnostics));
    }

    #[test]
    fn a_scheduled_attempt_without_a_value_does_not_block_the_next_direct_value() {
        let mut publication = NativeFrameDiagnosticsPublication::default();
        let diagnostics = diagnostics(1, 7);

        publication.require_schedule_admission();
        assert_eq!(publication.take_ready(), None);
        publication.mark_schedule_admission_recorded();

        publication.stage(diagnostics);
        publication.mark_observation_finalized();
        assert_eq!(publication.take_ready(), Some(diagnostics));
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "native frame diagnostics staged more than once before take")]
    fn duplicate_staging_is_rejected_in_debug_builds() {
        let mut publication = NativeFrameDiagnosticsPublication::default();
        publication.stage(diagnostics(1, 7));
        publication.stage(diagnostics(2, 11));
    }

    #[cfg(not(debug_assertions))]
    #[test]
    fn duplicate_staging_keeps_the_first_value_in_release_builds() {
        let mut publication = NativeFrameDiagnosticsPublication::default();
        let first = diagnostics(1, 7);
        publication.stage(first);
        publication.stage(diagnostics(2, 11));
        publication.mark_observation_finalized();

        assert_eq!(publication.take_ready(), Some(first));
    }
}
