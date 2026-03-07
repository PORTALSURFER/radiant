//! Minimal SVG icon parser/rasterizer for waveform toolbar glyphs.
//!
//! The native shell paint model is backend-neutral and currently supports
//! rectangles/circles/images/text primitives. This module keeps icon assets in
//! inline SVG form and rasterizes them into RGBA images so toolbar controls can
//! render iconography without adding a new primitive kind.

use super::*;
use std::sync::Arc;

/// Icon identifiers used by waveform toolbar transport controls.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum WaveformToolbarIcon {
    /// Mono channel-view icon.
    Mono,
    /// Stereo channel-view icon.
    Stereo,
    /// Normalize audition toggle icon.
    Normalize,
    /// BPM snap icon.
    BpmSnap,
    /// Transient snap icon.
    TransientSnap,
    /// Show transient markers icon.
    ShowTransients,
    /// Slice mode icon.
    Slice,
    /// Loop toggle icon.
    Loop,
    /// Stop transport icon.
    Stop,
    /// Play transport icon used in both idle and running states.
    Play,
    /// Record icon placeholder.
    Record,
}

#[derive(Clone, Debug, PartialEq)]
struct SvgDocument {
    view_box_min_x: f32,
    view_box_min_y: f32,
    view_box_width: f32,
    view_box_height: f32,
    shapes: Vec<SvgShape>,
}

#[derive(Clone, Debug, PartialEq)]
enum SvgShape {
    Rect {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    },
    Circle {
        cx: f32,
        cy: f32,
        radius: f32,
    },
    Polygon(Vec<(f32, f32)>),
}

/// Return a toolbar icon for one waveform toolbar button.
pub(super) fn toolbar_icon_for_button(
    button: &WaveformToolbarButton,
) -> Option<WaveformToolbarIcon> {
    button.icon
}

/// Emit one SVG-backed toolbar icon into the primitive list.
pub(super) fn emit_toolbar_svg_icon(
    primitives: &mut impl PrimitiveSink,
    icon: WaveformToolbarIcon,
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

fn rasterize_svg_icon(icon: WaveformToolbarIcon, side: usize, color: Rgba8) -> Option<ImageRgba> {
    let svg = icon_svg(icon);
    let document = parse_svg_document(svg)?;
    let mut pixels = vec![0_u8; side.saturating_mul(side).saturating_mul(4)];
    let sample_offsets = [
        (0.25_f32, 0.25_f32),
        (0.75, 0.25),
        (0.25, 0.75),
        (0.75, 0.75),
    ];

    for y in 0..side {
        for x in 0..side {
            let mut hits = 0_u8;
            for (offset_x, offset_y) in sample_offsets {
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
            let coverage = hits as f32 / sample_offsets.len() as f32;
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

fn parse_svg_document(svg: &str) -> Option<SvgDocument> {
    let view_box = extract_attr(svg, "viewBox")?;
    let view_box_values: Vec<f32> = view_box
        .split_whitespace()
        .map(|value| value.parse::<f32>().ok())
        .collect::<Option<Vec<_>>>()?;
    if view_box_values.len() != 4 {
        return None;
    }

    let mut shapes = Vec::new();
    for raw in svg.split('<').skip(1) {
        let end_index = raw.find('>')?;
        let tag = raw[..end_index].trim().trim_end_matches('/').trim();
        if tag.starts_with("rect ") {
            shapes.push(SvgShape::Rect {
                x: parse_attr_f32(tag, "x")?,
                y: parse_attr_f32(tag, "y")?,
                width: parse_attr_f32(tag, "width")?,
                height: parse_attr_f32(tag, "height")?,
            });
        } else if tag.starts_with("circle ") {
            shapes.push(SvgShape::Circle {
                cx: parse_attr_f32(tag, "cx")?,
                cy: parse_attr_f32(tag, "cy")?,
                radius: parse_attr_f32(tag, "r")?,
            });
        } else if tag.starts_with("polygon ") {
            let points = parse_points(extract_attr(tag, "points")?)?;
            shapes.push(SvgShape::Polygon(points));
        }
    }

    Some(SvgDocument {
        view_box_min_x: view_box_values[0],
        view_box_min_y: view_box_values[1],
        view_box_width: view_box_values[2],
        view_box_height: view_box_values[3],
        shapes,
    })
}

fn parse_attr_f32(tag: &str, attr: &str) -> Option<f32> {
    extract_attr(tag, attr)?.parse::<f32>().ok()
}

fn extract_attr<'a>(text: &'a str, attr: &str) -> Option<&'a str> {
    let needle = format!(r#"{attr}=""#);
    let start = text.find(&needle)? + needle.len();
    let end = start + text[start..].find('"')?;
    Some(&text[start..end])
}

fn parse_points(points: &str) -> Option<Vec<(f32, f32)>> {
    let mut coords = Vec::new();
    let mut token = String::new();
    for ch in points.chars() {
        if ch == ',' || ch.is_ascii_whitespace() {
            if !token.is_empty() {
                coords.push(token.parse::<f32>().ok()?);
                token.clear();
            }
        } else {
            token.push(ch);
        }
    }
    if !token.is_empty() {
        coords.push(token.parse::<f32>().ok()?);
    }
    if coords.len() < 6 || coords.len() % 2 != 0 {
        return None;
    }
    let mut output = Vec::with_capacity(coords.len() / 2);
    for pair in coords.chunks_exact(2) {
        output.push((pair[0], pair[1]));
    }
    Some(output)
}

fn point_in_svg_shapes(x: f32, y: f32, shapes: &[SvgShape]) -> bool {
    shapes.iter().any(|shape| point_in_svg_shape(x, y, shape))
}

fn point_in_svg_shape(x: f32, y: f32, shape: &SvgShape) -> bool {
    match shape {
        SvgShape::Rect {
            x: rect_x,
            y: rect_y,
            width,
            height,
        } => x >= *rect_x && x <= rect_x + width && y >= *rect_y && y <= rect_y + height,
        SvgShape::Circle { cx, cy, radius } => {
            let dx = x - cx;
            let dy = y - cy;
            (dx * dx) + (dy * dy) <= radius * radius
        }
        SvgShape::Polygon(points) => point_in_polygon(x, y, points),
    }
}

fn point_in_polygon(x: f32, y: f32, points: &[(f32, f32)]) -> bool {
    let mut inside = false;
    let mut previous = points[points.len() - 1];
    for current in points {
        let (x0, y0) = previous;
        let (x1, y1) = *current;
        let crosses = (y0 > y) != (y1 > y);
        if crosses {
            let t = (y - y0) / (y1 - y0);
            let x_intersect = x0 + (t * (x1 - x0));
            if x < x_intersect {
                inside = !inside;
            }
        }
        previous = *current;
    }
    inside
}

fn icon_svg(icon: WaveformToolbarIcon) -> &'static str {
    match icon {
        WaveformToolbarIcon::Mono => {
            r#"<svg viewBox="0 0 16 16"><rect x="6.5" y="3" width="3" height="10"/></svg>"#
        }
        WaveformToolbarIcon::Stereo => {
            r#"<svg viewBox="0 0 16 16"><rect x="3.5" y="3" width="3" height="10"/><rect x="9.5" y="3" width="3" height="10"/></svg>"#
        }
        WaveformToolbarIcon::Normalize => {
            r#"<svg viewBox="0 0 16 16"><polygon points="3,12 3,4 5.5,4 8.5,9 8.5,4 11,4 11,12 8.5,12 5.5,7 5.5,12"/></svg>"#
        }
        WaveformToolbarIcon::BpmSnap => {
            r#"<svg viewBox="0 0 16 16"><polygon points="8,2 5,6 11,6"/><rect x="6" y="6" width="4" height="7"/><rect x="4" y="13" width="8" height="1.5"/></svg>"#
        }
        WaveformToolbarIcon::TransientSnap => {
            r#"<svg viewBox="0 0 16 16"><polygon points="9,2 5,9 8,9 7,14 11,7 8,7"/></svg>"#
        }
        WaveformToolbarIcon::ShowTransients => {
            r#"<svg viewBox="0 0 16 16"><polygon points="2,8 5,5 11,5 14,8 11,11 5,11"/><circle cx="8" cy="8" r="2"/></svg>"#
        }
        WaveformToolbarIcon::Slice => {
            r#"<svg viewBox="0 0 16 16"><rect x="3" y="4" width="10" height="1.5"/><rect x="3" y="7.25" width="10" height="1.5"/><rect x="3" y="10.5" width="10" height="1.5"/><polygon points="9,3 12.5,6.5 11.2,7.8 7.7,4.3"/></svg>"#
        }
        WaveformToolbarIcon::Play => {
            r#"<svg viewBox="0 0 16 16"><polygon points="4,3 13,8 4,13"/></svg>"#
        }
        WaveformToolbarIcon::Stop => {
            r#"<svg viewBox="0 0 16 16"><rect x="4" y="4" width="8" height="8"/></svg>"#
        }
        WaveformToolbarIcon::Record => {
            r#"<svg viewBox="0 0 16 16"><circle cx="8" cy="8" r="4.5"/></svg>"#
        }
        WaveformToolbarIcon::Loop => {
            r#"<svg viewBox="0 0 16 16"><rect x="6" y="2.5" width="4" height="2"/><polygon points="10,1.5 13.5,3.5 10,5.5"/><rect x="10" y="4.5" width="2" height="4"/><rect x="8" y="10.5" width="4" height="2"/><rect x="4" y="10.5" width="4" height="2"/><rect x="4" y="8.5" width="2" height="2"/><rect x="4" y="4.5" width="2" height="4"/><rect x="6" y="4.5" width="2" height="2"/><polygon points="6,9.5 2.5,11.5 6,13.5"/></svg>"#
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::UiAction;
    use crate::gui::types::{Point, Rect, Rgba8};

    fn waveform_toolbar_button(label: &'static str, active: bool) -> WaveformToolbarButton {
        WaveformToolbarButton {
            rect: Rect::from_min_max(Point::new(0.0, 0.0), Point::new(18.0, 18.0)),
            label,
            icon: toolbar_icon_for_label(label),
            display_text: None,
            enabled: true,
            active,
            action: Some(UiAction::ToggleTransport),
            text_color: Rgba8 {
                r: 255,
                g: 255,
                b: 255,
                a: 255,
            },
        }
    }

    fn toolbar_icon_for_label(label: &'static str) -> Option<WaveformToolbarIcon> {
        match label {
            "Channel Mono" => Some(WaveformToolbarIcon::Mono),
            "Channel Stereo" => Some(WaveformToolbarIcon::Stereo),
            "Play" => Some(WaveformToolbarIcon::Play),
            _ => None,
        }
    }

    #[test]
    fn play_button_keeps_play_icon_while_transport_is_running() {
        let idle_button = waveform_toolbar_button("Play", false);
        let running_button = waveform_toolbar_button("Play", true);

        assert_eq!(
            toolbar_icon_for_button(&idle_button),
            Some(WaveformToolbarIcon::Play)
        );
        assert_eq!(
            toolbar_icon_for_button(&running_button),
            Some(WaveformToolbarIcon::Play)
        );
    }

    #[test]
    fn channel_button_swaps_icons_between_mono_and_stereo_states() {
        let mono_button = waveform_toolbar_button("Channel Mono", false);
        let stereo_button = waveform_toolbar_button("Channel Stereo", false);

        assert_eq!(
            toolbar_icon_for_button(&mono_button),
            Some(WaveformToolbarIcon::Mono)
        );
        assert_eq!(
            toolbar_icon_for_button(&stereo_button),
            Some(WaveformToolbarIcon::Stereo)
        );
    }
}
