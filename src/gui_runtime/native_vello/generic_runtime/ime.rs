//! Native Winit IME normalization and routing for every Vello window.

use super::{GenericNativeVelloRunner, GenericRouteOutcome};
use crate::runtime::RuntimeBridge;
use crate::widgets::{CompositionRange, CompositionSample};
use winit::event::Ime;

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
    /// Normalize and route one Winit IME event through the shared composition
    /// owner. The primary and auxiliary Vello loops both call this method.
    pub(super) fn route_native_ime_event(&mut self, event: Ime) -> GenericRouteOutcome {
        let normalized = match normalize_winit_ime_event(event) {
            Ok(normalized) => normalized,
            Err(_) => return self.route_native_ime_cancel(),
        };
        match normalized {
            // Winit's Enabled event reports platform capability. It never
            // captures the focused widget or starts a composition.
            NormalizedImeEvent::Enabled => GenericRouteOutcome::default(),
            NormalizedImeEvent::Preedit { preedit, selection } => {
                self.route_native_ime_preedit(preedit, selection)
            }
            NormalizedImeEvent::Commit(text) => {
                self.route_native_ime_sample(CompositionSample::commit(text))
            }
            NormalizedImeEvent::Disabled => self.route_native_ime_cancel(),
        }
    }

    fn route_native_ime_sample(&mut self, sample: CompositionSample) -> GenericRouteOutcome {
        self.frame.text_renderer.reset_native_caret_affinities();
        if !self.ensure_native_ime_composition() {
            return self.core.route_outcome(false);
        }

        let routed = self
            .core
            .runtime
            .dispatch_focused_composition_sample(sample)
            .is_some();
        self.core.route_outcome(routed)
    }

    fn route_native_ime_preedit(
        &mut self,
        preedit: String,
        selection: Option<CompositionRange>,
    ) -> GenericRouteOutcome {
        self.frame.text_renderer.reset_native_caret_affinities();
        if !self.ensure_native_ime_composition() {
            return self.core.route_outcome(false);
        }

        let routed = match selection {
            Some(selection) => {
                let Ok(sample) = CompositionSample::update(preedit, selection) else {
                    return self.route_native_ime_cancel();
                };
                self.core
                    .runtime
                    .dispatch_focused_composition_sample(sample)
                    .is_some()
            }
            None => self
                .core
                .runtime
                .dispatch_hidden_composition_update(preedit, None)
                .is_some(),
        };
        self.core.route_outcome(routed)
    }

    fn ensure_native_ime_composition(&mut self) -> bool {
        if self.core.managed_composition_is_active() {
            return true;
        }
        let Some(context) = self.core.focused_composition_start_context() else {
            let _ = self
                .core
                .runtime
                .dispatch_focused_composition_sample(CompositionSample::cancel());
            return false;
        };
        let Ok(start) = CompositionSample::start(context.replacement_range(), context.selection())
        else {
            return false;
        };
        self.core
            .runtime
            .dispatch_focused_composition_sample(start)
            .is_some()
    }

    fn route_native_ime_cancel(&mut self) -> GenericRouteOutcome {
        self.frame.text_renderer.reset_native_caret_affinities();
        let was_active = self.core.managed_composition_is_active();
        let _ = self
            .core
            .runtime
            .dispatch_focused_composition_sample(CompositionSample::cancel());
        self.core.route_outcome(was_active)
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

    #[derive(Clone)]
    struct ImeBridge {
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
                7,
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
            .find_widget(7)
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
        let expected = CompositionRange::new(0, 2, 3).expect("scalar selection");
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
}
