//! Native text and primitive color/geometry encoding helpers for Vello scenes.

use super::*;

#[derive(Clone, Debug)]
pub(super) struct GlyphLayout {
    id: u32,
    x: f32,
}

#[derive(Clone, Debug)]
pub(super) struct TextLayout {
    width: f32,
    glyphs: Vec<GlyphLayout>,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub(super) struct TextLayoutKey {
    text: Arc<str>,
    font_size_bits: u32,
}

const TEXT_LAYOUT_CACHE_CAPACITY: usize = 2_048;
const TEXT_ATOM_CACHE_CAPACITY: usize = 4_096;

#[derive(Clone)]
pub(super) struct LoadedFont {
    font: FontData,
}

pub(super) struct NativeTextRenderer {
    loaded_font: Option<LoadedFont>,
    layout_cache: HashMap<TextLayoutKey, TextLayout>,
    layout_cache_order: VecDeque<TextLayoutKey>,
    atom_cache: HashMap<Arc<str>, u64>,
    atom_cache_order: VecDeque<(Arc<str>, u64)>,
    atom_cache_clock: u64,
    text_layout_hits: u64,
    text_layout_misses: u64,
    text_layout_evictions: u64,
    text_atom_hits: u64,
    text_atom_misses: u64,
    text_atom_evictions: u64,
}

impl NativeTextRenderer {
    pub(super) fn new() -> Self {
        let loaded_font = load_native_font().map(|font| LoadedFont { font });
        if loaded_font.is_none() {
            eprintln!(
                "Native vello text renderer: no fallback font found; text runs will be skipped"
            );
        }
        Self {
            loaded_font,
            layout_cache: HashMap::with_capacity(TEXT_LAYOUT_CACHE_CAPACITY / 2),
            layout_cache_order: VecDeque::with_capacity(TEXT_LAYOUT_CACHE_CAPACITY),
            atom_cache: HashMap::with_capacity(TEXT_ATOM_CACHE_CAPACITY / 2),
            atom_cache_order: VecDeque::with_capacity(TEXT_ATOM_CACHE_CAPACITY),
            atom_cache_clock: 0,
            text_layout_hits: 0,
            text_layout_misses: 0,
            text_layout_evictions: 0,
            text_atom_hits: 0,
            text_atom_misses: 0,
            text_atom_evictions: 0,
        }
    }

    pub(super) fn draw_text_runs(&mut self, scene: &mut Scene, text_runs: &[TextRun]) {
        let Some(loaded_font) = self.loaded_font.as_ref() else {
            return;
        };
        let font_data = loaded_font.font.clone();
        for run in text_runs {
            if run.text.is_empty() || run.font_size <= 0.0 {
                continue;
            }
            let Some(layout) = self.layout_for(&font_data, &run.text, run.font_size) else {
                continue;
            };
            let mut origin_x = run.position.x;
            if let Some(max_width) = run.max_width {
                let extra = (max_width - layout.width).max(0.0);
                origin_x += match run.align {
                    TextAlign::Left => 0.0,
                    TextAlign::Center => extra * 0.5,
                    TextAlign::Right => extra,
                };
            }
            let clip_width = run.max_width.unwrap_or(f32::INFINITY);
            let baseline = run.position.y + run.font_size;
            let glyph_iter = layout
                .glyphs
                .iter()
                .take_while(|glyph| glyph.x <= clip_width)
                .map(|glyph| Glyph {
                    id: glyph.id,
                    x: origin_x + glyph.x,
                    y: baseline,
                });
            scene
                .draw_glyphs(&font_data)
                .font_size(run.font_size)
                .brush(color_from_rgba(run.color))
                .draw(Fill::NonZero, glyph_iter);
        }
    }

    pub(super) fn layout_for<'a>(
        &'a mut self,
        font: &FontData,
        text: &str,
        font_size: f32,
    ) -> Option<&'a TextLayout> {
        let text_atom = self.intern_text(text);
        let key = TextLayoutKey {
            text: text_atom,
            font_size_bits: font_size.to_bits(),
        };

        if let Some(layout) = self
            .layout_cache
            .get(&key)
            .map(|layout| layout as *const TextLayout)
        {
            self.text_layout_hits = self.text_layout_hits.saturating_add(1);
            return Some(unsafe { &*layout });
        }

        self.text_layout_misses = self.text_layout_misses.saturating_add(1);

        if self.layout_cache.len() >= TEXT_LAYOUT_CACHE_CAPACITY {
            if let Some(evicted_key) = self.layout_cache_order.pop_front() {
                if self.layout_cache.remove(&evicted_key).is_some() {
                    self.text_layout_evictions = self.text_layout_evictions.saturating_add(1);
                }
            }
        }

        let Some(layout) = Self::compute_layout(font, text, font_size) else {
            return None;
        };
        self.layout_cache_order.push_back(key.clone());
        let cached_layout = self.layout_cache.entry(key).or_insert(layout);
        return Some(cached_layout);
    }

    pub(super) fn take_layout_profile_counters(&mut self) -> (u64, u64, u64, u64, u64, u64) {
        let counters = (
            self.text_layout_hits,
            self.text_layout_misses,
            self.text_layout_evictions,
            self.text_atom_hits,
            self.text_atom_misses,
            self.text_atom_evictions,
        );
        self.text_layout_hits = 0;
        self.text_layout_misses = 0;
        self.text_layout_evictions = 0;
        self.text_atom_hits = 0;
        self.text_atom_misses = 0;
        self.text_atom_evictions = 0;
        counters
    }

    /// Intern text into a bounded atom cache so layout-key construction avoids
    /// hot-path `String` allocations on repeated runs.
    pub(super) fn intern_text(&mut self, text: &str) -> Arc<str> {
        self.atom_cache_clock = self.atom_cache_clock.saturating_add(1);
        let stamp = self.atom_cache_clock;
        if let Some((cached, _)) = self.atom_cache.get_key_value(text) {
            let atom = Arc::clone(cached);
            if let Some(last_seen) = self.atom_cache.get_mut(text) {
                *last_seen = stamp;
            }
            self.atom_cache_order.push_back((Arc::clone(&atom), stamp));
            self.text_atom_hits = self.text_atom_hits.saturating_add(1);
            return atom;
        }

        self.text_atom_misses = self.text_atom_misses.saturating_add(1);
        let atom: Arc<str> = Arc::from(text);
        self.atom_cache.insert(Arc::clone(&atom), stamp);
        self.atom_cache_order.push_back((Arc::clone(&atom), stamp));
        self.evict_stale_atoms();
        atom
    }

    /// Evict stale atom-cache entries using insertion stamps for bounded memory.
    pub(super) fn evict_stale_atoms(&mut self) {
        while self.atom_cache.len() > TEXT_ATOM_CACHE_CAPACITY {
            let Some((candidate, queued_stamp)) = self.atom_cache_order.pop_front() else {
                break;
            };
            let Some(current_stamp) = self.atom_cache.get(candidate.as_ref()) else {
                continue;
            };
            if *current_stamp != queued_stamp {
                continue;
            }
            if self.atom_cache.remove(candidate.as_ref()).is_some() {
                self.text_atom_evictions = self.text_atom_evictions.saturating_add(1);
            }
        }
    }

    pub(super) fn compute_layout(
        font: &FontData,
        text: &str,
        font_size: f32,
    ) -> Option<TextLayout> {
        let font_ref = skrifa::FontRef::from_index(font.data.as_ref(), font.index).ok()?;
        let charmap = font_ref.charmap();
        let metrics = font_ref.glyph_metrics(FontSize::new(font_size), LocationRef::default());
        let fallback_glyph = charmap.map('?');

        let mut x = 0.0_f32;
        let mut glyphs = Vec::with_capacity(text.len());
        for ch in text.chars() {
            if ch == '\n' || ch == '\r' {
                break;
            }
            if ch == '\t' {
                x += font_size * 2.0;
                continue;
            }
            if ch == ' ' {
                x += font_size * 0.33;
                continue;
            }
            if ch.is_control() {
                continue;
            }
            let glyph_id = charmap.map(ch).or(fallback_glyph);
            let Some(glyph_id) = glyph_id else {
                x += font_size * 0.5;
                continue;
            };
            glyphs.push(GlyphLayout {
                id: glyph_id.to_u32(),
                x,
            });
            let advance = metrics
                .advance_width(glyph_id)
                .unwrap_or(font_size * 0.55)
                .max(0.0);
            x += advance;
        }

        Some(TextLayout { width: x, glyphs })
    }
}

pub(super) fn load_native_font() -> Option<FontData> {
    for path in native_font_candidates() {
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        return Some(FontData::new(Blob::from(bytes), 0));
    }
    None
}

pub(super) fn native_font_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(path) = std::env::var("SEMPAL_NATIVE_FONT_PATH") {
        candidates.push(PathBuf::from(path));
    }

    #[cfg(target_os = "windows")]
    {
        if let Ok(windir) = std::env::var("WINDIR") {
            let base = PathBuf::from(windir).join("Fonts");
            candidates.push(base.join("segoeui.ttf"));
            candidates.push(base.join("arial.ttf"));
            candidates.push(base.join("consola.ttf"));
        }
    }
    #[cfg(target_os = "macos")]
    {
        candidates.push(PathBuf::from("/System/Library/Fonts/SFNS.ttf"));
        candidates.push(PathBuf::from(
            "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
        ));
        candidates.push(PathBuf::from("/Library/Fonts/Arial.ttf"));
    }
    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    {
        candidates.push(PathBuf::from(
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        ));
        candidates.push(PathBuf::from("/usr/share/fonts/dejavu/DejaVuSans.ttf"));
        candidates.push(PathBuf::from("/usr/share/fonts/TTF/DejaVuSans.ttf"));
        candidates.push(PathBuf::from(
            "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
        ));
    }

    candidates
}

pub(super) fn to_kurbo_rect(rect: UiRect) -> KurboRect {
    KurboRect::new(
        rect.min.x as f64,
        rect.min.y as f64,
        rect.max.x as f64,
        rect.max.y as f64,
    )
}

pub(super) fn color_from_rgba(color: Rgba8) -> Color {
    Color::from_rgba8(color.r, color.g, color.b, color.a)
}

pub(super) fn icon_from_rgba(icon: &WindowIconRgba) -> Option<Icon> {
    Icon::from_rgba(icon.rgba.clone(), icon.width, icon.height).ok()
}
