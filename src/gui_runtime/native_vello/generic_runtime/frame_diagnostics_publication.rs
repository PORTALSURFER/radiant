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
    }

    pub(crate) fn take(&mut self) -> Option<NativeFrameDiagnostics> {
        self.pending.take()
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
    fn stage_and_take_are_one_slot_and_do_not_retain_stale_values() {
        let mut publication = NativeFrameDiagnosticsPublication::default();
        let first = diagnostics(1, 7);
        let second = diagnostics(2, 11);

        publication.stage(first);
        assert_eq!(publication.take(), Some(first));
        assert_eq!(publication.take(), None);

        publication.stage(second);
        assert_eq!(publication.take(), Some(second));
        assert_eq!(publication.take(), None);
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

        assert_eq!(publication.take(), Some(first));
    }
}
