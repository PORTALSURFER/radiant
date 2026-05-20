use super::FocusSurface;

#[test]
fn focus_surface_defaults_to_none() {
    assert_eq!(FocusSurface::default(), FocusSurface::None);
}
