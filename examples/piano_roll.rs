//! Piano-roll editor sandbox for DAW-style GUI interaction.

use radiant::prelude::*;
use radiant::{
    runtime::{PaintFillRect, PaintStrokeRect},
    widgets::PaintBounds,
};

const PIANO_ROLL_WIDGET_ID: u64 = 92;
const STATUS_WIDGET_ID: u64 = 93;
const TOTAL_BEATS: f32 = 16.0;
const PITCH_ROWS: usize = 24;
const LOW_PITCH: i32 = 48;
const DEFAULT_NOTE_LENGTH: f32 = 1.0;
const DATA_SOURCE_NOTE: &str = "without_midi_or_dsp";

#[derive(Clone, Debug)]
struct PianoRollState {
    running: bool,
    frame: u64,
    playhead_beat: f32,
    selected_note: Option<u32>,
    notes: Vec<PianoNote>,
}

impl Default for PianoRollState {
    fn default() -> Self {
        Self {
            running: true,
            frame: 0,
            playhead_beat: 0.0,
            selected_note: Some(2),
            notes: vec![
                PianoNote::new(1, 48, 0.0, 1.0, 0.72),
                PianoNote::new(2, 55, 1.0, 1.5, 0.82),
                PianoNote::new(3, 60, 2.75, 0.75, 0.64),
                PianoNote::new(4, 64, 3.5, 1.25, 0.76),
                PianoNote::new(5, 52, 5.0, 2.0, 0.88),
                PianoNote::new(6, 67, 7.25, 0.75, 0.68),
                PianoNote::new(7, 62, 9.0, 1.0, 0.70),
                PianoNote::new(8, 69, 10.5, 1.5, 0.84),
                PianoNote::new(9, 57, 12.5, 2.0, 0.78),
            ],
        }
    }
}

impl PianoRollState {
    fn tick(&mut self) {
        if !self.running {
            return;
        }
        self.frame = self.frame.saturating_add(1);
        self.playhead_beat = (self.playhead_beat + 0.055) % TOTAL_BEATS;
    }

    fn reset(&mut self) {
        self.frame = 0;
        self.playhead_beat = 0.0;
        self.running = true;
        *self = Self::default();
    }

    fn status(&self) -> String {
        let transport = if self.running { "running" } else { "paused" };
        let selected = self
            .selected_note
            .and_then(|id| self.notes.iter().find(|note| note.id == id))
            .map(|note| {
                format!(
                    "{} beat {:.2} len {:.2}",
                    pitch_label(note.pitch),
                    note.start_beat,
                    note.length_beats
                )
            })
            .unwrap_or_else(|| "no note".into());
        format!(
            "{transport} | playhead {:.2} | selected {selected} | synthetic GUI data",
            self.playhead_beat
        )
    }

    fn apply_roll_message(&mut self, message: PianoRollMessage) {
        match message {
            PianoRollMessage::SelectNote(id) => {
                self.selected_note = Some(id);
            }
            PianoRollMessage::CreateNote { pitch, start_beat } => {
                let id = self.next_note_id();
                let note = PianoNote::new(
                    id,
                    pitch.clamp(LOW_PITCH, LOW_PITCH + PITCH_ROWS as i32 - 1),
                    quantize_beat(start_beat),
                    DEFAULT_NOTE_LENGTH,
                    synthetic_velocity(id),
                );
                self.notes.push(note);
                self.selected_note = Some(id);
            }
            PianoRollMessage::MoveNote {
                id,
                pitch,
                start_beat,
            } => {
                if let Some(note) = self.notes.iter_mut().find(|note| note.id == id) {
                    note.pitch = pitch.clamp(LOW_PITCH, LOW_PITCH + PITCH_ROWS as i32 - 1);
                    note.start_beat =
                        quantize_beat(start_beat).clamp(0.0, TOTAL_BEATS - note.length_beats);
                    self.selected_note = Some(id);
                }
            }
            PianoRollMessage::ResizeNote {
                id,
                start_beat,
                length_beats,
            } => {
                if let Some(note) = self.notes.iter_mut().find(|note| note.id == id) {
                    let end_beat = (start_beat + length_beats).clamp(0.25, TOTAL_BEATS);
                    note.start_beat = quantize_beat(start_beat).clamp(0.0, end_beat - 0.25);
                    note.length_beats = quantize_beat(end_beat - note.start_beat).clamp(0.25, 4.0);
                    self.selected_note = Some(id);
                }
            }
            PianoRollMessage::DeleteSelected => {
                if let Some(id) = self.selected_note.take() {
                    self.notes.retain(|note| note.id != id);
                }
            }
        }
    }

    fn next_note_id(&self) -> u32 {
        self.notes
            .iter()
            .map(|note| note.id)
            .max()
            .unwrap_or(0)
            .saturating_add(1)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct PianoNote {
    id: u32,
    pitch: i32,
    start_beat: f32,
    length_beats: f32,
    velocity: f32,
}

impl PianoNote {
    const fn new(id: u32, pitch: i32, start_beat: f32, length_beats: f32, velocity: f32) -> Self {
        Self {
            id,
            pitch,
            start_beat,
            length_beats,
            velocity,
        }
    }

    fn end_beat(self) -> f32 {
        self.start_beat + self.length_beats
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum AppMessage {
    Frame,
    ToggleRun,
    Reset,
    Roll(PianoRollMessage),
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum PianoRollMessage {
    SelectNote(u32),
    CreateNote {
        pitch: i32,
        start_beat: f32,
    },
    MoveNote {
        id: u32,
        pitch: i32,
        start_beat: f32,
    },
    ResizeNote {
        id: u32,
        start_beat: f32,
        length_beats: f32,
    },
    DeleteSelected,
}

fn main() -> radiant::Result {
    radiant::app(PianoRollState::default())
        .title("Radiant Piano Roll")
        .size(1040, 620)
        .min_size(820, 500)
        .view(project_surface)
        .animation(|state| state.running)
        .on_frame(|| AppMessage::Frame)
        .update(update)
        .run()
}

fn project_surface(state: &mut PianoRollState) -> View<AppMessage> {
    column([
        row([
            text("Piano Roll").height(30.0).fill_width(),
            button(if state.running { "Pause" } else { "Run" })
                .primary()
                .message(AppMessage::ToggleRun)
                .size(88.0, 30.0),
            button("Reset")
                .subtle()
                .message(AppMessage::Reset)
                .size(82.0, 30.0),
        ])
        .fill_width()
        .spacing(10.0),
        custom_widget_mapped(
            PianoRollWidget::new(
                state.notes.clone(),
                state.selected_note,
                state.playhead_beat,
            ),
            AppMessage::Roll,
        )
        .id(PIANO_ROLL_WIDGET_ID)
        .height(390.0)
        .fill_width(),
        row([
            stat_tile("Notes", state.notes.len().to_string()),
            stat_tile("Grid", "1/4 beat"),
            stat_tile("Range", "C3 - B4"),
            stat_tile("Source", DATA_SOURCE_NOTE),
            text(state.status())
                .id(STATUS_WIDGET_ID)
                .height(68.0)
                .fill_width(),
        ])
        .fill_width()
        .spacing(10.0),
    ])
    .style(WidgetStyle::default())
    .padding(16.0)
    .spacing(12.0)
    .fill()
}

fn stat_tile(label: impl Into<String>, value: impl Into<String>) -> View<AppMessage> {
    column([
        text(label.into()).height(22.0).fill_width(),
        text(value.into()).height(24.0).fill_width(),
    ])
    .style(WidgetStyle {
        tone: WidgetTone::Neutral,
        prominence: WidgetProminence::Subtle,
    })
    .padding(10.0)
    .spacing(4.0)
    .height(68.0)
    .fill_width()
}

fn update(state: &mut PianoRollState, message: AppMessage) {
    match message {
        AppMessage::Frame => state.tick(),
        AppMessage::ToggleRun => {
            state.running = !state.running;
        }
        AppMessage::Reset => state.reset(),
        AppMessage::Roll(message) => state.apply_roll_message(message),
    }
}

#[derive(Clone, Debug)]
struct PianoRollWidget {
    common: WidgetCommon,
    notes: Vec<PianoNote>,
    selected_note: Option<u32>,
    playhead_beat: f32,
    hover_note: Option<u32>,
    hover_position: Option<Point>,
    drag: Option<PianoDrag>,
}

impl PianoRollWidget {
    fn new(notes: Vec<PianoNote>, selected_note: Option<u32>, playhead_beat: f32) -> Self {
        let mut common = WidgetCommon::new(
            0,
            WidgetSizing::new(Vector2::new(760.0, 340.0), Vector2::new(1000.0, 390.0)),
        );
        common.focus = FocusBehavior::Pointer;
        common.paint.bounds = PaintBounds::ClipToRect;
        common.paint.paints_focus = false;
        common.paint.paints_state_layers = false;
        Self {
            common,
            notes,
            selected_note,
            playhead_beat,
            hover_note: None,
            hover_position: None,
            drag: None,
        }
    }

    fn keyboard_rect(&self, bounds: Rect) -> Rect {
        let editor = self.editor_rect(bounds);
        Rect::from_min_max(
            Point::new(bounds.min.x + 12.0, editor.min.y),
            Point::new(editor.min.x - 1.0, editor.max.y),
        )
    }

    fn editor_rect(&self, bounds: Rect) -> Rect {
        Rect::from_min_max(
            Point::new(bounds.min.x + 76.0, bounds.min.y + 36.0),
            Point::new(bounds.max.x - 14.0, bounds.max.y - 20.0),
        )
    }

    fn note_rect(&self, grid: Rect, note: PianoNote) -> Rect {
        let x0 = x_for_beat(grid, note.start_beat);
        let x1 = x_for_beat(grid, note.end_beat());
        let y0 = y_for_pitch(grid, note.pitch);
        Rect::from_min_max(
            Point::new(x0, y0 + 2.0),
            Point::new(x1, y0 + row_height(grid) - 2.0),
        )
    }

    fn note_at_position(&self, grid: Rect, position: Point) -> Option<u32> {
        self.notes
            .iter()
            .rev()
            .find(|note| self.note_rect(grid, **note).contains(position))
            .map(|note| note.id)
    }

    fn note_by_id(&self, id: u32) -> Option<PianoNote> {
        self.notes.iter().copied().find(|note| note.id == id)
    }
}

impl Widget for PianoRollWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        let grid = self.editor_rect(bounds);
        match input {
            WidgetInput::PointerMove { position } => {
                self.common.state.hovered = bounds.contains(position);
                self.hover_position = grid.contains(position).then_some(position);
                if let Some(drag) = self.drag {
                    return Some(WidgetOutput::custom(drag.message_for(grid, position)));
                }
                self.hover_note = self.note_at_position(grid, position);
                None
            }
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Primary,
                ..
            } if grid.contains(position) => {
                let beat = beat_for_x(grid, position.x);
                let pitch = pitch_for_y(grid, position.y);
                if let Some(id) = self.note_at_position(grid, position) {
                    let note = self.note_by_id(id)?;
                    self.selected_note = Some(id);
                    self.hover_note = Some(id);
                    self.drag = Some(PianoDrag::from_note_hit(grid, note, position));
                    Some(WidgetOutput::custom(PianoRollMessage::SelectNote(id)))
                } else {
                    Some(WidgetOutput::custom(PianoRollMessage::CreateNote {
                        pitch,
                        start_beat: beat,
                    }))
                }
            }
            WidgetInput::PointerRelease {
                position,
                button: PointerButton::Primary,
                ..
            }
            | WidgetInput::PointerDrop {
                position,
                button: PointerButton::Primary,
                ..
            } => {
                let drag = self.drag.take();
                self.hover_note = self.note_at_position(grid, position);
                drag.map(|drag| WidgetOutput::custom(drag.message_for(grid, position)))
            }
            WidgetInput::KeyPress(WidgetKey::Delete | WidgetKey::Backspace)
                if self.common.state.focused =>
            {
                Some(WidgetOutput::custom(PianoRollMessage::DeleteSelected))
            }
            WidgetInput::FocusChanged(focused) => {
                self.common.state.focused = focused;
                None
            }
            _ => None,
        }
    }

    fn prefers_pointer_move_paint_only(&self) -> bool {
        self.drag.is_none()
    }

    fn synchronize_from_previous(&mut self, previous: &dyn Widget) {
        if let Some(previous) = previous.as_any().downcast_ref::<Self>() {
            self.common.state = previous.common.state;
            self.hover_note = previous.hover_note;
            self.hover_position = previous.hover_position;
            self.drag = previous.drag;
        }
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        let grid = self.editor_rect(bounds);
        push_rect(primitives, self.common.id, bounds, theme.bg_secondary);
        self.append_keyboard(primitives, bounds, theme);
        self.append_grid(primitives, grid, theme);
        for note in &self.notes {
            self.append_note(primitives, grid, *note, theme);
        }
        push_stroke(primitives, self.common.id, grid, theme.border_emphasis, 1.0);
    }

    fn append_runtime_overlay_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        let grid = self.editor_rect(bounds);
        let playhead_x = x_for_beat(grid, self.playhead_beat);
        push_rect(
            primitives,
            self.common.id,
            Rect::from_min_max(
                Point::new(playhead_x, grid.min.y),
                Point::new(playhead_x + 2.0, grid.max.y),
            ),
            translucent(theme.highlight_orange, 210),
        );
        if let Some(position) = self.hover_position
            && grid.contains(position)
        {
            push_rect(
                primitives,
                self.common.id,
                Rect::from_min_max(
                    Point::new(position.x, grid.min.y),
                    Point::new(position.x + 1.0, grid.max.y),
                ),
                translucent(theme.text_muted, 90),
            );
        }
        if let Some(note) = self.hover_note.and_then(|id| self.note_by_id(id)) {
            push_stroke(
                primitives,
                self.common.id,
                self.note_rect(grid, note),
                translucent(theme.highlight_cyan, 190),
                2.0,
            );
        }
    }
}

impl PianoRollWidget {
    fn append_keyboard(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        theme: &ThemeTokens,
    ) {
        let keyboard = self.keyboard_rect(bounds);
        push_rect(primitives, self.common.id, keyboard, rgba(17, 19, 23, 255));
        for row in 0..PITCH_ROWS {
            let pitch = LOW_PITCH + (PITCH_ROWS - 1 - row) as i32;
            let y = keyboard.min.y + row as f32 * row_height(keyboard);
            let rect = Rect::from_min_max(
                Point::new(keyboard.min.x, y),
                Point::new(keyboard.max.x, y + row_height(keyboard)),
            );
            push_rect(
                primitives,
                self.common.id,
                rect,
                if is_black_key(pitch) {
                    rgba(33, 38, 45, 255)
                } else {
                    theme.surface_raised
                },
            );
            if pitch % 12 == 0 {
                push_text(
                    primitives,
                    self.common.id,
                    pitch_label(pitch),
                    rect,
                    theme.text_muted,
                    PaintTextAlign::Center,
                );
            }
        }
    }

    fn append_grid(&self, primitives: &mut Vec<PaintPrimitive>, grid: Rect, theme: &ThemeTokens) {
        push_rect(primitives, self.common.id, grid, rgba(8, 12, 18, 255));
        for row in 0..=PITCH_ROWS {
            let y = grid.min.y + row as f32 * row_height(grid);
            push_rect(
                primitives,
                self.common.id,
                Rect::from_min_max(Point::new(grid.min.x, y), Point::new(grid.max.x, y + 1.0)),
                if row % 12 == 0 {
                    translucent(theme.grid_strong, 170)
                } else {
                    translucent(theme.grid_soft, 105)
                },
            );
        }
        for beat in 0..=(TOTAL_BEATS as usize * 4) {
            let beat_value = beat as f32 / 4.0;
            let x = x_for_beat(grid, beat_value);
            let color = if beat % 16 == 0 {
                translucent(theme.grid_strong, 190)
            } else if beat % 4 == 0 {
                translucent(theme.grid_strong, 125)
            } else {
                translucent(theme.grid_soft, 80)
            };
            push_rect(
                primitives,
                self.common.id,
                Rect::from_min_max(Point::new(x, grid.min.y), Point::new(x + 1.0, grid.max.y)),
                color,
            );
            if beat % 4 == 0 && beat < (TOTAL_BEATS as usize * 4) {
                push_text(
                    primitives,
                    self.common.id,
                    format!("{}", beat / 4 + 1),
                    Rect::from_min_size(
                        Point::new(x + 4.0, grid.min.y - 24.0),
                        Vector2::new(42.0, 18.0),
                    ),
                    theme.text_muted,
                    PaintTextAlign::Left,
                );
            }
        }
    }

    fn append_note(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        grid: Rect,
        note: PianoNote,
        theme: &ThemeTokens,
    ) {
        let rect = self.note_rect(grid, note);
        let selected = self.selected_note == Some(note.id);
        let fill = if selected {
            theme.highlight_blue
        } else {
            blend_color(
                theme.highlight_cyan,
                theme.highlight_blue,
                note.velocity * 0.45,
            )
        };
        push_rect(primitives, self.common.id, rect, fill);
        push_stroke(
            primitives,
            self.common.id,
            rect,
            if selected {
                theme.border_emphasis
            } else {
                translucent(theme.border_emphasis, 145)
            },
            1.0,
        );
        push_rect(
            primitives,
            self.common.id,
            Rect::from_min_max(
                Point::new(rect.max.x - 5.0, rect.min.y + 2.0),
                Point::new(rect.max.x - 2.0, rect.max.y - 2.0),
            ),
            translucent(theme.text_primary, 150),
        );
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum PianoDrag {
    Move {
        id: u32,
        beat_offset: f32,
        pitch_offset: i32,
        length_beats: f32,
    },
    ResizeStart {
        id: u32,
        end_beat: f32,
    },
    ResizeEnd {
        id: u32,
        start_beat: f32,
    },
}

impl PianoDrag {
    fn from_note_hit(grid: Rect, note: PianoNote, position: Point) -> Self {
        let rect = Rect::from_min_max(
            Point::new(
                x_for_beat(grid, note.start_beat),
                y_for_pitch(grid, note.pitch),
            ),
            Point::new(
                x_for_beat(grid, note.end_beat()),
                y_for_pitch(grid, note.pitch) + row_height(grid),
            ),
        );
        if position.x <= rect.min.x + 8.0 {
            Self::ResizeStart {
                id: note.id,
                end_beat: note.end_beat(),
            }
        } else if position.x >= rect.max.x - 8.0 {
            Self::ResizeEnd {
                id: note.id,
                start_beat: note.start_beat,
            }
        } else {
            Self::Move {
                id: note.id,
                beat_offset: beat_for_x(grid, position.x) - note.start_beat,
                pitch_offset: pitch_for_y(grid, position.y) - note.pitch,
                length_beats: note.length_beats,
            }
        }
    }

    fn message_for(self, grid: Rect, position: Point) -> PianoRollMessage {
        match self {
            Self::Move {
                id,
                beat_offset,
                pitch_offset,
                ..
            } => PianoRollMessage::MoveNote {
                id,
                pitch: pitch_for_y(grid, position.y) - pitch_offset,
                start_beat: beat_for_x(grid, position.x) - beat_offset,
            },
            Self::ResizeStart { id, end_beat } => {
                let start_beat = quantize_beat(beat_for_x(grid, position.x)).min(end_beat - 0.25);
                PianoRollMessage::ResizeNote {
                    id,
                    start_beat,
                    length_beats: end_beat - start_beat,
                }
            }
            Self::ResizeEnd { id, start_beat } => PianoRollMessage::ResizeNote {
                id,
                start_beat,
                length_beats: quantize_beat(beat_for_x(grid, position.x) - start_beat).max(0.25),
            },
        }
    }
}

fn row_height(rect: Rect) -> f32 {
    rect.height() / PITCH_ROWS as f32
}

fn x_for_beat(grid: Rect, beat: f32) -> f32 {
    grid.min.x + grid.width() * (beat / TOTAL_BEATS).clamp(0.0, 1.0)
}

fn beat_for_x(grid: Rect, x: f32) -> f32 {
    ((x - grid.min.x) / grid.width().max(1.0) * TOTAL_BEATS).clamp(0.0, TOTAL_BEATS)
}

fn y_for_pitch(grid: Rect, pitch: i32) -> f32 {
    let row = (LOW_PITCH + PITCH_ROWS as i32 - 1 - pitch).clamp(0, PITCH_ROWS as i32 - 1);
    grid.min.y + row as f32 * row_height(grid)
}

fn pitch_for_y(grid: Rect, y: f32) -> i32 {
    let row = ((y - grid.min.y) / row_height(grid).max(1.0)).floor() as i32;
    (LOW_PITCH + PITCH_ROWS as i32 - 1 - row).clamp(LOW_PITCH, LOW_PITCH + PITCH_ROWS as i32 - 1)
}

fn quantize_beat(beat: f32) -> f32 {
    (beat * 4.0).round() / 4.0
}

fn synthetic_velocity(id: u32) -> f32 {
    0.55 + (id % 5) as f32 * 0.08
}

fn is_black_key(pitch: i32) -> bool {
    matches!(pitch.rem_euclid(12), 1 | 3 | 6 | 8 | 10)
}

fn pitch_label(pitch: i32) -> String {
    const NAMES: [&str; 12] = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    format!(
        "{}{}",
        NAMES[pitch.rem_euclid(12) as usize],
        pitch.div_euclid(12) - 1
    )
}

fn blend_color(a: Rgba8, b: Rgba8, t: f32) -> Rgba8 {
    let t = t.clamp(0.0, 1.0);
    rgba(
        (a.r as f32 + (b.r as f32 - a.r as f32) * t).round() as u8,
        (a.g as f32 + (b.g as f32 - a.g as f32) * t).round() as u8,
        (a.b as f32 + (b.b as f32 - a.b as f32) * t).round() as u8,
        255,
    )
}

fn rgba(r: u8, g: u8, b: u8, a: u8) -> Rgba8 {
    Rgba8 { r, g, b, a }
}

fn translucent(mut color: Rgba8, alpha: u8) -> Rgba8 {
    color.a = alpha;
    color
}

fn push_rect(primitives: &mut Vec<PaintPrimitive>, widget_id: u64, rect: Rect, color: Rgba8) {
    primitives.push(PaintPrimitive::FillRect(PaintFillRect {
        widget_id,
        rect,
        color,
    }));
}

fn push_stroke(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: u64,
    rect: Rect,
    color: Rgba8,
    width: f32,
) {
    primitives.push(PaintPrimitive::StrokeRect(PaintStrokeRect {
        widget_id,
        rect,
        color,
        width,
    }));
}

fn push_text(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: u64,
    text: impl Into<String>,
    rect: Rect,
    color: Rgba8,
    align: PaintTextAlign,
) {
    primitives.push(PaintPrimitive::Text(PaintTextRun {
        widget_id,
        text: text.into().into(),
        rect,
        font_size: 12.0,
        baseline: Some(16.0),
        color,
        align,
        wrap: TextWrap::None,
    }));
}

#[cfg(test)]
mod tests {
    use super::*;
    use radiant::runtime::{RuntimeBridge, SurfaceRuntime};

    #[test]
    fn piano_roll_tick_advances_synthetic_playhead_without_midi_or_dsp() {
        let mut state = PianoRollState::default();
        let initial = state.playhead_beat;

        state.tick();

        assert_eq!(state.frame, 1);
        assert!(state.playhead_beat > initial);
        assert_eq!(DATA_SOURCE_NOTE, "without_midi_or_dsp");
    }

    #[test]
    fn piano_roll_widget_paints_keyboard_grid_notes_and_playhead() {
        let state = PianoRollState::default();
        let widget = PianoRollWidget::new(state.notes, state.selected_note, state.playhead_beat);
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 390.0));
        let mut primitives = Vec::new();
        let mut overlay = Vec::new();

        widget.append_paint(
            &mut primitives,
            bounds,
            &LayoutOutput::default(),
            &ThemeTokens::default(),
        );
        widget.append_runtime_overlay_paint(
            &mut overlay,
            bounds,
            &LayoutOutput::default(),
            &ThemeTokens::default(),
        );

        assert!(
            primitives
                .iter()
                .filter(|primitive| matches!(primitive, PaintPrimitive::FillRect(_)))
                .count()
                > PITCH_ROWS
        );
        assert!(
            primitives
                .iter()
                .any(|primitive| matches!(primitive, PaintPrimitive::Text(text) if text.text.as_str() == "C4"))
        );
        assert!(
            overlay
                .iter()
                .any(|primitive| matches!(primitive, PaintPrimitive::FillRect(_))),
            "playhead should paint as a lightweight runtime overlay"
        );
    }

    #[test]
    fn piano_roll_clicking_empty_grid_creates_quantized_note() {
        let state = PianoRollState::default();
        let mut widget =
            PianoRollWidget::new(state.notes, state.selected_note, state.playhead_beat);
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 390.0));
        let grid = widget.editor_rect(bounds);

        let output = widget.handle_input(
            bounds,
            WidgetInput::PointerPress {
                position: Point::new(x_for_beat(grid, 6.10), y_for_pitch(grid, 58) + 4.0),
                button: PointerButton::Primary,
                modifiers: Default::default(),
            },
        );

        assert_eq!(
            output.and_then(|output| output.typed_ref::<PianoRollMessage>().copied()),
            Some(PianoRollMessage::CreateNote {
                pitch: 58,
                start_beat: 6.10
            })
        );
    }

    #[test]
    fn piano_roll_drag_routes_move_message() {
        let state = PianoRollState::default();
        let mut widget =
            PianoRollWidget::new(state.notes, state.selected_note, state.playhead_beat);
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 390.0));
        let grid = widget.editor_rect(bounds);
        let note = widget.note_by_id(2).expect("default note should exist");
        let start = widget.note_rect(grid, note).center();

        let _ = widget.handle_input(
            bounds,
            WidgetInput::PointerPress {
                position: start,
                button: PointerButton::Primary,
                modifiers: Default::default(),
            },
        );
        let output = widget.handle_input(
            bounds,
            WidgetInput::PointerMove {
                position: Point::new(
                    start.x + grid.width() / TOTAL_BEATS,
                    start.y - row_height(grid),
                ),
            },
        );

        assert!(matches!(
            output.and_then(|output| output.typed_ref::<PianoRollMessage>().copied()),
            Some(PianoRollMessage::MoveNote {
                id: 2,
                pitch: 56,
                ..
            })
        ));
        assert!(!widget.prefers_pointer_move_paint_only());
    }

    #[test]
    fn piano_roll_hover_uses_paint_only_runtime_overlay() {
        let state = PianoRollState::default();
        let mut widget =
            PianoRollWidget::new(state.notes, state.selected_note, state.playhead_beat);
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 390.0));
        let grid = widget.editor_rect(bounds);
        let note = widget.note_by_id(2).expect("default note should exist");

        let output = widget.handle_input(
            bounds,
            WidgetInput::PointerMove {
                position: widget.note_rect(grid, note).center(),
            },
        );

        assert!(output.is_none());
        assert_eq!(widget.hover_note, Some(2));
        assert!(widget.prefers_pointer_move_paint_only());
        let mut overlay = Vec::new();
        widget.append_runtime_overlay_paint(
            &mut overlay,
            bounds,
            &LayoutOutput::default(),
            &ThemeTokens::default(),
        );
        assert!(
            overlay
                .iter()
                .any(|primitive| matches!(primitive, PaintPrimitive::StrokeRect(_))),
            "hovered note should paint as a lightweight runtime overlay"
        );
    }

    #[test]
    fn piano_roll_runtime_hover_does_not_refresh_surface() {
        let bridge = piano_roll_test_bridge(PianoRollState::default());
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(1040.0, 620.0));
        let bounds = runtime.layout().rects[&PIANO_ROLL_WIDGET_ID];
        let first = runtime.dispatch_pointer_move_with_outcome(Point::new(
            bounds.min.x + 160.0,
            bounds.center().y,
        ));
        let second = runtime.dispatch_pointer_move_with_outcome(Point::new(
            bounds.min.x + 260.0,
            bounds.center().y,
        ));

        assert!(first.needs_scene_rebuild());
        assert!(second.paint_only_requested);
        assert!(
            !second.needs_scene_rebuild(),
            "stable piano-roll hover should avoid reprojection and full scene rebuilds"
        );
    }

    #[test]
    fn piano_roll_runtime_frame_messages_advance_status() {
        let bridge = piano_roll_test_bridge(PianoRollState::default());
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(1040.0, 620.0));
        let initial_status = status_text(&runtime);

        assert!(runtime.bridge_mut().needs_animation());
        assert!(runtime.bridge_mut().queue_animation_frame());
        let outcome = runtime.drain_runtime_messages();

        assert_eq!(outcome.messages_dispatched, 1);
        assert_ne!(status_text(&runtime), initial_status);
    }

    fn piano_roll_test_bridge(state: PianoRollState) -> impl RuntimeBridge<AppMessage> {
        radiant::app(state)
            .view(project_surface)
            .animation(|state| state.running)
            .on_frame(|| AppMessage::Frame)
            .update(update)
            .into_bridge()
    }

    fn status_text<Bridge>(runtime: &SurfaceRuntime<Bridge, AppMessage>) -> String
    where
        Bridge: RuntimeBridge<AppMessage>,
    {
        runtime
            .paint_plan(&ThemeTokens::default())
            .primitives
            .iter()
            .find_map(|primitive| match primitive {
                PaintPrimitive::Text(text) if text.widget_id == STATUS_WIDGET_ID => {
                    Some(text.text.as_str().to_string())
                }
                _ => None,
            })
            .expect("status text should be painted")
    }
}
