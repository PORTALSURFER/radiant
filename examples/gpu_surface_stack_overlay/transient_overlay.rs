use super::*;
use crate::model::DemoState;

pub(super) fn paint_transient_blob(
    state: &DemoState,
    plan: &SurfacePaintPlan,
    animation_time: Duration,
    primitives: &mut Vec<PaintPrimitive>,
) {
    let Some(bounds) = plan.first_widget_rect(10) else {
        return;
    };
    paint_bouncing_ball(
        primitives,
        11,
        bounds,
        (animation_time.as_secs_f32() * 0.42).fract(),
        overlay_accent(state.selected),
    );
}

pub(super) fn overlay_accent(selected: bool) -> Rgba8 {
    if selected {
        Rgba8 {
            r: 82,
            g: 168,
            b: 255,
            a: 220,
        }
    } else {
        Rgba8 {
            r: 255,
            g: 142,
            b: 92,
            a: 220,
        }
    }
}

fn paint_bouncing_ball(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: WidgetId,
    bounds: Rect,
    phase: f32,
    accent: Rgba8,
) {
    let travel_x = bounds.width() - 32.0;
    let travel_y = bounds.height() - 42.0;
    let x = bounds.min.x + 16.0 + travel_x * triangle_wave(phase);
    let y = bounds.min.y + 20.0 + travel_y * triangle_wave((phase * 1.37 + 0.19) % 1.0);
    let rows = [
        (-10.0, -5.0, 20.0),
        (-14.0, -9.0, 28.0),
        (-16.0, -11.0, 32.0),
        (-14.0, -9.0, 28.0),
        (-10.0, -5.0, 20.0),
    ];
    for (offset_y, offset_x, width) in rows {
        primitives.push(PaintPrimitive::FillRect(PaintFillRect {
            widget_id,
            rect: Rect::from_min_size(
                Point::new(x + offset_x, y + offset_y),
                Vector2::new(width, 5.0),
            ),
            color: Rgba8 {
                r: 255,
                g: 255_u8.saturating_sub(accent.g / 3),
                b: accent.b.saturating_add(24),
                a: 235,
            },
        }));
    }
}

pub(super) fn triangle_wave(phase: f32) -> f32 {
    let wrapped = phase.fract();
    if wrapped < 0.5 {
        wrapped * 2.0
    } else {
        2.0 - wrapped * 2.0
    }
}
