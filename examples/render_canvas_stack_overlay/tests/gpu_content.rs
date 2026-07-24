use crate::gpu_content::demo_render_canvas_content;
use radiant::runtime::RenderCanvasContent;
use std::sync::Arc;

#[test]
fn demo_render_canvas_content_reuses_static_atlas_payload() {
    let first = demo_render_canvas_content();
    let second = demo_render_canvas_content();

    let (
        RenderCanvasContent::RgbaAtlas { atlas: first, .. },
        RenderCanvasContent::RgbaAtlas { atlas: second, .. },
    ) = (&first, &second)
    else {
        panic!("demo content should remain an atlas-backed render canvas");
    };
    assert!(
        Arc::ptr_eq(&first, &second),
        "view reprojection should reuse the static GPU atlas instead of rebuilding pixel data"
    );
}
