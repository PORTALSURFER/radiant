//! Text layout performance scenarios.

use radiant::{
    gui::{
        text_layout::{
            TextLineInsets, TextLineLayoutCache, centered_text_line_with_cache,
            top_text_line_with_cache,
        },
        types::{Point, Rect, Vector2},
    },
    widgets::{TextEditCommand, TextInputState},
};
use std::hint::black_box;

const TEXT_ROWS: usize = 1_024;
const WORD_SELECTION_CARETS: usize = 1_024;
const WORD_DELETION_STATES: usize = 1_024;

pub(super) fn text_line_cache_1k() -> impl FnMut() {
    let mut bench = TextLineCacheBench::new();
    move || bench.step()
}

pub(super) fn text_word_selection_1k() -> impl FnMut() {
    let mut bench = TextWordSelectionBench::new();
    move || bench.step()
}

pub(super) fn text_word_deletion_1k() -> impl FnMut() {
    let mut bench = TextWordDeletionBench::new();
    move || bench.step()
}

struct TextLineCacheBench {
    cache: TextLineLayoutCache,
    rows: Vec<TextLineRequest>,
    tick: usize,
}

#[derive(Clone, Copy)]
struct TextLineRequest {
    bounds: Rect,
    font_size: f32,
    insets: TextLineInsets,
    min_top_inset: f32,
    family_id: u64,
    centered: bool,
}

impl TextLineCacheBench {
    fn new() -> Self {
        Self {
            cache: TextLineLayoutCache::new(),
            rows: text_line_requests(TEXT_ROWS),
            tick: 0,
        }
    }

    fn step(&mut self) {
        self.tick = self.tick.wrapping_add(1);
        let mut checksum = 0.0;
        for request in &self.rows {
            let rect = if request.centered {
                centered_text_line_with_cache(
                    &mut self.cache,
                    request.bounds,
                    request.font_size,
                    request.insets,
                    request.min_top_inset,
                    request.family_id,
                )
            } else {
                top_text_line_with_cache(
                    &mut self.cache,
                    request.bounds,
                    request.font_size,
                    request.insets,
                    request.family_id,
                )
            };
            checksum += rect.min.x + rect.min.y + rect.max.x + rect.max.y;
        }
        assert!(checksum.is_finite());
        assert!(!self.cache.is_empty());
        black_box((checksum, self.tick, self.cache.len()));
    }
}

struct TextWordSelectionBench {
    state: TextInputState,
    carets: Vec<usize>,
    next: usize,
}

impl TextWordSelectionBench {
    fn new() -> Self {
        let value = word_selection_text();
        let carets = word_selection_carets(&value, WORD_SELECTION_CARETS);
        assert_eq!(carets.len(), WORD_SELECTION_CARETS);
        Self {
            state: TextInputState::from_value(value),
            carets,
            next: 0,
        }
    }

    fn step(&mut self) {
        let caret = self.carets[self.next % self.carets.len()];
        self.next = self.next.wrapping_add(1);
        assert!(self.state.select_word_at(caret));
        black_box((caret, self.state.selected_text_slice(), self.next));
    }
}

struct TextWordDeletionBench {
    template: String,
    states: Vec<TextInputState>,
    next: usize,
}

impl TextWordDeletionBench {
    fn new() -> Self {
        let template = word_selection_text();
        let states = (0..WORD_DELETION_STATES)
            .map(|_| TextInputState::from_value(template.clone()))
            .collect();
        Self {
            template,
            states,
            next: 0,
        }
    }

    fn step(&mut self) {
        let state_index = self.next % self.states.len();
        let state = &mut self.states[state_index];
        if state.char_len() < 96 {
            *state = TextInputState::from_value(self.template.clone());
        }
        let caret = (state.char_len() / 2).max(1);
        state.set_caret(caret, false);
        let command = if self.next.is_multiple_of(2) {
            TextEditCommand::DeleteWordLeft
        } else {
            TextEditCommand::DeleteWordRight
        };
        let result = state.apply_edit_command(command, None);
        assert!(result.value_changed);
        self.next = self.next.wrapping_add(1);
        black_box((state_index, caret, state.char_len(), result, self.next));
    }
}

fn text_line_requests(count: usize) -> Vec<TextLineRequest> {
    (0..count)
        .map(|index| {
            let row = index % 128;
            let column = index / 128;
            let width = 88.0 + (column % 4) as f32 * 12.0;
            let height = 18.0 + (row % 3) as f32 * 2.0;
            TextLineRequest {
                bounds: Rect::from_min_size(
                    Point::new((column * 96) as f32, (row * 22) as f32),
                    Vector2::new(width, height),
                ),
                font_size: 11.0 + (row % 5) as f32,
                insets: TextLineInsets {
                    left: (index % 3) as f32,
                    right: (index % 5) as f32,
                    top: (index % 2) as f32,
                    bottom: (index % 4) as f32,
                },
                min_top_inset: (index % 6) as f32 * 0.5,
                family_id: (index % 8) as u64,
                centered: index % 2 == 0,
            }
        })
        .collect()
}

fn word_selection_text() -> String {
    "alpha beta_gamma delta epsilon zeta_eta theta lambda \u{65e5}\u{6587} ".repeat(128)
}

fn word_selection_carets(value: &str, count: usize) -> Vec<usize> {
    value
        .chars()
        .enumerate()
        .filter_map(|(index, ch)| is_word_selection_caret(ch).then_some(index))
        .take(count)
        .collect()
}

fn is_word_selection_caret(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}
