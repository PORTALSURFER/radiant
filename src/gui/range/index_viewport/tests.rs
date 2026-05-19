use super::IndexViewport;

#[test]
fn index_viewport_clamps_visible_span_and_offset_fraction() {
    let viewport = IndexViewport {
        start: 990,
        end: 1_020,
    }
    .clamp(1_000, 128);

    assert_eq!(
        viewport,
        IndexViewport {
            start: 872,
            end: 1_000
        }
    );
    assert_eq!(viewport.visible_items(), 128);
    assert_eq!(viewport.visible_fraction(1_000), 0.128);
    assert_eq!(viewport.offset_fraction(1_000), 1.0);
}

#[test]
fn index_viewport_zooms_and_pans_around_visible_anchor() {
    let viewport = IndexViewport {
        start: 200,
        end: 600,
    };

    assert_eq!(
        viewport.zoom_around_anchor(1_000, 100, 0.5, 0.25),
        IndexViewport {
            start: 250,
            end: 450
        }
    );
    assert_eq!(
        viewport.pan_by_visible_fraction(1_000, 100, 0.5),
        IndexViewport {
            start: 400,
            end: 800
        }
    );
}

#[test]
fn index_viewport_sets_offset_and_projects_visible_ratio() {
    let viewport = IndexViewport { start: 0, end: 250 }.with_offset_fraction(1_000, 100, 0.5);

    assert_eq!(
        viewport,
        IndexViewport {
            start: 375,
            end: 625
        }
    );
    assert_eq!(viewport.absolute_ratio_from_visible(1_000, 100, 0.5), 0.5);
}
