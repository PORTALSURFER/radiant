use crate::{
    application::StateCallback,
    gui::types::{Point, Rect},
    layout::Vector2,
    widgets::{WidgetProminence, WidgetStyle, WidgetTone},
};
use std::sync::Arc;

/// One clickable item in a generic menu or context menu.
pub struct MenuItem<State> {
    pub(crate) label: String,
    pub(crate) style: WidgetStyle,
    pub(crate) on_select: StateCallback<State>,
}

/// Named construction fields for a [`MenuItem`].
pub struct MenuItemParts<State> {
    /// Visible menu-item label.
    pub label: String,
    /// Visual styling applied to the backing button.
    pub style: WidgetStyle,
    /// State callback invoked when the item is activated.
    pub on_select: Arc<dyn Fn(&mut State) + Send + Sync>,
}

/// One clickable item in a generic message-emitting menu.
#[derive(Clone, Debug, PartialEq)]
pub struct MenuCommand<Message> {
    pub(crate) label: String,
    pub(crate) style: WidgetStyle,
    pub(crate) message: Message,
}

/// Named construction fields for a [`MenuCommand`].
#[derive(Clone, Debug, PartialEq)]
pub struct MenuCommandParts<Message> {
    /// Visible menu-item label.
    pub label: String,
    /// Visual styling applied to the backing button.
    pub style: WidgetStyle,
    /// Host message emitted when the item is activated.
    pub message: Message,
}

impl<State> MenuItem<State> {
    /// Build a menu item from named parts.
    pub fn from_parts(parts: MenuItemParts<State>) -> Self {
        Self {
            label: parts.label,
            style: parts.style,
            on_select: parts.on_select,
        }
    }

    /// Build a menu item that runs the supplied state callback when activated.
    pub fn new(
        label: impl Into<String>,
        on_select: impl Fn(&mut State) + Send + Sync + 'static,
    ) -> Self {
        Self::from_parts(MenuItemParts {
            label: label.into(),
            style: WidgetStyle::default(),
            on_select: Arc::new(on_select),
        })
    }

    /// Use accent styling for a primary menu action.
    pub fn primary(mut self) -> Self {
        self.style = WidgetStyle {
            tone: WidgetTone::Accent,
            prominence: WidgetProminence::Strong,
        };
        self
    }

    /// Use danger styling for a destructive menu action.
    pub fn danger(mut self) -> Self {
        self.style = WidgetStyle {
            tone: WidgetTone::Danger,
            prominence: WidgetProminence::Strong,
        };
        self
    }

    /// Use lower-prominence styling for a secondary menu action.
    pub fn subtle(mut self) -> Self {
        self.style.prominence = WidgetProminence::Subtle;
        self
    }
}

impl<Message> MenuCommand<Message> {
    /// Build a menu command from named parts.
    pub fn from_parts(parts: MenuCommandParts<Message>) -> Self {
        Self {
            label: parts.label,
            style: parts.style,
            message: parts.message,
        }
    }

    /// Build a menu command that emits the supplied host message when activated.
    pub fn new(label: impl Into<String>, message: Message) -> Self {
        Self::from_parts(MenuCommandParts {
            label: label.into(),
            style: WidgetStyle::default(),
            message,
        })
    }

    /// Use accent styling for a primary menu action.
    pub fn primary(mut self) -> Self {
        self.style = WidgetStyle {
            tone: WidgetTone::Accent,
            prominence: WidgetProminence::Strong,
        };
        self
    }

    /// Use danger styling for a destructive menu action.
    pub fn danger(mut self) -> Self {
        self.style = WidgetStyle {
            tone: WidgetTone::Danger,
            prominence: WidgetProminence::Strong,
        };
        self
    }

    /// Use lower-prominence styling for a secondary menu action.
    pub fn subtle(mut self) -> Self {
        self.style.prominence = WidgetProminence::Subtle;
        self
    }
}

/// Named construction fields for a compact vertical menu.
pub struct MenuParts<State> {
    /// Menu title shown above the action list.
    pub title: String,
    /// Ordered clickable menu items.
    pub items: Vec<MenuItem<State>>,
}

/// Named construction fields for a compact message-emitting menu.
pub struct MessageMenuParts<Message> {
    /// Menu title shown above the action list.
    pub title: String,
    /// Visual styling applied to the menu surface.
    pub style: WidgetStyle,
    /// Ordered clickable menu commands.
    pub commands: Vec<MenuCommand<Message>>,
}

/// Named construction fields for an anchored context-menu overlay.
pub struct ContextMenuOverlayParts<State> {
    /// Bounds of the surface that owns the overlay.
    pub bounds: Rect,
    /// Requested anchor point in surface coordinates.
    pub anchor: Point,
    /// Desired menu size.
    pub size: Vector2,
    /// Menu title shown above the action list.
    pub title: String,
    /// Ordered clickable menu items.
    pub items: Vec<MenuItem<State>>,
}

/// Named construction fields for a full-surface dismissible context-menu layer.
pub struct DismissibleContextMenuParts<Message> {
    /// Requested anchor point in surface coordinates.
    pub anchor: Point,
    /// Desired menu size.
    pub size: Vector2,
    /// Menu title shown above the action list.
    pub title: String,
    /// Visual styling applied to the menu surface.
    pub style: WidgetStyle,
    /// Ordered clickable menu commands.
    pub commands: Vec<MenuCommand<Message>>,
    /// Message emitted when the user activates the backing dismiss layer.
    pub dismiss_message: Message,
}
