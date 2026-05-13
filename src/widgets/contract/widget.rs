//! Object-safe widget trait shared by built-in primitives and custom widgets.

use crate::{
    gui::types::Rect,
    layout::LayoutOutput,
    runtime::PaintPrimitive,
    theme::ThemeTokens,
    widgets::{
        interaction::{WidgetInput, WidgetOutput},
        primitives::{TextAlign, TextWrap, WidgetCommon},
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
    /// resize handle preview. Stable pointer moves routed through this hook
    /// request repaint even when `handle_input` returns `None`, so widgets do
    /// not need to emit host messages merely to refresh transient hover chrome.
    /// Widget-local pointer state does not need to emit host messages.
    fn accepts_pointer_move(&self) -> bool {
        true
    }

    /// Return whether stable pointer motion can redraw this widget through
    /// [`Self::append_runtime_overlay_paint`] without rebuilding the base scene.
    ///
    /// Use this for editor affordances whose pointer-following visuals are
    /// fully transient, such as timeline cursors, hover handles, drag previews,
    /// or small selection markers. Widgets that paint pointer-motion state in
    /// [`Self::append_paint`] should keep the default `false` so the runtime
    /// rebuilds the scene when local pointer state changes.
    fn prefers_pointer_move_paint_only(&self) -> bool {
        false
    }

    /// Return the selected text for focused text-editing widgets.
    fn selected_text(&self) -> Option<String> {
        None
    }

    /// Apply a declarative text wrapping policy when this widget supports text layout.
    fn set_text_wrap(&mut self, _wrap: TextWrap) -> bool {
        false
    }

    /// Apply a declarative horizontal text alignment policy when this widget supports text layout.
    fn set_text_align(&mut self, _align: TextAlign) -> bool {
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
