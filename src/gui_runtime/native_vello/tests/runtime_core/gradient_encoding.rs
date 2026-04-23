use super::super::*;
use crate::gui::native_shell::FillLinearGradient;

const DRAW_TAG_COLOR: u32 = 0x44;
const DRAW_TAG_LINEAR_GRADIENT: u32 = 0x114;

#[test]
fn encode_frame_to_scene_uses_direct_linear_gradient_draw_tag() {
    let frame = NativeViewFrame {
        primitives: vec![Primitive::LinearGradient(FillLinearGradient {
            rect: UiRect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 80.0)),
            start: Point::new(10.0, 20.0),
            end: Point::new(110.0, 20.0),
            start_color: Rgba8 {
                r: 20,
                g: 30,
                b: 40,
                a: 64,
            },
            end_color: Rgba8 {
                r: 120,
                g: 130,
                b: 140,
                a: 192,
            },
        })],
        ..NativeViewFrame::default()
    };
    let mut scene = Scene::new();
    let mut text_renderer = NativeTextRenderer::new();
    let mut image_upload_blob_cache = HashMap::<ImageUploadBlobCacheKey, Blob<u8>>::new();
    let mut image_upload_blob_cache_order = VecDeque::<ImageUploadBlobCacheKey>::new();

    NativeVelloRunner::<PreviewBridge>::encode_frame_to_scene(
        &frame,
        &mut scene,
        &mut text_renderer,
        &mut image_upload_blob_cache,
        &mut image_upload_blob_cache_order,
    );

    let encoding = scene.encoding();
    assert_eq!(encoding.n_paths, 1);
    assert_eq!(
        encoding
            .draw_tags
            .iter()
            .filter(|tag| tag.0 == DRAW_TAG_LINEAR_GRADIENT)
            .count(),
        1,
        "expected direct Vello linear-gradient encoding"
    );
    assert!(
        !encoding.draw_tags.iter().any(|tag| tag.0 == DRAW_TAG_COLOR),
        "gradient primitives must not encode as solid-color rect bursts"
    );
}
