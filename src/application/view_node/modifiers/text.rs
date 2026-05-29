use super::super::ViewNode;
use crate::{
    layout::Vector2,
    widgets::{TextAlign, TextBackgroundRole, TextColorRole, TextWrap},
};

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

    /// Set a semantic background fill for text-like widgets.
    pub fn text_background(mut self, background: TextBackgroundRole) -> Self {
        self.text_background = Some(background);
        self
    }

    /// Set text insets inside the assigned widget bounds.
    pub fn text_inset(mut self, x: f32, y: f32) -> Self {
        self.text_inset = Some(Vector2::new(x.max(0.0), y.max(0.0)));
        self
    }

    /// Use muted foreground text.
    pub fn muted_text(self) -> Self {
        self.text_color(TextColorRole::Muted)
    }

    /// Use foreground text intended for strong accent backgrounds.
    pub fn on_accent_text(self) -> Self {
        self.text_color(TextColorRole::OnAccent)
    }
}
