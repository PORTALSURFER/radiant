use super::*;
use crate::{
    gui::{
        paint::{DrawImage, DrawSvg},
        types::{ImageRgba, Point, Rgba8},
    },
    runtime::PaintSvgDocument,
};
use std::sync::Arc;

fn descriptor(key: u64) -> RetainedSurfaceDescriptor {
    RetainedSurfaceDescriptor {
        key,
        revision: 1,
        dirty_mask: 0,
        volatile: false,
    }
}

fn frame(red: u8) -> PaintFrame {
    PaintFrame {
        clear_color: Rgba8 {
            r: red,
            g: 0,
            b: 0,
            a: 255,
        },
        ..PaintFrame::default()
    }
}

#[test]
fn retained_frame_cache_presizes_storage_from_policy() {
    let cache =
        RetainedSurfaceFrameCache::with_policy(RetainedSurfaceCachePolicy { max_frames: 16 });

    assert_eq!(cache.entry_count(), 0);
    assert!(cache.entries.capacity() >= 16);
}

#[test]
fn retained_frame_cache_default_presizes_standard_storage() {
    let cache = RetainedSurfaceFrameCache::default();

    assert_eq!(cache.entry_count(), 0);
    assert!(cache.entries.capacity() >= cache.policy().max_frames);
}

#[test]
fn retained_frame_cache_disabled_policy_does_not_allocate_storage() {
    let cache =
        RetainedSurfaceFrameCache::with_policy(RetainedSurfaceCachePolicy { max_frames: 0 });

    assert_eq!(cache.entry_count(), 0);
    assert_eq!(cache.entries.capacity(), 0);
}

#[test]
fn retained_frame_cache_evicts_oldest_entry_without_shifting_storage() {
    let rect = UiRect::from_min_size(Point::new(0.0, 0.0), Vector2::new(20.0, 20.0));
    let viewport = Vector2::new(100.0, 100.0);
    let mut cache =
        RetainedSurfaceFrameCache::with_policy(RetainedSurfaceCachePolicy { max_frames: 64 });

    for key in 0..=64 {
        cache.store(descriptor(key), rect, viewport, frame(key as u8));
    }

    assert_eq!(cache.entries.len(), 64);
    assert!(cache.cached_frame(descriptor(0), rect, viewport).is_none());
    assert_eq!(
        cache
            .cached_frame(descriptor(1), rect, viewport)
            .map(|frame| frame.clear_color.r),
        Some(1)
    );
    assert_eq!(
        cache
            .cached_frame(descriptor(64), rect, viewport)
            .map(|frame| frame.clear_color.r),
        Some(64)
    );
}

#[test]
fn retained_frame_cache_hit_refreshes_eviction_recency() {
    let rect = UiRect::from_min_size(Point::new(0.0, 0.0), Vector2::new(20.0, 20.0));
    let viewport = Vector2::new(100.0, 100.0);
    let mut cache =
        RetainedSurfaceFrameCache::with_policy(RetainedSurfaceCachePolicy { max_frames: 4 });

    for key in 0..4 {
        cache.store(descriptor(key), rect, viewport, frame(key as u8));
    }

    assert_eq!(
        cache
            .cached_frame(descriptor(0), rect, viewport)
            .map(|frame| frame.clear_color.r),
        Some(0)
    );
    cache.store(descriptor(4), rect, viewport, frame(4));

    assert_eq!(cache.entry_count(), 4);
    assert_eq!(
        cache
            .cached_frame(descriptor(0), rect, viewport)
            .map(|frame| frame.clear_color.r),
        Some(0)
    );
    assert!(cache.cached_frame(descriptor(1), rect, viewport).is_none());
    assert_eq!(
        cache
            .cached_frame(descriptor(4), rect, viewport)
            .map(|frame| frame.clear_color.r),
        Some(4)
    );
}

#[test]
fn retained_frame_cache_most_recent_hit_keeps_entry_order() {
    let rect = UiRect::from_min_size(Point::new(0.0, 0.0), Vector2::new(20.0, 20.0));
    let viewport = Vector2::new(100.0, 100.0);
    let mut cache =
        RetainedSurfaceFrameCache::with_policy(RetainedSurfaceCachePolicy { max_frames: 4 });

    cache.store(descriptor(1), rect, viewport, frame(1));
    cache.store(descriptor(2), rect, viewport, frame(2));

    assert_eq!(
        cache
            .cached_frame(descriptor(2), rect, viewport)
            .map(|frame| frame.clear_color.r),
        Some(2)
    );
    assert_eq!(
        cache
            .entries
            .iter()
            .map(|entry| entry.descriptor.key)
            .collect::<Vec<_>>(),
        vec![1, 2]
    );
}

#[test]
fn retained_frame_cache_recent_non_tail_hit_moves_to_back() {
    let rect = UiRect::from_min_size(Point::new(0.0, 0.0), Vector2::new(20.0, 20.0));
    let viewport = Vector2::new(100.0, 100.0);
    let mut cache =
        RetainedSurfaceFrameCache::with_policy(RetainedSurfaceCachePolicy { max_frames: 4 });

    cache.store(descriptor(1), rect, viewport, frame(1));
    cache.store(descriptor(2), rect, viewport, frame(2));
    cache.store(descriptor(3), rect, viewport, frame(3));

    assert_eq!(
        cache
            .cached_frame(descriptor(2), rect, viewport)
            .map(|frame| frame.clear_color.r),
        Some(2)
    );
    assert_eq!(
        cache
            .entries
            .iter()
            .map(|entry| entry.descriptor.key)
            .collect::<Vec<_>>(),
        vec![1, 3, 2]
    );
}

#[test]
fn retained_frame_cache_replaces_most_recent_matching_geometry_in_place() {
    let rect = UiRect::from_min_size(Point::new(0.0, 0.0), Vector2::new(20.0, 20.0));
    let viewport = Vector2::new(100.0, 100.0);
    let mut cache =
        RetainedSurfaceFrameCache::with_policy(RetainedSurfaceCachePolicy { max_frames: 4 });

    cache.store(descriptor(1), rect, viewport, frame(1));
    cache.store(
        RetainedSurfaceDescriptor {
            revision: 2,
            ..descriptor(1)
        },
        rect,
        viewport,
        frame(9),
    );

    assert_eq!(cache.entry_count(), 1);
    assert_eq!(
        cache
            .entries
            .iter()
            .map(|entry| (entry.descriptor.key, entry.descriptor.revision))
            .collect::<Vec<_>>(),
        vec![(1, 2)]
    );
    assert_eq!(
        cache
            .cached_frame(
                RetainedSurfaceDescriptor {
                    revision: 2,
                    ..descriptor(1)
                },
                rect,
                viewport,
            )
            .map(|frame| frame.clear_color.r),
        Some(9)
    );
}

#[test]
fn retained_frame_cache_policy_can_disable_storage() {
    let rect = UiRect::from_min_size(Point::new(0.0, 0.0), Vector2::new(20.0, 20.0));
    let viewport = Vector2::new(100.0, 100.0);
    let mut cache =
        RetainedSurfaceFrameCache::with_policy(RetainedSurfaceCachePolicy { max_frames: 0 });

    cache.store(descriptor(1), rect, viewport, frame(1));

    assert_eq!(cache.entry_count(), 0);
    assert!(cache.cached_frame(descriptor(1), rect, viewport).is_none());
}

#[test]
fn retained_frame_stats_count_media_primitives() {
    let rect = UiRect::from_min_size(Point::new(0.0, 0.0), Vector2::new(20.0, 20.0));
    let image = Arc::new(ImageRgba::new(1, 1, vec![255, 0, 0, 255]).expect("valid image"));
    let svg = PaintSvgDocument::from_svg(
        r##"<svg viewBox="0 0 4 4"><rect width="4" height="4"/></svg>"##,
    )
    .expect("valid svg");
    let frame = PaintFrame {
        primitives: vec![
            Primitive::Image(DrawImage {
                rect,
                image: Arc::clone(&image),
            }),
            Primitive::Svg(DrawSvg {
                rect,
                document: svg,
            }),
            Primitive::Image(DrawImage { rect, image }),
        ],
        ..PaintFrame::default()
    };
    let mut stats = RetainedSurfaceEncodeStats::default();

    stats.record_retained_frame(&frame);

    assert_eq!(stats.retained_frame_primitive_count, 3);
    assert_eq!(stats.image_count, 2);
    assert_eq!(stats.svg_document_count, 1);
}
