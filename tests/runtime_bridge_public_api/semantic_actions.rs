use radiant::{
    application::{button, column, selectable, text_input, toggle},
    gui::automation::AutomationNodeId,
    layout::Vector2,
    runtime::{
        Command, RuntimeBridge, SemanticAction, SemanticActionOutcome, SemanticActionSource,
        SurfaceRuntime,
    },
    widgets::InteractionProvenance,
};
use std::{cell::RefCell, rc::Rc};

#[derive(Debug, PartialEq)]
enum Message {
    Press(InteractionProvenance),
    Toggle(bool),
    Text(String),
    Select(bool),
}
#[derive(Default)]
struct State {
    checked: bool,
    text: String,
    selected: bool,
}
fn bridge(events: Rc<RefCell<Vec<Message>>>) -> impl RuntimeBridge<Message> {
    radiant::app(State::default())
        .view(|state: &State| {
            column([
                button("Press")
                    .mapped(|event| Message::Press(event.activation_provenance().unwrap()))
                    .id(1),
                toggle("Toggle", state.checked)
                    .message(Message::Toggle)
                    .id(2),
                text_input(state.text.clone()).message(Message::Text).id(3),
                selectable("Choose", state.selected)
                    .message(Message::Select)
                    .id(4),
            ])
            .id(100)
        })
        .update(move |state, message| {
            match &message {
                Message::Toggle(value) => state.checked = *value,
                Message::Text(value) => state.text = value.clone(),
                Message::Select(value) => state.selected = *value,
                Message::Press(_) => {}
            }
            events.borrow_mut().push(message);
        })
        .into_bridge()
}
fn id(value: u64) -> AutomationNodeId {
    AutomationNodeId::new(value.to_string())
}

#[test]
fn semantic_actions_use_current_targets_one_mapper_and_normal_widget_behavior() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = SurfaceRuntime::new(bridge(Rc::clone(&events)), Vector2::new(320.0, 200.0));
    let mut other = SurfaceRuntime::new(
        bridge(Rc::new(RefCell::new(Vec::new()))),
        Vector2::new(320.0, 200.0),
    );
    let first = runtime.semantic_action_target(&id(1)).unwrap();
    assert_eq!(runtime.focused_widget(), None);
    assert!(events.borrow().is_empty());
    assert!(
        runtime
            .semantic_action_target(&AutomationNodeId::new("virtual:offscreen"))
            .is_none()
    );
    assert_eq!(
        other.dispatch_semantic_action(
            &first,
            SemanticAction::Press,
            SemanticActionSource::Accessibility
        ),
        SemanticActionOutcome::Stale
    );
    assert_eq!(other.focused_widget(), None);
    assert_eq!(
        runtime.dispatch_semantic_action(
            &first,
            SemanticAction::Press,
            SemanticActionSource::Accessibility
        ),
        SemanticActionOutcome::Accepted
    );
    assert_eq!(runtime.focused_widget(), Some(1));
    assert_eq!(
        runtime.dispatch_semantic_action(
            &first,
            SemanticAction::Press,
            SemanticActionSource::Accessibility
        ),
        SemanticActionOutcome::Stale
    );
    let toggle = runtime.semantic_action_target(&id(2)).unwrap();
    assert_eq!(
        runtime.dispatch_semantic_action(
            &toggle,
            SemanticAction::Toggle,
            SemanticActionSource::Programmatic
        ),
        SemanticActionOutcome::Accepted
    );
    let snapshot = runtime.automation_target_snapshot();
    assert_eq!(
        snapshot
            .targets
            .iter()
            .find(|target| target.id == id(2))
            .unwrap()
            .checked,
        Some(true)
    );
    let text = runtime.semantic_action_target(&id(3)).unwrap();
    assert_eq!(
        runtime.dispatch_semantic_action(
            &text,
            SemanticAction::SetText("a\nb\tc".into()),
            SemanticActionSource::Accessibility
        ),
        SemanticActionOutcome::Accepted
    );
    let snapshot = runtime.automation_target_snapshot();
    assert_eq!(
        snapshot
            .targets
            .iter()
            .find(|target| target.id == id(3))
            .unwrap()
            .value
            .as_deref(),
        Some("ab c")
    );
    let text = runtime.semantic_action_target(&id(3)).unwrap();
    assert_eq!(
        runtime.dispatch_semantic_action(
            &text,
            SemanticAction::SetText(String::new()),
            SemanticActionSource::Accessibility
        ),
        SemanticActionOutcome::Accepted
    );
    let snapshot = runtime.automation_target_snapshot();
    assert_eq!(
        snapshot
            .targets
            .iter()
            .find(|target| target.id == id(3))
            .unwrap()
            .value
            .as_deref(),
        Some("")
    );
    let text = runtime.semantic_action_target(&id(3)).unwrap();
    assert_eq!(
        runtime.dispatch_semantic_action(
            &text,
            SemanticAction::Press,
            SemanticActionSource::Accessibility
        ),
        SemanticActionOutcome::Unsupported
    );
    assert_eq!(
        runtime.dispatch_semantic_action(
            &text,
            SemanticAction::SetText("x".repeat(65_537)),
            SemanticActionSource::Accessibility
        ),
        SemanticActionOutcome::InvalidRequest
    );
    assert_eq!(
        *events.borrow(),
        [
            Message::Press(InteractionProvenance::Accessibility),
            Message::Toggle(true),
            Message::Text("ab c".into()),
            Message::Text(String::new())
        ]
    );
    runtime.execute_command(Command::exit());
    assert!(runtime.semantic_action_target(&id(1)).is_none());
    assert_eq!(
        runtime.dispatch_semantic_action(
            &text,
            SemanticAction::SetText("closed".into()),
            SemanticActionSource::Accessibility
        ),
        SemanticActionOutcome::Unavailable
    );
    assert_eq!(events.borrow().len(), 4);
}

#[test]
fn selecting_is_idempotent_and_programmatic_activation_preserves_provenance() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = SurfaceRuntime::new(bridge(Rc::clone(&events)), Vector2::new(320.0, 240.0));
    for _ in 0..2 {
        let target = runtime.semantic_action_target(&id(4)).unwrap();
        assert_eq!(
            runtime.dispatch_semantic_action(
                &target,
                SemanticAction::Select,
                SemanticActionSource::Programmatic
            ),
            SemanticActionOutcome::Accepted
        );
    }
    let target = runtime.semantic_action_target(&id(1)).unwrap();
    assert_eq!(
        runtime.dispatch_semantic_action(
            &target,
            SemanticAction::Press,
            SemanticActionSource::Programmatic
        ),
        SemanticActionOutcome::Accepted
    );
    assert_eq!(
        *events.borrow(),
        [
            Message::Select(true),
            Message::Press(InteractionProvenance::Programmatic)
        ]
    );
}

#[derive(Clone)]
struct ActionProbe {
    button: radiant::widgets::ButtonWidget,
    read_version: u16,
    write_version: u16,
    supported: bool,
    calls: Rc<std::cell::Cell<usize>>,
}
impl radiant::widgets::WidgetSemanticActions for ActionProbe {
    fn revision(&self) -> radiant::widgets::WidgetSemanticActionRevision {
        radiant::widgets::WidgetSemanticActionRevision::exact((self.supported, self.write_version))
    }
    fn supports(&self, action: &SemanticAction) -> bool {
        self.supported && *action == SemanticAction::Press
    }
    fn dispatch(
        &mut self,
        _: SemanticAction,
        _: SemanticActionSource,
    ) -> radiant::widgets::WidgetSemanticActionResult {
        self.calls.set(self.calls.get() + 1);
        radiant::widgets::WidgetSemanticActionResult::Accepted(Some(
            radiant::widgets::WidgetOutput::typed(1_u8),
        ))
    }
}
impl radiant::widgets::Widget for ActionProbe {
    fn common(&self) -> &radiant::widgets::WidgetCommon {
        &self.button.common
    }
    fn common_mut(&mut self) -> &mut radiant::widgets::WidgetCommon {
        &mut self.button.common
    }
    fn handle_input(
        &mut self,
        _: radiant::layout::Rect,
        input: radiant::widgets::WidgetInput,
    ) -> Option<radiant::widgets::WidgetOutput> {
        if let radiant::widgets::WidgetInput::FocusChanged(focused) = input {
            self.button.common.state.focused = focused;
        }
        None
    }
    fn capabilities(&self) -> radiant::widgets::WidgetCapabilities<'_> {
        self.button.capabilities()
    }
    fn capabilities_v2(&self) -> radiant::widgets::WidgetCapabilitiesV2<'_> {
        radiant::widgets::WidgetCapabilitiesV2::new()
            .with_semantic_actions(self)
            .with_contract_version(self.read_version)
    }
    fn action_capabilities(&mut self) -> radiant::widgets::WidgetActionCapabilities<'_> {
        let version = self.write_version;
        if version == 0 {
            return radiant::widgets::WidgetActionCapabilities::none();
        }
        radiant::widgets::WidgetActionCapabilities::none()
            .with_semantic_actions(self)
            .with_contract_version(version)
    }
    fn append_paint(
        &self,
        _: &mut Vec<radiant::runtime::PaintPrimitive>,
        _: radiant::layout::Rect,
        _: &radiant::layout::LayoutOutput,
        _: &radiant::theme::ThemeTokens,
    ) {
    }
}

#[test]
fn action_descriptors_fail_closed_before_focus_and_execute_only_explicitly() {
    for (read_version, write_version, supported, accepted) in [
        (2, 1, true, true),
        (3, 1, true, false),
        (2, 2, true, false),
        (2, 0, true, false),
        (2, 1, false, false),
    ] {
        let calls = Rc::new(std::cell::Cell::new(0));
        let widget = ActionProbe {
            button: radiant::widgets::ButtonWidget::new(
                1,
                "Custom",
                radiant::widgets::WidgetSizing::fixed(Vector2::new(100.0, 28.0)),
            ),
            read_version,
            write_version,
            supported,
            calls: Rc::clone(&calls),
        };
        let reduced = Rc::new(std::cell::Cell::new(0));
        let reductions = Rc::clone(&reduced);
        let mut runtime = SurfaceRuntime::new(
            radiant::app(())
                .view(move |_: &()| {
                    radiant::application::custom_widget_mapped(widget.clone(), |value: u8| value)
                        .id(1)
                })
                .update(move |_, _| reductions.set(reductions.get() + 1))
                .into_bridge(),
            Vector2::new(200.0, 80.0),
        );
        let target = runtime.semantic_action_target(&id(1)).unwrap();
        assert_eq!(calls.get(), 0);
        assert_eq!(reduced.get(), 0);
        assert_eq!(runtime.focused_widget(), None);
        assert_eq!(
            runtime.dispatch_semantic_action(
                &target,
                SemanticAction::Press,
                SemanticActionSource::Accessibility
            ),
            if accepted {
                SemanticActionOutcome::Accepted
            } else {
                SemanticActionOutcome::Unsupported
            }
        );
        assert_eq!(calls.get(), usize::from(accepted));
        assert_eq!(reduced.get(), usize::from(accepted));
        assert_eq!(runtime.focused_widget(), accepted.then_some(1));
    }
}
