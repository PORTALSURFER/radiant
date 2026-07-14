use crate::{
    application::TextContent,
    gui::text_layout::{TextWidthEstimate, estimated_text_width_for_char_count_in_range},
    widgets::{WidgetProminence, WidgetStyle, WidgetTone},
};

const COMPACT_MENU_AVERAGE_ADVANCE_FACTOR: f32 = 0.62;
const COMPACT_MENU_FONT_SIZE: f32 = 14.0;
const COMPACT_MENU_HORIZONTAL_TEXT_PADDING: f32 = 48.0;
const COMPACT_MENU_HOTKEY_GAP_CHARS: usize = 3;
const COMPACT_MENU_MIN_WIDTH: f32 = 210.0;
const COMPACT_MENU_MAX_WIDTH: f32 = 380.0;

/// One clickable item in a generic message-emitting menu.
#[derive(Clone, Debug, PartialEq)]
pub struct MenuCommand<Message> {
    pub(crate) label: TextContent,
    pub(crate) hotkey_hint: Option<TextContent>,
    pub(crate) style: WidgetStyle,
    pub(crate) message: Message,
}

/// Named construction fields for a [`MenuCommand`].
#[derive(Clone, Debug, PartialEq)]
pub struct MenuCommandParts<Message> {
    /// Visible menu-item label.
    pub label: TextContent,
    /// Optional trailing shortcut or hotkey hint.
    pub hotkey_hint: Option<TextContent>,
    /// Visual styling applied to the backing button.
    pub style: WidgetStyle,
    /// Host message emitted when the item is activated.
    pub message: Message,
}

impl<Message> MenuCommand<Message> {
    /// Build a menu command from named parts.
    pub fn from_parts(parts: MenuCommandParts<Message>) -> Self {
        Self {
            label: parts.label,
            hotkey_hint: parts.hotkey_hint.filter(|hint| !hint.is_empty()),
            style: parts.style,
            message: parts.message,
        }
    }

    /// Build a menu command that emits the supplied host message when activated.
    pub fn new(label: impl Into<TextContent>, message: Message) -> Self {
        Self::from_parts(MenuCommandParts {
            label: label.into(),
            hotkey_hint: None,
            style: WidgetStyle::default(),
            message,
        })
    }

    /// Render a trailing shortcut or hotkey hint for this command.
    pub fn hotkey_hint(mut self, hint: impl Into<TextContent>) -> Self {
        let hint = hint.into();
        self.hotkey_hint = (!hint.is_empty()).then_some(hint);
        self
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

/// Internal construction fields for a compact message-emitting menu.
pub(super) struct MessageMenuParts<Message> {
    /// Menu title shown above the action list.
    pub(super) title: TextContent,
    /// Visual styling applied to the menu surface.
    pub(super) style: WidgetStyle,
    /// Ordered clickable menu commands.
    pub(super) commands: Vec<MenuCommand<Message>>,
}

/// Deterministic width policy for message-emitting menus.
///
/// This is intended for menu and context-menu layout decisions that must be
/// made before renderer text shaping metrics are available. It sizes from the
/// longest visible title or command label, then clamps to a caller-defined
/// logical-width range.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MessageMenuWidthPolicy {
    /// Approximate text-width inputs used before renderer shaping metrics exist.
    pub metrics: TextWidthEstimate,
    /// Minimum logical width reserved for the menu.
    pub min_width: f32,
    /// Maximum logical width reserved for the menu.
    pub max_width: f32,
}

impl MessageMenuWidthPolicy {
    /// Construct a menu width policy from explicit metrics and width bounds.
    pub fn new(metrics: TextWidthEstimate, min_width: f32, max_width: f32) -> Self {
        Self {
            metrics,
            min_width,
            max_width,
        }
    }

    /// Return Radiant's default compact-menu width policy.
    pub fn compact() -> Self {
        Self::new(
            TextWidthEstimate::from_font_size(
                COMPACT_MENU_FONT_SIZE,
                COMPACT_MENU_AVERAGE_ADVANCE_FACTOR,
                COMPACT_MENU_HORIZONTAL_TEXT_PADDING,
            ),
            COMPACT_MENU_MIN_WIDTH,
            COMPACT_MENU_MAX_WIDTH,
        )
    }

    /// Approximate menu width for a title and ordered command list.
    pub fn width_for_title_and_commands<Message>(
        self,
        title: &str,
        commands: &[MenuCommand<Message>],
    ) -> f32 {
        let title_chars = title.chars().count();
        let command_chars = commands
            .iter()
            .map(command_menu_width_chars)
            .max()
            .unwrap_or(0);
        estimated_text_width_for_char_count_in_range(
            title_chars.max(command_chars),
            self.metrics,
            self.min_width,
            self.max_width,
        )
    }
}

fn command_menu_width_chars<Message>(command: &MenuCommand<Message>) -> usize {
    let label_chars = command.label.chars().count();
    let Some(hint) = command.hotkey_hint.as_ref() else {
        return label_chars;
    };
    label_chars + COMPACT_MENU_HOTKEY_GAP_CHARS + hint.chars().count()
}
