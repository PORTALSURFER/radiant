use crate::gui::types::Point;

#[cfg(test)]
#[path = "pointer/tests.rs"]
mod tests;

/// Convert one logical pointer point into bounded integer coordinates.
///
/// Legacy action DTOs and automation payloads sometimes carry compact `u16`
/// coordinates. This helper keeps the clamp/round contract in the generic input
/// module so backend adapters do not hand-roll subtly different conversions.
pub fn logical_point_to_u16_coords(point: Point) -> (u16, u16) {
    (
        point.x.clamp(0.0, f32::from(u16::MAX)).round() as u16,
        point.y.clamp(0.0, f32::from(u16::MAX)).round() as u16,
    )
}
