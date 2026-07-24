use super::*;

#[test]
fn synthetic_waveform_renders_nonblank_mono_image() {
    let mono_samples: Vec<f32> = (0..512)
        .map(|index| ((index as f32 / 16.0).sin() * 0.8).clamp(-1.0, 1.0))
        .collect();
    let file = synthetic_file(mono_samples, 48_000, 2);

    let image = render_waveform_image(&file, WaveformViewport::full(file.frames), 128, 48);

    assert_eq!(image.width(), 128);
    assert_eq!(image.height(), 48);
    assert!(
        image
            .pixels()
            .chunks_exact(4)
            .any(|pixel| pixel[0] > 240 && pixel[1] > 180 && pixel[2] > 150),
        "waveform ridge should produce visible bright pixels"
    );
}

#[test]
fn waveform_widget_paints_cached_body_and_cursor_overlay() {
    let mono_samples: Vec<f32> = (0..512)
        .map(|index| ((index as f32 / 16.0).sin() * 0.8).clamp(-1.0, 1.0))
        .collect();
    let file = Arc::new(synthetic_file(mono_samples, 48_000, 2));
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(320.0, 96.0));
    let mut widget = WaveformWidget::new(
        Arc::clone(&file),
        WaveformViewport::full(file.frames),
        Some(0.42),
    );
    let mut primitives = Vec::new();

    assert_eq!(
        widget.handle_input(
            bounds,
            WidgetInput::PointerMove {
                position: Point::new(160.0, 48.0)
            }
        ),
        None,
        "hover cursor updates should stay local to the widget"
    );
    widget.append_paint(
        &mut primitives,
        bounds,
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );

    assert!(
        primitives.iter().any(|primitive| matches!(
            primitive,
            PaintPrimitive::GpuSurface(PaintGpuSurface {
                capabilities,
                ..
            }) if capabilities.runtime_overlays.pointer_vertical_line.is_some()
                && capabilities.fast_pointer_move
                && capabilities.coalesce_vertical_wheel
        )),
        "waveform body should use a GPU signal primitive so zoom does not regenerate pixels"
    );
    assert!(
        primitives.iter().any(|primitive| matches!(
            primitive,
            PaintPrimitive::GpuSurface(PaintGpuSurface {
                overlays,
                ..
            }) if matches!(
                overlays.as_slice(),
                [RenderCanvasOverlay::VerticalCursor { ratio, .. }] if (*ratio - 0.42).abs() < 0.001
            )
        )),
        "playhead/cursor state should render as a lightweight GPU-surface overlay"
    );
    assert!(
        !primitives
            .iter()
            .any(|primitive| matches!(primitive, PaintPrimitive::FillPolygon(_)))
    );
    assert!(
        !primitives
            .iter()
            .any(|primitive| matches!(primitive, PaintPrimitive::StrokePolyline(_))),
        "cursor line should be handled by the GPU waveform shader"
    );
}

#[test]
fn waveform_widget_omits_cursor_overlay_when_absent() {
    let mono_samples: Vec<f32> = (0..512)
        .map(|index| ((index as f32 / 16.0).sin() * 0.8).clamp(-1.0, 1.0))
        .collect();
    let file = Arc::new(synthetic_file(mono_samples, 48_000, 2));
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(320.0, 96.0));
    let widget = WaveformWidget::new(Arc::clone(&file), WaveformViewport::full(file.frames), None);
    let mut primitives = Vec::new();

    widget.append_paint(
        &mut primitives,
        bounds,
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );

    assert!(
        primitives.iter().any(|primitive| matches!(
            primitive,
            PaintPrimitive::GpuSurface(PaintGpuSurface {
                overlays,
                ..
            }) if overlays.is_empty()
        )),
        "non-playing waveform surfaces should not carry a playhead overlay"
    );
}
