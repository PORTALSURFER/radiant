use super::super::ViewNode;
use crate::widgets::{TextAlign, TextColorRole, TextWrap};

impl<Message> ViewNode<Message> {
    /// Allow text to wrap by words inside its assigned rectangle.
    pub fn wrap(mut self) -> Self {
        self.text_wrap = Some(TextWrap::Word);
        self
    }

    /// Keep text on one line and clip overflow.
    pub fn truncate(mut self) -> Self {
        self.text_wrap = Some(TextWrap::None);
        self
    }

    /// Set horizontal alignment for text widgets.
    pub fn align_text(mut self, align: TextAlign) -> Self {
        self.text_align = Some(align);
        self
    }

    /// Set the semantic foreground color for text-like widgets.
    pub fn text_color(mut self, color: TextColorRole) -> Self {
        self.text_color = Some(color);
        self
    }

    /// Use muted foreground text.
    pub fn muted_text(self) -> Self {
        self.text_color(TextColorRole::Muted)
    }
}
