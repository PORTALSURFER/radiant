use radiant::{gui::types::Rgba8, theme::ThemeTokens};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct PulseMeterVisual {
    pub(super) beat_markers: [PulseMarker; 5],
    pub(super) pulses: [PulseBar; 4],
    pub(super) playhead_center: f32,
    pub(super) playhead_radius: f32,
    pub(super) glow_radius: f32,
    pub(super) playhead_start: f32,
    pub(super) playhead_width: f32,
    running: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct PulseMarker {
    pub(super) center: f32,
    pub(super) width: f32,
    pub(super) color: Rgba8,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct PulseBar {
    pub(super) center: f32,
    pub(super) width: f32,
    pub(super) height_ratio: f32,
    pub(super) color: Rgba8,
}

impl PulseMeterVisual {
    pub(super) fn resolve(phase: f32, running: bool) -> Self {
        let phase = phase.clamp(0.0, 1.0);
        let beat = smoothstep(0.0, 1.0, 1.0 - (phase * 2.0 - 1.0).abs());
        let pulse = smoothstep(0.0, 1.0, (phase * std::f32::consts::TAU).sin() * 0.5 + 0.5);
        let playhead_width = 0.012;
        let playhead_center = phase * (1.0 - playhead_width) + playhead_width * 0.5;
        let playhead_start =
            (playhead_center - playhead_width * 0.5).clamp(0.0, 1.0 - playhead_width);
        let pulse_specs = [
            PulseBarSpec {
                center: playhead_center - 0.18,
                width: 0.007,
                height_ratio: 0.30,
                alpha: 54,
            },
            PulseBarSpec {
                center: playhead_center - 0.11,
                width: 0.009,
                height_ratio: 0.46,
                alpha: 84,
            },
            PulseBarSpec {
                center: playhead_center - 0.052,
                width: 0.011,
                height_ratio: 0.62 + pulse * 0.18,
                alpha: 120,
            },
            PulseBarSpec {
                center: playhead_center,
                width: 0.014,
                height_ratio: 0.78 + beat * 0.16,
                alpha: 190,
            },
        ];

        Self {
            beat_markers: [
                Self::marker(0.125, 48),
                Self::marker(0.3125, 40),
                Self::marker(0.5, 58),
                Self::marker(0.6875, 40),
                Self::marker(0.875, 48),
            ],
            pulses: pulse_specs.map(|spec| Self::bar(spec, running)),
            playhead_center,
            playhead_radius: if running { 4.8 + beat * 1.4 } else { 4.2 },
            glow_radius: if running { 9.0 + beat * 2.0 } else { 6.5 },
            playhead_start,
            playhead_width,
            running,
        }
    }

    fn marker(center: f32, alpha: u8) -> PulseMarker {
        PulseMarker {
            center,
            width: 0.0035,
            color: Rgba8 {
                r: 176,
                g: 182,
                b: 194,
                a: alpha,
            },
        }
    }

    fn bar(spec: PulseBarSpec, running: bool) -> PulseBar {
        PulseBar {
            center: wrap01(spec.center),
            width: spec.width,
            height_ratio: spec.height_ratio,
            color: Rgba8 {
                r: 255,
                g: 116,
                b: 76,
                a: if running { spec.alpha } else { spec.alpha / 3 },
            },
        }
    }

    pub(super) fn track_color(self, theme: &ThemeTokens) -> Rgba8 {
        if self.running {
            theme.surface_base
        } else {
            with_alpha(theme.surface_base, 210)
        }
    }

    pub(super) fn rail_color(self, theme: &ThemeTokens) -> Rgba8 {
        if self.running {
            with_alpha(theme.grid_soft, 95)
        } else {
            with_alpha(theme.grid_soft, 48)
        }
    }

    pub(super) fn glow_color(self, theme: &ThemeTokens) -> Rgba8 {
        if self.running {
            with_alpha(theme.highlight_orange, 70)
        } else {
            with_alpha(theme.highlight_orange, 32)
        }
    }

    pub(super) fn playhead_color(self, theme: &ThemeTokens) -> Rgba8 {
        if self.running {
            theme.highlight_orange
        } else {
            with_alpha(theme.highlight_orange, 115)
        }
    }

    pub(super) fn playhead_line_color(self, theme: &ThemeTokens) -> Rgba8 {
        if self.running {
            theme.text_primary
        } else {
            with_alpha(theme.text_primary, 150)
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct PulseBarSpec {
    center: f32,
    width: f32,
    height_ratio: f32,
    alpha: u8,
}

pub(super) fn wrap01(value: f32) -> f32 {
    value.rem_euclid(1.0)
}

fn with_alpha(mut color: Rgba8, alpha: u8) -> Rgba8 {
    color.a = alpha;
    color
}

fn smoothstep(edge0: f32, edge1: f32, value: f32) -> f32 {
    let t = ((value - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
