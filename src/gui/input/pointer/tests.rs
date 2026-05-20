use super::*;

#[test]
fn logical_point_to_u16_coords_clamps_and_rounds() {
    assert_eq!(logical_point_to_u16_coords(Point::new(-4.0, 12.4)), (0, 12));
    assert_eq!(
        logical_point_to_u16_coords(Point::new(12.5, 65_999.0)),
        (13, u16::MAX)
    );
}
