use crate::gui::types::Rgba8;

use super::{Primitive, TextRun};

/// Full frame emitted by a retained render pipeline.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PaintFrame {
    /// Root clear color.
    pub clear_color: Rgba8,
    /// Shape primitives.
    pub primitives: Vec<Primitive>,
    /// Text primitives.
    pub text_runs: Vec<TextRun>,
}
