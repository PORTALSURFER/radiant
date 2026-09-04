//! Platform-neutral shortcut display values.

use super::{ShortcutGesture, ShortcutModifier};
use std::sync::Arc;

/// Platform family used when presenting a semantic shortcut.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub enum ShortcutPlatform {
    /// Apple command presentation.
    #[default]
    Mac,
    /// Windows control presentation.
    Windows,
    /// Linux and other control-based desktop presentation.
    Other,
}

/// Caller-supplied semantic label for the primary shortcut key.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum ShortcutKeyLabel {
    /// A semantic character supplied by the caller.
    Character(char),
    /// A named key such as `Escape` or `Page Up` supplied by the caller.
    Named(Arc<str>),
}

impl ShortcutKeyLabel {
    /// Construct a semantic character label.
    pub const fn character(value: char) -> Self {
        Self::Character(value)
    }

    /// Construct a named key label.
    pub fn named(value: impl AsRef<str>) -> Self {
        Self::Named(Arc::from(value.as_ref()))
    }

    fn as_str(&self) -> Arc<str> {
        match self {
            Self::Character(value) => Arc::from(value.to_string()),
            Self::Named(value) => Arc::clone(value),
        }
    }
}

/// A shortcut gesture paired with its semantic display key.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShortcutDisplaySpec {
    /// Matching gesture, retained unchanged from shortcut routing.
    pub gesture: ShortcutGesture,
    /// Caller-owned semantic key label.
    pub key_label: ShortcutKeyLabel,
}

impl ShortcutDisplaySpec {
    /// Pair a gesture with a semantic character or named-key label.
    pub const fn new(gesture: ShortcutGesture, key_label: ShortcutKeyLabel) -> Self {
        Self { gesture, key_label }
    }
}

/// Both compact menu text and spoken/help text for one shortcut.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ShortcutPresentation {
    compact: Arc<str>,
    spoken: Arc<str>,
}

impl ShortcutPresentation {
    /// Return compact menu/toolbar text.
    pub fn compact_text(&self) -> &str {
        &self.compact
    }

    /// Return the expanded spoken/help text.
    pub fn spoken_text(&self) -> &str {
        &self.spoken
    }
}

/// Formats semantic shortcut values for one platform family.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ShortcutPresenter {
    platform: ShortcutPlatform,
}

impl ShortcutPresenter {
    /// Construct a presenter for a platform family.
    pub const fn new(platform: ShortcutPlatform) -> Self {
        Self { platform }
    }

    /// Format a shortcut without inspecting its physical key code.
    pub fn present(&self, spec: &ShortcutDisplaySpec) -> ShortcutPresentation {
        let compact = self.compact(spec);
        let spoken = self.spoken(spec);
        ShortcutPresentation {
            compact: Arc::from(compact),
            spoken: Arc::from(spoken),
        }
    }

    fn compact(&self, spec: &ShortcutDisplaySpec) -> String {
        let mut result = String::new();
        self.push_compact_modifier(&mut result, "command", spec.gesture.command);
        self.push_compact_modifier(&mut result, "control", spec.gesture.control);
        self.push_compact_modifier(&mut result, "shift", spec.gesture.shift);
        self.push_compact_modifier(&mut result, "alt", spec.gesture.alt);
        result.push_str(&spec.key_label.as_str());
        result
    }

    fn spoken(&self, spec: &ShortcutDisplaySpec) -> String {
        let mut parts = Vec::new();
        Self::push_spoken_modifier(&mut parts, "Command", spec.gesture.command);
        Self::push_spoken_modifier(&mut parts, "Control", spec.gesture.control);
        Self::push_spoken_modifier(&mut parts, "Shift", spec.gesture.shift);
        Self::push_spoken_modifier(&mut parts, "Option", spec.gesture.alt);
        parts.push(spec.key_label.as_str().to_string());
        parts.join("+")
    }

    fn push_compact_modifier(&self, output: &mut String, name: &str, modifier: ShortcutModifier) {
        let Some(label) = self.compact_modifier(name, modifier) else {
            return;
        };
        output.push_str(label);
    }

    fn compact_modifier(&self, name: &str, modifier: ShortcutModifier) -> Option<&'static str> {
        if modifier == ShortcutModifier::Off {
            return None;
        }
        let label = match (self.platform, name, modifier) {
            (_, "command", ShortcutModifier::Any) => "Any+",
            (_, "control", ShortcutModifier::Any) => "Any+",
            (_, "shift", ShortcutModifier::Any) => "Any+",
            (_, "alt", ShortcutModifier::Any) => "Any+",
            (ShortcutPlatform::Mac, "command", _) => "⌘",
            (ShortcutPlatform::Mac, "control", _) => "⌃",
            (ShortcutPlatform::Mac, "shift", _) => "⇧",
            (ShortcutPlatform::Mac, "alt", _) => "⌥",
            (_, "command", _) => "Ctrl+",
            (_, "control", _) => "Control+",
            (_, "shift", _) => "Shift+",
            (_, "alt", _) => "Alt+",
            _ => "",
        };
        Some(label)
    }

    fn push_spoken_modifier(parts: &mut Vec<String>, label: &str, modifier: ShortcutModifier) {
        match modifier {
            ShortcutModifier::Off => {}
            ShortcutModifier::On => parts.push(label.to_owned()),
            ShortcutModifier::Any => parts.push(format!("Any {label}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ShortcutDisplaySpec, ShortcutKeyLabel, ShortcutPresenter};
    use crate::{
        gui::shortcuts::ShortcutPlatform,
        gui::{input::KeyCode, shortcuts::ShortcutGesture},
    };

    #[test]
    fn presenter_uses_semantic_labels_and_platform_modifier_names() {
        let spec = ShortcutDisplaySpec::new(
            ShortcutGesture::with_command(KeyCode::S),
            ShortcutKeyLabel::character('S'),
        );
        let mac = ShortcutPresenter::new(ShortcutPlatform::Mac).present(&spec);
        assert_eq!(mac.compact_text(), "⌘S");
        assert_eq!(mac.spoken_text(), "Command+S");

        let windows = ShortcutPresenter::new(ShortcutPlatform::Windows).present(&spec);
        assert_eq!(windows.compact_text(), "Ctrl+S");
    }

    #[test]
    fn physical_control_and_any_are_explicit() {
        let spec = ShortcutDisplaySpec::new(
            ShortcutGesture {
                key: KeyCode::Escape,
                command: super::ShortcutModifier::Off,
                control: super::ShortcutModifier::On,
                shift: super::ShortcutModifier::Any,
                alt: super::ShortcutModifier::Off,
            },
            ShortcutKeyLabel::named("Escape"),
        );
        let presentation = ShortcutPresenter::new(ShortcutPlatform::Mac).present(&spec);
        assert_eq!(presentation.compact_text(), "⌃Any+Escape");
        assert_eq!(presentation.spoken_text(), "Control+Any Shift+Escape");
    }
}
