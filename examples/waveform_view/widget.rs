use super::{
    WAVEFORM_HEIGHT, WAVEFORM_WIDTH,
    model::WaveformInteraction,
    source::{BAND_COUNT, SignalSource, WaveformViewport},
};
use radiant::{
    gui::{
        types::{Point, Rect, Rgba8},
        visualization::HorizontalValueAxis,
    },
    layout::LayoutOutput,
    prelude as ui,
    runtime::{
        GpuSurfaceCapabilities, GpuSurfaceContent, GpuSurfaceLineStyle, GpuSurfaceOverlay,
        GpuSurfaceRuntimeOverlays, PaintGpuSurface, PaintPrimitive,
    },
    theme::ThemeTokens,
    widgets::{FocusBehavior, PaintBounds, Widget, WidgetCommon, WidgetInput, WidgetOutput},
};
use std::sync::Arc;

pub(super) fn waveform_viewport(
    source: Arc<SignalSource>,
    viewport: WaveformViewport,
    cursor_ratio: Option<f32>,
) -> ui::View<WaveformInteraction> {
    ui::custom_widget(
        WaveformWidget::new(source, viewport, cursor_ratio),
        |output| output.typed_ref::<WaveformInteraction>().copied(),
    )
}

#[derive(Clone, Debug)]
pub(super) struct WaveformWidget {
    common: WidgetCommon,
    source: Arc<SignalSource>,
    viewport: WaveformViewport,
    cursor_ratio: Option<f32>,
}

impl WaveformWidget {
    pub(super) fn new(
        source: Arc<SignalSource>,
        viewport: WaveformViewport,
        cursor_ratio: Option<f32>,
    ) -> Self {
        let mut common = WidgetCommon::fixed(0, WAVEFORM_WIDTH as f32, WAVEFORM_HEIGHT as f32);
        common.focus = FocusBehavior::Pointer;
        common.paint.bounds = PaintBounds::ClipToRect;
        common.paint.paints_focus = false;
        common.paint.paints_state_layers = false;
        Self {
            common,
            source,
            viewport,
            cursor_ratio,
        }
    }

    fn ratio_from_position(&self, bounds: Rect, position: Point) -> f32 {
        HorizontalValueAxis::normalized(bounds).value_for_x(position.x)
    }
}

impl Widget for WaveformWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        match input {
            WidgetInput::PointerMove { position } if bounds.contains(position) => {
                self.common.state.hovered = true;
                None
            }
            WidgetInput::PointerMove { .. } => {
                self.common.state.hovered = false;
                None
            }
            WidgetInput::Wheel {
                position, delta, ..
            } if bounds.contains(position) => {
                Some(WidgetOutput::typed(WaveformInteraction::Wheel {
                    delta,
                    anchor_ratio: self.ratio_from_position(bounds, position),
                }))
            }
            _ => None,
        }
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        _theme: &ThemeTokens,
    ) {
        let overlays = self
            .cursor_ratio
            .filter(|ratio| ratio.is_finite())
            .map(|ratio| {
                vec![GpuSurfaceOverlay::VerticalCursor {
                    ratio: ratio.clamp(0.0, 1.0),
                    color: Rgba8 {
                        r: 255,
                        g: 232,
                        b: 180,
                        a: 245,
                    },
                    width: 2.0,
                }]
            })
            .unwrap_or_default();

        primitives.push(PaintPrimitive::GpuSurface(PaintGpuSurface {
            widget_id: self.common.id,
            key: self.source.identity_hash(),
            revision: 0,
            rect: bounds,
            content: GpuSurfaceContent::SignalSummaryBands {
                frames: self.source.frames,
                band_count: BAND_COUNT,
                frame_range: [self.viewport.start as f32, self.viewport.end as f32],
                summary: Arc::clone(&self.source.gpu_signal_summary),
                gain_preview: None,
            },
            capabilities: GpuSurfaceCapabilities {
                fast_pointer_move: true,
                coalesce_vertical_wheel: true,
                runtime_overlays: GpuSurfaceRuntimeOverlays::pointer_vertical_line(
                    GpuSurfaceLineStyle {
                        color: Rgba8 {
                            r: 255,
                            g: 255,
                            b: 255,
                            a: 235,
                        },
                        width: 1.5,
                    },
                ),
            },
            overlays,
        }));
    }
}
