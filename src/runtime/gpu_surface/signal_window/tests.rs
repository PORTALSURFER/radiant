use super::*;
use crate::runtime::GpuSignalPosition;

fn window(origin: u64) -> GpuSignalSummaryWindow {
    GpuSignalSummaryWindow::new(
        origin + 100,
        origin,
        1,
        1,
        &[GpuSignalSummaryBucket {
            min: -0.5,
            max: 0.75,
        }; 64],
        7,
        1,
    )
    .unwrap()
}
fn presentation(origin: u64) -> GpuPreciseSignalPresentation {
    GpuPreciseSignalPresentation::new(
        GpuSignalViewport::new(GpuSignalPosition::new(origin + 8, 0.25).unwrap(), 16.0).unwrap(),
    )
}
fn floats(bytes: &[u8]) -> Vec<f32> {
    bytes
        .chunks_exact(4)
        .map(|x| f32::from_le_bytes(x.try_into().unwrap()))
        .collect()
}

#[test]
fn near_and_far_geometry_and_gain_bytes_are_identical() {
    let mut reference = None;
    for origin in [0, (1 << 24) - 16, 1 << 24, 1 << 40] {
        let mut p = presentation(origin);
        let selection =
            GpuSignalViewport::new(GpuSignalPosition::new(origin + 4, 0.0).unwrap(), 32.0).unwrap();
        let mut gain = GpuPreciseSignalGainPreview::new(selection);
        gain.fade_in_length = 0.5;
        gain.fade_out_length = 0.75;
        gain.fade_in_extension = 0.2;
        p.gain_preview = Some(gain);
        let bytes = window(origin).presentation_bytes(&p).unwrap();
        if let Some(previous) = &reference {
            assert_eq!(previous, &bytes);
        }
        reference = Some(bytes);
    }
}

#[test]
fn adjacent_samples_and_fractional_pan_remain_distinct() {
    for origin in [1 << 24, 1 << 40] {
        let w = window(origin);
        let p = presentation(origin);
        let first = floats(&w.presentation_bytes(&p).unwrap());
        let mut next = p;
        next.viewport = p.viewport.translated_frames(1).unwrap();
        let second = floats(&w.presentation_bytes(&next).unwrap());
        assert_eq!(second[0] - first[0], 1.0);
        assert_eq!(first[0], 8.25);
    }
}

#[test]
fn signed_slide_uses_exact_integer_source_coordinates() {
    let origin = 1 << 40;
    let w = window(origin);
    let mut p = presentation(origin);
    p.slide_frames = 5;
    assert_eq!(floats(&w.presentation_bytes(&p).unwrap())[0], 3.25);
    p.slide_frames = -5;
    assert_eq!(floats(&w.presentation_bytes(&p).unwrap())[0], 13.25);
    p.slide_frames = 20;
    assert_eq!(
        w.presentation_bytes(&p),
        Err(GpuSignalWindowError::MissingWindow)
    );
}

#[test]
fn wrap_requires_both_sides_in_the_provided_window() {
    let buckets = [GpuSignalSummaryBucket {
        min: -1.0,
        max: 1.0,
    }; 16];
    let full = GpuSignalSummaryWindow::new(16, 0, 1, 1, &buckets, 1, 0).unwrap();
    let partial = GpuSignalSummaryWindow::new(16, 8, 1, 1, &buckets[..8], 2, 0).unwrap();
    let mut p = GpuPreciseSignalPresentation::new(
        GpuSignalViewport::new(GpuSignalPosition::new(0, 0.0).unwrap(), 4.0).unwrap(),
    );
    p.slide_frames = 2;
    let projected = floats(&full.presentation_bytes(&p).unwrap());
    assert_eq!(&projected[..4], &[14.0, 4.0, 0.5, 16.0]);
    assert_eq!(
        partial.presentation_bytes(&p),
        Err(GpuSignalWindowError::MissingWindow)
    );
    p.slide_frames = i64::MIN;
    assert!(full.presentation_bytes(&p).is_ok());
}

#[test]
fn truncated_final_bucket_and_bucket_phase_are_validated() {
    let origin = 1 << 40;
    let buckets = [GpuSignalSummaryBucket::default(); 2];
    let w = GpuSignalSummaryWindow::new(origin + 7, origin, 4, 1, &buckets, 1, 0).unwrap();
    let p = GpuPreciseSignalPresentation::new(
        GpuSignalViewport::new(GpuSignalPosition::new(origin + 3, 0.5).unwrap(), 3.5).unwrap(),
    );
    assert_eq!(
        &floats(&w.presentation_bytes(&p).unwrap())[..2],
        &[0.875, 0.875]
    );
    let mut outside = p;
    outside.viewport = GpuSignalViewport::new(p.viewport.start(), 4.0).unwrap();
    assert_eq!(
        w.presentation_bytes(&outside),
        Err(GpuSignalWindowError::ViewportOutsideSource)
    );
    assert!(matches!(
        GpuSignalSummaryWindow::new(origin + 4, origin, 4, 1, &buckets, 1, 0),
        Err(GpuSignalWindowError::SourceRangeOverflow)
    ));
}

#[test]
fn viewport_motion_shares_immutable_storage() {
    let w = window(1 << 40);
    let p = presentation(1 << 40);
    let GpuSurfaceContent::CustomShader { descriptor: first } = w.content(&p).unwrap() else {
        panic!("custom shader content");
    };
    let mut moved = p;
    moved.viewport = p.viewport.translated_frames(1).unwrap();
    let GpuSurfaceContent::CustomShader { descriptor: second } = w.content(&moved).unwrap() else {
        panic!("custom shader content");
    };
    assert!(Arc::ptr_eq(&first.storage_bytes, &second.storage_bytes));
    assert!(Arc::ptr_eq(&first.uniform_bytes, &second.uniform_bytes));
    assert!(Arc::ptr_eq(
        first.wgsl_source.as_ref().unwrap(),
        second.wgsl_source.as_ref().unwrap()
    ));
    assert_ne!(
        first.presentation_uniform_bytes,
        second.presentation_uniform_bytes
    );
}

#[test]
fn malformed_data_and_presentation_are_explicit_errors() {
    assert!(matches!(
        GpuSignalSummaryWindow::new(10, 0, 1, 0, &[], 0, 0),
        Err(GpuSignalWindowError::InvalidShape)
    ));
    assert!(matches!(
        GpuSignalSummaryWindow::new(
            10,
            0,
            1,
            1,
            &[GpuSignalSummaryBucket {
                min: f32::NAN,
                max: 0.0
            }],
            0,
            0
        ),
        Err(GpuSignalWindowError::InvalidBucket)
    ));
    let w = window(0);
    let mut p = presentation(0);
    p.cursor_ratio = Some(f32::NAN);
    assert_eq!(
        w.presentation_bytes(&p),
        Err(GpuSignalWindowError::InvalidPresentation)
    );
    p.cursor_ratio = None;
    let mut gain = GpuPreciseSignalGainPreview::new(p.viewport);
    gain.fade_in_length = 2.0;
    p.gain_preview = Some(gain);
    assert_eq!(
        w.presentation_bytes(&p),
        Err(GpuSignalWindowError::InvalidPresentation)
    );
}

#[test]
fn precise_shader_parses_and_validates() {
    let module = naga::front::wgsl::parse_str(&precise_shader()).unwrap();
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    )
    .validate(&module)
    .unwrap();
}
