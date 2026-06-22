use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
};

use crate::gui::types::{Rect, Rgba8};
use crate::runtime::{PaintPrimitive, PaintSvg, PaintSvgDocument, SvgParseError};
use crate::widgets::WidgetId;

/// Retained SVG icon parsed once for backend rendering.
#[derive(Clone, Debug)]
pub struct SvgIcon {
    document: Option<PaintSvgDocument>,
}

impl SvgIcon {
    /// Construct an icon that emits no SVG paint primitives.
    pub fn empty() -> Self {
        Self { document: None }
    }

    /// Parse an SVG icon from embedded source text.
    pub fn from_svg(svg: &str) -> Option<Self> {
        Self::try_from_svg(svg).ok()
    }

    /// Parse an SVG icon from embedded source text with diagnostics.
    pub fn try_from_svg(svg: &str) -> Result<Self, SvgParseError> {
        Ok(Self {
            document: Some(PaintSvgDocument::try_from_svg(svg)?),
        })
    }

    /// Parse a single-color SVG icon after injecting `currentColor` from a
    /// Radiant color.
    ///
    /// This is intended for app-owned monochrome icons whose shapes stay
    /// stable but whose color follows interaction state or theme tokens.
    /// Repeated projection paths should prefer [`SvgIconTintCache`] so the
    /// tinted SVG document is parsed once per color.
    pub fn from_svg_with_current_color(svg: &str, color: Rgba8) -> Option<Self> {
        Self::try_from_svg_with_current_color(svg, color).ok()
    }

    /// Parse a single-color SVG icon with parser diagnostics after injecting
    /// `currentColor` from a Radiant color.
    pub fn try_from_svg_with_current_color(svg: &str, color: Rgba8) -> Result<Self, SvgParseError> {
        Self::try_from_svg(&svg_with_current_color(svg, color))
    }

    /// Append this icon as a retained SVG paint primitive inside `rect`.
    pub fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        widget_id: WidgetId,
        rect: Rect,
    ) {
        let Some(document) = self.document.clone() else {
            return;
        };
        primitives.push(PaintPrimitive::Svg(PaintSvg {
            widget_id,
            document,
            rect,
        }));
    }
}

/// Retained tinted icon cache for app-owned monochrome SVG assets.
///
/// Hosts can keep one static cache per icon shape and request a color-specific
/// [`SvgIcon`] during projection. Radiant parses each tint once and clones the
/// retained document on subsequent calls.
#[derive(Debug)]
pub struct SvgIconTintCache {
    svg: &'static str,
    icons: OnceLock<Mutex<HashMap<u32, SvgIcon>>>,
}

/// Tint colors for stateful monochrome SVG icons.
///
/// This keeps common enabled/active/disabled color selection near Radiant's
/// retained SVG cache while host applications keep owning their theme choices.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SvgIconTintPalette {
    /// Tint for enabled, inactive icons.
    pub enabled: Rgba8,
    /// Tint for enabled, active icons.
    pub active: Rgba8,
    /// Tint for disabled icons.
    pub disabled: Rgba8,
}

impl SvgIconTintPalette {
    /// Build a stateful monochrome icon tint palette.
    pub const fn new(enabled: Rgba8, active: Rgba8, disabled: Rgba8) -> Self {
        Self {
            enabled,
            active,
            disabled,
        }
    }

    /// Resolve the tint color for a generic enabled/active icon state.
    pub const fn color(self, enabled: bool, active: bool) -> Rgba8 {
        if !enabled {
            self.disabled
        } else if active {
            self.active
        } else {
            self.enabled
        }
    }

    /// Return a cached icon tinted for a generic enabled/active icon state.
    pub fn icon(self, cache: &SvgIconTintCache, enabled: bool, active: bool) -> SvgIcon {
        cache.icon(self.color(enabled, active))
    }
}

impl SvgIconTintCache {
    /// Create a tint cache for one static SVG source string.
    pub const fn new(svg: &'static str) -> Self {
        Self {
            svg,
            icons: OnceLock::new(),
        }
    }

    /// Return the icon for `color`, or an empty no-paint icon if the SVG cannot
    /// be parsed.
    pub fn icon(&self, color: Rgba8) -> SvgIcon {
        self.try_icon(color).unwrap_or_else(|_| SvgIcon::empty())
    }

    /// Return the icon resolved through a generic enabled/active tint palette.
    pub fn icon_for_state(
        &self,
        palette: SvgIconTintPalette,
        enabled: bool,
        active: bool,
    ) -> SvgIcon {
        palette.icon(self, enabled, active)
    }

    /// Return the icon for `color`, preserving parser diagnostics on failure.
    pub fn try_icon(&self, color: Rgba8) -> Result<SvgIcon, SvgParseError> {
        let key = color_key(color);
        let cache = self.icons.get_or_init(|| Mutex::new(HashMap::new()));
        let mut icons = cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(icon) = icons.get(&key) {
            return Ok(icon.clone());
        }
        let icon = SvgIcon::try_from_svg_with_current_color(self.svg, color)?;
        icons.insert(key, icon.clone());
        Ok(icon)
    }
}

/// Inject a Radiant color as inherited `currentColor` on the root SVG element.
///
/// This helper is intentionally narrow: it is for monochrome icon SVGs that do
/// not need independent per-shape colors. Existing explicit child fills still
/// follow normal SVG inheritance and override the inherited fill.
pub fn svg_with_current_color(svg: &str, color: Rgba8) -> String {
    let opacity = if color.a == u8::MAX {
        String::new()
    } else {
        format!(r#" fill-opacity="{:.3}""#, color.a as f32 / u8::MAX as f32)
    };
    let attributes = format!(
        r##"<svg color="#{:02x}{:02x}{:02x}" fill="currentColor"{opacity} "##,
        color.r, color.g, color.b
    );
    svg.replacen("<svg ", attributes.as_str(), 1)
}

fn color_key(color: Rgba8) -> u32 {
    u32::from_be_bytes([color.r, color.g, color.b, color.a])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        gui::types::{Point, Vector2},
        runtime::PaintPrimitive,
    };

    const TEST_ICON: &str = r#"<svg viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg">
  <rect x="2" y="2" width="12" height="12"/>
</svg>"#;

    #[test]
    fn svg_with_current_color_injects_root_color_and_fill() {
        let tinted = svg_with_current_color(TEST_ICON, Rgba8::new(10, 20, 30, 128));

        assert!(tinted.contains(r##"color="#0a141e""##));
        assert!(tinted.contains(r#"fill="currentColor""#));
        assert!(tinted.contains(r#"fill-opacity="0.502""#));
    }

    #[test]
    fn tint_cache_reuses_retained_document_for_same_color() {
        let cache = SvgIconTintCache::new(TEST_ICON);
        let color = Rgba8::new(238, 238, 238, 255);
        let first = cache.icon(color);
        let second = cache.icon(color);
        let rect = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(16.0, 16.0));
        let mut first_primitives = Vec::new();
        let mut second_primitives = Vec::new();

        first.append_paint(&mut first_primitives, 1, rect);
        second.append_paint(&mut second_primitives, 1, rect);

        let Some(PaintPrimitive::Svg(first_svg)) = first_primitives.first() else {
            panic!("first icon should paint svg");
        };
        let Some(PaintPrimitive::Svg(second_svg)) = second_primitives.first() else {
            panic!("second icon should paint svg");
        };
        assert_eq!(first_svg.document, second_svg.document);
    }

    #[test]
    fn tint_palette_resolves_state_colors_and_cached_icons() {
        let palette = SvgIconTintPalette::new(
            Rgba8::new(220, 220, 220, 255),
            Rgba8::new(255, 160, 82, 255),
            Rgba8::new(120, 120, 120, 255),
        );
        let cache = SvgIconTintCache::new(TEST_ICON);

        assert_eq!(palette.color(true, false), palette.enabled);
        assert_eq!(palette.color(true, true), palette.active);
        assert_eq!(palette.color(false, true), palette.disabled);

        let icon = cache.icon_for_state(palette, true, true);
        let rect = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(16.0, 16.0));
        let mut primitives = Vec::new();
        icon.append_paint(&mut primitives, 1, rect);

        assert!(
            primitives
                .iter()
                .any(|primitive| matches!(primitive, PaintPrimitive::Svg(_))),
            "stateful tint palette should still produce a retained SVG"
        );
    }
}
