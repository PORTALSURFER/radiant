use super::{SurfaceRuntime, owner::AuxiliaryWindowOwner};
use crate::runtime::Command;
use crate::widgets::WidgetId;

/// Focus work emitted by an auxiliary-owned effect and applied by that exact
/// native child after its projection is current.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AuxiliaryFocusCommand {
    Focus(WidgetId),
    Clear,
}

/// One generation-fenced focus request waiting for native auxiliary routing.
pub(crate) struct AuxiliaryFocusRequest {
    owner: AuxiliaryWindowOwner,
    command: AuxiliaryFocusCommand,
}

impl AuxiliaryFocusRequest {
    pub(crate) fn new(owner: AuxiliaryWindowOwner, command: AuxiliaryFocusCommand) -> Self {
        Self { owner, command }
    }

    pub(crate) fn owner(&self) -> &AuxiliaryWindowOwner {
        &self.owner
    }

    #[cfg(test)]
    pub(crate) const fn command(&self) -> AuxiliaryFocusCommand {
        self.command
    }

    pub(crate) fn into_command<Message>(self) -> Command<Message> {
        match self.command {
            AuxiliaryFocusCommand::Focus(widget_id) => Command::Focus(widget_id),
            AuxiliaryFocusCommand::Clear => Command::ClearFocus,
        }
    }
}

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: crate::runtime::RuntimeBridge<Message>,
{
    pub(crate) fn enqueue_auxiliary_focus_request(
        &mut self,
        owner: AuxiliaryWindowOwner,
        command: AuxiliaryFocusCommand,
    ) {
        self.pending_auxiliary_focus_requests
            .push(AuxiliaryFocusRequest::new(owner, command));
    }

    pub(crate) fn take_pending_auxiliary_focus_requests(&mut self) -> Vec<AuxiliaryFocusRequest> {
        std::mem::take(&mut self.pending_auxiliary_focus_requests)
    }

    pub(crate) fn discard_pending_auxiliary_focus_requests_for(
        &mut self,
        owner: &AuxiliaryWindowOwner,
    ) {
        self.pending_auxiliary_focus_requests
            .retain(|request| !request.owner().is_same_generation(owner));
    }

    pub(crate) fn clear_pending_auxiliary_focus_requests(&mut self) {
        self.pending_auxiliary_focus_requests.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::{AuxiliaryFocusCommand, SurfaceRuntime};
    use crate::gui::types::Rect;
    use crate::gui::types::Vector2;
    use crate::layout::LayoutOutput;
    use crate::runtime::{
        Command, PaintPrimitive, RuntimeBridge, RuntimeHostCapabilities, SurfaceNode, UiSurface,
        WidgetMessageMapper,
    };
    use crate::theme::ThemeTokens;
    use crate::widgets::{
        FocusBehavior, FocusLossDecision, InteractiveRowWidget, Widget, WidgetCommon, WidgetInput,
        WidgetOutput, WidgetSizing,
    };
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
        sync::Arc,
    };

    struct AuxiliaryFocusBridge {
        project_count: usize,
    }

    impl RuntimeBridge<usize> for AuxiliaryFocusBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<usize>> {
            self.project_count += 1;
            crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::widget(
                InteractiveRowWidget::new(43, WidgetSizing::fixed(Vector2::new(160.0, 28.0))),
                WidgetMessageMapper::none(),
            )))
        }

        fn update(&mut self, message: usize) -> Command<usize> {
            match message {
                1 => Command::focus(43),
                2 => Command::clear_focus(),
                3 => Command::batch([
                    Command::focus(43),
                    Command::clear_focus(),
                    Command::focus(43),
                ]),
                _ => Command::none(),
            }
        }

        fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, usize> {
            RuntimeHostCapabilities::new()
        }
    }

    fn runtime() -> SurfaceRuntime<AuxiliaryFocusBridge, usize> {
        SurfaceRuntime::new(
            AuxiliaryFocusBridge { project_count: 0 },
            Vector2::new(160.0, 28.0),
        )
    }

    #[test]
    fn auxiliary_focus_does_not_mutate_parent_focus_and_preserves_command_order() {
        let mut runtime = runtime();
        assert!(runtime.focus_widget(43));
        let owner = runtime.acquire_auxiliary_effect_owner("settings");

        let _ = runtime.dispatch_message_from_auxiliary(3, owner.clone());

        assert_eq!(runtime.focused_widget(), Some(43));
        assert_eq!(runtime.bridge().project_count, 2);
        let requests = runtime.take_pending_auxiliary_focus_requests();
        assert_eq!(requests.len(), 3);
        assert!(
            requests
                .iter()
                .all(|request| request.owner().is_same_generation(&owner))
        );
        assert_eq!(
            requests
                .iter()
                .map(|request| request.command())
                .collect::<Vec<_>>(),
            [
                AuxiliaryFocusCommand::Focus(43),
                AuxiliaryFocusCommand::Clear,
                AuxiliaryFocusCommand::Focus(43),
            ]
        );
    }

    #[test]
    fn auxiliary_clear_focus_does_not_clear_parent_focus() {
        let mut runtime = runtime();
        assert!(runtime.focus_widget(43));
        let owner = runtime.acquire_auxiliary_effect_owner("settings");

        let _ = runtime.dispatch_message_from_auxiliary(2, owner);

        assert_eq!(runtime.focused_widget(), Some(43));
        assert_eq!(
            runtime
                .take_pending_auxiliary_focus_requests()
                .into_iter()
                .map(|request| request.command())
                .collect::<Vec<_>>(),
            [AuxiliaryFocusCommand::Clear]
        );
    }

    #[test]
    fn application_focus_command_remains_parent_local() {
        let mut runtime = runtime();

        let _ = runtime.dispatch_message(1);

        assert_eq!(runtime.focused_widget(), Some(43));
        assert!(runtime.take_pending_auxiliary_focus_requests().is_empty());
    }

    #[derive(Clone)]
    struct FocusLossProbeWidget {
        common: WidgetCommon,
        decision: Rc<Cell<FocusLossDecision>>,
        changes: Rc<RefCell<Vec<bool>>>,
    }

    impl FocusLossProbeWidget {
        fn new(decision: Rc<Cell<FocusLossDecision>>, changes: Rc<RefCell<Vec<bool>>>) -> Self {
            Self {
                common: WidgetCommon::fixed(10, 160.0, 28.0)
                    .with_focus(FocusBehavior::Keyboard)
                    .without_default_chrome(),
                decision,
                changes,
            }
        }
    }

    impl Widget for FocusLossProbeWidget {
        fn common(&self) -> &WidgetCommon {
            &self.common
        }

        fn common_mut(&mut self) -> &mut WidgetCommon {
            &mut self.common
        }

        fn prepare_focus_loss(&mut self) -> FocusLossDecision {
            self.decision.get()
        }

        fn handle_input(&mut self, _bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
            if let WidgetInput::FocusChanged(focused) = input {
                self.common.state.focused = focused;
                self.changes.borrow_mut().push(focused);
            }
            None
        }

        fn append_paint(
            &self,
            _primitives: &mut Vec<PaintPrimitive>,
            _bounds: Rect,
            _layout: &LayoutOutput,
            _theme: &ThemeTokens,
        ) {
        }
    }

    struct FocusLossBridge {
        decision: Rc<Cell<FocusLossDecision>>,
        changes: Rc<RefCell<Vec<bool>>>,
    }

    impl RuntimeBridge<()> for FocusLossBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<()>> {
            crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::widget(
                FocusLossProbeWidget::new(self.decision.clone(), self.changes.clone()),
                WidgetMessageMapper::none(),
            )))
        }

        fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, ()> {
            RuntimeHostCapabilities::new()
        }
    }

    #[test]
    fn child_focus_request_uses_existing_veto_and_focus_loss_cleanup_path() {
        let decision = Rc::new(Cell::new(FocusLossDecision::Veto));
        let changes = Rc::new(RefCell::new(Vec::new()));
        let mut child = SurfaceRuntime::new(
            FocusLossBridge {
                decision: decision.clone(),
                changes: changes.clone(),
            },
            Vector2::new(160.0, 28.0),
        );
        assert!(child.focus_widget(10));
        let owner = child.acquire_auxiliary_effect_owner("settings");
        child.enqueue_auxiliary_focus_request(owner.clone(), AuxiliaryFocusCommand::Clear);
        let request = child
            .take_pending_auxiliary_focus_requests()
            .pop()
            .expect("queued child clear request");
        let _ = child.execute_command(request.into_command());

        assert_eq!(child.focused_widget(), Some(10));
        assert_eq!(*changes.borrow(), [true]);

        decision.set(FocusLossDecision::Allow);
        child.enqueue_auxiliary_focus_request(owner, AuxiliaryFocusCommand::Clear);
        let request = child
            .take_pending_auxiliary_focus_requests()
            .pop()
            .expect("queued child clear request");
        let _ = child.execute_command(request.into_command());

        assert_eq!(child.focused_widget(), None);
        assert_eq!(*changes.borrow(), [true, false]);
        assert_eq!(child.pointer_capture(), None);
    }

    #[test]
    fn retiring_auxiliary_generation_discards_queued_focus_requests() {
        let mut runtime = runtime();
        let owner = runtime.acquire_auxiliary_effect_owner("settings");
        runtime.enqueue_auxiliary_focus_request(owner.clone(), AuxiliaryFocusCommand::Focus(43));

        assert!(runtime.retire_auxiliary_effect_owner(&owner));
        assert!(runtime.take_pending_auxiliary_focus_requests().is_empty());

        let replacement = runtime.acquire_auxiliary_effect_owner("settings");
        assert!(!owner.is_same_generation(&replacement));
        assert!(runtime.auxiliary_effect_owner_is_active(&replacement));
    }
}
