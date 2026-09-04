//! Platform-neutral shortcut display values.

use super::{ShortcutGesture, ShortcutModifier};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DisplayModifierKind {
    Command,
    Control,
    Shift,
    Option,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct DisplayModifier {
    kind: DisplayModifierKind,
    state: ShortcutModifier,
}

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
        for modifier in self.display_modifiers(spec.gesture) {
            let label = match (self.platform, modifier.kind, modifier.state) {
                (_, _, ShortcutModifier::Any) => "Any+",
                (ShortcutPlatform::Mac, DisplayModifierKind::Command, _) => "⌘",
                (ShortcutPlatform::Mac, DisplayModifierKind::Control, _) => "⌃",
                (ShortcutPlatform::Mac, DisplayModifierKind::Shift, _) => "⇧",
                (ShortcutPlatform::Mac, DisplayModifierKind::Option, _) => "⌥",
                (_, DisplayModifierKind::Command | DisplayModifierKind::Control, _) => "Ctrl+",
                (_, DisplayModifierKind::Shift, _) => "Shift+",
                (_, DisplayModifierKind::Option, _) => "Alt+",
            };
            result.push_str(label);
        }
        result.push_str(&spec.key_label.as_str());
        result
    }

    fn spoken(&self, spec: &ShortcutDisplaySpec) -> String {
        let mut parts = self
            .display_modifiers(spec.gesture)
            .into_iter()
            .map(|modifier| {
                let label = match (self.platform, modifier.kind) {
                    (ShortcutPlatform::Mac, DisplayModifierKind::Command) => "Command",
                    (_, DisplayModifierKind::Command | DisplayModifierKind::Control) => "Control",
                    (_, DisplayModifierKind::Shift) => "Shift",
                    (ShortcutPlatform::Mac, DisplayModifierKind::Option) => "Option",
                    (_, DisplayModifierKind::Option) => "Alt",
                };
                match modifier.state {
                    ShortcutModifier::On => label.to_owned(),
                    ShortcutModifier::Any => format!("Any {label}"),
                    ShortcutModifier::Off => unreachable!("off modifiers are filtered"),
                }
            })
            .collect::<Vec<_>>();
        parts.push(spec.key_label.as_str().to_string());
        parts.join("+")
    }

    fn display_modifiers(&self, gesture: ShortcutGesture) -> Vec<DisplayModifier> {
        let mut modifiers = Vec::with_capacity(4);
        let entries = [
            (DisplayModifierKind::Command, gesture.command),
            (DisplayModifierKind::Control, gesture.control),
            (DisplayModifierKind::Shift, gesture.shift),
            (DisplayModifierKind::Option, gesture.alt),
        ];
        for (kind, state) in entries {
            if state == ShortcutModifier::Off {
                continue;
            }
            let alias = if self.platform == ShortcutPlatform::Mac {
                kind
            } else {
                match kind {
                    DisplayModifierKind::Command | DisplayModifierKind::Control => {
                        DisplayModifierKind::Control
                    }
                    kind => kind,
                }
            };
            if let Some(existing) = modifiers
                .iter_mut()
                .find(|modifier: &&mut DisplayModifier| modifier.kind == alias)
            {
                existing.state = match (existing.state, state) {
                    (ShortcutModifier::Any, ShortcutModifier::On)
                    | (ShortcutModifier::On, ShortcutModifier::Any) => ShortcutModifier::On,
                    (ShortcutModifier::Any, ShortcutModifier::Any) => ShortcutModifier::Any,
                    (state, _) => state,
                };
            } else {
                modifiers.push(DisplayModifier { kind: alias, state });
            }
        }
        modifiers
    }
}

#[cfg(test)]
mod tests {
    use super::{ShortcutDisplaySpec, ShortcutKeyLabel, ShortcutModifier, ShortcutPresenter};
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
        assert_eq!(windows.spoken_text(), "Control+S");

        let other = ShortcutPresenter::new(ShortcutPlatform::Other).present(&spec);
        assert_eq!(other.compact_text(), "Ctrl+S");
        assert_eq!(other.spoken_text(), "Control+S");
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

    #[test]
    fn non_mac_command_and_physical_control_share_one_ordered_alias() {
        let spec = ShortcutDisplaySpec::new(
            ShortcutGesture {
                key: KeyCode::S,
                command: ShortcutModifier::On,
                control: ShortcutModifier::Any,
                shift: ShortcutModifier::On,
                alt: ShortcutModifier::On,
            },
            ShortcutKeyLabel::character('S'),
        );

        for platform in [ShortcutPlatform::Windows, ShortcutPlatform::Other] {
            let presentation = ShortcutPresenter::new(platform).present(&spec);
            assert_eq!(presentation.compact_text(), "Ctrl+Shift+Alt+S");
            assert_eq!(presentation.spoken_text(), "Control+Shift+Alt+S");
        }
    }

    #[test]
    fn non_mac_alias_combination_preserves_any_only_when_both_are_any() {
        let both_any = ShortcutDisplaySpec::new(
            ShortcutGesture {
                key: KeyCode::S,
                command: ShortcutModifier::Any,
                control: ShortcutModifier::Any,
                shift: ShortcutModifier::Off,
                alt: ShortcutModifier::Off,
            },
            ShortcutKeyLabel::character('S'),
        );
        let command_any = ShortcutDisplaySpec::new(
            ShortcutGesture {
                key: KeyCode::S,
                command: ShortcutModifier::Any,
                control: ShortcutModifier::On,
                shift: ShortcutModifier::Off,
                alt: ShortcutModifier::Off,
            },
            ShortcutKeyLabel::character('S'),
        );
        let both_on = ShortcutDisplaySpec::new(
            ShortcutGesture {
                key: KeyCode::S,
                command: ShortcutModifier::On,
                control: ShortcutModifier::On,
                shift: ShortcutModifier::Off,
                alt: ShortcutModifier::Off,
            },
            ShortcutKeyLabel::character('S'),
        );

        for platform in [ShortcutPlatform::Windows, ShortcutPlatform::Other] {
            let presenter = ShortcutPresenter::new(platform);
            let presentation = presenter.present(&both_any);
            assert_eq!(presentation.compact_text(), "Any+S");
            assert_eq!(presentation.spoken_text(), "Any Control+S");

            let presentation = presenter.present(&command_any);
            assert_eq!(presentation.compact_text(), "Ctrl+S");
            assert_eq!(presentation.spoken_text(), "Control+S");

            let presentation = presenter.present(&both_on);
            assert_eq!(presentation.compact_text(), "Ctrl+S");
            assert_eq!(presentation.spoken_text(), "Control+S");
        }
    }

    #[test]
    fn mac_keeps_command_and_physical_control_distinct() {
        let spec = ShortcutDisplaySpec::new(
            ShortcutGesture {
                key: KeyCode::S,
                command: ShortcutModifier::On,
                control: ShortcutModifier::On,
                shift: ShortcutModifier::Off,
                alt: ShortcutModifier::Off,
            },
            ShortcutKeyLabel::character('S'),
        );
        let presentation = ShortcutPresenter::new(ShortcutPlatform::Mac).present(&spec);
        assert_eq!(presentation.compact_text(), "⌘⌃S");
        assert_eq!(presentation.spoken_text(), "Command+Control+S");
    }
}
