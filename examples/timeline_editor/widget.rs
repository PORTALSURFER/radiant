use super::{
    HEADER_WIDTH, LANE_COUNT, RESIZE_HANDLE_WIDTH, TOTAL_BEATS,
    model::{BeatRange, TimelineClip, TimelineEditorState},
};
#[path = "widget/geometry.rs"]
mod geometry;
#[path = "widget/input.rs"]
mod input;
#[path = "widget/overlay.rs"]
mod overlay;
#[path = "widget/paint.rs"]
mod paint;

use radiant::layout::{LayoutOutput, Rect, Vector2};
use radiant::runtime::PaintPrimitive;
use radiant::theme::ThemeTokens;
use radiant::widgets::{Widget, WidgetCommon, WidgetInput, WidgetOutput, WidgetSizing};

pub(super) use geometry::TimelineGeometry;

#[derive(Clone, Debug)]
pub(super) struct ArrangementTimelineWidget {
    pub(super) common: WidgetCommon,
    pub(super) clips: Vec<TimelineClip>,
    pub(super) selected_clip: Option<u32>,
    pub(super) selection: Option<BeatRange>,
    pub(super) playhead_beat: u32,
    pub(super) cursor: overlay::TimelineCursorOverlay,
    pub(super) hover_clip_id: Option<u32>,
    pub(super) drag: Option<TimelineDrag>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum TimelineDrag {
    Selecting {
        lane: usize,
        anchor_beat: u32,
    },
    MovingClip {
        clip_id: u32,
        clip_name: &'static str,
        source_lane: usize,
        pointer_offset: u32,
        duration: u32,
        current_lane: usize,
        current_start: u32,
    },
    ResizingClip {
        clip_id: u32,
        clip_name: &'static str,
        source_lane: usize,
        edge: ResizeEdge,
        fixed_beat: u32,
        current_range: BeatRange,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ResizeEdge {
    Start,
    End,
}

impl ArrangementTimelineWidget {
    pub(super) fn new(state: &TimelineEditorState) -> Self {
        Self {
            common: WidgetCommon::new(
                0,
                WidgetSizing::new(Vector2::new(520.0, 224.0), Vector2::new(760.0, 252.0)),
            )
            .with_keyboard_focus(),
            clips: state.clip_store.clips.clone(),
            selected_clip: state.edit.selected_clip,
            selection: state.edit.selection,
            playhead_beat: state.playback.playhead_beat,
            cursor: overlay::TimelineCursorOverlay::default(),
            hover_clip_id: None,
            drag: None,
        }
    }

    pub(super) fn geometry(&self, bounds: Rect) -> TimelineGeometry {
        TimelineGeometry::new(bounds)
    }
}

impl Widget for ArrangementTimelineWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        input::handle_timeline_input(self, bounds, input)
    }

    fn accepts_pointer_move(&self) -> bool {
        true
    }

    fn prefers_pointer_move_paint_only(&self) -> bool {
        true
    }

    fn synchronize_from_previous(&mut self, previous: &dyn Widget) {
        if let Some(previous) = previous.as_any().downcast_ref::<Self>() {
            self.common.state = previous.common.state;
            self.drag = previous.drag;
            self.cursor = previous.cursor;
            self.hover_clip_id = previous.hover_clip_id;
        }
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        paint::append_timeline_paint(self, primitives, bounds, theme);
    }

    fn append_runtime_overlay_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        overlay::append_runtime_timeline_overlay(self, primitives, bounds, theme);
    }
}
