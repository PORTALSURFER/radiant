//! Regression fixtures for the native shell visual scene graph.

use super::*;
use crate::{
    app::{AppModel, BrowserRowModel, FolderRowModel, SourceRowModel},
    gui::types::{ImageRgba, Point, Rgba8, Vector2},
};

mod fixtures;
mod models;
mod raster;
mod snapshot;

fn write_or_compare_shot(name: &str, viewport: Vector2, model: AppModel, write_mode: bool) {
    fixtures::write_or_compare_shot(name, viewport, model, write_mode);
}

#[test]
fn startup_shot_matches_fixture() {
    write_or_compare_shot(
        "startup",
        Vector2::new(1280.0, 720.0),
        models::startup_scene_model(),
        false,
    );
}

#[test]
fn browser_dense_shot_matches_fixture() {
    write_or_compare_shot(
        "browser_dense",
        Vector2::new(1600.0, 900.0),
        models::browser_dense_model(),
        false,
    );
}

#[test]
fn waveform_selection_shot_matches_fixture() {
    write_or_compare_shot(
        "waveform_selection",
        Vector2::new(1440.0, 810.0),
        models::waveform_selection_model(),
        false,
    );
}

#[ignore = "Generate snapshot fixtures with `cargo test --package radiant native_shell::shots::update_shot_fixtures -- --ignored`"]
#[test]
fn update_shot_fixtures() {
    write_or_compare_shot(
        "startup",
        Vector2::new(1280.0, 720.0),
        models::startup_scene_model(),
        true,
    );
    write_or_compare_shot(
        "browser_dense",
        Vector2::new(1600.0, 900.0),
        models::browser_dense_model(),
        true,
    );
    write_or_compare_shot(
        "waveform_selection",
        Vector2::new(1440.0, 810.0),
        models::waveform_selection_model(),
        true,
    );
}
