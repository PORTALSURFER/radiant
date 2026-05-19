use crate::gui_runtime::native_vello::TextCursorStop;

pub(super) fn cursor_stop_x(stops: &[TextCursorStop], byte_index: usize) -> f32 {
    if let Some(stop) = stops.iter().find(|stop| stop.byte_index == byte_index)
        && let Some(x) = finite_stop_x(stop)
    {
        return x;
    }
    stops
        .iter()
        .rev()
        .find(|stop| stop.byte_index <= byte_index && finite_stop_x(stop).is_some())
        .and_then(finite_stop_x)
        .unwrap_or(0.0)
}

pub(super) fn stop_index_for_byte(stops: &[TextCursorStop], byte_index: usize) -> usize {
    stops
        .iter()
        .position(|stop| stop.byte_index == byte_index)
        .unwrap_or_else(|| stops.len().saturating_sub(1))
}

pub(super) fn last_stop_at_or_before_x(stops: &[TextCursorStop], x: f32) -> usize {
    if !x.is_finite() {
        return 0;
    }
    stops
        .iter()
        .take_while(|stop| finite_stop_x(stop).is_some_and(|stop_x| stop_x <= x))
        .last()
        .map(|stop| stop.byte_index)
        .unwrap_or(0)
}

pub(super) fn visible_end_stop_index(
    stops: &[TextCursorStop],
    visible_start_index: usize,
    scroll_start_x: f32,
    width: f32,
) -> usize {
    let mut end = visible_start_index;
    while end + 1 < stops.len()
        && stop_local_x(&stops[end + 1], scroll_start_x).is_some_and(|x| x <= width)
    {
        end += 1;
    }
    if end == visible_start_index
        && end + 1 < stops.len()
        && stop_local_x(&stops[end + 1], scroll_start_x).is_some()
    {
        end += 1;
    }
    end
}

pub(super) fn build_visible_cursor_stops(
    stops: &[TextCursorStop],
    visible_start_index: usize,
    visible_end_index: usize,
    visible_start_byte: usize,
    scroll_start_x: f32,
    width: f32,
) -> Vec<TextCursorStop> {
    stops[visible_start_index..=visible_end_index]
        .iter()
        .map(|stop| TextCursorStop {
            byte_index: stop.byte_index.saturating_sub(visible_start_byte),
            x: stop_local_x(stop, scroll_start_x)
                .map(|x| x.clamp(0.0, width))
                .unwrap_or(0.0),
        })
        .collect()
}

pub(super) fn text_field_width(available_width: f32) -> f32 {
    if available_width.is_finite() && available_width > 0.0 {
        available_width
    } else {
        1.0
    }
}

fn finite_stop_x(stop: &TextCursorStop) -> Option<f32> {
    stop.x.is_finite().then_some(stop.x.max(0.0))
}

fn stop_local_x(stop: &TextCursorStop, scroll_start_x: f32) -> Option<f32> {
    let x = finite_stop_x(stop)? - scroll_start_x;
    x.is_finite().then_some(x)
}

#[cfg(test)]
mod tests {
    use super::{build_visible_cursor_stops, cursor_stop_x};
    use crate::gui_runtime::native_vello::TextCursorStop;

    #[test]
    fn cursor_stop_lookup_falls_back_from_non_finite_positions() {
        let stops = [
            TextCursorStop {
                byte_index: 0,
                x: 0.0,
            },
            TextCursorStop {
                byte_index: 2,
                x: f32::NAN,
            },
            TextCursorStop {
                byte_index: 4,
                x: 12.0,
            },
        ];

        assert_eq!(cursor_stop_x(&stops, 2), 0.0);
        assert_eq!(cursor_stop_x(&stops, 4), 12.0);
    }

    #[test]
    fn visible_cursor_stops_replace_invalid_positions_with_origin() {
        let stops = [
            TextCursorStop {
                byte_index: 0,
                x: 0.0,
            },
            TextCursorStop {
                byte_index: 2,
                x: f32::INFINITY,
            },
        ];

        let visible = build_visible_cursor_stops(&stops, 0, 1, 0, 0.0, 24.0);

        assert_eq!(
            visible,
            vec![
                TextCursorStop {
                    byte_index: 0,
                    x: 0.0,
                },
                TextCursorStop {
                    byte_index: 2,
                    x: 0.0,
                }
            ]
        );
    }
}
