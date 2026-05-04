//! Deterministic generic runtime flow for declarative Radiant surfaces.
//!
//! This controller keeps the generic host bridge, projected surface, and
//! layout output together so backends can route normalized widget input without
//! depending on host-specific shell contracts.

use super::{Command, RuntimeBridge, SurfacePaintPlan, UiSurface};
use crate::{
    gui::types::{Point, Rect, Vector2},
    layout::{LayoutOutput, layout_tree},
    theme::ThemeTokens,
    widgets::{PointerButton, WidgetId, WidgetInput, WidgetKey},
};

/// Direction for deterministic keyboard focus traversal.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FocusTraversal {
    /// Move to the next keyboard-focusable widget in declarative tree order.
    Forward,
    /// Move to the previous keyboard-focusable widget in declarative tree order.
    Backward,
}

/// Backend-neutral runtime event routed through a [`SurfaceRuntime`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    /// Viewport size changed and layout should be recomputed.
    Resize {
        /// New logical viewport size.
        viewport: Vector2,
    },
    /// Pointer hover moved across the surface.
    PointerMove {
        /// Pointer position in surface logical coordinates.
        position: Point,
    },
    /// Pointer press started at the given surface position.
    PointerPress {
        /// Pointer position in surface logical coordinates.
        position: Point,
        /// Pointer button that started the press.
        button: PointerButton,
    },
    /// Pointer press ended at the given surface position.
    PointerRelease {
        /// Pointer position in surface logical coordinates.
        position: Point,
        /// Pointer button that ended the press.
        button: PointerButton,
    },
    /// One non-text key intent should route to the focused widget.
    KeyPress(WidgetKey),
    /// One printable character should route to the focused widget.
    Character(char),
    /// Move keyboard focus in declarative tree order.
    TraverseFocus(FocusTraversal),
    /// Clear current runtime focus ownership.
    ClearFocus,
}

/// Borrowed runtime context for one projected Radiant surface.
///
/// This context exposes the current viewport, immutable view tree, and resolved
/// layout without giving renderers or host code ownership of the runtime
/// controller. Style remains an explicit argument to paint-plan generation so
/// hosts can swap themes without rebuilding runtime state.
pub struct RuntimeContext<'a, Message> {
    /// Current logical viewport rectangle.
    pub viewport: Rect,
    /// Current immutable declarative view snapshot.
    pub surface: &'a UiSurface<Message>,
    /// Current resolved layout output for the surface.
    pub layout: &'a LayoutOutput,
}

/// Summary of one command-dispatch pass through a [`SurfaceRuntime`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CommandOutcome {
    /// Number of host-defined messages reduced during this pass.
    pub messages_dispatched: usize,
    /// Whether any command requested a repaint.
    pub repaint_requested: bool,
}

/// Stateful generic runtime controller for message-driven Radiant hosts.
///
/// The controller preserves one-way data flow:
/// 1. project an immutable [`UiSurface`] from host state
/// 2. run public layout on that surface
/// 3. route backend-neutral [`WidgetInput`] into a widget
/// 4. map widget output into a host-defined message
/// 5. reduce that message into host state
/// 6. project the next immutable surface snapshot
pub struct SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    bridge: Bridge,
    viewport: Rect,
    surface: UiSurface<Message>,
    layout: LayoutOutput,
    widget_hit_order: Vec<WidgetId>,
    focused_widget: Option<WidgetId>,
    hovered_widget: Option<WidgetId>,
    pointer_capture: Option<WidgetId>,
    repaint_requested: bool,
}

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    /// Build a generic runtime controller for the provided viewport.
    pub fn new(mut bridge: Bridge, viewport: Vector2) -> Self {
        let viewport = normalized_viewport(viewport);
        let surface = bridge.pull_surface();
        let layout = layout_tree(&surface.layout_node(), viewport);
        let widget_hit_order = surface.widget_paint_order();
        Self {
            bridge,
            viewport,
            surface,
            layout,
            widget_hit_order,
            focused_widget: None,
            hovered_widget: None,
            pointer_capture: None,
            repaint_requested: false,
        }
    }

    /// Return the current projected surface snapshot.
    pub fn surface(&self) -> &UiSurface<Message> {
        &self.surface
    }

    /// Return the current layout output for the projected surface.
    pub fn layout(&self) -> &LayoutOutput {
        &self.layout
    }

    /// Return a borrowed context view of the current runtime state.
    pub fn context(&self) -> RuntimeContext<'_, Message> {
        RuntimeContext {
            viewport: self.viewport,
            surface: &self.surface,
            layout: &self.layout,
        }
    }

    /// Project the current surface and layout into backend-neutral paint data.
    pub fn paint_plan(&self, theme: &ThemeTokens) -> SurfacePaintPlan {
        self.surface.paint_plan(&self.layout, theme)
    }

    /// Return the current logical viewport size.
    pub fn viewport(&self) -> Vector2 {
        Vector2::new(self.viewport.width(), self.viewport.height())
    }

    /// Return the widget that currently owns keyboard focus.
    pub fn focused_widget(&self) -> Option<WidgetId> {
        self.focused_widget
    }

    /// Return the widget that currently owns pointer capture.
    pub fn pointer_capture(&self) -> Option<WidgetId> {
        self.pointer_capture
    }

    /// Return the widget currently receiving hover state.
    pub fn hovered_widget(&self) -> Option<WidgetId> {
        self.hovered_widget
    }

    /// Return whether the host update flow requested another repaint.
    pub fn repaint_requested(&self) -> bool {
        self.repaint_requested
    }

    /// Return and clear the current repaint request flag.
    pub fn take_repaint_requested(&mut self) -> bool {
        let repaint_requested = self.repaint_requested;
        self.repaint_requested = false;
        repaint_requested
    }

    /// Return an immutable reference to the owned bridge.
    pub fn bridge(&self) -> &Bridge {
        &self.bridge
    }

    /// Return a mutable reference to the owned bridge.
    pub fn bridge_mut(&mut self) -> &mut Bridge {
        &mut self.bridge
    }

    /// Consume the runtime controller and return the owned bridge.
    pub fn into_bridge(self) -> Bridge {
        self.bridge
    }

    /// Replace the viewport and recompute layout for the current surface.
    pub fn set_viewport(&mut self, viewport: Vector2) {
        self.viewport = normalized_viewport(viewport);
        self.relayout();
    }

    /// Reproject the latest host state into a fresh immutable surface snapshot.
    pub fn refresh(&mut self) {
        self.surface = self.bridge.pull_surface();
        self.relayout();
        if self
            .focused_widget
            .is_some_and(|widget_id| !self.surface.is_focusable_widget(widget_id))
        {
            self.focused_widget = None;
        }
        if self
            .pointer_capture
            .is_some_and(|widget_id| self.surface.find_widget(widget_id).is_none())
        {
            self.pointer_capture = None;
        }
        if self
            .hovered_widget
            .is_some_and(|widget_id| self.surface.find_widget(widget_id).is_none())
        {
            self.hovered_widget = None;
        }
        if let Some(widget_id) = self.focused_widget {
            self.route_focus_changed(widget_id, true);
        }
    }

    /// Give keyboard focus to one focusable widget.
    ///
    /// Returns `false` when the widget is absent or does not participate in
    /// focus. Focus changes are routed into affected widgets so their retained
    /// interaction state can update before the next paint plan.
    pub fn focus_widget(&mut self, widget_id: WidgetId) -> bool {
        if !self.surface.is_focusable_widget(widget_id) {
            return false;
        }
        if self.focused_widget == Some(widget_id) {
            return true;
        }

        if let Some(previous) = self.focused_widget {
            self.route_focus_changed(previous, false);
        }
        self.focused_widget = Some(widget_id);
        self.route_focus_changed(widget_id, true);
        true
    }

    /// Clear keyboard focus when a surface or backend loses focus ownership.
    pub fn clear_focus(&mut self) {
        if let Some(previous) = self.focused_widget.take() {
            self.route_focus_changed(previous, false);
        }
    }

    /// Move keyboard focus through the current declarative tree.
    ///
    /// Traversal uses stable tree order and wraps at either end. Returns the new
    /// focus target, or `None` when no keyboard-focusable widgets are projected.
    pub fn traverse_focus(&mut self, direction: FocusTraversal) -> Option<WidgetId> {
        let order = self.surface.keyboard_focus_order();
        let next = next_focus_target(self.focused_widget, &order, direction)?;
        self.focus_widget(next).then_some(next)
    }

    /// Route a keyboard interaction to the current focus target.
    ///
    /// Pointer events should continue to use [`SurfaceRuntime::dispatch_input_at`]
    /// or [`SurfaceRuntime::dispatch_input`], because they carry their own hit
    /// target. Keyboard events are resolved through focused widget identity.
    pub fn dispatch_focused_input(&mut self, input: WidgetInput) -> Option<WidgetId> {
        let widget_id = self.focused_widget?;
        self.dispatch_input(widget_id, input).then_some(widget_id)
    }

    /// Route one backend-neutral runtime event.
    ///
    /// Returns the targeted widget id when the event routes to a widget. Events
    /// that only update runtime state, such as resize or focus clearing, return
    /// `None`.
    pub fn dispatch_event(&mut self, event: Event) -> Option<WidgetId> {
        match event {
            Event::Resize { viewport } => {
                self.set_viewport(viewport);
                None
            }
            Event::PointerMove { position } => self.dispatch_pointer_move(position),
            Event::PointerPress { position, button } => {
                let Some(widget_id) = self.widget_at(position) else {
                    self.pointer_capture = None;
                    self.clear_focus();
                    return None;
                };
                self.pointer_capture = Some(widget_id);
                self.dispatch_input_at(position, WidgetInput::PointerPress { position, button })
            }
            Event::PointerRelease { position, button } => {
                let widget_id = self
                    .pointer_capture
                    .take()
                    .or_else(|| self.widget_at(position))?;
                self.dispatch_input(widget_id, WidgetInput::PointerRelease { position, button })
                    .then_some(widget_id)
            }
            Event::KeyPress(key) => self.dispatch_focused_input(WidgetInput::KeyPress(key)),
            Event::Character(character) => {
                self.dispatch_focused_input(WidgetInput::Character(character))
            }
            Event::TraverseFocus(direction) => self.traverse_focus(direction),
            Event::ClearFocus => {
                self.clear_focus();
                None
            }
        }
    }

    /// Reduce one host-defined message and execute its runtime-visible command.
    pub fn dispatch_message(&mut self, message: Message) -> CommandOutcome {
        let mut outcome = CommandOutcome::default();
        self.dispatch_message_inner(message, &mut outcome);
        self.refresh();
        outcome
    }

    /// Execute a command without an initial widget message.
    ///
    /// This is useful for backend adapters or host shells that need to replay a
    /// queued command through the same message/repaint handling path used by
    /// widget dispatch.
    pub fn execute_command(&mut self, command: Command<Message>) -> CommandOutcome {
        let mut outcome = CommandOutcome::default();
        self.execute_command_inner(command, &mut outcome);
        self.refresh();
        outcome
    }

    /// Route one normalized widget interaction by widget id.
    ///
    /// Returns `true` when the interaction targeted a projected widget, even if
    /// that interaction did not emit a host-defined message.
    pub fn dispatch_input(&mut self, widget_id: WidgetId, input: WidgetInput) -> bool {
        let Some(bounds) = self.layout.rects.get(&widget_id).copied() else {
            return false;
        };
        let Some(output) = self.surface.dispatch_widget_input(widget_id, bounds, input) else {
            return self.surface.find_widget(widget_id).is_some();
        };
        if let Some(message) = self.surface.dispatch_widget_output(widget_id, output) {
            self.dispatch_message(message);
        } else {
            self.relayout();
        }
        true
    }

    /// Return the first projected widget whose laid-out bounds contain `point`.
    pub fn widget_at(&self, point: Point) -> Option<WidgetId> {
        self.widget_hit_order
            .iter()
            .rev()
            .copied()
            .find(|widget_id| {
                self.layout
                    .rects
                    .get(widget_id)
                    .is_some_and(|rect| rect.contains(point))
            })
    }

    /// Route one normalized widget interaction by point hit test.
    ///
    /// Returns the targeted widget id when a projected widget handled the point.
    pub fn dispatch_input_at(&mut self, point: Point, input: WidgetInput) -> Option<WidgetId> {
        let widget_id = self.widget_at(point)?;
        if matches!(input, WidgetInput::PointerPress { .. }) {
            let _ = self.focus_widget(widget_id);
        }
        self.dispatch_input(widget_id, input).then_some(widget_id)
    }

    fn dispatch_pointer_move(&mut self, position: Point) -> Option<WidgetId> {
        let pointer_widget = self.widget_at(position);
        let hover_widget = self
            .pointer_capture
            .filter(|widget_id| {
                self.layout
                    .rects
                    .get(widget_id)
                    .is_some_and(|rect| rect.contains(position))
            })
            .or_else(|| {
                self.pointer_capture
                    .is_none()
                    .then_some(pointer_widget)
                    .flatten()
            });
        if self.hovered_widget != hover_widget {
            if let Some(previous) = self.hovered_widget {
                let _ = self.dispatch_input(previous, WidgetInput::PointerMove { position });
            }
            self.hovered_widget = hover_widget;
        }

        let target = self.pointer_capture.or(pointer_widget)?;
        self.dispatch_input(target, WidgetInput::PointerMove { position })
            .then_some(target)
    }

    fn relayout(&mut self) {
        self.layout = layout_tree(&self.surface.layout_node(), self.viewport);
        self.widget_hit_order = self.surface.widget_paint_order();
    }

    fn route_focus_changed(&mut self, widget_id: WidgetId, focused: bool) {
        let Some(bounds) = self.layout.rects.get(&widget_id).copied() else {
            return;
        };
        let _ = self.surface.dispatch_widget_input(
            widget_id,
            bounds,
            WidgetInput::FocusChanged(focused),
        );
    }

    fn dispatch_message_inner(&mut self, message: Message, outcome: &mut CommandOutcome) {
        outcome.messages_dispatched += 1;
        let command = self.bridge.update(message);
        self.execute_command_inner(command, outcome);
    }

    fn execute_command_inner(&mut self, command: Command<Message>, outcome: &mut CommandOutcome) {
        match command {
            Command::None => {}
            Command::Message(message) => self.dispatch_message_inner(message, outcome),
            Command::Batch(commands) => {
                for command in commands {
                    self.execute_command_inner(command, outcome);
                }
            }
            Command::RequestRepaint => {
                self.repaint_requested = true;
                outcome.repaint_requested = true;
            }
        }
    }
}

fn normalized_viewport(viewport: Vector2) -> Rect {
    Rect::from_min_size(
        Point::new(0.0, 0.0),
        Vector2::new(viewport.x.max(1.0), viewport.y.max(1.0)),
    )
}

fn next_focus_target(
    current: Option<WidgetId>,
    order: &[WidgetId],
    direction: FocusTraversal,
) -> Option<WidgetId> {
    if order.is_empty() {
        return None;
    }
    let current_index = current.and_then(|widget_id| order.iter().position(|id| *id == widget_id));
    let next_index = match (current_index, direction) {
        (Some(index), FocusTraversal::Forward) => (index + 1) % order.len(),
        (Some(0), FocusTraversal::Backward) => order.len() - 1,
        (Some(index), FocusTraversal::Backward) => index - 1,
        (None, FocusTraversal::Forward) => 0,
        (None, FocusTraversal::Backward) => order.len() - 1,
    };
    Some(order[next_index])
}
