mod track;

pub use track::{
    horizontal_discrete_meter_fill_rect, horizontal_meter_fill_rect,
    horizontal_progress_activity_rect, horizontal_progress_fill_rect,
    horizontal_progress_track_rect,
};

/// Progress overlay state for long-running operations.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct ProgressOverlay {
    /// Whether the overlay is currently visible.
    pub visible: bool,
    /// Whether the overlay is modal.
    pub modal: bool,
    /// Title text for the progress surface.
    pub title: String,
    /// Optional detail line.
    pub detail: Option<String>,
    /// Completed steps.
    pub completed: usize,
    /// Total steps.
    pub total: usize,
    /// Whether the running operation supports cancel.
    pub cancelable: bool,
    /// Whether cancel has already been requested.
    pub cancel_requested: bool,
}

#[cfg(test)]
mod tests {
    use super::{
        ProgressOverlay, horizontal_discrete_meter_fill_rect, horizontal_meter_fill_rect,
        horizontal_progress_activity_rect, horizontal_progress_fill_rect,
        horizontal_progress_track_rect,
    };
    use crate::gui::types::{Point, Rect};

    #[test]
    fn progress_overlay_defaults_to_hidden_and_empty() {
        let overlay = ProgressOverlay::default();

        assert!(!overlay.visible);
        assert!(!overlay.modal);
        assert_eq!(overlay.title, "");
        assert_eq!(overlay.detail, None);
        assert_eq!(overlay.completed, 0);
        assert_eq!(overlay.total, 0);
        assert!(!overlay.cancelable);
        assert!(!overlay.cancel_requested);
    }

    #[test]
    fn horizontal_progress_fill_rect_clamps_to_track() {
        let track = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 28.0));

        let overfilled = horizontal_progress_fill_rect(track, 1.5).expect("filled rect");
        assert_eq!(overfilled.min, track.min);
        assert_eq!(overfilled.max, track.max);

        let partial = horizontal_progress_fill_rect(track, 0.25).expect("partial rect");
        assert_eq!(partial.min, track.min);
        assert_eq!(partial.max, Point::new(35.0, 28.0));
    }

    #[test]
    fn horizontal_progress_fill_rect_omits_empty_tracks_and_zero_fraction() {
        let track = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 28.0));
        let empty_width = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(10.0, 28.0));
        let empty_height = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 20.0));

        assert_eq!(horizontal_progress_fill_rect(track, 0.0), None);
        assert_eq!(horizontal_progress_fill_rect(track, -0.5), None);
        assert_eq!(horizontal_progress_fill_rect(empty_width, 0.5), None);
        assert_eq!(horizontal_progress_fill_rect(empty_height, 0.5), None);
    }

    #[test]
    fn horizontal_progress_activity_rect_resolves_moving_segment() {
        let track = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 28.0));

        let start =
            horizontal_progress_activity_rect(track, 0.0, 0.24, 18.0).expect("start segment");
        assert_eq!(start.min, track.min);
        assert_eq!(start.max, Point::new(34.0, 28.0));

        let end = horizontal_progress_activity_rect(track, 1.0, 0.24, 18.0).expect("end segment");
        assert_eq!(end.min, Point::new(86.0, 20.0));
        assert_eq!(end.max, track.max);
    }

    #[test]
    fn horizontal_progress_activity_rect_clamps_cramped_tracks() {
        let track = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(20.0, 28.0));

        let segment =
            horizontal_progress_activity_rect(track, 0.5, 0.24, 18.0).expect("cramped segment");
        assert_eq!(segment, track);

        assert_eq!(
            horizontal_progress_activity_rect(track, 0.5, 0.0, 0.0),
            None
        );
    }

    #[test]
    fn horizontal_progress_track_rect_switches_between_activity_and_fill() {
        let track = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 28.0));

        let activity =
            horizontal_progress_track_rect(track, 0, 0, 0.5, 0.24, 18.0).expect("activity");
        assert_eq!(activity.min, Point::new(48.0, 20.0));
        assert_eq!(activity.max, Point::new(72.0, 28.0));

        let determinate =
            horizontal_progress_track_rect(track, 1, 4, 0.5, 0.24, 18.0).expect("fill");
        assert_eq!(determinate.min, track.min);
        assert_eq!(determinate.max, Point::new(35.0, 28.0));
    }

    #[test]
    fn horizontal_meter_fill_rect_clamps_level_and_minimum_width() {
        let track = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 28.0));

        let minimum = horizontal_meter_fill_rect(track, 0.0, 1.0).expect("minimum meter");
        assert_eq!(minimum.min, track.min);
        assert_eq!(minimum.max, Point::new(11.0, 28.0));

        let overfilled = horizontal_meter_fill_rect(track, 2.0, 1.0).expect("overfilled meter");
        assert_eq!(overfilled, track);

        assert_eq!(horizontal_meter_fill_rect(track, 0.0, 0.0), None);
    }

    #[test]
    fn horizontal_discrete_meter_fill_rect_rounds_and_clamps_byte_levels() {
        let track = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 28.0));

        assert_eq!(horizontal_discrete_meter_fill_rect(track, 0, 255), None);
        assert_eq!(horizontal_discrete_meter_fill_rect(track, 1, 255), None);

        let half = horizontal_discrete_meter_fill_rect(track, 128, 255).expect("half meter");
        assert_eq!(half.min, track.min);
        assert_eq!(half.max, Point::new(60.0, 28.0));

        let full = horizontal_discrete_meter_fill_rect(track, 999, 255).expect("full meter");
        assert_eq!(full, track);
    }
}
