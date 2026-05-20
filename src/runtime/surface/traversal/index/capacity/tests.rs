use super::*;

#[test]
fn widget_clip_capacity_is_zero_without_scroll_containers() {
    assert_eq!(
        widget_clip_capacity(SurfaceTraversalStats {
            widgets: 8,
            scroll_containers: 0,
            ..SurfaceTraversalStats::default()
        }),
        0
    );
}

#[test]
fn widget_clip_capacity_tracks_widgets_when_scroll_containers_exist() {
    assert_eq!(
        widget_clip_capacity(SurfaceTraversalStats {
            widgets: 8,
            scroll_containers: 1,
            ..SurfaceTraversalStats::default()
        }),
        8
    );
}
