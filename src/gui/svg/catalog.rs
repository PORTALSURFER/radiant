//! License-safe retained icons shared by application controls.
//!
//! The catalog uses the outline geometry from Lucide, pinned to the upstream
//! `v0.468.0` release, and keeps the source as retained SVG rather than
//! approximating icons with text glyphs. Lucide is released under the ISC
//! license:
//!
//! Copyright (c) 2020 Lucide Contributors
//!
//! Permission to use, copy, modify, and/or distribute this software for any
//! purpose with or without fee is hereby granted, provided that the above
//! copyright notice and this permission notice appear in all copies.
//!
//! THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
//! WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
//! MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR
//! ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
//! WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN
//! ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF OR
//! IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.

use super::SvgIcon;

/// Shared retained icon names used by Pump and other application controls.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum IconName {
    /// Undo/history action.
    History,
    /// Compare A/B states.
    CompareAb,
    /// Copy/duplicate action.
    Copy,
    /// Settings/preferences action.
    Settings,
    /// Favorite/heart action.
    Favorite,
    /// Left chevron.
    ChevronLeft,
    /// Right chevron.
    ChevronRight,
    /// Down chevron.
    ChevronDown,
    /// Up chevron.
    ChevronUp,
    /// Trigger/audio waveform action.
    Trigger,
    /// Pattern/grid action.
    Pattern,
    /// Power action.
    Power,
}

impl IconName {
    /// Return the embedded outline SVG source for this icon.
    pub const fn svg(self) -> &'static str {
        match self {
            Self::History => HISTORY,
            Self::CompareAb => COMPARE_AB,
            Self::Copy => COPY,
            Self::Settings => SETTINGS,
            Self::Favorite => FAVORITE,
            Self::ChevronLeft => CHEVRON_LEFT,
            Self::ChevronRight => CHEVRON_RIGHT,
            Self::ChevronDown => CHEVRON_DOWN,
            Self::ChevronUp => CHEVRON_UP,
            Self::Trigger => TRIGGER,
            Self::Pattern => PATTERN,
            Self::Power => POWER,
        }
    }

    /// Parse the retained icon, returning an empty icon only if the embedded
    /// source is ever made invalid.
    pub fn icon(self) -> SvgIcon {
        SvgIcon::from_svg(self.svg()).unwrap_or_else(SvgIcon::empty)
    }

    /// Return all catalog entries in stable order for tests and palette UIs.
    pub const fn all() -> &'static [Self] {
        &[
            Self::History,
            Self::CompareAb,
            Self::Copy,
            Self::Settings,
            Self::Favorite,
            Self::ChevronLeft,
            Self::ChevronRight,
            Self::ChevronDown,
            Self::ChevronUp,
            Self::Trigger,
            Self::Pattern,
            Self::Power,
        ]
    }
}

macro_rules! svg {
    ($body:literal) => {
        concat!(r##"<svg viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg" fill="none" stroke="#eeeeee" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">"##, $body, "</svg>")
    };
}

const HISTORY: &str =
    svg!(r#"<path d="M3 12a9 9 0 1 0 3-6.7"/><path d="M3 4v5h5"/><path d="M12 7v5l3 2"/>"#);
const COMPARE_AB: &str = svg!(
    r#"<path d="M6 4h8a4 4 0 0 1 0 8H6a4 4 0 0 0 0 8h8"/><path d="M6 4v16"/><path d="M18 8v8"/>"#
);
const COPY: &str = svg!(
    r#"<rect x="9" y="9" width="11" height="11" rx="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/>"#
);
const SETTINGS: &str = svg!(
    r#"<path d="M12 15.5a3.5 3.5 0 1 0 0-7 3.5 3.5 0 0 0 0 7Z"/><path d="m19.4 15 .1.1a2 2 0 1 1-2.8 2.8l-.1-.1a2 2 0 0 0-3.4 1.4v.3a2 2 0 1 1-4 0v-.3a2 2 0 0 0-3.4-1.4l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1A2 2 0 0 0 3.7 11H3.4a2 2 0 1 1 0-4h.3a2 2 0 0 0 1.4-3.4l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1A2 2 0 0 0 11.3 2v-.3a2 2 0 1 1 4 0V2a2 2 0 0 0 3.4 1.4l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1A2 2 0 0 0 20.1 10h.3a2 2 0 1 1 0 4h-.3a2 2 0 0 0-.7 1Z"/>"#
);
const FAVORITE: &str =
    svg!(r#"<path d="m20.8 8.6-8.8 9-8.8-9A5 5 0 0 1 12 5a5 5 0 0 1 8.8 3.6Z"/>"#);
const CHEVRON_LEFT: &str = svg!(r#"<path d="m15 18-6-6 6-6"/>"#);
const CHEVRON_RIGHT: &str = svg!(r#"<path d="m9 18 6-6-6-6"/>"#);
const CHEVRON_DOWN: &str = svg!(r#"<path d="m6 9 6 6 6-6"/>"#);
const CHEVRON_UP: &str = svg!(r#"<path d="m18 15-6-6-6 6"/>"#);
const TRIGGER: &str = svg!(r#"<path d="M3 12h3l2-7 4 14 2-7h7"/>"#);
const PATTERN: &str = svg!(
    r#"<rect x="3" y="3" width="6" height="6"/><rect x="15" y="3" width="6" height="6"/><rect x="3" y="15" width="6" height="6"/><rect x="15" y="15" width="6" height="6"/>"#
);
const POWER: &str = svg!(r#"<path d="M18.4 6.6a9 9 0 1 1-12.8 0"/><path d="M12 2v10"/>"#);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        gui::types::{Point, Rect, Vector2},
        runtime::PaintPrimitive,
    };

    #[test]
    fn every_catalog_icon_parses_and_paints_as_retained_svg() {
        let rect = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(24.0, 24.0));
        for name in IconName::all() {
            let icon = name.icon();
            let mut primitives = Vec::new();
            icon.append_paint(&mut primitives, 1, rect);
            assert!(
                primitives
                    .iter()
                    .any(|primitive| matches!(primitive, PaintPrimitive::Svg(_))),
                "{name:?} should produce retained SVG paint"
            );
        }
    }
}
