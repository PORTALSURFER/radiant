//! Regression fixtures for the native shell visual scene graph.

use super::*;
use crate::{
    app::{AppModel, BrowserRowModel, FolderRowModel, SourceRowModel},
    gui::types::{ImageRgba, Point, Rgba8, Vector2},
};
use image::{Rgba, RgbaImage};
use serde::Serialize;
use std::{env, fs, path::PathBuf};

#[derive(Debug, Clone, Serialize)]
struct ShotColor {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

#[derive(Debug, Clone, Serialize)]
struct ShotPoint {
    x: f32,
    y: f32,
}

#[derive(Debug, Clone, Serialize)]
struct ShotRect {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum ShotPrimitive {
    Rect {
        rect: ShotRect,
        color: ShotColor,
    },
    Circle {
        center: ShotPoint,
        radius: f32,
        color: ShotColor,
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
struct ShotSnapshot {
    name: String,
    viewport_width: u32,
    viewport_height: u32,
    clear_color: ShotColor,
    primitive_count: usize,
    text_run_count: usize,
    primitives: Vec<ShotPrimitive>,
    text_runs: Vec<ShotTextRun>,
}

fn quantize(value: f32) -> f32 {
    (value * 1000.0).round() / 1000.0
}

fn shot_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("shots")
}

fn fixture_paths(name: &str) -> (PathBuf, PathBuf) {
    let root = shot_root();
    (
        root.join(format!("{name}.json")),
        root.join(format!("{name}.png")),
    )
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

fn build_snapshot(name: &str, viewport: Vector2, model: &AppModel) -> ShotSnapshot {
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

fn canonicalize_json(value: serde_json::Value) -> serde_json::Value {
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

fn blend_pixel(target: &mut Rgba<u8>, source: &ShotColor) {
    let source_alpha = source.a as f32 / 255.0;
    if source_alpha <= 0.0 {
        return;
    }
    let target_alpha = target[3] as f32 / 255.0;
    let out_alpha = source_alpha + target_alpha * (1.0 - source_alpha);
    if out_alpha <= 0.0 {
        *target = Rgba([0, 0, 0, 0]);
        return;
    }
    let source_contrib = 1.0 - source_alpha;
    let out_r = (source.r as f32 * source_alpha + target[0] as f32 * target_alpha * source_contrib)
        / out_alpha;
    let out_g = (source.g as f32 * source_alpha + target[1] as f32 * target_alpha * source_contrib)
        / out_alpha;
    let out_b = (source.b as f32 * source_alpha + target[2] as f32 * target_alpha * source_contrib)
        / out_alpha;
    *target = Rgba([
        out_r.clamp(0.0, 255.0).round() as u8,
        out_g.clamp(0.0, 255.0).round() as u8,
        out_b.clamp(0.0, 255.0).round() as u8,
        (out_alpha * 255.0).clamp(0.0, 255.0).round() as u8,
    ]);
}

fn rasterize_shot(snapshot: &ShotSnapshot) -> RgbaImage {
    let mut image = RgbaImage::from_pixel(
        snapshot.viewport_width,
        snapshot.viewport_height,
        Rgba([
            snapshot.clear_color.r,
            snapshot.clear_color.g,
            snapshot.clear_color.b,
            snapshot.clear_color.a,
        ]),
    );

    let width = i64::from(snapshot.viewport_width);
    let height = i64::from(snapshot.viewport_height);

    for primitive in &snapshot.primitives {
        match primitive {
            ShotPrimitive::Rect { rect, color } => {
                let left = rect.x.floor().clamp(0.0, width as f32) as i64;
                let right = (rect.x + rect.width).ceil().clamp(0.0, width as f32) as i64;
                let top = rect.y.floor().clamp(0.0, height as f32) as i64;
                let bottom = (rect.y + rect.height).ceil().clamp(0.0, height as f32) as i64;

                for y in top.max(0)..bottom.min(height) {
                    for x in left.max(0)..right.min(width) {
                        let pixel = image.get_pixel_mut(
                            u32::try_from(x).unwrap_or(0),
                            u32::try_from(y).unwrap_or(0),
                        );
                        blend_pixel(pixel, color);
                    }
                }
            }
            ShotPrimitive::Circle {
                center,
                radius,
                color,
            } => {
                let min_x = (center.x - *radius).floor().clamp(0.0, width as f32) as i64;
                let max_x = (center.x + *radius).ceil().clamp(0.0, width as f32) as i64;
                let min_y = (center.y - *radius).floor().clamp(0.0, height as f32) as i64;
                let max_y = (center.y + *radius).ceil().clamp(0.0, height as f32) as i64;
                let radius_sq = radius * radius;

                for y in min_y.max(0)..max_y.min(height) {
                    for x in min_x.max(0)..max_x.min(width) {
                        let x_offset = x as f32 + 0.5 - center.x;
                        let y_offset = y as f32 + 0.5 - center.y;
                        if x_offset * x_offset + y_offset * y_offset <= radius_sq {
                            let pixel = image.get_pixel_mut(
                                u32::try_from(x).unwrap_or(0),
                                u32::try_from(y).unwrap_or(0),
                            );
                            blend_pixel(pixel, color);
                        }
                    }
                }
            }
        }
    }
    image
}

fn write_or_compare_shot(name: &str, viewport: Vector2, model: AppModel, write_mode: bool) {
    let snapshot = build_snapshot(name, viewport, &model);
    let (json_path, png_path) = fixture_paths(name);

    if write_mode {
        fs::create_dir_all(shot_root()).unwrap_or_else(|err| {
            panic!("create fixture directory {}: {err}", shot_root().display())
        });
        fs::write(
            &json_path,
            serde_json::to_string_pretty(&snapshot)
                .unwrap_or_else(|err| panic!("serialize shot snapshot for {name}: {err}")),
        )
        .unwrap_or_else(|err| {
            panic!(
                "write shot JSON fixture for {name} to {}: {err}",
                json_path.display()
            )
        });
        rasterize_shot(&snapshot)
            .save(&png_path)
            .unwrap_or_else(|err| {
                panic!(
                    "write shot PNG fixture for {name} to {}: {err}",
                    png_path.display()
                )
            });
        return;
    }

    let expected_json = fs::read_to_string(&json_path).unwrap_or_else(|err| {
        panic!(
            "read expected JSON shot {name} from {}: {err}",
            json_path.display()
        )
    });
    let expected_json: serde_json::Value =
        serde_json::from_str(&expected_json).unwrap_or_else(|err| {
            panic!(
                "parse expected JSON shot {name} from {}: {err}",
                json_path.display()
            )
        });
    let expected_json = canonicalize_json(expected_json);
    let actual_json: serde_json::Value = serde_json::from_str(
        &serde_json::to_string_pretty(&snapshot)
            .unwrap_or_else(|err| panic!("serialize actual shot snapshot for {name}: {err}")),
    )
    .unwrap_or_else(|err| panic!("parse actual shot snapshot for {name}: {err}"));
    let actual_json = canonicalize_json(actual_json);
    assert_eq!(
        expected_json,
        actual_json,
        "shot fixture mismatch for {name}: {}",
        json_path.display()
    );

    let expected_png = image::open(&png_path).unwrap_or_else(|err| {
        panic!(
            "read expected PNG shot {name} from {}: {err}",
            png_path.display()
        )
    });
    let expected = expected_png.to_rgba8();
    let actual = rasterize_shot(&snapshot);
    assert_eq!(
        expected.width(),
        actual.width(),
        "PNG width mismatch for shot {name}"
    );
    assert_eq!(
        expected.height(),
        actual.height(),
        "PNG height mismatch for shot {name}"
    );
    assert_eq!(
        expected.into_raw(),
        actual.into_raw(),
        "PNG bytes mismatch for shot {name}"
    );
}

fn startup_scene_model() -> AppModel {
    let mut model = AppModel::default();
    model.title = String::from("Sempal Native");
    model.backend_label = String::from("radiant/native_vello");
    model.transport_running = true;
    model.status.left = String::from("Ready");
    model.status.center = String::from("rows: 18 | selected: 1 | anchor: — | search: —");
    model.status.right = String::from("col: 2/3");
    model.status_text = String::from("Startup scene");
    model.sources.header = String::from("Sources");
    model.sources.search_query = String::from("kick");
    model.sources.folder_search_query = String::from("drum");
    model.sources.selected_row = Some(0);
    model.sources.focused_folder_row = Some(0);
    model.sources.folder_actions.can_create_folder = true;
    model.sources.folder_actions.can_create_folder_at_root = true;
    model.sources.folder_actions.can_rename_folder = true;
    model.sources.folder_actions.can_delete_folder = true;
    model.sources.folder_actions.can_clear_recovery_log = true;
    model.sources.folder_recovery.in_progress = false;
    model.sources.folder_recovery.entry_count = 3;
    for index in 0..8 {
        model.sources.rows.push(SourceRowModel::new(
            format!("source_{index:02}"),
            format!("/samples/source_{index:02}"),
            index == 1,
            false,
        ));
    }
    for index in 0..10 {
        model.sources.folder_rows.push(FolderRowModel::new(
            format!("folder_{index:02}"),
            String::new(),
            index % 2,
            index == 0,
            index == 1,
            index == 0,
            false,
            false,
        ));
    }
    model.columns[0].item_count = 5;
    model.columns[1].item_count = 12;
    model.columns[2].item_count = 1;
    model.browser.visible_count = 12;
    model.browser.selected_visible_row = Some(1);
    model.browser.anchor_visible_row = Some(1);
    model.browser.search_placeholder = Some(String::from("Search samples"));
    model.browser.search_query = String::from("kick");
    model.browser.sort_label = Some(String::from("List order"));
    model.browser.active_tab_label = Some(String::from("Samples"));
    model.browser_chrome.item_count_label = String::from("12 items");
    model.browser_actions.can_delete = true;
    model.browser_actions.can_tag = true;
    model.browser_actions.can_rename = true;
    model
}

fn browser_dense_model() -> AppModel {
    let mut model = AppModel::default();
    model.title = String::from("Sempal - Dense Browser");
    model.status.left = String::from("Focus list");
    model.status.center = String::from("rows: 500 | selected: 7 | anchor: 72 | search: dense");
    model.status.right = String::from("col: 2/3");
    model.status_text = String::from("Dense browser scene");
    model.transport_running = true;

    for index in 0..14 {
        model.sources.rows.push(SourceRowModel::new(
            format!("source_{index:02}"),
            format!("/source_{index:02}.wav"),
            index == 3,
            false,
        ));
    }

    for index in 0..16 {
        model.sources.folder_rows.push(FolderRowModel::new(
            format!("folder_{index:02}"),
            String::new(),
            index % 3,
            index == 8,
            index == 1,
            index == 0,
            true,
            true,
        ));
    }

    for index in 0..500 {
        let mut row = BrowserRowModel::new(
            index,
            format!("row_{index:03}.wav"),
            index % 3,
            index % 11 == 0,
            index == 72,
        );
        if index % 5 == 0 {
            row = row.with_bucket_label(format!("BPM {index}"));
        }
        model.browser.rows.push(row);
    }

    model.browser.visible_count = 500;
    model.browser.selected_path_count = 7;
    model.browser.selected_visible_row = Some(72);
    model.browser.anchor_visible_row = Some(68);
    model.browser.search_query = String::from("kick");
    model.browser.search_placeholder = Some(String::from("Search samples"));
    model.browser.sort_label = Some(String::from("Name"));
    model.browser.active_tab_label = Some(String::from("Samples"));
    model.browser_chrome.item_count_label = String::from("500 items");
    model.browser_actions.can_delete = true;
    model.browser_actions.can_tag = true;
    model.browser_actions.can_rename = false;
    model.columns[0].item_count = 40;
    model.columns[1].item_count = 460;
    model.columns[2].item_count = 0;
    model
}

fn waveform_waveform_image(width: usize, height: usize) -> ImageRgba {
    let mut pixels = Vec::with_capacity(width.saturating_mul(height).saturating_mul(4));
    for y in 0..height {
        for x in 0..width {
            let x_u8 = u8::try_from(x * 5).unwrap_or(0);
            let y_u8 = u8::try_from((y * 13) % 256).unwrap_or(0);
            let alpha = if (x + y) % 9 == 0 {
                255
            } else if (x + y) % 13 == 0 {
                128
            } else {
                200
            };
            let color = x_u8.saturating_add(y_u8 / 2);
            pixels.extend_from_slice(&[color, 255_u8.wrapping_sub(color), color / 2, alpha]);
        }
    }
    ImageRgba::new(width, height, pixels).unwrap_or_else(|| {
        panic!(
            "failed to construct waveform fixture image ({width}x{height}) with {} px",
            width.saturating_mul(height).saturating_mul(4)
        )
    })
}

fn waveform_selection_model() -> AppModel {
    let mut model = AppModel::default();
    model.title = String::from("Sempal Native Waveform");
    model.status.left = String::from("Waveform focus");
    model.status.center = String::from("rows: 48 | selected: 2 | anchor: 1 | search: wav");
    model.status.right = String::from("col: 2/3");
    model.status_text = String::from("Waveform scene");
    model.transport_running = true;
    model.sources.search_query = String::from("sample");
    for index in 0..20 {
        model.sources.rows.push(SourceRowModel::new(
            format!("source_{index:02}"),
            String::new(),
            index == 9,
            false,
        ));
        model.browser.rows.push(
            BrowserRowModel::new(
                index,
                format!("track_{index:03}.wav"),
                index % 3,
                index % 6 == 0,
                index == 1,
            )
            .with_bucket_label(format!("{} bpm", 90 + index)),
        );
    }
    model.browser.visible_count = 20;
    model.browser.selected_visible_row = Some(1);
    model.browser.anchor_visible_row = Some(1);
    model.browser.search_query = String::from("track");
    model.browser.selected_path_count = 2;
    model.browser_actions.can_delete = true;
    model.browser_actions.can_tag = true;
    model.browser_actions.can_rename = true;

    model.waveform.loaded_label = Some(String::from("track_001.wav"));
    model.waveform.cursor_milli = Some(315);
    model.waveform.playhead_milli = Some(620);
    model.waveform.selection_milli = Some(crate::app::NormalizedRangeModel::new(200, 760));
    model.waveform.view_start_milli = 50;
    model.waveform.view_end_milli = 950;
    model.waveform.loop_enabled = true;
    model.waveform.tempo_label = Some(String::from("128.0 BPM"));
    model.waveform.zoom_label = Some(String::from("125%"));
    model.waveform.waveform_image = Some(std::sync::Arc::new(waveform_waveform_image(160, 32)));
    model.waveform.waveform_image_signature = Some(7_123_456);
    model.waveform_chrome.transport_hint = String::from("Loop enabled");

    model.map.active = false;
    model.map.summary = String::from("Waveform scene");

    model.browser_chrome.item_count_label = String::from("20 items");
    model.update.status = crate::app::UpdateStatusModel::Idle;
    model.update.status_label = String::from("Updates: idle");
    model.update.action_hint_label = String::from("Action: check");
    model
}

#[test]
fn startup_shot_matches_fixture() {
    write_or_compare_shot(
        "startup",
        Vector2::new(1280.0, 720.0),
        startup_scene_model(),
        false,
    );
}

#[test]
fn browser_dense_shot_matches_fixture() {
    write_or_compare_shot(
        "browser_dense",
        Vector2::new(1600.0, 900.0),
        browser_dense_model(),
        false,
    );
}

#[test]
fn waveform_selection_shot_matches_fixture() {
    write_or_compare_shot(
        "waveform_selection",
        Vector2::new(1440.0, 810.0),
        waveform_selection_model(),
        false,
    );
}

#[ignore = "Generate snapshot fixtures with `cargo test --package radiant native_shell::shots::update_shot_fixtures -- --ignored`"]
#[test]
fn update_shot_fixtures() {
    write_or_compare_shot(
        "startup",
        Vector2::new(1280.0, 720.0),
        startup_scene_model(),
        true,
    );
    write_or_compare_shot(
        "browser_dense",
        Vector2::new(1600.0, 900.0),
        browser_dense_model(),
        true,
    );
    write_or_compare_shot(
        "waveform_selection",
        Vector2::new(1440.0, 810.0),
        waveform_selection_model(),
        true,
    );
}
