use crate::widgets::{FocusBehavior, PaintBounds, WidgetProminence, WidgetSizing, WidgetStyle};

#[path = "builder/defaults.rs"]
mod defaults;
#[path = "builder/drag_drop.rs"]
mod drag_drop;
#[path = "builder/messages.rs"]
mod messages;
#[path = "builder/policy.rs"]
mod policy;
#[path = "builder/underlay/mod.rs"]
mod underlay;
#[path = "builder/widget.rs"]
mod widget;

pub use defaults::{interactive_row, row_actions};
pub use policy::DenseRowPolicy;
pub use underlay::{InteractiveRowUnderlayBuilder, interactive_row_underlay};

/// Builder for selectable, draggable, droppable dense rows.
pub struct InteractiveRowBuilder {
    style: Option<WidgetStyle>,
    sizing: WidgetSizing,
    focus: Option<FocusBehavior>,
    paint_bounds: Option<PaintBounds>,
    paints_focus: Option<bool>,
    paints_state_layers: Option<bool>,
    draggable: bool,
    droppable: bool,
    drop_hover: bool,
    clear_drop_on_hover: bool,
    drag_active: bool,
    drag_source: bool,
    drag_source_motion: bool,
    suppress_hover: bool,
    hover_messages: bool,
    clear_hover_on_sync: bool,
    activation_modifiers: bool,
    pointer_motion_during_interaction: bool,
    pointer_motion_active: bool,
}

impl InteractiveRowBuilder {
    /// Apply an explicit widget style before binding this row.
    pub fn style(mut self, style: WidgetStyle) -> Self {
        self.style = Some(style);
        self
    }

    /// Use a lower-prominence treatment.
    pub fn subtle(mut self) -> Self {
        let mut style = self.style.unwrap_or_default();
        style.prominence = WidgetProminence::Subtle;
        self.style = Some(style);
        self
    }

    /// Override the row widget sizing.
    pub fn sizing(mut self, sizing: WidgetSizing) -> Self {
        self.sizing = sizing;
        self
    }

    /// Override keyboard focus behavior.
    pub fn focus(mut self, focus: FocusBehavior) -> Self {
        self.focus = Some(focus);
        self
    }

    /// Override how this row's paint is bounded.
    pub fn paint_bounds(mut self, bounds: PaintBounds) -> Self {
        self.paint_bounds = Some(bounds);
        self
    }

    /// Control whether this row paints focus affordances.
    pub fn paint_focus(mut self, paint: bool) -> Self {
        self.paints_focus = Some(paint);
        self
    }

    /// Control whether this row paints its built-in hover and pressed layers.
    pub fn paint_state_layers(mut self, paint: bool) -> Self {
        self.paints_state_layers = Some(paint);
        self
    }

    /// Configure this row as an input-only layer for app-owned custom painting.
    ///
    /// The row still routes pointer, keyboard, drag, and drop interactions, but
    /// it does not request keyboard focus or paint Radiant's built-in focus and
    /// hover/pressed layers. Custom composite widgets can use this preset when
    /// they want generic row input behavior with their own visual state model.
    pub fn custom_paint_hit_target(mut self) -> Self {
        self.focus = Some(FocusBehavior::None);
        self.paint_bounds = Some(PaintBounds::ClipToRect);
        self.paints_focus = Some(false);
        self.paints_state_layers = Some(false);
        self
    }

    /// Ignore hover updates for this row while preserving activation and drag behavior.
    pub fn suppress_hover(mut self, suppress: bool) -> Self {
        self.suppress_hover = suppress;
        self
    }

    /// Emit host messages when pointer hover moves over this row.
    pub fn hover_messages(mut self, enabled: bool) -> Self {
        self.hover_messages = enabled;
        self
    }

    /// Clear retained hover state when this row is synchronized from a previous tree.
    pub fn clear_hover_on_sync(mut self) -> Self {
        self.clear_hover_on_sync = true;
        self
    }

    /// Include primary-release modifier state in pointer activation messages.
    pub fn activation_modifiers(mut self) -> Self {
        self.activation_modifiers = true;
        self
    }

    /// Restrict pointer-motion routing to active row interactions.
    pub fn pointer_motion_during_interaction(mut self) -> Self {
        self.pointer_motion_during_interaction = true;
        self
    }

    /// Mark app-owned interaction state that should keep pointer motion routed.
    pub fn pointer_motion_active(mut self, active: bool) -> Self {
        self.pointer_motion_active = active;
        self
    }
}
