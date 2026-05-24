use std::{fs, path::PathBuf};

#[test]
fn vertex_buffer_upload_is_non_panicking() {
    let source = fs::read_to_string(
        "src/gui_runtime/native_vello/generic_runtime/post_gpu_overlay/buffer.rs",
    )
    .expect("post GPU overlay vertex buffer should be readable");

    assert!(
        !source.contains(".expect(") && !source.contains(".unwrap("),
        "post GPU overlay vertex buffer upload should handle missing cached buffers without panicking"
    );
}

#[test]
fn geometry_tests_stay_grouped_by_replay_concern() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root =
        fs::read_to_string(manifest_dir.join(
            "src/gui_runtime/native_vello/generic_runtime/post_gpu_overlay/geometry/tests.rs",
        ))
        .expect("post GPU overlay geometry test root should be readable");
    let suffix = fs::read_to_string(manifest_dir.join(
        "src/gui_runtime/native_vello/generic_runtime/post_gpu_overlay/geometry/tests/suffix.rs",
    ))
    .expect("post GPU overlay suffix tests should be readable");
    let vertices = fs::read_to_string(manifest_dir.join(
        "src/gui_runtime/native_vello/generic_runtime/post_gpu_overlay/geometry/tests/vertices.rs",
    ))
    .expect("post GPU overlay vertex tests should be readable");
    let regions = fs::read_to_string(manifest_dir.join(
        "src/gui_runtime/native_vello/generic_runtime/post_gpu_overlay/geometry/tests/regions.rs",
    ))
    .expect("post GPU overlay region tests should be readable");
    let fixtures = fs::read_to_string(manifest_dir.join(
        "src/gui_runtime/native_vello/generic_runtime/post_gpu_overlay/geometry/tests/fixtures.rs",
    ))
    .expect("post GPU overlay geometry fixtures should be readable");

    assert!(
        root.contains("mod fixtures;")
            && root.contains("mod suffix;")
            && root.contains("mod vertices;")
            && root.contains("mod regions;")
            && !root.contains("fn replayable_vertices_batch_fill_and_stroke_rectangles"),
        "post GPU overlay geometry test root should index focused replay groups instead of owning all cases"
    );
    assert!(
        suffix.contains("fn replayable_suffix_starts_after_last_gpu_surface")
            && vertices.contains("fn replayable_vertices_batch_fill_and_stroke_rectangles")
            && regions.contains(
                "fn replayable_vertices_in_regions_clip_translucent_fills_to_gpu_regions"
            )
            && fixtures.contains("fn translucent_white"),
        "post GPU overlay geometry tests should stay grouped by suffix, full-target vertices, region clipping, and fixtures"
    );
}

#[test]
fn bitmap_text_stays_out_of_geometry_root() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let geometry = fs::read_to_string(
        manifest_dir
            .join("src/gui_runtime/native_vello/generic_runtime/post_gpu_overlay/geometry.rs"),
    )
    .expect("post GPU overlay geometry root should be readable");
    let text = fs::read_to_string(
        manifest_dir
            .join("src/gui_runtime/native_vello/generic_runtime/post_gpu_overlay/geometry/text.rs"),
    )
    .expect("post GPU overlay bitmap text helper should be readable");

    assert!(
        geometry.contains("mod text;")
            && geometry.contains("use text::push_text_vertices;")
            && !geometry.contains("fn glyph_rows"),
        "post GPU overlay geometry should delegate bitmap glyph policy to geometry/text.rs"
    );
    assert!(
        text.contains("struct BitmapTextLayout")
            && text.contains("fn glyph_rows")
            && text.contains("fn push_glyph_vertices"),
        "post GPU overlay bitmap text layout and glyph replay should stay in geometry/text.rs"
    );
}
