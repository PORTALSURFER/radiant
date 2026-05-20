use super::*;

#[test]
fn paint_path_preserves_backend_neutral_commands() {
    let path = PaintPath::from([
        PaintPathCommand::MoveTo(Point::new(0.0, 0.0)),
        PaintPathCommand::LineTo(Point::new(10.0, 0.0)),
        PaintPathCommand::QuadTo {
            control: Point::new(12.0, 4.0),
            to: Point::new(10.0, 10.0),
        },
        PaintPathCommand::Close,
    ]);

    assert_eq!(path.commands().len(), 4);
    assert!(!path.is_empty());
}

#[test]
fn paint_transform_uses_stable_affine_coefficients() {
    assert_eq!(
        PaintTransform::translate(3.0, 4.0).coefficients(),
        [1.0, 0.0, 0.0, 1.0, 3.0, 4.0]
    );
    assert_eq!(
        PaintTransform::scale_non_uniform(2.0, 5.0).coefficients(),
        [2.0, 0.0, 0.0, 5.0, 0.0, 0.0]
    );
}

#[test]
fn paint_transform_reports_finite_coefficients() {
    assert!(PaintTransform::IDENTITY.is_finite());
    assert!(!PaintTransform::new([1.0, 0.0, 0.0, f64::NAN, 0.0, 0.0]).is_finite());
}
