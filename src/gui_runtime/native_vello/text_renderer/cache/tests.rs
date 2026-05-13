use super::*;

fn cached_layout(text: &str, stamp: u64) -> CachedTextLayout {
    CachedTextLayout {
        layout: TextLayout::empty_for(text),
        stamp,
    }
}

fn layout_key(label: &str) -> TextLayoutKey {
    TextLayoutKey {
        text: Arc::from(label),
        font_size_bits: 12.0_f32.to_bits(),
    }
}

#[test]
fn layout_cache_eviction_keeps_recently_used_entries() {
    let mut cache = TextLayoutCache::new();
    for index in 0..TEXT_LAYOUT_CACHE_CAPACITY {
        let key = layout_key(&format!("label-{index}"));
        cache
            .layout_cache
            .insert(key.clone(), cached_layout(key.text.as_ref(), 0));
        cache.touch_layout_cache_key(&key);
    }

    let hot_key = layout_key("label-0");
    cache.touch_layout_cache_key(&hot_key);
    cache.evict_stale_layouts();

    let fresh_key = layout_key("label-fresh");
    cache
        .layout_cache
        .insert(fresh_key.clone(), cached_layout(fresh_key.text.as_ref(), 0));
    cache.touch_layout_cache_key(&fresh_key);

    assert!(cache.layout_cache.contains_key(&hot_key));
    assert!(cache.layout_cache.contains_key(&fresh_key));
    assert!(cache.layout_cache.len() <= TEXT_LAYOUT_CACHE_CAPACITY);
    assert_eq!(cache.text_layout_evictions, 1);
}

#[test]
fn layout_cache_hit_queue_compacts_after_repeated_reuse() {
    let mut cache = TextLayoutCache::new();
    let key = layout_key("content row");
    cache
        .layout_cache
        .insert(key.clone(), cached_layout(key.text.as_ref(), 0));
    cache.touch_layout_cache_key(&key);

    for _ in 0..=TEXT_LAYOUT_CACHE_CAPACITY.saturating_mul(2) {
        cache.touch_layout_cache_key(&key);
    }

    assert_eq!(cache.layout_cache.len(), 1);
    assert!(cache.layout_cache_order.len() <= TEXT_LAYOUT_CACHE_CAPACITY);
}
