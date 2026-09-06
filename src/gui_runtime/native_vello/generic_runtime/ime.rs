//! Native Winit IME normalization and routing for every Vello window.

use super::{GenericNativeVelloRunner, GenericRouteOutcome};
use crate::gui::input::InputTimestamp;
use crate::runtime::RuntimeBridge;
use crate::runtime::{
    NativeImeAdapterObservation, NativeImeAdapterUnavailableReason, NativeImeBackend,
    NativeImeCandidateCapability, NativeImeCompositionCapability, NativeImeMatchingKeySuppression,
    NativeImeMatchingKeySuppressionUnavailableReason, NativeWindowDiagnosticIdentity,
};
use crate::widgets::{CompositionRange, CompositionSample};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::event::Ime;
use winit::window::Window;

#[cfg(test)]
#[path = "ime/metadata_tests.rs"]
mod metadata_tests;

/// Evidence for transport into the existing shared composition owner. This
/// slot never owns focus, text, or a second composition lifecycle.
#[derive(Default)]
pub(super) struct NativeImeSession {
    sequence: Option<u64>,
    discard_until_boundary: bool,
}

// Reject oversized native strings before scanning UTF-8 or entering widgets.
const MAX_NATIVE_IME_BYTES: usize = 1 << 20;

pub(super) fn native_ime_adapter_observation(
    window: &Window,
    window_identity: Option<NativeWindowDiagnosticIdentity>,
) -> NativeImeAdapterObservation {
    let (backend, composition, candidate, matching_key_suppression) =
        native_ime_adapter_observation_for_handle_result(
            window
                .window_handle()
                .map(|handle| handle.as_raw())
                .map_err(|_| ()),
        );
    NativeImeAdapterObservation {
        window_identity,
        backend,
        composition,
        candidate,
        matching_key_suppression,
    }
}

fn native_ime_adapter_observation_for_raw_handle(
    handle: RawWindowHandle,
) -> (
    NativeImeBackend,
    NativeImeCompositionCapability,
    NativeImeCandidateCapability,
    NativeImeMatchingKeySuppression,
) {
    ime_capabilities_for_backend(match handle {
        RawWindowHandle::AppKit(_) => NativeImeBackend::AppKit,
        RawWindowHandle::Win32(_) => NativeImeBackend::Win32,
        RawWindowHandle::Wayland(_) => NativeImeBackend::Wayland,
        RawWindowHandle::Xlib(_) | RawWindowHandle::Xcb(_) => NativeImeBackend::X11,
        _ => NativeImeBackend::Unknown,
    })
}

fn native_ime_adapter_observation_for_handle_result(
    handle: Result<RawWindowHandle, ()>,
) -> (
    NativeImeBackend,
    NativeImeCompositionCapability,
    NativeImeCandidateCapability,
    NativeImeMatchingKeySuppression,
) {
    handle.map_or_else(
        |_| {
            (
                NativeImeBackend::Unknown,
                NativeImeCompositionCapability::Unavailable(
                    NativeImeAdapterUnavailableReason::WindowHandleUnavailable,
                ),
                NativeImeCandidateCapability::Unavailable(
                    NativeImeAdapterUnavailableReason::WindowHandleUnavailable,
                ),
                NativeImeMatchingKeySuppression::Unavailable(
                    NativeImeMatchingKeySuppressionUnavailableReason::WindowHandleUnavailable,
                ),
            )
        },
        native_ime_adapter_observation_for_raw_handle,
    )
}

const fn ime_capabilities_for_backend(
    backend: NativeImeBackend,
) -> (
    NativeImeBackend,
    NativeImeCompositionCapability,
    NativeImeCandidateCapability,
    NativeImeMatchingKeySuppression,
) {
    let composition = match backend {
        NativeImeBackend::AppKit
        | NativeImeBackend::Win32
        | NativeImeBackend::Wayland
        | NativeImeBackend::X11 => NativeImeCompositionCapability::SupportedByWinit,
        NativeImeBackend::Unknown => NativeImeCompositionCapability::Unavailable(
            NativeImeAdapterUnavailableReason::UnknownBackend,
        ),
    };
    let candidate = match backend {
        NativeImeBackend::AppKit | NativeImeBackend::Win32 | NativeImeBackend::Wayland => {
            NativeImeCandidateCapability::FullCursorAreaByWinit
        }
        NativeImeBackend::X11 => NativeImeCandidateCapability::PositionOnlyByWinit,
        NativeImeBackend::Unknown => NativeImeCandidateCapability::Unavailable(
            NativeImeAdapterUnavailableReason::UnknownBackend,
        ),
    };
    let suppression = match backend {
        NativeImeBackend::AppKit => NativeImeMatchingKeySuppression::VerifiedWinitAppKit,
        NativeImeBackend::Win32 => NativeImeMatchingKeySuppression::Unavailable(
            NativeImeMatchingKeySuppressionUnavailableReason::Win32,
        ),
        NativeImeBackend::Wayland => NativeImeMatchingKeySuppression::Unavailable(
            NativeImeMatchingKeySuppressionUnavailableReason::Wayland,
        ),
        NativeImeBackend::X11 => NativeImeMatchingKeySuppression::Unavailable(
            NativeImeMatchingKeySuppressionUnavailableReason::X11,
        ),
        NativeImeBackend::Unknown => NativeImeMatchingKeySuppression::Unavailable(
            NativeImeMatchingKeySuppressionUnavailableReason::UnknownBackend,
        ),
    };
    (backend, composition, candidate, suppression)
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum NormalizedImeEvent {
    Enabled,
    Preedit {
        preedit: String,
        selection: Option<CompositionRange>,
    },
    Commit(String),
    Disabled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NativeImeNormalizationError {
    InvalidPreeditRange,
    PayloadTooLarge,
}

fn scalar_range_from_winit_bytes(
    text: &str,
    start_byte: usize,
    end_byte: usize,
) -> Result<CompositionRange, NativeImeNormalizationError> {
    if start_byte > end_byte
        || end_byte > text.len()
        || !text.is_char_boundary(start_byte)
        || !text.is_char_boundary(end_byte)
    {
        return Err(NativeImeNormalizationError::InvalidPreeditRange);
    }
    let scalar_len = text.chars().count();
    let start = text[..start_byte].chars().count();
    let end = text[..end_byte].chars().count();
    CompositionRange::new(start, end, scalar_len)
        .map_err(|_| NativeImeNormalizationError::InvalidPreeditRange)
}

fn normalize_winit_ime_event(
    event: Ime,
) -> Result<NormalizedImeEvent, NativeImeNormalizationError> {
    if matches!(&event, Ime::Preedit(text, _) | Ime::Commit(text) if text.len() > MAX_NATIVE_IME_BYTES)
    {
        return Err(NativeImeNormalizationError::PayloadTooLarge);
    }
    Ok(match event {
        Ime::Enabled => NormalizedImeEvent::Enabled,
        Ime::Preedit(preedit, cursor_range) => NormalizedImeEvent::Preedit {
            selection: cursor_range
                .map(|(start_byte, end_byte)| {
                    scalar_range_from_winit_bytes(&preedit, start_byte, end_byte)
                })
                .transpose()?,
            preedit,
        },
        Ime::Commit(text) => NormalizedImeEvent::Commit(text),
        Ime::Disabled => NormalizedImeEvent::Disabled,
    })
}

impl<Bridge, Message> GenericNativeVelloRunner<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    #[cfg(test)]
    pub(super) fn route_native_ime_event(&mut self, event: Ime) -> GenericRouteOutcome {
        self.route_native_ime_event_with_timestamp(event, None)
    }

    /// Primary and auxiliary adapters retain their admitted receipt timestamp.
    pub(super) fn route_native_ime_event_with_timestamp(
        &mut self,
        event: Ime,
        timestamp: Option<InputTimestamp>,
    ) -> GenericRouteOutcome {
        let normalized = match normalize_winit_ime_event(event) {
            Ok(normalized) => normalized,
            Err(_) => {
                let outcome = self.route_native_ime_cancel(timestamp);
                self.input.native_ime.discard_until_boundary = true;
                return outcome;
            }
        };
        match normalized {
            // Enabled is a fresh native capability boundary, never a Start.
            NormalizedImeEvent::Enabled => {
                if self.input.native_ime.sequence
                    != self.core.runtime.managed_composition_sequence()
                {
                    self.input.native_ime.sequence = None;
                }
                self.input.native_ime.discard_until_boundary = false;
                GenericRouteOutcome::default()
            }
            NormalizedImeEvent::Preedit { preedit, selection } => {
                self.route_native_ime_preedit(preedit, selection, timestamp)
            }
            NormalizedImeEvent::Commit(text) => {
                self.frame.text_renderer.reset_native_caret_affinities();
                let admitted = self.ensure_native_ime_composition(timestamp);
                // Retire transport evidence before the terminal mapper can
                // synchronously create another composition or move focus.
                self.input.native_ime = NativeImeSession::default();
                let routed = admitted
                    && self
                        .core
                        .runtime
                        .dispatch_focused_composition_sample(
                            CompositionSample::commit_with_metadata(text, timestamp),
                        )
                        .is_some();
                self.core.route_outcome(routed)
            }
            NormalizedImeEvent::Disabled => self.route_native_ime_cancel(timestamp),
        }
    }

    fn route_native_ime_preedit(
        &mut self,
        preedit: String,
        selection: Option<CompositionRange>,
        timestamp: Option<InputTimestamp>,
    ) -> GenericRouteOutcome {
        self.frame.text_renderer.reset_native_caret_affinities();
        if !self.ensure_native_ime_composition(timestamp) {
            return self.core.route_outcome(false);
        }
        let routed = match selection {
            Some(selection) => {
                let Ok(sample) =
                    CompositionSample::update_with_metadata(preedit, selection, timestamp)
                else {
                    let outcome = self.route_native_ime_cancel(timestamp);
                    self.input.native_ime.discard_until_boundary = true;
                    return outcome;
                };
                self.core
                    .runtime
                    .dispatch_focused_composition_sample(sample)
                    .is_some()
            }
            None => self
                .core
                .runtime
                .dispatch_hidden_composition_update(preedit, timestamp)
                .is_some(),
        };
        self.core.route_outcome(routed)
    }

    fn ensure_native_ime_composition(&mut self, timestamp: Option<InputTimestamp>) -> bool {
        if self.input.native_ime.discard_until_boundary {
            return false;
        }
        let current = self.core.runtime.managed_composition_sequence();
        if let Some(sequence) = self.input.native_ime.sequence {
            if current == Some(sequence) {
                return true;
            }
            self.input.native_ime.sequence = None;
            self.input.native_ime.discard_until_boundary = true;
            return false;
        }
        // Never adopt a composition started through another producer.
        if current.is_some() {
            return false;
        }
        let Some(context) = self.core.focused_composition_start_context() else {
            return false;
        };
        let Ok(start) = CompositionSample::start_with_metadata(
            context.replacement_range(),
            context.selection(),
            timestamp,
        ) else {
            return false;
        };
        self.input.native_ime.sequence = self
            .core
            .runtime
            .dispatch_composition_start_with_sequence(start);
        self.input.native_ime.sequence.is_some()
    }

    fn route_native_ime_cancel(
        &mut self,
        timestamp: Option<InputTimestamp>,
    ) -> GenericRouteOutcome {
        self.frame.text_renderer.reset_native_caret_affinities();
        let previous = std::mem::take(&mut self.input.native_ime);
        let owns_current = previous.sequence.is_some()
            && previous.sequence == self.core.runtime.managed_composition_sequence();
        if owns_current {
            let _ = self.core.runtime.dispatch_focused_composition_sample(
                CompositionSample::cancel_with_metadata(timestamp),
            );
        }
        // Cancelling a retained preedit is routed even when its widget emits
        // no message; the restored committed value still needs repainting.
        self.core.route_outcome(owns_current)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        NativeImeNormalizationError, normalize_winit_ime_event, scalar_range_from_winit_bytes,
    };
    use crate::{
        gui::types::Vector2,
        runtime::{Command, RuntimeBridge, SurfaceNode, UiSurface, WidgetMessageMapper},
        widgets::{CompositionRange, TextInputMessage, TextInputWidget, WidgetSizing},
    };
    use std::{cell::RefCell, rc::Rc, sync::Arc};
    use winit::event::Ime;

    #[test]
    fn ime_adapter_capabilities_are_classified_per_known_backend() {
        use crate::runtime::{
            NativeImeAdapterUnavailableReason, NativeImeBackend, NativeImeCandidateCapability,
            NativeImeCompositionCapability, NativeImeMatchingKeySuppression,
            NativeImeMatchingKeySuppressionUnavailableReason,
        };
        use raw_window_handle::{RawWindowHandle, WebWindowHandle};

        assert_eq!(
            super::ime_capabilities_for_backend(NativeImeBackend::AppKit),
            (
                NativeImeBackend::AppKit,
                NativeImeCompositionCapability::SupportedByWinit,
                NativeImeCandidateCapability::FullCursorAreaByWinit,
                NativeImeMatchingKeySuppression::VerifiedWinitAppKit,
            )
        );
        for (backend, candidate, reason) in [
            (
                NativeImeBackend::Win32,
                NativeImeCandidateCapability::FullCursorAreaByWinit,
                NativeImeMatchingKeySuppressionUnavailableReason::Win32,
            ),
            (
                NativeImeBackend::Wayland,
                NativeImeCandidateCapability::FullCursorAreaByWinit,
                NativeImeMatchingKeySuppressionUnavailableReason::Wayland,
            ),
            (
                NativeImeBackend::X11,
                NativeImeCandidateCapability::PositionOnlyByWinit,
                NativeImeMatchingKeySuppressionUnavailableReason::X11,
            ),
        ] {
            assert_eq!(
                super::ime_capabilities_for_backend(backend),
                (
                    backend,
                    NativeImeCompositionCapability::SupportedByWinit,
                    candidate,
                    NativeImeMatchingKeySuppression::Unavailable(reason)
                )
            );
        }
        assert_eq!(
            super::ime_capabilities_for_backend(NativeImeBackend::Unknown),
            (
                NativeImeBackend::Unknown,
                NativeImeCompositionCapability::Unavailable(
                    NativeImeAdapterUnavailableReason::UnknownBackend,
                ),
                NativeImeCandidateCapability::Unavailable(
                    NativeImeAdapterUnavailableReason::UnknownBackend,
                ),
                NativeImeMatchingKeySuppression::Unavailable(
                    NativeImeMatchingKeySuppressionUnavailableReason::UnknownBackend,
                ),
            )
        );
        assert_eq!(
            super::native_ime_adapter_observation_for_raw_handle(RawWindowHandle::Web(
                WebWindowHandle::new(1),
            )),
            (
                NativeImeBackend::Unknown,
                NativeImeCompositionCapability::Unavailable(
                    NativeImeAdapterUnavailableReason::UnknownBackend,
                ),
                NativeImeCandidateCapability::Unavailable(
                    NativeImeAdapterUnavailableReason::UnknownBackend,
                ),
                NativeImeMatchingKeySuppression::Unavailable(
                    NativeImeMatchingKeySuppressionUnavailableReason::UnknownBackend,
                ),
            )
        );
        assert_eq!(
            super::native_ime_adapter_observation_for_handle_result(Err(())),
            (
                NativeImeBackend::Unknown,
                NativeImeCompositionCapability::Unavailable(
                    NativeImeAdapterUnavailableReason::WindowHandleUnavailable,
                ),
                NativeImeCandidateCapability::Unavailable(
                    NativeImeAdapterUnavailableReason::WindowHandleUnavailable,
                ),
                NativeImeMatchingKeySuppression::Unavailable(
                    NativeImeMatchingKeySuppressionUnavailableReason::WindowHandleUnavailable,
                ),
            )
        );
    }

    #[derive(Clone)]
    struct ImeBridge {
        id: u64,
        messages: Rc<RefCell<Vec<TextInputMessage>>>,
        value: String,
        disabled: bool,
        read_only: bool,
        caret: usize,
        selection_anchor: usize,
    }

    impl Default for ImeBridge {
        fn default() -> Self {
            Self {
                id: 7,
                messages: Rc::new(RefCell::new(Vec::new())),
                value: String::from("a"),
                disabled: false,
                read_only: false,
                caret: 1,
                selection_anchor: 0,
            }
        }
    }

    impl RuntimeBridge<TextInputMessage> for ImeBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<TextInputMessage>> {
            let mut input = TextInputWidget::new(
                self.id,
                self.value.clone(),
                WidgetSizing::fixed(Vector2::new(160.0, 28.0)),
            );
            input.common.state.disabled = self.disabled;
            input.common.state.read_only = self.read_only;
            input.state.caret = self.caret;
            input.state.selection_anchor = self.selection_anchor;
            crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::widget(
                input,
                WidgetMessageMapper::typed(|message: TextInputMessage| message),
            )))
        }

        fn update(&mut self, message: TextInputMessage) -> Command<TextInputMessage> {
            if let TextInputMessage::Changed { value } = &message {
                self.value = value.clone();
            }
            self.messages.borrow_mut().push(message);
            Command::none()
        }
    }

    fn runner() -> super::super::GenericNativeVelloRunner<ImeBridge, TextInputMessage> {
        super::super::GenericNativeVelloRunner::new(
            crate::gui_runtime::NativeRunOptions::default(),
            ImeBridge::default(),
            Vector2::new(200.0, 40.0),
        )
    }

    fn focused_runner() -> super::super::GenericNativeVelloRunner<ImeBridge, TextInputMessage> {
        let mut runner = runner();
        assert!(runner.core.runtime.focus_widget(7));
        runner
    }

    fn focused_auxiliary_runner()
    -> super::super::GenericNativeVelloRunner<ImeBridge, TextInputMessage> {
        let mut runner = super::super::GenericNativeVelloRunner::new_auxiliary(
            crate::gui_runtime::NativeRunOptions::default(),
            ImeBridge::default(),
            Vector2::new(200.0, 40.0),
            String::from("ime"),
        );
        assert!(runner.core.runtime.focus_widget(7));
        runner
    }

    fn text_value(
        runner: &super::super::GenericNativeVelloRunner<ImeBridge, TextInputMessage>,
    ) -> String {
        runner
            .core
            .runtime
            .surface()
            .find_widget(runner.core.runtime.bridge().id)
            .and_then(|widget| {
                widget
                    .widget_object()
                    .as_any()
                    .downcast_ref::<TextInputWidget>()
            })
            .expect("IME fixture should retain its text input")
            .state
            .value
            .clone()
    }

    #[test]
    fn winit_byte_offsets_convert_to_ordered_scalar_ranges() {
        assert_eq!(
            scalar_range_from_winit_bytes("a界b", 1, 4),
            Ok(CompositionRange::new(1, 2, 3).expect("valid scalar range")),
        );
        assert_eq!(
            scalar_range_from_winit_bytes("a界b", 4, 4),
            Ok(CompositionRange::new(2, 2, 3).expect("valid scalar caret")),
        );
    }

    #[test]
    fn malformed_winit_byte_offsets_are_rejected_without_clamping() {
        for (start, end) in [(4, 1), (0, 6), (2, 3), (1, 2)] {
            assert_eq!(
                scalar_range_from_winit_bytes("a界b", start, end),
                Err(NativeImeNormalizationError::InvalidPreeditRange),
                "range {start}..{end} should be rejected",
            );
        }
    }

    #[test]
    fn winit_none_cursor_range_remains_explicitly_hidden() {
        let normalized = normalize_winit_ime_event(Ime::Preedit("あい".into(), None))
            .expect("hidden cursor is valid Winit evidence");

        assert!(matches!(
            normalized,
            super::NormalizedImeEvent::Preedit {
                selection: None,
                ..
            }
        ));
    }

    #[test]
    fn enabled_is_capability_only_and_first_preedit_captures_focused_context() {
        let mut runner = focused_runner();

        assert_eq!(
            runner
                .core
                .focused_composition_start_context()
                .expect("focused text input should expose start context")
                .replacement_range(),
            CompositionRange::new(0, 1, 1).expect("selected scalar context"),
        );
        assert!(!runner.core.managed_composition_is_active());
        assert!(!runner.route_native_ime_event(Ime::Enabled).routed);
        assert!(!runner.core.managed_composition_is_active());

        let outcome =
            runner.route_native_ime_event(Ime::Preedit(String::from("あい"), Some((3, 6))));
        assert!(outcome.routed);
        assert!(runner.core.managed_composition_is_active());
        assert_eq!(text_value(&runner), "あい");
        assert!(runner.core.runtime.bridge().messages.borrow().is_empty());
    }

    #[test]
    fn focused_start_context_uses_current_multibyte_scalar_selection() {
        let mut runner = runner();
        {
            let bridge = runner.core.runtime.bridge_mut();
            bridge.value = String::from("a界b");
            bridge.caret = 1;
            bridge.selection_anchor = 0;
        }
        runner.core.runtime.refresh();
        assert!(runner.core.runtime.focus_widget(7));

        let context = runner
            .core
            .focused_composition_start_context()
            .expect("focused text input should expose start context");
        let expected = CompositionRange::new(0, 1, 3).expect("scalar selection");
        assert_eq!(context.replacement_range(), expected);
        assert_eq!(context.selection(), expected);
    }

    #[test]
    fn direct_commit_replaces_once_and_disabled_cancels_without_commit() {
        let mut direct = focused_runner();
        assert!(
            direct
                .route_native_ime_event(Ime::Commit(String::from("界")))
                .routed
        );
        assert_eq!(text_value(&direct), "界");
        assert_eq!(direct.core.runtime.bridge().messages.borrow().len(), 1);
        assert!(!direct.core.managed_composition_is_active());

        let mut cancelled = focused_runner();
        assert!(
            cancelled
                .route_native_ime_event(Ime::Preedit(String::from("あ"), None))
                .routed
        );
        assert!(cancelled.route_native_ime_event(Ime::Disabled).routed);
        assert_eq!(text_value(&cancelled), "a");
        assert!(cancelled.core.runtime.bridge().messages.borrow().is_empty());
        assert!(!cancelled.core.managed_composition_is_active());
    }

    #[test]
    fn malformed_native_range_cancels_active_preedit_without_mutation() {
        let mut runner = focused_runner();
        assert!(
            runner
                .route_native_ime_event(Ime::Preedit(String::from("あ"), None))
                .routed
        );
        assert_eq!(text_value(&runner), "あ");

        assert!(
            runner
                .route_native_ime_event(Ime::Preedit(String::from("界"), Some((1, 2))))
                .routed
        );
        assert_eq!(text_value(&runner), "a");
        assert!(runner.core.runtime.bridge().messages.borrow().is_empty());
        assert!(!runner.core.managed_composition_is_active());
    }

    #[test]
    fn hidden_preedit_replaces_repeatedly_and_empty_hidden_stays_owner_bound() {
        let mut runner = focused_runner();

        assert!(
            runner
                .route_native_ime_event(Ime::Preedit(String::from("あ"), None))
                .routed
        );
        assert_eq!(text_value(&runner), "あ");
        assert!(runner.core.managed_composition_is_active());

        assert!(
            runner
                .route_native_ime_event(Ime::Preedit(String::from("いう"), None))
                .routed
        );
        assert_eq!(text_value(&runner), "いう");

        assert!(
            runner
                .route_native_ime_event(Ime::Preedit(String::new(), None))
                .routed
        );
        assert_eq!(text_value(&runner), "");
        assert!(runner.core.managed_composition_is_active());

        assert!(
            runner
                .route_native_ime_event(Ime::Preedit(String::from("あい"), Some((3, 6))))
                .routed
        );
        assert_eq!(text_value(&runner), "あい");
    }

    #[test]
    fn malformed_native_range_without_active_owner_is_not_routed() {
        let mut runner = runner();

        assert!(
            !runner
                .route_native_ime_event(Ime::Preedit(String::from("あ"), Some((1, 2))))
                .routed
        );
        assert_eq!(text_value(&runner), "a");
        assert!(!runner.core.managed_composition_is_active());
    }

    #[test]
    fn disabled_and_read_only_focus_targets_do_not_admit_native_composition() {
        for (disabled, read_only) in [(true, false), (false, true)] {
            let mut runner = runner();
            runner.core.runtime.bridge_mut().disabled = disabled;
            runner.core.runtime.bridge_mut().read_only = read_only;
            runner.core.runtime.refresh();
            let _ = runner.core.runtime.focus_widget(7);
            assert_eq!(runner.core.focused_composition_start_context(), None);
            assert!(
                !runner
                    .route_native_ime_event(Ime::Preedit(String::from("あ"), None))
                    .routed
            );
            assert_eq!(text_value(&runner), "a");
        }
    }

    #[test]
    fn primary_and_auxiliary_runners_share_identical_ime_behavior() {
        let mut primary = focused_runner();
        let mut auxiliary = focused_auxiliary_runner();
        for runner in [&mut primary, &mut auxiliary] {
            assert!(!runner.route_native_ime_event(Ime::Enabled).routed);
            assert!(
                runner
                    .route_native_ime_event(Ime::Preedit(String::from("あ"), None))
                    .routed
            );
            assert!(
                runner
                    .route_native_ime_event(Ime::Preedit(String::new(), Some((0, 0))))
                    .routed
            );
            assert!(
                runner
                    .route_native_ime_event(Ime::Preedit(String::from("あい"), Some((3, 6)),))
                    .routed
            );
            assert!(
                runner
                    .route_native_ime_event(Ime::Commit(String::from("愛")))
                    .routed
            );
        }

        assert_eq!(text_value(&primary), text_value(&auxiliary));
        assert_eq!(
            primary.core.runtime.bridge().messages.borrow().clone(),
            auxiliary.core.runtime.bridge().messages.borrow().clone(),
        );
    }
    #[test]
    fn retired_native_preedit_and_commit_do_not_rebind_to_new_focus() {
        for auxiliary in [false, true] {
            let mut runner = if auxiliary {
                focused_auxiliary_runner()
            } else {
                focused_runner()
            };
            assert!(
                runner
                    .route_native_ime_event(Ime::Preedit("old".into(), None))
                    .routed
            );
            runner.core.runtime.bridge_mut().id = 9;
            runner.core.runtime.bridge_mut().value = "successor".into();
            runner.core.runtime.refresh();
            assert!(runner.core.runtime.focus_widget(9));
            assert!(
                !runner
                    .route_native_ime_event(Ime::Preedit("late".into(), None))
                    .routed
            );
            assert!(
                !runner
                    .route_native_ime_event(Ime::Commit("late commit".into()))
                    .routed
            );
            assert_eq!(text_value(&runner), "successor");
            assert!(runner.core.runtime.bridge().messages.borrow().is_empty());
            assert!(
                runner
                    .route_native_ime_event(Ime::Preedit("new".into(), None))
                    .routed
            );
            assert!(
                runner
                    .route_native_ime_event(Ime::Commit("new".into()))
                    .routed
            );
            assert_eq!(runner.core.runtime.bridge().messages.borrow().len(), 1);
        }
    }

    #[test]
    fn external_text_revision_retires_native_owner_without_a_new_start() {
        let mut runner = focused_runner();
        assert!(
            runner
                .route_native_ime_event(Ime::Preedit("old".into(), None))
                .routed
        );
        runner.core.runtime.bridge_mut().value = "replacement".into();
        runner.core.runtime.refresh();
        assert!(
            !runner
                .route_native_ime_event(Ime::Commit("late".into()))
                .routed
        );
        assert_eq!(text_value(&runner), "replacement");
        assert!(runner.core.runtime.bridge().messages.borrow().is_empty());
    }

    #[test]
    fn stale_native_terminal_cannot_commit_or_cancel_a_new_shared_owner() {
        use crate::widgets::CompositionSample;
        for terminal in [Ime::Commit("old".into()), Ime::Disabled] {
            let mut runner = focused_runner();
            assert!(
                runner
                    .route_native_ime_event(Ime::Preedit("old".into(), None))
                    .routed
            );
            let previous = runner.core.runtime.managed_composition_sequence();
            runner
                .core
                .runtime
                .dispatch_composition_sample(CompositionSample::cancel());
            let context = runner.core.focused_composition_start_context().unwrap();
            runner.core.runtime.dispatch_composition_sample(
                CompositionSample::start(context.replacement_range(), context.selection()).unwrap(),
            );
            let current = runner.core.runtime.managed_composition_sequence();
            assert_ne!(previous, current);
            runner
                .core
                .runtime
                .dispatch_hidden_composition_update("new".into(), None);
            assert!(!runner.route_native_ime_event(terminal).routed);
            assert_eq!(runner.core.runtime.managed_composition_sequence(), current);
            assert_eq!(text_value(&runner), "new");
            assert!(runner.core.runtime.bridge().messages.borrow().is_empty());
        }
    }

    #[test]
    fn enabled_boundary_permits_fresh_owner_without_committing_retired_text() {
        let mut runner = focused_runner();
        assert!(
            runner
                .route_native_ime_event(Ime::Preedit("old".into(), None))
                .routed
        );
        runner.core.runtime.bridge_mut().value = "replacement".into();
        runner.core.runtime.refresh();
        assert!(!runner.route_native_ime_event(Ime::Enabled).routed);
        assert!(
            runner
                .route_native_ime_event(Ime::Preedit("fresh".into(), None))
                .routed
        );
        assert!(runner.core.runtime.bridge().messages.borrow().is_empty());
    }

    #[test]
    fn oversized_transport_is_rejected_before_utf8_range_work_and_fences_continuations() {
        for event in [
            Ime::Preedit(
                "x".repeat(super::MAX_NATIVE_IME_BYTES + 1),
                Some((0, usize::MAX)),
            ),
            Ime::Commit("x".repeat(super::MAX_NATIVE_IME_BYTES + 1)),
        ] {
            assert_eq!(
                super::normalize_winit_ime_event(event.clone()),
                Err(super::NativeImeNormalizationError::PayloadTooLarge)
            );
            let mut runner = focused_runner();
            assert!(
                runner
                    .route_native_ime_event(Ime::Preedit("old".into(), None))
                    .routed
            );
            assert!(runner.route_native_ime_event(event).routed);
            assert_eq!(text_value(&runner), "a");
            assert!(
                !runner
                    .route_native_ime_event(Ime::Preedit("late".into(), None))
                    .routed
            );
            assert!(
                !runner
                    .route_native_ime_event(Ime::Commit("late".into()))
                    .routed
            );
            assert_eq!(text_value(&runner), "a");
        }
    }
}
