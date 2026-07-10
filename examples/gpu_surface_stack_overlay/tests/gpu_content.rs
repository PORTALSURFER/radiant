use crate::gpu_content::demo_gpu_content;
use radiant::runtime::GpuSurfaceContent;
use std::sync::Arc;

#[test]
fn demo_gpu_content_reuses_static_atlas_payload() {
    let first = demo_gpu_content();
    let second = demo_gpu_content();

    let (
        GpuSurfaceContent::RgbaAtlas { atlas: first, .. },
        GpuSurfaceContent::RgbaAtlas { atlas: second, .. },
    ) = (&first, &second)
    else {
        panic!("demo content should remain an atlas-backed GPU surface");
    };
    assert!(
        Arc::ptr_eq(&first, &second),
        "view reprojection should reuse the static GPU atlas instead of rebuilding pixel data"
    );
}
