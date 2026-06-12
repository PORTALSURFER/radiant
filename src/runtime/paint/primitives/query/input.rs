use super::super::{PaintPrimitive, PaintTextInput, SurfacePaintPlan};

impl SurfacePaintPlan {
    /// Iterate over native text-input paint primitives in paint order.
    pub fn text_inputs(&self) -> impl Iterator<Item = &PaintTextInput> {
        self.primitives
            .iter()
            .filter_map(PaintPrimitive::text_input)
    }

    /// Return the first native text-input paint primitive in paint order.
    pub fn first_text_input(&self) -> Option<&PaintTextInput> {
        self.text_inputs().next()
    }

    /// Return whether this paint plan contains any native text-input paint
    /// primitive.
    pub fn contains_text_input(&self) -> bool {
        self.first_text_input().is_some()
    }
}
