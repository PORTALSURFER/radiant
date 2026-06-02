//! Object-safe widget trait shared by built-in primitives and custom widgets.

use crate::{
    gui::types::{Point, Rect},
    layout::{LayoutOutput, Vector2},
    runtime::{PaintPrimitive, SurfacePaintPlan},
    theme::ThemeTokens,
    widgets::{
        interaction::{WidgetCursor, WidgetInput, WidgetOutput},
        primitives::{TextAlign, TextBackgroundRole, TextColorRole, TextWrap, WidgetCommon},
    },
};
use std::any::Any;

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
pub trait Widget: WidgetClone + Send + Sync + Any {
    /// Return the shared identity, sizing, focus, state, and style contract.
    fn common(&self) -> &WidgetCommon;

    /// Return the shared contract mutably for runtime-owned state updates.
    fn common_mut(&mut self) -> &mut WidgetCommon;

    /// Route one backend-neutral input event into this widget.
    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput>;

    /// Reconcile retained widget-local state from the previous projected widget.
    ///
    /// The generic runtime calls this when a host message reprojects the
    /// declarative surface. Built-in and custom widgets can preserve transient
    /// interaction details such as caret, selection, or drag state without
    /// requiring the runtime controller to know concrete widget types.
    fn synchronize_from_previous(&mut self, _previous: &dyn Widget) {}

    /// Return whether this widget needs refresh-time state reconciliation.
    ///
    /// Custom widgets default to `true` so existing widgets keep their previous
    /// behavior unless they explicitly declare that they are stateless. Passive
    /// built-in widgets can return `false` to keep large refreshes from spending
    /// work on guaranteed no-op state synchronization.
    fn needs_state_synchronization(&self) -> bool {
        true
    }

    /// Return whether this widget accepts text-editing input while focused.
    fn accepts_text_input(&self) -> bool {
        false
    }

    /// Return whether this widget wants wheel input before scroll fallback.
    fn accepts_wheel_input(&self) -> bool {
        false
    }

    /// Return whether this widget needs pointer-move events after hover state is stable.
    ///
    /// Widgets that only use pointer motion to maintain hover/pressed state can
    /// return `false`; the runtime still routes enter, leave, and captured drag
    /// motion. Custom widgets default to `true` so richer pointer-driven
    /// behavior is preserved unless a widget explicitly opts out.
    ///
    /// Keep this enabled when a widget updates local paint state from pointer
    /// motion, such as a snapped timeline cursor, canvas hover highlight, or
    /// resize handle preview. Stable pointer moves routed through this hook,
    /// and captured drag moves routed to the active widget, request repaint
    /// even when `handle_input` returns `None`, so widgets do not need to emit
    /// host messages merely to refresh transient hover or drag chrome.
    /// In short: request repaint even when `handle_input` returns `None` for
    /// widget-local pointer preview state.
    /// Widget-local pointer state does not need to emit host messages.
    fn accepts_pointer_move(&self) -> bool {
        true
    }

    /// Return whether other widgets under the pointer may receive pointer-move
    /// events while this widget owns pointer capture.
    ///
    /// Keep this enabled for drag sources that need live drop-target hover
    /// feedback. Disable it for exclusive controls such as splitters and resize
    /// handles where moving away from the handle should not activate unrelated
    /// hover surfaces before release.
    fn allows_captured_pointer_pass_through(&self) -> bool {
        true
    }

    /// Return the cursor this widget wants at `point` inside `bounds`.
    ///
    /// Returning `None` lets the runtime continue with the default cursor.
    /// Implementations should compute this directly from widget state and
    /// geometry; the runtime may call it on every pointer move.
    fn cursor_for_point(&self, _bounds: Rect, _point: Point) -> Option<WidgetCursor> {
        None
    }

    /// Return whether stable pointer motion can redraw this widget through
    /// [`Self::append_runtime_overlay_paint`] without rebuilding the base scene.
    ///
    /// Use this for editor affordances whose pointer-following visuals are
    /// fully transient, such as timeline cursors, hover handles, captured drag
    /// previews, or small selection markers. Widgets that paint pointer-motion
    /// state in [`Self::append_paint`] should keep the default `false` so the
    /// runtime rebuilds the scene when local pointer state changes.
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
