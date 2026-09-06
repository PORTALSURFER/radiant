use super::source::SourceMetadata;
use crate::{
    UiAffinity,
    gui::automation::AutomationNodeSemantics,
    gui::types::{Point, Rect},
    layout::LayoutNode,
    widgets::{
        CompositionSample, FocusBehavior, FocusedKeyDisposition, PointerCapturePolicy,
        PointerPressAdmission, WheelSample, Widget, WidgetCursor, WidgetHitTestResult, WidgetId,
        WidgetInput, WidgetOutput, WidgetPointerMotionRevision, WidgetRevision,
        WidgetSemanticsRevision,
    },
};
use std::rc::Rc;
use std::time::Instant;

mod mapper;

pub use mapper::{
    EventMapper, MessageMapper, NativeFileDropMessageMapper, ScrollMessageMapper,
    WidgetMessageMapper,
};
pub(crate) use mapper::{MapperDescriptor, MapperRelation};

/// One widget leaf inside a generic declarative [`UiSurface`](super::UiSurface).
///
/// Installed widgets remain owned by the UI runtime and cannot be sent to a
/// worker thread.
///
/// ```compile_fail
/// use radiant::{
///     layout::Vector2,
///     runtime::{SurfaceWidget, WidgetMessageMapper},
///     widgets::{TextWidget, WidgetSizing},
/// };
///
/// let widget = SurfaceWidget::new(
///     TextWidget::new(1, "UI-local", WidgetSizing::fixed(Vector2::new(80.0, 20.0))),
///     WidgetMessageMapper::none(),
/// );
/// std::thread::spawn(move || drop(widget));
/// ```
pub struct SurfaceWidget<Message> {
    _ui_affinity: UiAffinity,
    widget: Box<dyn Widget>,
    messages: WidgetMessageMapper<Message>,
    accepts_native_file_drop: bool,
    revision_evidence: SurfaceWidgetRevisionEvidence,
    pub(in crate::runtime::surface) source: Option<Rc<SourceMetadata>>,
    pub(in crate::runtime::surface) command_scope:
        Option<crate::application::CommandScopeAttachment>,
}

/// Immutable widget evidence captured when the widget crosses the erased
/// `SurfaceWidget` boundary.  View-delta classification borrows this record
/// and never dispatches back into the live widget object.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SurfaceWidgetRevisionEvidence {
    pub(crate) id: WidgetId,
    pub(crate) compatibility_kind: &'static str,
    pub(crate) revision: WidgetRevision,
    pub(crate) capabilities: WidgetCapabilityEvidence,
    pub(crate) valid: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct WidgetCapabilityEvidence {
    /// Source-compatible v1 contract version and semantics evidence.
    pub(crate) contract_version: u16,
    pub(crate) semantics_revision: Option<WidgetSemanticsRevision>,
    /// Additive v2 contract version and optional-behavior evidence.
    pub(crate) v2_contract_version: u16,
    pub(crate) v2_semantics_revision: Option<WidgetSemanticsRevision>,
    pub(crate) hit_test_revision: Option<crate::widgets::WidgetHitTestRevision>,
    pub(crate) pointer_motion_revision: Option<WidgetPointerMotionRevision>,
    pub(crate) semantic_actions_revision: Option<crate::widgets::WidgetSemanticActionRevision>,
    pub(crate) gestures_revision: Option<WidgetSemanticsRevision>,
}

impl WidgetCapabilityEvidence {
    fn capture(widget: &dyn Widget) -> Self {
        let capabilities = widget.capabilities();
        let capabilities_v2 = widget.capabilities_v2();
        Self {
            contract_version: capabilities.contract_version,
            semantics_revision: capabilities.semantics_revision(),
            v2_contract_version: capabilities_v2.contract_version(),
            v2_semantics_revision: capabilities_v2.semantics_revision(),
            hit_test_revision: capabilities_v2.hit_test_revision(),
            pointer_motion_revision: capabilities_v2.pointer_motion_revision(),
            semantic_actions_revision: capabilities_v2.semantic_actions_revision(),
            gestures_revision: capabilities_v2.gestures_revision(),
        }
    }

    fn conservative() -> Self {
        Self {
            contract_version: 0,
            semantics_revision: None,
            v2_contract_version: 0,
            v2_semantics_revision: None,
            hit_test_revision: None,
            pointer_motion_revision: None,
            semantic_actions_revision: None,
            gestures_revision: None,
        }
    }

    pub(crate) fn needs_conservative_fallback(&self) -> bool {
        !crate::widgets::supports_semantics_contract(self.contract_version)
            || !crate::widgets::supports_capabilities_v2_contract(self.v2_contract_version)
            || self
                .v2_semantics_revision
                .as_ref()
                .is_some_and(|revision| !revision.is_exact())
            || self
                .hit_test_revision
                .as_ref()
                .is_some_and(|revision| !revision.is_exact())
            || self
                .gestures_revision
                .as_ref()
                .is_some_and(|revision| !revision.is_exact())
            || self
                .semantic_actions_revision
                .as_ref()
                .is_some_and(|revision| !revision.is_exact())
            || self
                .pointer_motion_revision
                .as_ref()
                .is_some_and(|revision| !revision.is_exact())
    }
}

impl SurfaceWidgetRevisionEvidence {
    fn capture(widget: &dyn Widget) -> Self {
        Self {
            id: widget.common().id,
            compatibility_kind: widget.compatibility_kind(),
            revision: widget.revision(),
            capabilities: WidgetCapabilityEvidence::capture(widget),
            valid: true,
        }
    }
}

impl<Message> Clone for SurfaceWidget<Message> {
    fn clone(&self) -> Self {
        Self {
            _ui_affinity: self._ui_affinity,
            widget: self.widget.clone(),
            messages: self.messages.clone(),
            accepts_native_file_drop: self.accepts_native_file_drop,
            revision_evidence: self.revision_evidence.clone(),
            source: self.source.clone(),
            command_scope: self.command_scope.clone(),
        }
    }
}

impl<Message> SurfaceWidget<Message> {
    /// Build a widget leaf plus host-defined message mapper.
    pub fn new(
        widget: impl Widget + Clone + 'static,
        messages: WidgetMessageMapper<Message>,
    ) -> Self {
        Self::from_boxed(Box::new(widget), messages)
    }

    /// Build a custom widget leaf plus host-defined message mapper.
    pub fn custom(
        widget: impl Widget + Clone + 'static,
        messages: WidgetMessageMapper<Message>,
    ) -> Self {
        Self::from_boxed(Box::new(widget), messages)
    }

    /// Build a custom boxed widget leaf plus host-defined message mapper.
    pub fn custom_box(widget: Box<dyn Widget>, messages: WidgetMessageMapper<Message>) -> Self {
        Self::from_boxed(widget, messages)
    }

    fn from_boxed(widget: Box<dyn Widget>, messages: WidgetMessageMapper<Message>) -> Self {
        let revision_evidence = SurfaceWidgetRevisionEvidence::capture(widget.as_ref());
        Self {
            _ui_affinity: UiAffinity::new(),
            widget,
            messages,
            accepts_native_file_drop: false,
            revision_evidence,
            source: None,
            command_scope: None,
        }
    }

    /// Return the stable widget identifier.
    pub fn id(&self) -> WidgetId {
        self.widget.common().id
    }

    /// Return the runtime widget object.
    pub fn widget(&self) -> &dyn Widget {
        self.widget.as_ref()
    }

    /// Return the runtime widget object mutably.
    pub fn widget_mut(&mut self) -> &mut dyn Widget {
        self.invalidate_revision_evidence();
        self.widget.as_mut()
    }

    /// Return the runtime widget object.
    pub fn widget_object(&self) -> &dyn Widget {
        self.widget.as_ref()
    }

    /// Return the runtime widget object mutably.
    pub fn widget_object_mut(&mut self) -> &mut dyn Widget {
        self.invalidate_revision_evidence();
        self.widget.as_mut()
    }

    pub(in crate::runtime) fn compatibility_kind(&self) -> &'static str {
        self.revision_evidence.compatibility_kind
    }

    /// Return the declarative revision metadata supplied by the widget.
    pub fn revision(&self) -> WidgetRevision {
        self.revision_evidence.revision.clone()
    }

    /// Return live revision evidence from the erased widget object.
    ///
    /// The cached revision record is intentionally immutable across runtime
    /// state mutation. Prepared synchronization therefore captures this live
    /// value separately and admits it only when it still equals the cached
    /// declarative witness.
    pub(in crate::runtime::surface) fn live_revision(&self) -> WidgetRevision {
        self.widget.revision()
    }

    pub(in crate::runtime::surface) fn live_capability_evidence(&self) -> WidgetCapabilityEvidence {
        WidgetCapabilityEvidence::capture(self.widget.as_ref())
    }

    pub(in crate::runtime::surface) fn cached_revision_is_exact(&self) -> bool {
        self.revision_evidence.revision.exact_components().is_some()
    }

    pub(in crate::runtime::surface) fn prepared_state_membership(&self) -> [bool; 7] {
        [
            self.is_focusable(),
            self.is_keyboard_focusable(),
            self.receives_pointer_hit_testing(),
            self.receives_wheel_input(),
            self.accepts_native_file_drop(),
            self.needs_state_synchronization(),
            self.suppresses_container_hover(),
        ]
    }

    pub(in crate::runtime::surface) fn revision_evidence(&self) -> &SurfaceWidgetRevisionEvidence {
        &self.revision_evidence
    }

    /// Borrow the erased widget for runtime-owned state mutation.  These
    /// mutations intentionally preserve declarative revision evidence.
    pub(in crate::runtime) fn widget_object_mut_runtime(&mut self) -> &mut dyn Widget {
        self.widget.as_mut()
    }

    pub(in crate::runtime) fn native_text_input_delegate_mut(
        &mut self,
    ) -> Option<&mut crate::widgets::TextInputWidget> {
        self.widget_object_mut_runtime()
            .native_text_input_delegate_mut()
    }

    /// Reidentify a projected widget during lowering without invalidating the
    /// declarative evidence captured for its concrete widget.
    pub(in crate::runtime::surface) fn set_id_runtime(&mut self, id: WidgetId) {
        self.widget_object_mut_runtime().common_mut().id = id;
        self.revision_evidence.id = id;
    }

    fn invalidate_revision_evidence(&mut self) {
        self.revision_evidence.valid = false;
        self.revision_evidence.revision = WidgetRevision::conservative();
        self.revision_evidence.capabilities = WidgetCapabilityEvidence::conservative();
    }

    /// Return whether this widget participates in runtime focus management.
    pub fn is_focusable(&self) -> bool {
        self.widget.common().focus != FocusBehavior::None && !self.widget.common().state.disabled
    }

    /// Return whether this widget participates in keyboard focus traversal.
    pub fn is_keyboard_focusable(&self) -> bool {
        self.widget.common().focus == FocusBehavior::Keyboard
            && !self.widget.common().state.disabled
    }

    /// Return whether this widget can be a pointer hit-test target.
    pub fn receives_pointer_hit_testing(&self) -> bool {
        let common = self.widget.common();
        !common.state.disabled
            && (common.focus != FocusBehavior::None
                || common.paint.suppresses_container_hover
                || self.messages.maps_any_output())
    }

    pub(in crate::runtime) fn accepts_native_file_drop(&self) -> bool {
        !self.widget.common().state.disabled && self.accepts_native_file_drop
    }

    pub(in crate::runtime) fn tooltip(&self) -> Option<&str> {
        self.widget.common().tooltip.as_deref()
    }

    pub(in crate::runtime::surface) fn uses_dynamic_output_callback(&self) -> bool {
        self.messages.uses_dynamic_output_callback()
    }

    pub(in crate::runtime::surface) fn output_mapper_descriptor(&self) -> MapperDescriptor {
        self.messages.output_mapper_descriptor()
    }

    pub(in crate::runtime::surface) fn native_file_drop_mapper_descriptor(
        &self,
    ) -> MapperDescriptor {
        let descriptor = self.messages.native_file_drop_mapper_descriptor();
        if matches!(descriptor, MapperDescriptor::Absent) && self.accepts_native_file_drop {
            MapperDescriptor::Conservative
        } else {
            descriptor
        }
    }

    pub(in crate::runtime) fn receives_wheel_input(&self) -> bool {
        !self.widget.common().state.disabled && self.widget.accepts_wheel_input()
    }

    pub(in crate::runtime) fn retains_managed_wheel_sequence(&self) -> bool {
        self.widget.retains_managed_wheel_sequence()
    }

    pub(in crate::runtime) fn accepts_composition_input(&self) -> bool {
        !self.widget.common().state.disabled && self.widget.accepts_composition_input()
    }

    pub(in crate::runtime) fn focused_key_disposition(
        &self,
        key: crate::widgets::WidgetKey,
    ) -> FocusedKeyDisposition {
        self.widget.focused_key_disposition(key)
    }

    pub(in crate::runtime) fn supports_accessibility_action(
        &self,
        action: &crate::widgets::NumericAccessibilityAction,
    ) -> bool {
        self.widget.supports_accessibility_action(action)
    }

    pub(in crate::runtime) fn accessibility_action_owner(
        &self,
    ) -> Option<crate::widgets::NumericAccessibilityBlockOwner> {
        self.widget.accessibility_action_owner()
    }

    pub(in crate::runtime) fn retains_managed_composition(&self) -> bool {
        self.widget.retains_managed_composition()
    }

    pub(in crate::runtime) fn accepts_pointer_move(&self) -> bool {
        if self.widget.common().state.disabled {
            return false;
        }
        let capabilities = self.widget.capabilities_v2();
        if capabilities.has_pointer_motion() {
            capabilities
                .pointer_motion()
                .is_some_and(|motion| motion.accepts_pointer_move())
        } else {
            self.widget.accepts_pointer_move()
        }
    }

    pub(in crate::runtime) fn accepts_pointer_input(&self, input: &WidgetInput) -> bool {
        !self.widget.common().state.disabled && self.widget.accepts_pointer_input(input)
    }

    pub(in crate::runtime) fn hit_test(
        &self,
        bounds: Rect,
        point: Point,
        input: &WidgetInput,
    ) -> WidgetHitTestResult {
        if self.widget.common().state.disabled {
            return WidgetHitTestResult::PassThrough;
        }
        let capabilities = self.widget.capabilities_v2();
        if capabilities.has_hit_test() {
            capabilities
                .hit_test()
                .map_or(WidgetHitTestResult::Opaque, |hit_test| {
                    hit_test.hit_test(bounds, point, input)
                })
        } else if self.accepts_pointer_input(input) {
            WidgetHitTestResult::Opaque
        } else {
            WidgetHitTestResult::PassThrough
        }
    }

    pub(in crate::runtime) fn preflight_pointer_press(
        &self,
        bounds: Rect,
        input: &WidgetInput,
    ) -> PointerPressAdmission {
        self.widget.preflight_pointer_press(bounds, input)
    }

    pub(in crate::runtime) fn retains_managed_pointer_capture(&self) -> bool {
        self.widget.retains_managed_pointer_capture()
    }

    pub(in crate::runtime) fn pointer_move_overlay_is_valid(&self) -> bool {
        if self.widget.common().state.disabled {
            return false;
        }
        let capabilities = self.widget.capabilities_v2();
        if capabilities.has_pointer_motion() {
            capabilities.pointer_motion().is_some_and(|motion| {
                motion.prefers_pointer_move_paint_only() && motion.pointer_move_overlay_is_valid()
            })
        } else {
            self.widget.prefers_pointer_move_paint_only()
        }
    }

    pub(in crate::runtime) fn pointer_capture_policy(&self) -> PointerCapturePolicy {
        if self.widget.common().state.disabled {
            PointerCapturePolicy::Exclusive
        } else {
            let capabilities = self.widget.capabilities_v2();
            if capabilities.has_pointer_motion() {
                capabilities
                    .pointer_motion()
                    .map_or(PointerCapturePolicy::PassThrough, |motion| {
                        motion.pointer_capture_policy()
                    })
            } else {
                self.widget.pointer_capture_policy()
            }
        }
    }

    pub(in crate::runtime) fn cursor_for_point(
        &self,
        bounds: Rect,
        point: Point,
    ) -> Option<WidgetCursor> {
        if self.widget.common().state.disabled {
            return None;
        }
        let capabilities = self.widget.capabilities_v2();
        if capabilities.has_hit_test() {
            capabilities
                .hit_test()
                .and_then(|hit_test| hit_test.cursor_for_point(bounds, point))
        } else {
            self.widget.cursor_for_point(bounds, point)
        }
    }

    pub(in crate::runtime::surface) fn automation_semantics(&self) -> AutomationNodeSemantics {
        self.widget.automation_semantics()
    }

    pub(in crate::runtime::surface) fn automation_available_actions(&self) -> Option<Vec<String>> {
        self.widget.automation_available_actions()
    }

    pub(in crate::runtime) fn needs_state_synchronization(&self) -> bool {
        self.widget.needs_state_synchronization()
    }

    pub(in crate::runtime) fn supports_prepared_state_synchronization(&self) -> bool {
        self.widget.supports_prepared_state_synchronization()
    }

    pub(in crate::runtime::surface) fn prepare_replacement(
        &mut self,
        successor: Option<&dyn Widget>,
    ) -> Option<WidgetOutput> {
        self.widget_object_mut_runtime()
            .prepare_replacement(successor)
    }

    pub(in crate::runtime) fn suppresses_container_hover(&self) -> bool {
        let common = self.widget.common();
        !common.state.disabled
            && common.paint.paints_state_layers
            && (common.focus != FocusBehavior::None || common.paint.suppresses_container_hover)
    }

    pub(super) fn layout_node_with_environment(
        &self,
        environment: &crate::runtime::ResolvedEnvironment,
    ) -> LayoutNode {
        self.widget.layout_node_with_environment(environment)
    }

    #[cfg(test)]
    pub(super) fn handle_input(
        &mut self,
        widget_id: WidgetId,
        bounds: Rect,
        input: WidgetInput,
    ) -> Option<WidgetOutput> {
        (self.id() == widget_id)
            .then(|| self.widget.handle_input(bounds, input))
            .flatten()
    }

    pub(super) fn handle_input_with_environment(
        &mut self,
        widget_id: WidgetId,
        bounds: Rect,
        input: WidgetInput,
        environment: &crate::runtime::ResolvedEnvironment,
    ) -> Option<WidgetOutput> {
        (self.id() == widget_id)
            .then(|| {
                self.widget
                    .handle_input_with_environment(bounds, input, environment)
            })
            .flatten()
    }

    pub(in crate::runtime) fn dispatch_pointer_capture_cancelled_at(
        &mut self,
        widget_id: WidgetId,
        bounds: Rect,
        now: Instant,
    ) -> super::WidgetDispatchResult<Message> {
        let Some(output) = (self.id() == widget_id)
            .then(|| self.widget.handle_pointer_capture_cancelled_at(bounds, now))
            .flatten()
        else {
            return super::WidgetDispatchResult::NoOutput;
        };
        self.messages.dispatch_output(output)
    }

    pub(in crate::runtime) fn dispatch_pointer_event(
        &mut self,
        widget_id: WidgetId,
        bounds: Rect,
        event: crate::gui::pointer_ingress::PointerEvent,
    ) -> super::WidgetDispatchResult<Message> {
        let Some(output) = (self.id() == widget_id)
            .then(|| self.widget.handle_pointer_event(bounds, event))
            .flatten()
        else {
            return super::WidgetDispatchResult::NoOutput;
        };
        self.messages
            .map_pointer_output(output)
            .map(super::WidgetDispatchResult::Message)
            .unwrap_or(super::WidgetDispatchResult::UnmappedOutput)
    }

    pub(in crate::runtime) fn is_command_control(&self) -> bool {
        self.messages.is_command_control()
    }

    pub(in crate::runtime) fn has_pointer_output(&self) -> bool {
        self.messages.has_pointer_output()
    }

    pub(in crate::runtime) fn dispatch_focus_changed_at(
        &mut self,
        widget_id: WidgetId,
        bounds: Rect,
        focused: bool,
        now: Instant,
    ) -> super::WidgetDispatchResult<Message> {
        let Some(output) = (self.id() == widget_id)
            .then(|| self.widget.handle_focus_changed_at(bounds, focused, now))
            .flatten()
        else {
            return super::WidgetDispatchResult::NoOutput;
        };
        self.messages.dispatch_output(output)
    }

    #[cfg(test)]
    pub(in crate::runtime) fn dispatch_input(
        &mut self,
        widget_id: WidgetId,
        bounds: Rect,
        input: WidgetInput,
    ) -> super::WidgetDispatchResult<Message> {
        let Some(output) = self.handle_input(widget_id, bounds, input) else {
            return super::WidgetDispatchResult::NoOutput;
        };
        self.messages.dispatch_output(output)
    }

    pub(in crate::runtime) fn dispatch_input_with_environment(
        &mut self,
        widget_id: WidgetId,
        bounds: Rect,
        input: WidgetInput,
        environment: &crate::runtime::ResolvedEnvironment,
    ) -> super::WidgetDispatchResult<Message> {
        let Some(output) =
            self.handle_input_with_environment(widget_id, bounds, input, environment)
        else {
            return super::WidgetDispatchResult::NoOutput;
        };
        self.messages.dispatch_output(output)
    }

    pub(in crate::runtime) fn gesture_policy(
        &self,
    ) -> Option<(crate::widgets::GesturePolicy, WidgetSemanticsRevision)> {
        let caps = self.widget.capabilities_v2();
        let gestures = caps.gestures()?;
        Some((gestures.policy(), gestures.revision()))
    }
    pub(in crate::runtime) fn has_gesture_handler(
        &mut self,
        policy: crate::widgets::GesturePolicy,
    ) -> bool {
        self.widget
            .action_capabilities()
            .into_gestures()
            .is_some_and(|handler| handler.policy() == policy)
    }
    pub(in crate::runtime) fn dispatch_gesture(
        &mut self,
        event: crate::widgets::GestureEvent,
    ) -> Option<super::WidgetDispatchResult<Message>> {
        let policy = self.gesture_policy()?.0;
        let handler = self.widget.action_capabilities().into_gestures()?;
        if handler.policy() != policy {
            return None;
        }
        Some(
            handler
                .dispatch(event)
                .map_or(super::WidgetDispatchResult::NoOutput, |output| {
                    self.messages.dispatch_output(output)
                }),
        )
    }

    pub(in crate::runtime) fn supports_semantic_action(
        &self,
        action: &crate::widgets::SemanticAction,
    ) -> bool {
        self.widget
            .capabilities_v2()
            .semantic_actions()
            .is_some_and(|actions| actions.supports(action))
    }

    pub(in crate::runtime) fn has_semantic_action_handler(
        &mut self,
        action: &crate::widgets::SemanticAction,
    ) -> bool {
        self.widget
            .action_capabilities()
            .into_semantic_actions()
            .is_some_and(|actions| actions.supports(action))
    }

    pub(in crate::runtime) fn dispatch_semantic_action(
        &mut self,
        action: crate::widgets::SemanticAction,
        source: crate::widgets::SemanticActionSource,
    ) -> Result<super::WidgetDispatchResult<Message>, ()> {
        let Some(handler) = self.widget.action_capabilities().into_semantic_actions() else {
            return Err(());
        };
        if !handler.supports(&action) {
            return Err(());
        }
        match handler.dispatch(action, source) {
            crate::widgets::WidgetSemanticActionResult::Unsupported => Err(()),
            crate::widgets::WidgetSemanticActionResult::Accepted(None) => {
                Ok(super::WidgetDispatchResult::NoOutput)
            }
            crate::widgets::WidgetSemanticActionResult::Accepted(Some(output)) => {
                Ok(self.messages.dispatch_output(output))
            }
        }
    }

    pub(in crate::runtime) fn dispatch_accessibility_action(
        &mut self,
        widget_id: WidgetId,
        action: crate::widgets::NumericAccessibilityAction,
    ) -> Option<(WidgetOutput, super::WidgetDispatchResult<Message>)> {
        if self.id() != widget_id {
            return None;
        }
        let output = self.widget.handle_accessibility_action(action)?;
        let mapped = self
            .messages
            .map_accessibility_output(output.clone())
            .map(super::WidgetDispatchResult::Message)
            .unwrap_or(super::WidgetDispatchResult::UnmappedOutput);
        Some((output, mapped))
    }

    pub(in crate::runtime) fn dispatch_wheel_sample(
        &mut self,
        widget_id: WidgetId,
        bounds: Rect,
        position: Point,
        sample: WheelSample,
    ) -> (super::WidgetDispatchResult<Message>, bool) {
        let Some(output) = (self.id() == widget_id)
            .then(|| self.widget.handle_wheel_sample(bounds, position, sample))
            .flatten()
        else {
            return (
                super::WidgetDispatchResult::NoOutput,
                self.id() == widget_id && self.widget.retains_managed_wheel_sequence(),
            );
        };
        let retains = self.widget.retains_managed_wheel_sequence();
        let result = self.messages.dispatch_output(output);
        (result, retains)
    }

    pub(in crate::runtime) fn dispatch_composition_sample(
        &mut self,
        widget_id: WidgetId,
        sample: CompositionSample,
    ) -> (super::WidgetDispatchResult<Message>, bool) {
        let Some(output) = (self.id() == widget_id)
            .then(|| self.widget.handle_composition_sample(sample))
            .flatten()
        else {
            return (
                super::WidgetDispatchResult::NoOutput,
                self.id() == widget_id && self.widget.retains_managed_composition(),
            );
        };
        let retains = self.widget.retains_managed_composition();
        let result = self.messages.dispatch_output(output);
        (result, retains)
    }

    pub(in crate::runtime) fn dispatch_hidden_composition_update(
        &mut self,
        widget_id: WidgetId,
        preedit: String,
        timestamp: Option<crate::gui::input::InputTimestamp>,
    ) -> (super::WidgetDispatchResult<Message>, bool) {
        let Some(output) = (self.id() == widget_id)
            .then(|| {
                self.widget
                    .handle_hidden_composition_update(preedit, timestamp)
            })
            .flatten()
        else {
            return (
                super::WidgetDispatchResult::NoOutput,
                self.id() == widget_id && self.widget.retains_managed_composition(),
            );
        };
        let retains = self.widget.retains_managed_composition();
        let result = self.messages.dispatch_output(output);
        (result, retains)
    }

    pub(super) fn dispatch_output(
        &self,
        widget_id: WidgetId,
        output: WidgetOutput,
    ) -> Option<Message> {
        (self.id() == widget_id)
            .then(|| self.messages.map_output(output))
            .flatten()
    }

    pub(in crate::runtime) fn dispatch_native_file_drop(
        &self,
        widget_id: WidgetId,
        drop: crate::runtime::NativeFileDrop,
    ) -> Option<Message> {
        (self.id() == widget_id)
            .then(|| self.messages.map_native_file_drop(drop))
            .flatten()
    }

    pub(in crate::runtime) fn with_native_file_drop(
        mut self,
        map: impl Fn(crate::runtime::NativeFileDrop) -> Message + 'static,
    ) -> Self {
        self.accepts_native_file_drop = true;
        self.messages = self.messages.with_native_file_drop(map);
        self
    }

    pub(crate) fn with_native_file_drop_mapped(
        mut self,
        map: EventMapper<crate::runtime::NativeFileDrop, Message>,
    ) -> Self {
        self.accepts_native_file_drop = true;
        self.messages = self.messages.with_native_file_drop_mapped(map);
        self
    }

    pub(in crate::runtime) fn accepting_native_file_drop(mut self) -> Self {
        self.accepts_native_file_drop = true;
        self
    }
}
