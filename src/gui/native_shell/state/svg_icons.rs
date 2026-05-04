//! Minimal SVG icon parser/rasterizer for native shell glyphs.
//!
//! The native shell paint model is backend-neutral and currently supports
//! rectangles/circles/images/text primitives. This module loads toolbar glyph
//! definitions from asset-backed SVG files and rasterizes them into RGBA images
//! so toolbar controls can render iconography without adding a new primitive
//! kind.

use super::*;
#[cfg(test)]
use std::sync::Arc;

use crate::gui::svg::{parse_svg_document, point_in_svg_shapes};

/// Icon identifiers used by native shell controls.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ShellSvgIcon {
    /// Mono channel-view icon.
    Mono,
    /// Stereo channel-view icon.
    Stereo,
    /// Normalize audition toggle icon.
    Normalize,
    /// BPM snap icon.
    BpmSnap,
    /// Selection-relative BPM grid origin icon.
    RelativeBpmGrid,
    /// Transient snap icon.
    TransientSnap,
    /// Show transient markers icon.
    ShowTransients,
    /// Slice mode icon.
    Slice,
    /// Loop toggle icon.
    Loop,
    /// Lock overlay icon used by locked controls.
    Lock,
    /// Stop transport icon.
    Stop,
    /// Play transport icon used in both idle and running states.
    Play,
    /// Record icon placeholder.
    Record,
    /// Random-navigation toggle icon.
    Dice,
    /// Focused-row similarity-search icon.
    Similarity,
    /// Panel filter toggle icon.
    Filter,
    /// Panel flattened-view toggle icon.
    Flatten,
    /// Recency filter icon for content with no activation history.
    RecencyNever,
    /// Recency filter icon for content older than one month.
    RecencyOlderThanMonth,
    /// Recency filter icon for content older than one week.
    RecencyOlderThanWeek,
    /// Marked-only filter icon.
    Marked,
}

/// Return a toolbar icon for one waveform toolbar button.
pub(super) fn shell_svg_icon_for_button(button: &WaveformToolbarButton) -> Option<ShellSvgIcon> {
    button.icon
}

/// Emit one SVG-backed toolbar icon into the primitive list.
pub(super) fn emit_toolbar_svg_icon(
    primitives: &mut impl PrimitiveSink,
    icon: ShellSvgIcon,
    rect: Rect,
    color: Rgba8,
) -> bool {
    let side = rect.width().min(rect.height()).round().clamp(8.0, 32.0) as usize;
    let Some(image) = rasterize_svg_icon(icon, side, color) else {
        return false;
    };
    emit_primitive(
        primitives,
        Primitive::Image(DrawImage {
            rect,
            image: Arc::new(image),
        }),
    );
    true
}

fn rasterize_svg_icon(icon: ShellSvgIcon, side: usize, color: Rgba8) -> Option<ImageRgba> {
    let svg = icon_svg_asset(icon);
    let document = parse_svg_document(svg)?;
    let mut pixels = vec![0_u8; side.saturating_mul(side).saturating_mul(4)];
    let coverage_offsets = [
        (0.25_f32, 0.25_f32),
        (0.75, 0.25),
        (0.25, 0.75),
        (0.75, 0.75),
    ];

    for y in 0..side {
        for x in 0..side {
            let mut hits = 0_u8;
            for (offset_x, offset_y) in coverage_offsets {
                let world_x = document.view_box_min_x
                    + ((x as f32 + offset_x) / side as f32) * document.view_box_width;
                let world_y = document.view_box_min_y
                    + ((y as f32 + offset_y) / side as f32) * document.view_box_height;
                if point_in_svg_shapes(world_x, world_y, &document.shapes) {
                    hits = hits.saturating_add(1);
                }
            }
            if hits == 0 {
                continue;
            }
            let coverage = hits as f32 / coverage_offsets.len() as f32;
            let alpha = ((color.a as f32) * coverage).round().clamp(0.0, 255.0) as u8;
            let index = (y * side + x) * 4;
            pixels[index] = color.r;
            pixels[index + 1] = color.g;
            pixels[index + 2] = color.b;
            pixels[index + 3] = alpha;
        }
    }

    ImageRgba::new(side, side, pixels)
}
fn icon_svg_asset(icon: ShellSvgIcon) -> &'static str {
    match icon {
        ShellSvgIcon::Mono => {
            include_str!("../assets/icons/waveform_toolbar/mono.svg")
        }
        ShellSvgIcon::Stereo => {
            include_str!("../assets/icons/waveform_toolbar/stereo.svg")
        }
        ShellSvgIcon::Normalize => {
            include_str!("../assets/icons/waveform_toolbar/normalize.svg")
        }
        ShellSvgIcon::BpmSnap => {
            include_str!("../assets/icons/waveform_toolbar/bpm_snap.svg")
        }
        ShellSvgIcon::RelativeBpmGrid => {
            include_str!("../assets/icons/waveform_toolbar/relative_bpm_grid.svg")
        }
        ShellSvgIcon::TransientSnap => {
            include_str!("../assets/icons/waveform_toolbar/transient_snap.svg")
        }
        ShellSvgIcon::ShowTransients => {
            include_str!("../assets/icons/waveform_toolbar/show_transients.svg")
        }
        ShellSvgIcon::Slice => {
            include_str!("../assets/icons/waveform_toolbar/slice.svg")
        }
        ShellSvgIcon::Play => {
            include_str!("../assets/icons/waveform_toolbar/play.svg")
        }
        ShellSvgIcon::Stop => {
            include_str!("../assets/icons/waveform_toolbar/stop.svg")
        }
        ShellSvgIcon::Record => {
            include_str!("../assets/icons/waveform_toolbar/record.svg")
        }
        ShellSvgIcon::Loop => {
            include_str!("../assets/icons/waveform_toolbar/loop.svg")
        }
        ShellSvgIcon::Lock => {
            include_str!("../assets/icons/ui/lock.svg")
        }
        ShellSvgIcon::Dice => {
            include_str!("../assets/icons/ui/dice.svg")
        }
        ShellSvgIcon::Similarity => {
            include_str!("../assets/icons/ui/similarity.svg")
        }
        ShellSvgIcon::Filter => {
            include_str!("../assets/icons/ui/filter.svg")
        }
        ShellSvgIcon::Flatten => {
            include_str!("../assets/icons/ui/flatten.svg")
        }
        ShellSvgIcon::RecencyNever => include_str!("../assets/icons/ui/recency_never.svg"),
        ShellSvgIcon::RecencyOlderThanMonth => {
            include_str!("../assets/icons/ui/recency_older_than_month.svg")
        }
        ShellSvgIcon::RecencyOlderThanWeek => {
            include_str!("../assets/icons/ui/recency_older_than_week.svg")
        }
        ShellSvgIcon::Marked => include_str!("../assets/icons/ui/marked.svg"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::types::{Point, Rect, Rgba8};

    fn waveform_toolbar_button(label: &'static str, active: bool) -> WaveformToolbarButton {
        WaveformToolbarButton {
            rect: Rect::from_min_max(Point::new(0.0, 0.0), Point::new(18.0, 18.0)),
            label,
            icon: toolbar_icon_for_label(label),
            overlay_icon: None,
            display_text: None,
            enabled: true,
            active,
            action: None,
            text_color: Rgba8 {
                r: 255,
                g: 255,
                b: 255,
                a: 255,
            },
        }
    }

    fn toolbar_icon_for_label(label: &'static str) -> Option<ShellSvgIcon> {
        match label {
            "Channel Mono" => Some(ShellSvgIcon::Mono),
            "Channel Stereo" => Some(ShellSvgIcon::Stereo),
            "Play" => Some(ShellSvgIcon::Play),
            "Stop" => Some(ShellSvgIcon::Stop),
            _ => None,
        }
    }

    #[test]
    fn transport_button_swaps_between_play_and_stop_icons() {
        let idle_button = waveform_toolbar_button("Play", false);
        let running_button = waveform_toolbar_button("Stop", true);

        assert_eq!(
            shell_svg_icon_for_button(&idle_button),
            Some(ShellSvgIcon::Play)
        );
        assert_eq!(
            shell_svg_icon_for_button(&running_button),
            Some(ShellSvgIcon::Stop)
        );
    }

    #[test]
    fn channel_button_swaps_icons_between_mono_and_stereo_states() {
        let mono_button = waveform_toolbar_button("Channel Mono", false);
        let stereo_button = waveform_toolbar_button("Channel Stereo", false);

        assert_eq!(
            shell_svg_icon_for_button(&mono_button),
            Some(ShellSvgIcon::Mono)
        );
        assert_eq!(
            shell_svg_icon_for_button(&stereo_button),
            Some(ShellSvgIcon::Stereo)
        );
    }

    #[test]
    fn asset_backed_svg_icons_parse_successfully() {
        for icon in [
            ShellSvgIcon::Mono,
            ShellSvgIcon::Stereo,
            ShellSvgIcon::Normalize,
            ShellSvgIcon::BpmSnap,
            ShellSvgIcon::RelativeBpmGrid,
            ShellSvgIcon::TransientSnap,
            ShellSvgIcon::ShowTransients,
            ShellSvgIcon::Slice,
            ShellSvgIcon::Loop,
            ShellSvgIcon::Lock,
            ShellSvgIcon::Stop,
            ShellSvgIcon::Play,
            ShellSvgIcon::Record,
            ShellSvgIcon::Dice,
            ShellSvgIcon::Similarity,
            ShellSvgIcon::Filter,
            ShellSvgIcon::Flatten,
            ShellSvgIcon::RecencyNever,
            ShellSvgIcon::RecencyOlderThanMonth,
            ShellSvgIcon::RecencyOlderThanWeek,
            ShellSvgIcon::Marked,
        ] {
            let document = parse_svg_document(icon_svg_asset(icon));
            assert!(document.is_some(), "svg asset for {icon:?} should parse");
            assert!(
                !document.expect("document should exist").shapes.is_empty(),
                "svg asset for {icon:?} should yield visible shapes"
            );
        }
    }
}
