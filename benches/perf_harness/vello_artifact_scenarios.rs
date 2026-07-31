//! Direct-Vello strategy probes for resource-free paint-segment artifacts.
//!
//! These fixtures intentionally bypass Radiant runtime types. They only compare
//! the Vello encoding work represented by four independent rectangle segments.

use crate::runner::ScenarioCounters;
use std::hint::black_box;
use vello::{
    Scene,
    kurbo::{Affine, Rect},
    peniko::{Color, Fill},
};

const SEGMENT_COUNT: usize = 4;
const PRIMITIVES_PER_SEGMENT: usize = 256;
const TOTAL_PRIMITIVES: usize = SEGMENT_COUNT * PRIMITIVES_PER_SEGMENT;
const CHANGED_SEGMENT: usize = 2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct EncodingCounts {
    path_tags: usize,
    path_data: usize,
    draw_tags: usize,
    draw_data: usize,
    transforms: usize,
    styles: usize,
    n_paths: u32,
    n_path_segments: u32,
    n_clips: u32,
    n_open_clips: u32,
}

impl EncodingCounts {
    fn from_scene(scene: &Scene) -> Self {
        let encoding = scene.encoding();
        Self {
            path_tags: encoding.path_tags.len(),
            path_data: encoding.path_data.len(),
            draw_tags: encoding.draw_tags.len(),
            draw_data: encoding.draw_data.len(),
            transforms: encoding.transforms.len(),
            styles: encoding.styles.len(),
            n_paths: encoding.n_paths,
            n_path_segments: encoding.n_path_segments,
            n_clips: encoding.n_clips,
            n_open_clips: encoding.n_open_clips,
        }
    }
}

pub(super) fn strategy_4x256_full_reencode() -> impl FnMut() -> ScenarioCounters {
    let expected = expected_final_encoding_counts();
    let mut destination = Scene::new();
    move || {
        destination.reset();
        let encoded_paint_primitive_count = encode_segments(&mut destination, false);
        assert_eq!(encoded_paint_primitive_count, TOTAL_PRIMITIVES);
        assert_eq!(EncodingCounts::from_scene(&destination), expected);
        black_box(destination.encoding());
        ScenarioCounters::default()
            .with_encoded_paint_primitive_count(encoded_paint_primitive_count as u64)
            .with_scene_append_count(0)
    }
}

pub(super) fn strategy_4x256_append_one_changed() -> impl FnMut() -> ScenarioCounters {
    let expected = expected_final_encoding_counts();
    let retained_segments = (0..SEGMENT_COUNT)
        .map(|segment| segment_scene(segment, false))
        .collect::<Vec<_>>();
    let mut changed_segment = Scene::new();
    let mut destination = Scene::new();
    move || {
        changed_segment.reset();
        let encoded_paint_primitive_count =
            encode_segment(&mut changed_segment, CHANGED_SEGMENT, true);
        destination.reset();
        let mut scene_append_count = 0;
        for (segment, retained_scene) in retained_segments.iter().enumerate() {
            let scene = if segment == CHANGED_SEGMENT {
                &changed_segment
            } else {
                retained_scene
            };
            destination.append(scene, None);
            scene_append_count += 1;
        }
        assert_eq!(encoded_paint_primitive_count, PRIMITIVES_PER_SEGMENT);
        assert_eq!(scene_append_count, SEGMENT_COUNT);
        assert_eq!(EncodingCounts::from_scene(&destination), expected);
        black_box(destination.encoding());
        ScenarioCounters::default()
            .with_encoded_paint_primitive_count(encoded_paint_primitive_count as u64)
            .with_scene_append_count(scene_append_count as u64)
    }
}

fn expected_final_encoding_counts() -> EncodingCounts {
    let mut baseline = Scene::new();
    assert_eq!(encode_segments(&mut baseline, false), TOTAL_PRIMITIVES);
    EncodingCounts::from_scene(&baseline)
}

fn segment_scene(segment: usize, changed: bool) -> Scene {
    let mut scene = Scene::new();
    encode_segment(&mut scene, segment, changed);
    scene
}

fn encode_segments(scene: &mut Scene, changed: bool) -> usize {
    (0..SEGMENT_COUNT)
        .map(|segment| encode_segment(scene, segment, changed && segment == CHANGED_SEGMENT))
        .sum()
}

fn encode_segment(scene: &mut Scene, segment: usize, changed: bool) -> usize {
    scene.encoding_mut().force_next_transform_and_style();
    let color = if changed {
        Color::from_rgb8(220, 80, 120)
    } else {
        Color::from_rgb8(80 + (segment as u8 * 20), 120, 180)
    };
    let transform = Affine::new([1.0, 0.0, 0.0, 1.0, segment as f64, 0.0]);
    for index in 0..PRIMITIVES_PER_SEGMENT {
        let column = index % 16;
        let row = index / 16;
        let x = (segment * 320 + column * 18) as f64;
        let y = (row * 12) as f64;
        scene.fill(
            Fill::NonZero,
            transform,
            color,
            None,
            &Rect::new(x, y, x + 16.0, y + 10.0),
        );
    }
    PRIMITIVES_PER_SEGMENT
}
