use super::*;
use radiant::{
    gui::svg::SvgIcon,
    widgets::{IconButtonWidget, PaintBounds},
};

const GENERIC_BUTTON_ID: u64 = 41;
const STANDARD_ICON_ID: u64 = 42;
const BARE_ICON_ID: u64 = 43;

fn viewport() -> Vector2 {
    Vector2::new(260.0, 32.0)
}

fn combined_button_surface() -> Arc<UiSurface<()>> {
    let icon = SvgIcon::from_svg(
        r##"<svg viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg">
  <rect fill="#5a6b7c" x="5" y="5" width="6" height="6"/>
</svg>"##,
    )
    .expect("valid focus regression icon");
    let mut button = ButtonWidget::new(
        GENERIC_BUTTON_ID,
        "Generic",
        WidgetSizing::fixed(Vector2::new(80.0, 24.0)),
    );
    let mut standard_icon = IconButtonWidget::new(
        STANDARD_ICON_ID,
        icon.clone(),
        WidgetSizing::fixed(Vector2::new(28.0, 24.0)),
    );
    let mut bare_icon = IconButtonWidget::new(
        BARE_ICON_ID,
        icon,
        WidgetSizing::fixed(Vector2::new(28.0, 24.0)),
    )
    .bare();
    for common in [
        &mut button.common,
        &mut standard_icon.common,
        &mut bare_icon.common,
    ] {
        common.state.selected = true;
        common.state.automation_active = true;
        assert_eq!(common.paint.bounds, PaintBounds::ClipToRect);
    }

    arc_surface(UiSurface::new(SurfaceNode::row(
        40,
        8.0,
        vec![
            SurfaceChild::new(intrinsic_slot(), SurfaceNode::static_widget(button)),
            SurfaceChild::new(intrinsic_slot(), SurfaceNode::static_widget(standard_icon)),
            SurfaceChild::new(intrinsic_slot(), SurfaceNode::static_widget(bare_icon)),
        ],
    )))
}

fn combined_button_runtime() -> SurfaceRuntime<impl RuntimeBridge<()>, ()> {
    SurfaceRuntime::new(
        declarative_runtime_bridge(
            (),
            |_state: &mut ()| combined_button_surface(),
            |_state: &mut (), _message: ()| {},
        ),
        viewport(),
    )
}

#[test]
fn clipped_runtime_frames_keep_each_combined_button_focus_cue_in_bounds() {
    let theme = ThemeTokens::default();
    let expected_tokens = resolve_widget_visual_tokens(
        &theme,
        WidgetStyle::default(),
        WidgetState {
            focused: true,
            selected: true,
            automation_active: true,
            ..WidgetState::default()
        },
    );
    let mut runtime = combined_button_runtime();

    for target_id in [GENERIC_BUTTON_ID, STANDARD_ICON_ID, BARE_ICON_ID] {
        assert!(runtime.focus_widget(target_id));
        let bounds = runtime.layout().rects[&target_id];
        let frame = runtime.borrowed_frame(&theme);
        let primitives = &frame.paint_plan.primitives;
        let clip_start = primitives
            .iter()
            .position(|primitive| {
                matches!(primitive, PaintPrimitive::ClipStart(clip)
                    if clip.node_id == target_id && clip.rect == bounds)
            })
            .unwrap_or_else(|| panic!("button {target_id} should start its ClipToRect region"));
        let clip_end = primitives
            .iter()
            .enumerate()
            .skip(clip_start + 1)
            .find_map(|(index, primitive)| {
                matches!(primitive, PaintPrimitive::ClipEnd(clip) if clip.node_id == target_id)
                    .then_some(index)
            })
            .unwrap_or_else(|| panic!("button {target_id} should end its ClipToRect region"));
        let clipped = &primitives[clip_start + 1..clip_end];

        let focus = clipped
            .iter()
            .filter_map(|primitive| match primitive {
                PaintPrimitive::StrokePolygon(stroke)
                    if stroke.widget_id == target_id
                        && stroke.color == expected_tokens.foreground
                        && (stroke.width - 2.0).abs() < f32::EPSILON =>
                {
                    Some(stroke)
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            focus.len(),
            1,
            "button {target_id} should paint one contrasting focus polygon"
        );
        let inset_width = bounds.width() - 2.0;
        let inset_height = bounds.height() - 2.0;
        let cut = (inset_height.min(inset_width) * 0.18).clamp(4.0, 8.0);
        assert_eq!(
            focus[0].points.as_ref(),
            [
                Point::new(bounds.min.x + 1.0, bounds.min.y + 1.0),
                Point::new(bounds.max.x - 1.0, bounds.min.y + 1.0),
                Point::new(bounds.max.x - 1.0, bounds.max.y - 1.0 - cut),
                Point::new(bounds.max.x - 1.0 - cut, bounds.max.y - 1.0),
                Point::new(bounds.min.x + 1.0, bounds.max.y - 1.0),
            ]
        );
        assert!(focus[0].points.iter().all(|point| {
            point.x >= bounds.min.x
                && point.x <= bounds.max.x
                && point.y >= bounds.min.y
                && point.y <= bounds.max.y
        }));

        let marker_x = clipped
            .iter()
            .filter_map(|primitive| match primitive {
                PaintPrimitive::StrokePolyline(marker)
                    if marker.widget_id == target_id
                        && marker.points.len() == 2
                        && marker.color == expected_tokens.foreground
                        && (marker.width - 2.0).abs() < f32::EPSILON =>
                {
                    Some(marker.points[0].x)
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            marker_x,
            vec![bounds.min.x + 2.0, bounds.max.x - 2.0],
            "button {target_id} should keep one leading and one trailing marker"
        );

        assert_eq!(
            primitives
                .iter()
                .filter(|primitive| {
                    matches!(primitive, PaintPrimitive::StrokePolygon(stroke)
                        if stroke.color == expected_tokens.foreground
                            && (stroke.width - 2.0).abs() < f32::EPSILON)
                })
                .count(),
            1,
            "only the individually focused button should paint a focus polygon"
        );
    }
}

#[cfg(target_os = "macos")]
#[test]
fn macos_offscreen_capture_keeps_clipped_bare_icon_focus_pixels_visible() {
    use radiant::{gui_runtime::OffscreenVelloCapture, theme::DpiScale};

    let theme = ThemeTokens::default();
    let expected_tokens = resolve_widget_visual_tokens(
        &theme,
        WidgetStyle::default(),
        WidgetState {
            focused: true,
            selected: true,
            automation_active: true,
            ..WidgetState::default()
        },
    );
    let mut runtime = combined_button_runtime();
    assert!(runtime.focus_widget(BARE_ICON_ID));
    let bounds = runtime.layout().rects[&BARE_ICON_ID];
    let plan = runtime.paint_plan(&theme);
    let dpi = DpiScale::new(2.0);
    let viewport = viewport();
    let mut capture = OffscreenVelloCapture::new(viewport, dpi)
        .expect("macOS focus pixel proof requires a compatible Vello adapter");
    let pixels = capture
        .capture(&plan)
        .expect("clipped button frame should render offscreen");
    let physical_width = dpi.logical_to_physical(viewport.x).round() as usize;
    let x_start = dpi.logical_to_physical(bounds.min.x + 6.0).round() as usize;
    let x_end = dpi.logical_to_physical(bounds.max.x - 6.0).round() as usize;
    let y_start = dpi.logical_to_physical(bounds.min.y).round() as usize;
    let y_end = dpi.logical_to_physical(bounds.min.y + 3.0).round() as usize;
    let foreground = [
        expected_tokens.foreground.r,
        expected_tokens.foreground.g,
        expected_tokens.foreground.b,
        expected_tokens.foreground.a,
    ];

    let visible_focus_pixels = (y_start..y_end)
        .flat_map(|y| (x_start..x_end).map(move |x| (y, x)))
        .filter(|(y, x)| {
            let offset = (y * physical_width + x) * 4;
            pixels[offset..offset + 4] == foreground
        })
        .count();
    assert!(
        visible_focus_pixels > 0,
        "the in-bounds top focus stroke should survive ClipToRect in rendered pixels"
    );
}
