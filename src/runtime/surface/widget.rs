use super::source::SourceMetadata;
use crate::{
    gui::types::{Point, Rect},
    layout::LayoutNode,
    widgets::{
        FocusBehavior, PointerCapturePolicy, PointerPressPreflight, RuntimePointerCaptureContract,
        Widget, WidgetCursor, WidgetId, WidgetInput, WidgetOutput, WidgetRevision,
        WidgetSemanticsRevision,
    },
};
use std::rc::Rc;

mod mapper;

pub use mapper::{
    EventMapper, MessageMapper, NativeFileDropMessageMapper, ScrollMessageMapper,
    WidgetMessageMapper,
};
pub(crate) use mapper::{MapperDescriptor, MapperRelation};

#[derive(Clone, Copy)]
struct RuntimePointerCaptureContractHooks {
    take_pointer_capture_termination_request: fn(&mut dyn Widget) -> bool,
    continues_pointer_capture_after_release: fn(&dyn Widget, &WidgetInput) -> bool,
}

fn take_pointer_capture_termination_request<T>(widget: &mut dyn Widget) -> bool
where
    T: Widget + RuntimePointerCaptureContract + 'static,
{
    widget
        .as_any_mut()
        .downcast_mut::<T>()
        .is_some_and(RuntimePointerCaptureContract::take_pointer_capture_termination_request)
}

fn continues_pointer_capture_after_release<T>(widget: &dyn Widget, release: &WidgetInput) -> bool
where
    T: Widget + RuntimePointerCaptureContract + 'static,
{
    widget.as_any().downcast_ref::<T>().is_some_and(|widget| {
        RuntimePointerCaptureContract::continues_pointer_capture_after_release(widget, release)
    })
}

fn runtime_pointer_capture_contract_hooks<T>() -> RuntimePointerCaptureContractHooks
where
    T: Widget + RuntimePointerCaptureContract + 'static,
{
    RuntimePointerCaptureContractHooks {
        take_pointer_capture_termination_request: take_pointer_capture_termination_request::<T>,
        continues_pointer_capture_after_release: continues_pointer_capture_after_release::<T>,
    }
}

/// One widget leaf inside a generic declarative [`UiSurface`](super::UiSurface).
pub struct SurfaceWidget<Message> {
    widget: Box<dyn Widget>,
    messages: WidgetMessageMapper<Message>,
    accepts_native_file_drop: bool,
    runtime_pointer_capture_contract: Option<RuntimePointerCaptureContractHooks>,
    revision_evidence: SurfaceWidgetRevisionEvidence,
    pub(in crate::runtime::surface) source: Option<Rc<SourceMetadata>>,
}

/// Immutable widget evidence captured when the widget crosses the erased
/// `SurfaceWidget` boundary.  View-delta classification borrows this record
/// and never dispatches back into the live widget object.
#[derive(Clone)]
pub(crate) struct SurfaceWidgetRevisionEvidence {
    pub(crate) id: WidgetId,
    pub(crate) compatibility_kind: &'static str,
    pub(crate) revision: WidgetRevision,
    pub(crate) capabilities: WidgetCapabilityEvidence,
    pub(crate) valid: bool,
}

#[derive(Clone, Default)]
pub(crate) struct WidgetCapabilityEvidence {
    pub(crate) contract_version: u16,
    pub(crate) semantics_revision: Option<WidgetSemanticsRevision>,
}

impl WidgetCapabilityEvidence {
    fn capture(widget: &dyn Widget) -> Self {
        let capabilities = widget.capabilities();
        Self {
            contract_version: capabilities.contract_version,
            semantics_revision: capabilities.semantics_revision(),
        }
    }

    fn conservative() -> Self {
        Self {
            contract_version: 0,
            semantics_revision: None,
        }
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
            widget: self.widget.clone(),
            messages: self.messages.clone(),
            accepts_native_file_drop: self.accepts_native_file_drop,
            runtime_pointer_capture_contract: self.runtime_pointer_capture_contract,
            revision_evidence: self.revision_evidence.clone(),
            source: self.source.clone(),
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
        Self::from_boxed_with_runtime_pointer_capture_contract(widget, messages, None)
    }

    pub(crate) fn with_runtime_pointer_capture_contract<T>(
        widget: T,
        messages: WidgetMessageMapper<Message>,
    ) -> Self
    where
        T: Widget + RuntimePointerCaptureContract + Clone + 'static,
    {
        Self::from_boxed_with_runtime_pointer_capture_contract(
            Box::new(widget),
            messages,
            Some(runtime_pointer_capture_contract_hooks::<T>()),
        )
    }

    fn from_boxed_with_runtime_pointer_capture_contract(
        widget: Box<dyn Widget>,
        messages: WidgetMessageMapper<Message>,
        runtime_pointer_capture_contract: Option<RuntimePointerCaptureContractHooks>,
    ) -> Self {
        let revision_evidence = SurfaceWidgetRevisionEvidence::capture(widget.as_ref());
        Self {
            widget,
            messages,
            accepts_native_file_drop: false,
            runtime_pointer_capture_contract,
            revision_evidence,
            source: None,
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

    pub(in crate::runtime::surface) fn revision_evidence(&self) -> &SurfaceWidgetRevisionEvidence {
        &self.revision_evidence
    }

    /// Borrow the erased widget for runtime-owned state mutation.  These
    /// mutations intentionally preserve declarative revision evidence.
    pub(in crate::runtime) fn widget_object_mut_runtime(&mut self) -> &mut dyn Widget {
        self.widget.as_mut()
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

    pub(in crate::runtime) fn accepts_pointer_move(&self) -> bool {
        !self.widget.common().state.disabled && self.widget.accepts_pointer_move()
    }

    pub(in crate::runtime) fn accepts_pointer_input(&self, input: &WidgetInput) -> bool {
        !self.widget.common().state.disabled && self.widget.accepts_pointer_input(input)
    }

    pub(in crate::runtime) fn preflight_pointer_press(
        &self,
        input: &WidgetInput,
    ) -> PointerPressPreflight {
        self.widget.preflight_pointer_press(input)
    }

    pub(in crate::runtime) fn take_pointer_capture_termination_request(&mut self) -> bool {
        self.runtime_pointer_capture_contract
            .map(|hooks| (hooks.take_pointer_capture_termination_request)(self.widget.as_mut()))
            .unwrap_or(false)
    }

    pub(in crate::runtime) fn continues_pointer_capture_after_release(
        &self,
        release: &WidgetInput,
    ) -> bool {
        self.runtime_pointer_capture_contract
            .map(|hooks| {
                (hooks.continues_pointer_capture_after_release)(self.widget.as_ref(), release)
            })
            .unwrap_or(false)
    }

    pub(in crate::runtime) fn prefers_pointer_move_paint_only(&self) -> bool {
        !self.widget.common().state.disabled && self.widget.prefers_pointer_move_paint_only()
    }

    pub(in crate::runtime) fn pointer_capture_policy(&self) -> PointerCapturePolicy {
        if self.widget.common().state.disabled {
            PointerCapturePolicy::Exclusive
        } else {
            self.widget.pointer_capture_policy()
        }
    }

    pub(in crate::runtime) fn cursor_for_point(
        &self,
        bounds: Rect,
        point: Point,
    ) -> Option<WidgetCursor> {
        (!self.widget.common().state.disabled)
            .then(|| self.widget.cursor_for_point(bounds, point))
            .flatten()
    }

    pub(in crate::runtime) fn needs_state_synchronization(&self) -> bool {
        self.widget.needs_state_synchronization()
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

    pub(super) fn layout_node(&self) -> LayoutNode {
        self.widget.common().layout_node()
    }

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

    pub(in crate::runtime) fn dispatch_pointer_capture_cancelled(
        &mut self,
        widget_id: WidgetId,
        bounds: Rect,
    ) -> super::WidgetDispatchResult<Message> {
        let Some(output) = (self.id() == widget_id)
            .then(|| self.widget.handle_pointer_capture_cancelled(bounds))
            .flatten()
        else {
            return super::WidgetDispatchResult::NoOutput;
        };
        self.messages
            .map_output(output)
            .map(super::WidgetDispatchResult::Message)
            .unwrap_or(super::WidgetDispatchResult::UnmappedOutput)
    }

    pub(in crate::runtime) fn dispatch_input(
        &mut self,
        widget_id: WidgetId,
        bounds: Rect,
        input: WidgetInput,
    ) -> super::WidgetDispatchResult<Message> {
        let Some(output) = self.handle_input(widget_id, bounds, input) else {
            return super::WidgetDispatchResult::NoOutput;
        };
        self.messages
            .map_output(output)
            .map(super::WidgetDispatchResult::Message)
            .unwrap_or(super::WidgetDispatchResult::UnmappedOutput)
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
