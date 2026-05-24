use super::MixerPanelMessage;
use super::model::{CHANNEL_COUNT, MixerChannel};
use super::paint::{push_stroke, translucent};
use radiant::prelude::*;
use radiant::widgets::PaintBounds;

#[derive(Clone, Debug)]
pub(crate) struct MixerPanelWidget {
    pub(super) common: WidgetCommon,
    pub(super) channels: [MixerChannel; CHANNEL_COUNT],
    pub(super) selected_channel: usize,
    pub(super) frame: u64,
    pub(crate) hover_channel: Option<usize>,
    hover_position: Option<Point>,
    drag_channel: Option<usize>,
}

impl MixerPanelWidget {
    pub(crate) fn new(
        channels: [MixerChannel; CHANNEL_COUNT],
        selected_channel: usize,
        frame: u64,
    ) -> Self {
        let mut common = WidgetCommon::new(
            0,
            WidgetSizing::new(Vector2::new(760.0, 340.0), Vector2::new(1000.0, 380.0)),
        );
        common.focus = FocusBehavior::Pointer;
        common.paint.bounds = PaintBounds::ClipToRect;
        common.paint.paints_focus = false;
        common.paint.paints_state_layers = false;
        Self {
            common,
            channels,
            selected_channel,
            frame,
            hover_channel: None,
            hover_position: None,
            drag_channel: None,
        }
    }

    fn console_rect(&self, bounds: Rect) -> Rect {
        Rect::from_min_max(
            Point::new(bounds.min.x + 12.0, bounds.min.y + 12.0),
            Point::new(bounds.max.x - 12.0, bounds.max.y - 12.0),
        )
    }

    pub(crate) fn strip_rect(&self, bounds: Rect, channel: usize) -> Rect {
        let console = self.console_rect(bounds);
        let gap = 8.0;
        let strip_width =
            (console.width() - gap * (CHANNEL_COUNT - 1) as f32) / CHANNEL_COUNT as f32;
        let x = console.min.x + channel as f32 * (strip_width + gap);
        Rect::from_min_size(
            Point::new(x, console.min.y),
            Vector2::new(strip_width.max(1.0), console.height()),
        )
    }

    pub(crate) fn fader_rect(&self, strip: Rect) -> Rect {
        Rect::from_min_max(
            Point::new(strip.min.x + strip.width() * 0.58, strip.min.y + 56.0),
            Point::new(strip.min.x + strip.width() * 0.84, strip.max.y - 116.0),
        )
    }

    pub(super) fn meter_rect(&self, strip: Rect) -> Rect {
        Rect::from_min_max(
            Point::new(strip.min.x + strip.width() * 0.20, strip.min.y + 48.0),
            Point::new(strip.min.x + strip.width() * 0.46, strip.max.y - 112.0),
        )
    }

    pub(super) fn button_rect(&self, strip: Rect, index: usize) -> Rect {
        let width = (strip.width() - 22.0) / 3.0;
        let x = strip.min.x + 8.0 + index as f32 * (width + 3.0);
        Rect::from_min_size(
            Point::new(x, strip.max.y - 82.0),
            Vector2::new(width.max(18.0), 24.0),
        )
    }

    fn channel_at(&self, bounds: Rect, position: Point) -> Option<usize> {
        (0..CHANNEL_COUNT).find(|channel| self.strip_rect(bounds, *channel).contains(position))
    }

    fn fader_ratio_at(&self, strip: Rect, position: Point) -> f32 {
        let fader = self.fader_rect(strip);
        ((fader.max.y - position.y) / fader.height().max(1.0)).clamp(0.0, 1.0)
    }
}

impl Widget for MixerPanelWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        match input {
            WidgetInput::PointerMove { position } => self.handle_pointer_move(bounds, position),
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Primary,
                ..
            } if bounds.contains(position) => self.handle_primary_press(bounds, position),
            WidgetInput::PointerRelease {
                position,
                button: PointerButton::Primary,
                ..
            }
            | WidgetInput::PointerDrop {
                position,
                button: PointerButton::Primary,
                ..
            } => self.handle_primary_release(bounds, position),
            WidgetInput::FocusChanged(focused) => {
                self.common.state.focused = focused;
                None
            }
            _ => None,
        }
    }

    fn prefers_pointer_move_paint_only(&self) -> bool {
        self.drag_channel.is_none()
    }

    fn synchronize_from_previous(&mut self, previous: &dyn Widget) {
        if let Some(previous) = previous.as_any().downcast_ref::<Self>() {
            self.common.state = previous.common.state;
            self.hover_channel = previous.hover_channel;
            self.hover_position = previous.hover_position;
            self.drag_channel = previous.drag_channel;
        }
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        self.append_console_paint(primitives, bounds, theme);
    }

    fn append_runtime_overlay_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        let Some(channel) = self.hover_channel else {
            return;
        };
        let strip = self.strip_rect(bounds, channel);
        push_stroke(
            primitives,
            self.common.id,
            strip,
            translucent(theme.highlight_cyan, 170),
            2.0,
        );
    }
}

impl MixerPanelWidget {
    fn handle_pointer_move(&mut self, bounds: Rect, position: Point) -> Option<WidgetOutput> {
        self.common.state.hovered = bounds.contains(position);
        self.hover_position = bounds.contains(position).then_some(position);
        self.hover_channel = self.channel_at(bounds, position);
        self.drag_channel.map(|channel| {
            let strip = self.strip_rect(bounds, channel);
            WidgetOutput::custom(MixerPanelMessage::SetGain {
                channel,
                ratio: self.fader_ratio_at(strip, position),
            })
        })
    }

    fn handle_primary_press(&mut self, bounds: Rect, position: Point) -> Option<WidgetOutput> {
        let channel = self.channel_at(bounds, position)?;
        let strip = self.strip_rect(bounds, channel);
        self.selected_channel = channel;
        self.hover_channel = Some(channel);
        if self.fader_rect(strip).contains(position) {
            self.drag_channel = Some(channel);
            return Some(WidgetOutput::custom(MixerPanelMessage::SetGain {
                channel,
                ratio: self.fader_ratio_at(strip, position),
            }));
        }
        Some(WidgetOutput::custom(button_or_select_message(
            self, strip, channel, position,
        )))
    }

    fn handle_primary_release(&mut self, bounds: Rect, position: Point) -> Option<WidgetOutput> {
        let drag = self.drag_channel.take();
        self.hover_channel = self.channel_at(bounds, position);
        drag.map(|channel| {
            let strip = self.strip_rect(bounds, channel);
            WidgetOutput::custom(MixerPanelMessage::SetGain {
                channel,
                ratio: self.fader_ratio_at(strip, position),
            })
        })
    }
}

fn button_or_select_message(
    widget: &MixerPanelWidget,
    strip: Rect,
    channel: usize,
    position: Point,
) -> MixerPanelMessage {
    if widget.button_rect(strip, 0).contains(position) {
        MixerPanelMessage::ToggleMute(channel)
    } else if widget.button_rect(strip, 1).contains(position) {
        MixerPanelMessage::ToggleSolo(channel)
    } else if widget.button_rect(strip, 2).contains(position) {
        MixerPanelMessage::ToggleArm(channel)
    } else {
        MixerPanelMessage::Select(channel)
    }
}
