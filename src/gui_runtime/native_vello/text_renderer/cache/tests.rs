use super::*;

fn cached_layout(text: &str, stamp: u64) -> CachedTextLayout {
    CachedTextLayout {
        layout: TextLayout::empty_for(text),
        stamp,
    }
}

fn cached_layout_with_glyph_diagnostics(
    text: &str,
    stamp: u64,
    unsupported_shaping_runs: u64,
    unsupported_shaping_scalars: u64,
    fallback_glyphs: u64,
    missing_glyphs: u64,
) -> CachedTextLayout {
    let mut layout = TextLayout::empty_for(text);
    layout.unsupported_shaping_runs = unsupported_shaping_runs;
    layout.unsupported_shaping_scalars = unsupported_shaping_scalars;
    layout.fallback_glyphs = fallback_glyphs;
    layout.missing_glyphs = missing_glyphs;
    CachedTextLayout { layout, stamp }
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
    assert_eq!(cache.layout_profile.evictions, 1);
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

#[test]
fn cached_layout_hits_report_glyph_diagnostics_for_current_frame() {
    let mut cache = TextLayoutCache::new();
    let key = layout_key("fallback row");
    cache.layout_cache.insert(
        key.clone(),
        cached_layout_with_glyph_diagnostics(key.text.as_ref(), 0, 1, 4, 2, 1),
    );
    cache.touch_layout_cache_key(&key);

    let _ = cache.record_layout_cache_hit(&key);

    let counters = cache.take_profile_counters();
    assert_eq!(counters.layout.hits, 1);
    assert_eq!(counters.quality.unsupported_shaping_runs, 1);
    assert_eq!(counters.quality.unsupported_shaping_scalars, 4);
    assert_eq!(counters.quality.fallback_glyphs, 2);
    assert_eq!(counters.quality.missing_glyphs, 1);
}
