use super::TextEditorWidget;
use crate::{
    gui::types::Rect,
    runtime::{
        PaintFillRect, PaintPrimitive, PaintStrokeRect, PaintTextEditor, ResolvedEnvironment,
    },
    theme::ThemeTokens,
};
impl TextEditorWidget {
    pub(super) fn paint_editor(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        theme: &ThemeTokens,
        environment: &ResolvedEnvironment,
    ) {
        let tokens = crate::widgets::resolve_widget_visual_tokens(
            theme,
            self.common.style,
            self.common.state,
        );
        primitives.push(PaintPrimitive::FillRect(PaintFillRect {
            widget_id: self.common.id,
            rect: bounds,
            color: theme.bg_primary,
        }));
        primitives.push(PaintPrimitive::StrokeRect(PaintStrokeRect {
            widget_id: self.common.id,
            rect: bounds,
            color: if self.common.state.focused {
                tokens.emphasis
            } else {
                tokens.border
            },
            width: 1.0,
        }));
        primitives.push(PaintPrimitive::TextEditor(Box::new(PaintTextEditor {
            reveal_caret: self.reveal_pending,
            request: self.layout_request(bounds, environment),
            selection: self.display_selection().unwrap_or_default(),
            scroll: self.scroll,
            color: tokens.foreground,
            selection_color: crate::runtime::blend_color(theme.bg_primary, tokens.emphasis, 0.34),
            caret_color: tokens.emphasis,
            focused: self.common.state.focused,
            hide_adornments: self.hide_adornments,
        })));
    }
}
