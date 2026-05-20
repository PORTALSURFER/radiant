use super::super::super::TimelineViewportParts;

pub(super) fn timeline_viewport_parts(
    start_milli: u16,
    end_milli: u16,
    start_micros: u32,
    end_micros: u32,
    start_nanos: u32,
    end_nanos: u32,
) -> TimelineViewportParts {
    TimelineViewportParts {
        start_milli,
        end_milli,
        start_micros,
        end_micros,
        start_nanos,
        end_nanos,
    }
}
