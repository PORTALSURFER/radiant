use super::*;

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
fn retained_frame_cache_policy_can_disable_storage() {
    let rect = UiRect::from_min_size(Point::new(0.0, 0.0), Vector2::new(20.0, 20.0));
    let viewport = Vector2::new(100.0, 100.0);
    let mut cache =
        RetainedSurfaceFrameCache::with_policy(RetainedSurfaceCachePolicy { max_frames: 0 });

    cache.store(descriptor(1), rect, viewport, frame(1));

    assert_eq!(cache.entry_count(), 0);
    assert!(cache.cached_frame(descriptor(1), rect, viewport).is_none());
}
