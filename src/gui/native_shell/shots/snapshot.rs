use super::*;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub(super) struct ShotColor {
    pub(super) r: u8,
    pub(super) g: u8,
    pub(super) b: u8,
    pub(super) a: u8,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct ShotPoint {
    pub(super) x: f32,
    pub(super) y: f32,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct ShotRect {
    pub(super) x: f32,
    pub(super) y: f32,
    pub(super) width: f32,
    pub(super) height: f32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(super) enum ShotPrimitive {
    Rect {
        rect: ShotRect,
        color: ShotColor,
    },
    Circle {
        center: ShotPoint,
        radius: f32,
        color: ShotColor,
    },
    Image {
        rect: ShotRect,
        width: u32,
        height: u32,
        pixels: Vec<u8>,
    },
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
enum ShotAlign {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Serialize)]
struct ShotTextRun {
    text: String,
    position: ShotPoint,
    font_size: f32,
    color: ShotColor,
    max_width: Option<f32>,
    align: ShotAlign,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct ShotSnapshot {
    name: String,
    pub(super) viewport_width: u32,
    pub(super) viewport_height: u32,
    pub(super) clear_color: ShotColor,
    primitive_count: usize,
    text_run_count: usize,
    pub(super) primitives: Vec<ShotPrimitive>,
    text_runs: Vec<ShotTextRun>,
}

fn quantize(value: f32) -> f32 {
    (value * 1000.0).round() / 1000.0
}

fn snap_color(color: Rgba8) -> ShotColor {
    ShotColor {
        r: color.r,
        g: color.g,
        b: color.b,
        a: color.a,
    }
}

fn snap_point(point: Point) -> ShotPoint {
    ShotPoint {
        x: quantize(point.x),
        y: quantize(point.y),
    }
}

fn snap_rect(rect: crate::gui::types::Rect) -> ShotRect {
    ShotRect {
        x: quantize(rect.min.x),
        y: quantize(rect.min.y),
        width: quantize(rect.width()),
        height: quantize(rect.height()),
    }
}

fn snap_align(align: TextAlign) -> ShotAlign {
    match align {
        TextAlign::Left => ShotAlign::Left,
        TextAlign::Center => ShotAlign::Center,
        TextAlign::Right => ShotAlign::Right,
    }
}

pub(super) fn build_snapshot(name: &str, viewport: Vector2, model: &AppModel) -> ShotSnapshot {
    let layout = ShellLayout::build(viewport);
    let mut state = NativeShellState::new();
    state.sync_from_model(model);
    let frame = state.build_frame(&layout, model);

    let viewport_width =
        u32::try_from(layout.root.rect.width().round().max(1.0) as i64).unwrap_or(1);
    let viewport_height =
        u32::try_from(layout.root.rect.height().round().max(1.0) as i64).unwrap_or(1);

    let primitives: Vec<ShotPrimitive> = frame
        .primitives
        .iter()
        .map(|primitive| match primitive {
            Primitive::Rect(fill_rect) => ShotPrimitive::Rect {
                rect: snap_rect(fill_rect.rect),
                color: snap_color(fill_rect.color),
            },
            Primitive::Circle(fill_circle) => ShotPrimitive::Circle {
                center: snap_point(fill_circle.center),
                radius: quantize(fill_circle.radius),
                color: snap_color(fill_circle.color),
            },
            Primitive::Image(draw_image) => ShotPrimitive::Image {
                rect: snap_rect(draw_image.rect),
                width: u32::try_from(draw_image.image.width).unwrap_or(0),
                height: u32::try_from(draw_image.image.height).unwrap_or(0),
                pixels: draw_image.image.pixels.as_ref().to_vec(),
            },
        })
        .collect();

    let text_runs: Vec<ShotTextRun> = frame
        .text_runs
        .iter()
        .map(|run| ShotTextRun {
            text: run.text.clone(),
            position: snap_point(run.position),
            font_size: quantize(run.font_size),
            color: snap_color(run.color),
            max_width: run.max_width.map(quantize),
            align: snap_align(run.align),
        })
        .collect();

    ShotSnapshot {
        name: name.to_string(),
        viewport_width,
        viewport_height,
        clear_color: snap_color(frame.clear_color),
        primitive_count: primitives.len(),
        text_run_count: text_runs.len(),
        primitives,
        text_runs,
    }
}

pub(super) fn canonicalize_json(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let mut entries: Vec<(String, serde_json::Value)> = map.into_iter().collect();
            entries.sort_by(|left, right| left.0.cmp(&right.0));
            let canonical = entries
                .into_iter()
                .map(|(key, nested)| (key, canonicalize_json(nested)))
                .collect::<serde_json::Map<String, serde_json::Value>>();
            serde_json::Value::Object(canonical)
        }
        serde_json::Value::Array(values) => {
            serde_json::Value::Array(values.into_iter().map(canonicalize_json).collect())
        }
        primitive => primitive,
    }
}
