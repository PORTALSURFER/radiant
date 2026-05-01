//! Compatibility shell snapshot capture used by host-owned GUI fixtures.

use super::{
    AppModel, NativeShellState, Primitive, ShellLayout, ShellLayoutRuntime, StyleTokens, TextAlign,
    Vector2,
};
use crate::gui::{
    native_shell::NativeViewFrame,
    types::{Point, Rect, Rgba8},
};
use serde::Serialize;

/// Serializable color captured from one native-shell frame.
#[derive(Debug, Clone, Serialize)]
pub struct NativeShellShotColor {
    /// Red channel.
    pub r: u8,
    /// Green channel.
    pub g: u8,
    /// Blue channel.
    pub b: u8,
    /// Alpha channel.
    pub a: u8,
}

/// Serializable point captured from one native-shell frame.
#[derive(Debug, Clone, Serialize)]
pub struct NativeShellShotPoint {
    /// X coordinate in logical window space.
    pub x: f32,
    /// Y coordinate in logical window space.
    pub y: f32,
}

/// Serializable rectangle captured from one native-shell frame.
#[derive(Debug, Clone, Serialize)]
pub struct NativeShellShotRect {
    /// Minimum X coordinate in logical window space.
    pub x: f32,
    /// Minimum Y coordinate in logical window space.
    pub y: f32,
    /// Width in logical points.
    pub width: f32,
    /// Height in logical points.
    pub height: f32,
}

/// Serializable primitive captured from one native-shell frame.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum NativeShellShotPrimitive {
    /// Filled rectangle primitive.
    Rect {
        /// Primitive bounds.
        rect: NativeShellShotRect,
        /// Fill color.
        color: NativeShellShotColor,
    },
    /// Filled circle primitive.
    Circle {
        /// Circle center.
        center: NativeShellShotPoint,
        /// Circle radius.
        radius: f32,
        /// Fill color.
        color: NativeShellShotColor,
    },
    /// Filled linear-gradient primitive.
    LinearGradient {
        /// Primitive bounds.
        rect: NativeShellShotRect,
        /// Gradient start point.
        start: NativeShellShotPoint,
        /// Gradient end point.
        end: NativeShellShotPoint,
        /// Gradient start color.
        start_color: NativeShellShotColor,
        /// Gradient end color.
        end_color: NativeShellShotColor,
    },
    /// RGBA image primitive.
    Image {
        /// Image placement bounds.
        rect: NativeShellShotRect,
        /// Source image width.
        width: u32,
        /// Source image height.
        height: u32,
        /// Source image RGBA pixels.
        pixels: Vec<u8>,
    },
}

/// Text alignment captured from one native-shell frame.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum NativeShellShotAlign {
    /// Left-aligned text.
    Left,
    /// Center-aligned text.
    Center,
    /// Right-aligned text.
    Right,
}

/// Serializable text run captured from one native-shell frame.
#[derive(Debug, Clone, Serialize)]
pub struct NativeShellShotTextRun {
    /// Text content.
    pub text: String,
    /// Text anchor position.
    pub position: NativeShellShotPoint,
    /// Font size in logical points.
    pub font_size: f32,
    /// Text color.
    pub color: NativeShellShotColor,
    /// Optional max width for text layout.
    pub max_width: Option<f32>,
    /// Text alignment.
    pub align: NativeShellShotAlign,
}

/// Deterministic snapshot of one compatibility native-shell frame.
#[derive(Debug, Clone, Serialize)]
pub struct NativeShellShotSnapshot {
    /// Fixture name.
    pub name: String,
    /// Viewport width in logical pixels.
    pub viewport_width: u32,
    /// Viewport height in logical pixels.
    pub viewport_height: u32,
    /// Frame clear color.
    pub clear_color: NativeShellShotColor,
    /// Number of captured primitives.
    pub primitive_count: usize,
    /// Number of captured text runs.
    pub text_run_count: usize,
    /// Captured paint primitives.
    pub primitives: Vec<NativeShellShotPrimitive>,
    /// Captured text runs.
    pub text_runs: Vec<NativeShellShotTextRun>,
}

/// Capture a deterministic native-shell visual snapshot without launching a window.
pub fn capture_native_shell_shot_snapshot(
    name: impl Into<String>,
    viewport: [f32; 2],
    model: &AppModel,
) -> NativeShellShotSnapshot {
    let viewport = Vector2::new(viewport[0].max(1.0), viewport[1].max(1.0));
    let style = StyleTokens::for_viewport_width(viewport.x);
    let mut runtime = ShellLayoutRuntime::default();
    let layout = ShellLayout::build_with_style_and_runtime(viewport, &style, &mut runtime);
    let mut state = NativeShellState::new();
    state.sync_from_model(model);
    let mut frame = NativeViewFrame {
        clear_color: style.clear_color,
        primitives: Vec::new(),
        text_runs: Vec::new(),
    };
    state.build_frame_with_style_into_static(&layout, &style, model, &mut frame);
    snapshot_from_frame(name.into(), &layout, &frame)
}

fn snapshot_from_frame(
    name: String,
    layout: &ShellLayout,
    frame: &NativeViewFrame,
) -> NativeShellShotSnapshot {
    let viewport_width =
        u32::try_from(layout.root.rect.width().round().max(1.0) as i64).unwrap_or(1);
    let viewport_height =
        u32::try_from(layout.root.rect.height().round().max(1.0) as i64).unwrap_or(1);
    let primitives = frame.primitives.iter().map(snap_primitive).collect();
    let text_runs = frame
        .text_runs
        .iter()
        .map(|run| NativeShellShotTextRun {
            text: run.text.clone(),
            position: snap_point(run.position),
            font_size: quantize(run.font_size),
            color: snap_color(run.color),
            max_width: run.max_width.map(quantize),
            align: snap_align(run.align),
        })
        .collect();

    NativeShellShotSnapshot {
        name,
        viewport_width,
        viewport_height,
        clear_color: snap_color(frame.clear_color),
        primitive_count: frame.primitives.len(),
        text_run_count: frame.text_runs.len(),
        primitives,
        text_runs,
    }
}

fn snap_primitive(primitive: &Primitive) -> NativeShellShotPrimitive {
    match primitive {
        Primitive::Rect(fill_rect) => NativeShellShotPrimitive::Rect {
            rect: snap_rect(fill_rect.rect),
            color: snap_color(fill_rect.color),
        },
        Primitive::Circle(fill_circle) => NativeShellShotPrimitive::Circle {
            center: snap_point(fill_circle.center),
            radius: quantize(fill_circle.radius),
            color: snap_color(fill_circle.color),
        },
        Primitive::LinearGradient(fill_gradient) => NativeShellShotPrimitive::LinearGradient {
            rect: snap_rect(fill_gradient.rect),
            start: snap_point(fill_gradient.start),
            end: snap_point(fill_gradient.end),
            start_color: snap_color(fill_gradient.start_color),
            end_color: snap_color(fill_gradient.end_color),
        },
        Primitive::Image(draw_image) => NativeShellShotPrimitive::Image {
            rect: snap_rect(draw_image.rect),
            width: u32::try_from(draw_image.image.width).unwrap_or(0),
            height: u32::try_from(draw_image.image.height).unwrap_or(0),
            pixels: draw_image.image.pixels.as_ref().to_vec(),
        },
    }
}

fn quantize(value: f32) -> f32 {
    (value * 1000.0).round() / 1000.0
}

fn snap_color(color: Rgba8) -> NativeShellShotColor {
    NativeShellShotColor {
        r: color.r,
        g: color.g,
        b: color.b,
        a: color.a,
    }
}

fn snap_point(point: Point) -> NativeShellShotPoint {
    NativeShellShotPoint {
        x: quantize(point.x),
        y: quantize(point.y),
    }
}

fn snap_rect(rect: Rect) -> NativeShellShotRect {
    NativeShellShotRect {
        x: quantize(rect.min.x),
        y: quantize(rect.min.y),
        width: quantize(rect.width()),
        height: quantize(rect.height()),
    }
}

fn snap_align(align: TextAlign) -> NativeShellShotAlign {
    match align {
        TextAlign::Left => NativeShellShotAlign::Left,
        TextAlign::Center => NativeShellShotAlign::Center,
        TextAlign::Right => NativeShellShotAlign::Right,
    }
}
