//! Object-safe widget trait shared by built-in primitives and custom widgets.

use crate::{
    gui::{
        input::InputTimestamp,
        types::{Point, Rect},
    },
    layout::{LayoutNode, LayoutOutput, Vector2},
    runtime::{PaintPrimitive, ResolvedEnvironment, SurfacePaintPlan},
    theme::ThemeTokens,
    widgets::{
        DeclaredTextMetrics, TextScaleParticipation, WidgetRevision,
        interaction::{
            CompositionSample, CompositionStartContext, NumericAccessibilityAction,
            NumericAccessibilityBlockOwner, WheelSample, WidgetCursor, WidgetInput, WidgetKey,
            WidgetOutput,
        },
        primitives::{TextAlign, TextBackgroundRole, TextColorRole, TextWrap, WidgetCommon},
    },
};
use std::any::Any;
use std::time::Instant;

use super::{
    paint::WidgetPaintContext,
    semantics::{
        WidgetCapabilities, WidgetCapabilitiesV2, automation_available_actions,
        resolve_automation_semantics,
    },
};
use crate::gui::automation::AutomationNodeSemantics;

/// Pointer routing behavior while a widget owns pointer capture.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PointerCapturePolicy {
    /// Pointer motion is routed only to the captured widget.
    ///
    /// Use this for exclusive controls such as resize handles and splitters,
    /// where moving over unrelated widgets before release should not activate
    /// their hover or pointer-motion behavior.
    Exclusive,
    /// Pointer motion may pass through to widgets under the pointer.
    ///
    /// Use this for drag sources that need live feedback from drop targets or
    /// other widgets under the pointer while the source remains captured.
    #[default]
    PassThrough,
}

/// Decision returned when the focused widget is asked to release focus.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FocusLossDecision {
    /// Allow the controller to deliver focus loss and continue the transition.
    #[default]
    Allow,
    /// Keep the controller-owned focus target and reject the transition.
    Veto,
}

/// Admission result for a pointer press before widget interaction begins.
///
/// The default [`Widget::preflight_pointer_press`] implementation returns
/// [`Self::Legacy`], preserving the existing controller-owned press behavior.
/// [`Self::ManagedCapture`] opts the widget into the bounded controller-managed
/// capture authority, while [`Self::Blocked`] refuses the press without
/// transferring focus, installing capture, dispatching input, or mapping
/// output.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PointerPressAdmission {
    /// Preserve the existing pointer press and capture behavior.
    #[default]
    Legacy,
    /// Admit the press through the controller-managed capture authority.
    ManagedCapture,
    /// Refuse the press before widget or controller interaction mutation.
    Blocked,
}

/// Clone support for boxed [`Widget`] trait objects.
pub trait WidgetClone {
    /// Clone this widget into an owned trait object.
    fn clone_box(&self) -> Box<dyn Widget>;
}

impl<T> WidgetClone for T
where
    T: Widget + Clone + 'static,
{
    fn clone_box(&self) -> Box<dyn Widget> {
        Box::new(self.clone())
    }
}

impl Clone for Box<dyn Widget> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

/// Public object-safe contract for user-defined Radiant widgets.
///
/// Built-in primitives and custom widgets implement this same trait and travel
/// through the runtime, input, message, paint, and application-builder paths
/// without adding a new Radiant enum variant.
pub trait Widget: WidgetClone + Any {
    /// Return the concrete compatibility kind used for retained-state reuse.
    ///
    /// The default is derived from the implementing type, so existing custom
    /// widgets remain source-compatible while incompatible replacements can be
    /// detected without exposing a `TypeId` contract to callers.
    fn compatibility_kind(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    /// Return declarative revision metadata for this widget's immutable inputs.
    ///
    /// The conservative default is always correct and keeps existing custom
    /// widgets source-compatible. Widgets that can prove exact changes may
    /// return `WidgetRevision::exact(structure, geometry, paint, interaction)`
    /// with four independently typed `Eq + 'static` values. Component type
    /// mismatches and unavailable evidence widen safely through the classifier;
    /// exact revisions are clonable UI-local values rather than `Copy` values
    /// because they retain arbitrary component ownership. No production refresh
    /// or repaint optimization consumes this hook yet.
    fn revision(&self) -> WidgetRevision {
        WidgetRevision::conservative()
    }

    /// Prepare to release keyboard focus.
    ///
    /// This synchronous, allocation-free decision seam lets a focused widget
    /// retain focus when a terminal edit is invalid or incomplete. The default
    /// preserves the existing focus-loss behavior for custom widgets.
    fn prepare_focus_loss(&mut self) -> FocusLossDecision {
        FocusLossDecision::Allow
    }

    /// Return whether this widget currently supports one neutral numeric
    /// accessibility action.
    ///
    /// The default keeps custom widgets out of the numeric action boundary.
    /// Runtime callers must still revalidate identity, focus, ownership, and
    /// enabled/editable state before invoking the action hook.
    fn supports_accessibility_action(&self, _action: &NumericAccessibilityAction) -> bool {
        false
    }

    /// Report a current local interaction owner that blocks an accessibility
    /// action without changing focus or widget state.
    fn accessibility_action_owner(&self) -> Option<NumericAccessibilityBlockOwner> {
        None
    }

    /// Consume one already-admitted neutral accessibility action.
    ///
    /// The default is inert. Implementations return a type-erased
    /// [`WidgetOutput`] so application-owned typed policy envelopes remain
    /// available without making the generic runtime know their type parameters.
    fn handle_accessibility_action(
        &mut self,
        _action: NumericAccessibilityAction,
    ) -> Option<WidgetOutput> {
        None
    }

    /// Preflight one pointer press without mutating widget state.
    ///
    /// The hook is synchronous, object-safe, allocation-free, and immutable so
    /// the controller can decide whether to proceed before preparing focus loss
    /// or installing pointer capture. Existing widgets retain the legacy path
    /// through the default.
    fn preflight_pointer_press(
        &self,
        _bounds: Rect,
        _input: &WidgetInput,
    ) -> PointerPressAdmission {
        PointerPressAdmission::Legacy
    }

    /// Report whether this widget still owns an admitted managed pointer press.
    ///
    /// The controller consults this only after creating a record from
    /// [`PointerPressAdmission::ManagedCapture`]. The default keeps existing
    /// widgets on the legacy capture path.
    fn retains_managed_pointer_capture(&self) -> bool {
        false
    }

    /// Return the shared identity, sizing, focus, state, and style contract.
    fn common(&self) -> &WidgetCommon;

    /// Return the shared contract mutably for runtime-owned state updates.
    fn common_mut(&mut self) -> &mut WidgetCommon;

    /// Return whether this widget's declared text metrics follow application
    /// text scaling. Custom widgets remain unscaled unless they opt in.
    fn text_scale_participation(&self) -> TextScaleParticipation {
        TextScaleParticipation::Unscaled
    }

    /// Project this widget's intrinsic sizing for one resolved environment.
    ///
    /// The default preserves the legacy sizing contract. Text-aware widgets
    /// opt in by returning [`TextScaleParticipation::Scaled`].
    fn layout_node_with_environment(&self, environment: &ResolvedEnvironment) -> LayoutNode {
        LayoutNode::Widget(
            DeclaredTextMetrics::new(self.common().sizing, 1.0, Vector2::new(0.0, 0.0))
                .resolve(environment, self.text_scale_participation())
                .layout_node(self.common().id),
        )
    }

    /// Route one backend-neutral input event into this widget.
    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput>;

    /// Route input with the immutable environment used by the current frame.
    /// The default delegates to the required legacy hook for custom widgets.
    fn handle_input_with_environment(
        &mut self,
        bounds: Rect,
        input: WidgetInput,
        _environment: &ResolvedEnvironment,
    ) -> Option<WidgetOutput> {
        self.handle_input(bounds, input)
    }

    /// Route a focus transition with the runtime's current monotonic clock.
    ///
    /// The default preserves the existing [`WidgetInput::FocusChanged`] path.
    /// Widgets with delayed focus-loss visuals can override this additive seam
    /// to consume the supplied clock without adding host-specific state.
    fn handle_focus_changed_at(
        &mut self,
        bounds: Rect,
        focused: bool,
        _now: Instant,
    ) -> Option<WidgetOutput> {
        self.handle_input(bounds, WidgetInput::FocusChanged(focused))
    }

    /// Route one exact wheel sample into this widget.
    ///
    /// The default projects the sample into the existing logical-pixel
    /// [`WidgetInput::Wheel`] contract, preserving source compatibility for
    /// existing custom widgets. Widgets that need line/pixel or phase evidence
    /// can override this object-safe hook without changing `WidgetInput`.
    fn handle_wheel_sample(
        &mut self,
        bounds: Rect,
        position: Point,
        sample: WheelSample,
    ) -> Option<WidgetOutput> {
        sample
            .to_widget_input(position)
            .and_then(|input| self.handle_input(bounds, input))
    }

    /// Report whether this widget still owns an admitted explicit wheel
    /// sequence after its most recent exact sample.
    ///
    /// The controller only uses this evidence to pin routing to the exact
    /// widget that handled an explicit `Started` sample. The default retains
    /// legacy wheel behavior and never installs managed wheel authority.
    fn retains_managed_wheel_sequence(&self) -> bool {
        false
    }

    /// Cancel widget-local pointer-capture state without changing focus or
    /// delivering a legacy focus-loss output to the host.
    ///
    /// The default clears only the universally shared pressed state. Widgets
    /// with richer pointer gesture state or an explicit typed
    /// capture-cancellation contract may override this hook.
    fn handle_pointer_capture_cancelled(&mut self, _bounds: Rect) -> Option<WidgetOutput> {
        self.common_mut().state.pressed = false;
        None
    }

    /// Cancel pointer-capture state with the runtime's current monotonic clock.
    ///
    /// The default preserves the existing cancellation contract. Widgets with
    /// delayed cancellation visuals can override this additive seam.
    fn handle_pointer_capture_cancelled_at(
        &mut self,
        bounds: Rect,
        _now: Instant,
    ) -> Option<WidgetOutput> {
        self.handle_pointer_capture_cancelled(bounds)
    }

    /// Reconcile retained widget-local state from the previous projected widget.
    ///
    /// The generic runtime calls this when a host message reprojects the
    /// declarative surface. Built-in and custom widgets can preserve transient
    /// interaction details such as caret, selection, or drag state without
    /// requiring the runtime controller to know concrete widget types.
    fn synchronize_from_previous(&mut self, _previous: &dyn Widget) {}

    /// Return whether this widget opts into the private prepared-surface state
    /// synchronization boundary.
    ///
    /// A `true` implementation promises that its synchronization callback only
    /// mutates successor-owned local state, reads the previous widget, and does
    /// not dispatch output or re-enter projection, layout, paint, or runtime
    /// state. The default keeps existing built-in and custom widgets on the
    /// established direct refresh path.
    fn supports_prepared_state_synchronization(&self) -> bool {
        false
    }

    /// Prepare this widget's local interaction state for removal or loss of
    /// authority during surface reconciliation.
    ///
    /// The runtime calls this at most once for each installed stateful widget
    /// that crosses a refresh boundary. `Some` is an exact, proposed
    /// compatible successor; `None` conservatively represents removal,
    /// identity loss, incompatibility, or ambiguous evidence. Implementations
    /// own the teardown of their local state before returning an optional
    /// terminal [`WidgetOutput`]. The runtime maps that output through this
    /// retiring widget and never retains the successor reference.
    ///
    /// The default is a no-op so existing custom [`Widget`] implementations
    /// remain source-compatible.
    fn prepare_replacement(&mut self, _successor: Option<&dyn Widget>) -> Option<WidgetOutput> {
        None
    }

    /// Return whether this widget needs refresh-time state reconciliation.
    ///
    /// Custom widgets default to `true` so existing widgets keep their previous
    /// behavior unless they explicitly declare that they are stateless. Passive
    /// built-in widgets can return `false` to keep large refreshes from spending
    /// work on guaranteed no-op state synchronization.
    fn needs_state_synchronization(&self) -> bool {
        true
    }

    /// Return the next deadline at which widget-local visual state must advance.
    ///
    /// Native runtimes use this to wake for finite delays such as hover intent
    /// without polling continuously or requiring another pointer event.
    fn timed_repaint_deadline(&self) -> Option<Instant> {
        None
    }

    /// Advance widget-local visual state whose deadline has elapsed.
    ///
    /// Return `true` when paint output changed and the surface must be redrawn.
    fn advance_timed_repaint(&mut self, _now: Instant) -> bool {
        false
    }

    /// Return whether this widget accepts text-editing input while focused.
    fn accepts_text_input(&self) -> bool {
        false
    }

    /// Return whether this focused widget accepts backend-neutral composition
    /// samples.  The default keeps existing widgets off the composition path.
    fn accepts_composition_input(&self) -> bool {
        false
    }

    /// Return the exact current committed-value context for composition start.
    ///
    /// The default keeps existing custom widgets out of native IME admission.
    /// Implementations must use Unicode-scalar ranges from their current text
    /// state; native adapters never derive this context from preedit bytes.
    fn composition_start_context(&self) -> Option<CompositionStartContext> {
        None
    }

    /// Route one validated backend-neutral composition sample into this widget.
    ///
    /// The hook is object-safe and intentionally separate from [`WidgetInput`]
    /// so existing `Event` and `WidgetInput` compatibility remains unchanged.
    fn handle_composition_sample(&mut self, _sample: CompositionSample) -> Option<WidgetOutput> {
        None
    }

    /// Route a native preedit update whose selection/caret is explicitly
    /// hidden by the platform adapter.
    ///
    /// Existing custom widgets conservatively receive the established cancel
    /// behavior instead of retaining a previous visible selection. Widgets
    /// that support hidden preedit delivery can override this object-safe hook.
    fn handle_hidden_composition_update(
        &mut self,
        _preedit: String,
        timestamp: Option<InputTimestamp>,
    ) -> Option<WidgetOutput> {
        self.handle_composition_sample(CompositionSample::cancel_with_metadata(timestamp))
    }

    /// Report whether this widget retains the runtime-managed composition
    /// authority after its most recent sample.
    fn retains_managed_composition(&self) -> bool {
        false
    }

    /// Return whether this focused widget opts into metadata-aware focused-key
    /// routing.
    ///
    /// Opted-in widgets participate in the generic host-first initial-press
    /// decision and may establish one runtime-owned key capture after they
    /// explicitly report it through [`Self::captured_focused_key`]. The
    /// default preserves the existing key-only compatibility behavior.
    fn participates_in_focused_key_routing(&self) -> bool {
        false
    }

    /// Return the normalized key currently captured by this widget, if any.
    ///
    /// This query is evidence from the widget's current interaction state; it
    /// is not a request to capture a key and cannot create runtime authority by
    /// itself. The default leaves existing widgets without metadata-aware
    /// capture.
    fn captured_focused_key(&self) -> Option<WidgetKey> {
        None
    }

    /// Return whether this focused widget explicitly owns a key before host shortcuts.
    ///
    /// Use this sparingly for widgets whose focused editing contract depends on
    /// a key that the host also uses globally. Returning `true` does not route
    /// the key by itself; it lets the native backend give the focused widget
    /// first refusal before resolving host-level shortcuts.
    fn preempts_host_shortcut_key(&self, _key: WidgetKey) -> bool {
        false
    }

    /// Return whether this widget wants wheel input before scroll fallback.
    fn accepts_wheel_input(&self) -> bool {
        false
    }

    /// Return whether this widget wants stable pointer-motion delivery and
    /// transient pointer-state repaint opportunities.
    ///
    /// Stable pointer moves routed through this hook, and captured drag moves
    /// routed to the active widget, request repaint even when `handle_input`
    /// returns `None`, so widgets do not need to emit host messages merely to
    /// refresh transient hover or drag chrome. The default preserves the
    /// historical admission behavior.
    fn accepts_pointer_move(&self) -> bool {
        true
    }

    /// Return whether this widget can be selected as the target for a direct
    /// pointer input.
    ///
    /// The default is permissive so existing interactive widgets keep their
    /// historical hit-testing behavior. Event-aware descriptors may provide a
    /// more specific decision for individual input kinds.
    fn accepts_pointer_input(&self, _input: &WidgetInput) -> bool {
        true
    }

    /// Return optional semantics intentionally exported by this widget through
    /// the source-compatible v1 descriptor.
    fn capabilities(&self) -> WidgetCapabilities<'_> {
        WidgetCapabilities::none()
    }

    /// Return the additive v2 descriptor set for optional interaction behavior.
    fn capabilities_v2(&self) -> WidgetCapabilitiesV2<'_> {
        WidgetCapabilitiesV2::none()
    }

    /// Return backend-neutral automation semantics for this widget.
    ///
    /// This compatibility query first observes the supported v2 semantics
    /// descriptor, then the source-compatible v1 semantics descriptor. A
    /// widget may override this virtual method when its automation snapshot is
    /// not descriptor-derived.
    fn automation_semantics(&self) -> AutomationNodeSemantics {
        resolve_automation_semantics(self.common(), self.capabilities(), self.capabilities_v2())
    }

    /// Return explicit automation action names when this widget's interaction
    /// policy is richer than role-derived defaults.
    ///
    /// The default resolves v2 semantics first and then valid v1 semantics.
    /// Advertisement is observational only; runtime action dispatch remains a
    /// separate authority boundary.
    fn automation_available_actions(&self) -> Option<Vec<String>> {
        automation_available_actions(self.capabilities(), self.capabilities_v2())
    }

    /// Return whether other widgets under the pointer may receive pointer-move
    /// events while this widget owns pointer capture.
    ///
    /// Keep this enabled for drag sources that need live drop-target hover
    /// feedback. Disable it for exclusive controls such as splitters and
    /// resize handles. The v2 pointer-motion descriptor takes precedence when
    /// it is supported and present.
    fn allows_captured_pointer_pass_through(&self) -> bool {
        true
    }

    /// Return this widget's pointer routing behavior while it owns capture.
    ///
    /// The default preserves the historical boolean hook semantics so existing
    /// custom widgets retain their behavior.
    fn pointer_capture_policy(&self) -> PointerCapturePolicy {
        if self.allows_captured_pointer_pass_through() {
            PointerCapturePolicy::PassThrough
        } else {
            PointerCapturePolicy::Exclusive
        }
    }

    /// Return the cursor this widget wants at `point` inside `bounds`.
    ///
    /// Returning `None` lets the runtime continue with the default cursor.
    /// The v2 hit-test descriptor takes precedence when it is supported and
    /// present.
    fn cursor_for_point(&self, _bounds: Rect, _point: Point) -> Option<WidgetCursor> {
        None
    }

    /// Return whether stable pointer motion may repaint only through
    /// [`Self::append_runtime_overlay_paint`].
    ///
    /// The runtime additionally requires valid v2 overlay evidence before it
    /// takes the paint-only path. The legacy hook remains the fallback when no
    /// supported v2 pointer-motion descriptor is present.
    fn prefers_pointer_move_paint_only(&self) -> bool {
        false
    }

    /// Return the selected text for focused text-editing widgets as a borrowed slice.
    fn selected_text_slice(&self) -> Option<&str> {
        None
    }

    /// Return the selected text for focused text-editing widgets as an owned string.
    fn selected_text(&self) -> Option<String> {
        self.selected_text_slice().map(str::to_owned)
    }

    /// Apply a declarative text wrapping policy when this widget supports text layout.
    fn set_text_wrap(&mut self, _wrap: TextWrap) -> bool {
        false
    }

    /// Apply a declarative horizontal text alignment policy when this widget supports text layout.
    fn set_text_align(&mut self, _align: TextAlign) -> bool {
        false
    }

    /// Apply a semantic foreground color role when this widget supports text paint.
    fn set_text_color(&mut self, _color: TextColorRole) -> bool {
        false
    }

    /// Apply a semantic background fill role when this widget supports text paint.
    fn set_text_background(&mut self, _background: TextBackgroundRole) -> bool {
        false
    }

    /// Apply text insets inside the assigned widget bounds when supported.
    fn set_text_inset(&mut self, _inset: Vector2) -> bool {
        false
    }

    /// Append backend-neutral paint primitives for this widget.
    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        layout: &LayoutOutput,
        theme: &ThemeTokens,
    );

    /// Append paint using one immutable context-aware view of layout, theme,
    /// bounds, and window environment.
    ///
    /// The default delegates exactly once to the required legacy hook so
    /// existing widgets and trait objects retain their behavior unchanged.
    fn append_paint_with_context(&self, context: &mut WidgetPaintContext<'_>) {
        let bounds = context.bounds();
        let layout = context.layout();
        let theme = context.theme();
        let primitives = context.primitives();
        self.append_paint(primitives, bounds, layout, theme);
    }

    /// Return this widget's paint primitives for the given bounds.
    ///
    /// This is a convenience for tests, automation, previews, and embedded
    /// hosts that need to inspect one widget's paint output without manually
    /// allocating a primitive buffer. Use [`Self::append_paint`] when callers
    /// already own the paint buffer or need tight allocation control.
    fn paint_primitives(
        &self,
        bounds: Rect,
        layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) -> Vec<PaintPrimitive> {
        let mut primitives = Vec::new();
        self.append_paint(&mut primitives, bounds, layout, theme);
        primitives
    }

    /// Return this widget's paint output as a queryable paint plan for the given bounds.
    ///
    /// This is useful for tests, automation, previews, and embedded hosts that
    /// want [`SurfacePaintPlan`] query helpers for one widget without wrapping
    /// it in a temporary `UiSurface`.
    fn paint_plan(
        &self,
        bounds: Rect,
        layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) -> SurfacePaintPlan {
        let mut plan = SurfacePaintPlan::empty(theme);
        self.append_paint(&mut plan.primitives, bounds, layout, theme);
        plan
    }

    /// Return this widget's paint primitives with default layout and theme.
    ///
    /// Use this for focused widget tests and small previews where custom layout
    /// metadata or theme tokens are not part of the behavior being checked.
    fn paint_primitives_with_defaults(&self, bounds: Rect) -> Vec<PaintPrimitive> {
        self.paint_primitives(bounds, &LayoutOutput::default(), &ThemeTokens::default())
    }

    /// Return this widget's paint output as a queryable paint plan with default
    /// layout and theme.
    ///
    /// Use this for focused widget tests and small previews where the caller
    /// wants paint-plan query helpers and default layout/theme are sufficient.
    fn paint_plan_with_defaults(&self, bounds: Rect) -> SurfacePaintPlan {
        self.paint_plan(bounds, &LayoutOutput::default(), &ThemeTokens::default())
    }

    /// Append small runtime-owned overlay primitives for the current widget state.
    ///
    /// Native backends draw these over the cached scene on paint-only pointer
    /// motion. Keep this output lightweight and limited to replayable overlay
    /// primitives such as filled and stroked rectangles; text and full widget
    /// chrome still belong in [`Self::append_paint`].
    fn append_runtime_overlay_paint(
        &self,
        _primitives: &mut Vec<PaintPrimitive>,
        _bounds: Rect,
        _layout: &LayoutOutput,
        _theme: &ThemeTokens,
    ) {
    }

    /// Append runtime-overlay paint using one immutable context-aware view.
    ///
    /// The default delegates exactly once to the legacy overlay hook for
    /// compatibility with existing widgets.
    fn append_runtime_overlay_paint_with_context(&self, context: &mut WidgetPaintContext<'_>) {
        let bounds = context.bounds();
        let layout = context.layout();
        let theme = context.theme();
        let primitives = context.primitives();
        self.append_runtime_overlay_paint(primitives, bounds, layout, theme);
    }
}

impl dyn Widget {
    /// Return this widget as `Any` for compatibility adapters.
    pub fn as_any(&self) -> &dyn Any {
        self
    }

    /// Return this widget mutably as `Any` for compatibility adapters.
    pub fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
