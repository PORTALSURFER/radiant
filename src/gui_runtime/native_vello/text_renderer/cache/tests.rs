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

fn layout_key(label: &str) -> TextViewKey {
    let shape = TextLayoutKey {
        text: Arc::from(label),
        font_size_bits: 12.0_f32.to_bits(),
        presentation: Default::default(),
        font_generation: 0,
    };
    TextViewKey {
        shape,
        width_bits: u32::MAX,
        align: TextAlign::Left,
        wrap: TextWrap::None,
        break_policy_id: super::super::model::LINE_BREAK_POLICY_ID,
    }
}

#[test]
fn layout_cache_eviction_keeps_recently_used_entries() {
    let mut cache = TextLayoutCache::new();
    for index in 0..VIEW_CACHE_ENTRY_BUDGET {
        let key = layout_key(&format!("label-{index}"));
        cache
            .view_cache
            .insert(key.clone(), cached_layout(key.shape.text.as_ref(), 0));
        cache.touch_view_cache_key(&key);
    }

    let hot_key = layout_key("label-0");
    cache.touch_view_cache_key(&hot_key);
    cache.evict_view_cache_until(0);

    let fresh_key = layout_key("label-fresh");
    cache.view_cache.insert(
        fresh_key.clone(),
        cached_layout(fresh_key.shape.text.as_ref(), 0),
    );
    cache.touch_view_cache_key(&fresh_key);

    assert!(cache.view_cache.contains_key(&hot_key));
    assert!(cache.view_cache.contains_key(&fresh_key));
    assert!(cache.view_cache.len() <= VIEW_CACHE_ENTRY_BUDGET);
    assert_eq!(cache.layout_profile.evictions, 1);
}

#[test]
fn layout_cache_hit_queue_compacts_after_repeated_reuse() {
    let mut cache = TextLayoutCache::new();
    let key = layout_key("content row");
    cache
        .view_cache
        .insert(key.clone(), cached_layout(key.shape.text.as_ref(), 0));
    cache.touch_view_cache_key(&key);

    for _ in 0..=VIEW_CACHE_ENTRY_BUDGET.saturating_mul(2) {
        cache.touch_view_cache_key(&key);
    }

    assert_eq!(cache.view_cache.len(), 1);
    assert!(cache.view_cache_order.len() <= VIEW_CACHE_ENTRY_BUDGET);
}

#[test]
fn consecutive_layout_cache_touches_coalesce_recency_queue_entry() {
    let mut cache = TextLayoutCache::new();
    let key = layout_key("same label");
    cache
        .view_cache
        .insert(key.clone(), cached_layout(key.shape.text.as_ref(), 0));

    cache.touch_view_cache_key(&key);
    cache.touch_view_cache_key(&key);
    cache.touch_view_cache_key(&key);

    assert_eq!(cache.view_cache_order.len(), 1);
    assert_eq!(cache.view_cache_order[0].0, key);
    assert_eq!(cache.view_cache_order[0].1, 3);
}

#[test]
fn cached_layout_hits_report_glyph_diagnostics_for_current_frame() {
    let mut cache = TextLayoutCache::new();
    let key = layout_key("fallback row");
    cache.view_cache.insert(
        key.clone(),
        cached_layout_with_glyph_diagnostics(key.shape.text.as_ref(), 0, 1, 4, 2, 1),
    );
    cache.touch_view_cache_key(&key);

    let _ = cache.record_view_cache_hit(&key);

    let counters = cache.take_profile_counters();
    assert_eq!(counters.layout.hits, 1);
    assert_eq!(counters.quality.unsupported_shaping_runs, 1);
    assert_eq!(counters.quality.unsupported_shaping_scalars, 4);
    assert_eq!(counters.quality.fallback_glyphs, 2);
    assert_eq!(counters.quality.missing_glyphs, 1);
}

#[test]
fn cached_face_indices_remain_valid_when_font_stack_appends() {
    let mut cache = TextLayoutCache::new();
    let mut stack = super::super::font::NativeFontStack::from_test_bytes(&[include_bytes!(
        "../../../../../tests/fixtures/fonts/primary.ttf"
    )]);

    let first_face = cache
        .layout_for(&mut stack, "A", 20.0)
        .expect("primary fixture loads")
        .glyphs[0]
        .face_index;
    assert_eq!(first_face, 0);

    let _ = cache
        .layout_for(&mut stack, "Ω", 20.0)
        .expect("primary-only layout uses replacement glyph");
    stack
        .append_test_bytes(include_bytes!(
            "../../../../../tests/fixtures/fonts/secondary.ttf"
        ))
        .expect("append secondary fixture");

    let cached_face = cache
        .layout_for(&mut stack, "A", 20.0)
        .expect("cached primary layout remains available")
        .glyphs[0]
        .face_index;
    assert_eq!(cached_face, first_face);
}

#[test]
fn shape_cache_is_reused_while_width_views_reproject_independently() {
    let mut cache = TextLayoutCache::new();
    let mut stack = super::super::font::NativeFontStack::from_test_bytes(&[include_bytes!(
        "../../../../../tests/fixtures/fonts/primary.ttf"
    )]);

    let first_shape = cache
        .layout_for_view(
            &mut stack,
            "A",
            20.0,
            Some(40.0),
            TextAlign::Left,
            TextWrap::None,
        )
        .expect("first view should be complete")
        .snapshot
        .shaped
        .clone();
    let second_shape = cache
        .layout_for_view(
            &mut stack,
            "A",
            20.0,
            Some(80.0),
            TextAlign::Left,
            TextWrap::None,
        )
        .expect("second width should be complete")
        .snapshot
        .shaped
        .clone();
    let _ = cache
        .layout_for_view(
            &mut stack,
            "A",
            20.0,
            Some(80.0),
            TextAlign::Left,
            TextWrap::None,
        )
        .expect("cached width should be complete");

    assert!(Arc::ptr_eq(&first_shape, &second_shape));
    assert_eq!(cache.shape_cache.len(), 1);
    assert_eq!(cache.view_cache.len(), 2);
    let counters = cache.take_profile_counters();
    assert_eq!((counters.shape.hits, counters.shape.misses), (2, 1));
    assert_eq!((counters.width.hits, counters.width.misses), (1, 2));
    assert_eq!((counters.view.hits, counters.view.misses), (1, 2));
}

#[test]
fn view_cache_counts_shared_shaped_geometry_once_near_budget() {
    let mut cache = TextLayoutCache::new();
    let mut stack = super::super::font::NativeFontStack::from_test_bytes(&[include_bytes!(
        "../../../../../tests/fixtures/fonts/primary.ttf"
    )]);

    let first = cache
        .layout_for_view(
            &mut stack,
            "A",
            20.0,
            Some(40.0),
            TextAlign::Left,
            TextWrap::None,
        )
        .expect("first view should be complete")
        .clone();
    let shaped = first.snapshot.shaped.clone();
    let second = TextLayout::from_snapshot(ParagraphSnapshot::from_shaped(
        shaped.clone(),
        Some(80.0),
        TextAlign::Left,
    ));
    let byte_budget = first
        .estimated_local_bytes()
        .saturating_add(second.estimated_local_bytes())
        .saturating_add(shaped.estimated_bytes());

    cache.view_cache.clear();
    cache.view_cache_order.clear();
    cache.view_cache_bytes = 0;
    cache.view_cache_byte_budget_override = Some(byte_budget);

    let _ = cache.layout_for_view(
        &mut stack,
        "A",
        20.0,
        Some(40.0),
        TextAlign::Left,
        TextWrap::None,
    );
    let _ = cache.layout_for_view(
        &mut stack,
        "A",
        20.0,
        Some(80.0),
        TextAlign::Left,
        TextWrap::None,
    );

    assert_eq!(cache.view_cache.len(), 2);
    assert_eq!(cache.view_cache_bytes, byte_budget);
}

#[test]
fn width_and_view_cache_eviction_obeys_entry_and_byte_budgets() {
    let mut cache = TextLayoutCache::new();
    let key = layout_key("byte-budget");
    cache
        .view_cache
        .insert(key.clone(), cached_layout(key.shape.text.as_ref(), 1));
    cache.view_cache_bytes = VIEW_CACHE_BYTE_BUDGET;
    cache.touch_view_cache_key(&key);

    cache.evict_view_cache_until(1);

    assert!(cache.view_cache.is_empty());
    assert_eq!(cache.view_cache_bytes, 0);
    assert_eq!(cache.view_profile.evictions, 1);
    assert_eq!(cache.width_profile.evictions, 1);
    assert_eq!(cache.layout_profile.evictions, 1);
}

#[test]
fn oversized_view_is_returned_complete_without_becoming_unbounded_cache_state() {
    let mut cache = TextLayoutCache::new();
    let mut stack = super::super::font::NativeFontStack::from_test_bytes(&[include_bytes!(
        "../../../../../tests/fixtures/fonts/primary.ttf"
    )]);
    cache.view_cache_byte_budget_override = Some(1);
    let text = "A";

    let layout = cache
        .layout_for_view(
            &mut stack,
            text,
            20.0,
            Some(240.0),
            TextAlign::Left,
            TextWrap::None,
        )
        .expect("oversized paragraph still needs complete geometry");

    assert_eq!(
        layout.snapshot.grapheme_geometry.len(),
        text.chars().count()
    );
    assert!(cache.transient_layout.is_some());
    assert!(cache.view_cache.is_empty());
}

#[test]
fn paragraph_presentation_partitions_shapes_and_reuses_previous_locale() {
    use super::super::model::TextPresentation;
    use crate::application::{ApplicationEnvironment, LocaleId, WritingDirection};

    let mut cache = TextLayoutCache::new();
    let mut stack = NativeFontStack::from_test_bytes(&[include_bytes!(
        "../../../../../tests/fixtures/fonts/primary.ttf"
    )]);
    let english = ApplicationEnvironment::new(LocaleId::english());
    cache.presentation = TextPresentation::from_environment(&english);
    let original = cache.layout_for(&mut stack, "A", 20.0).unwrap().snapshot();
    let repeated = cache.layout_for(&mut stack, "A", 20.0).unwrap().snapshot();
    assert!(Arc::ptr_eq(&original.shaped, &repeated.shaped));
    assert_eq!(cache.take_profile_counters().shape.misses, 1);

    cache.presentation = TextPresentation::from_environment(&ApplicationEnvironment::new(
        LocaleId::new("sr").unwrap(),
    ));
    let localized = cache.layout_for(&mut stack, "A", 20.0).unwrap().snapshot();
    assert!(!Arc::ptr_eq(&original.shaped, &localized.shaped));
    assert_eq!(cache.take_profile_counters().shape.misses, 1);

    cache.presentation = TextPresentation::from_environment(
        &english
            .clone()
            .with_writing_direction(WritingDirection::Rtl),
    );
    let rtl = cache.layout_for(&mut stack, "A", 20.0).unwrap().snapshot();
    assert!(!Arc::ptr_eq(&original.shaped, &rtl.shaped));
    assert_eq!(rtl.bidi_runs[0].level, 2);
    assert_eq!(original.bidi_runs[0].level, 0);
    assert_eq!(cache.take_profile_counters().shape.misses, 1);

    cache.presentation = TextPresentation::from_environment(&english);
    let restored = cache.layout_for(&mut stack, "A", 20.0).unwrap().snapshot();
    assert!(Arc::ptr_eq(&original.shaped, &restored.shaped));
    assert_eq!(cache.take_profile_counters().shape.misses, 0);
}
