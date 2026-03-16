use std::{fs, path::PathBuf};

use super::*;

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

pub(super) fn write_or_compare_shot(name: &str, viewport: Vector2, model: AppModel, write_mode: bool) {
    let snapshot = snapshot::build_snapshot(name, viewport, &model);
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
        raster::rasterize_shot(&snapshot)
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
    let expected_json = snapshot::canonicalize_json(expected_json);
    let actual_json: serde_json::Value = serde_json::from_str(
        &serde_json::to_string_pretty(&snapshot)
            .unwrap_or_else(|err| panic!("serialize actual shot snapshot for {name}: {err}")),
    )
    .unwrap_or_else(|err| panic!("parse actual shot snapshot for {name}: {err}"));
    let actual_json = snapshot::canonicalize_json(actual_json);
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
    let actual = raster::rasterize_shot(&snapshot);
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
