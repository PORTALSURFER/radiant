use super::{TextEditorCompositionDelta, TextEditorDelta, TextEditorSelection, TextEditorWidget};
use crate::widgets::interaction::CompositionStartContext;
use crate::{
    gui::{
        text_layout::paragraph::ParagraphCaret,
        types::{Point, Rect},
    },
    runtime::ResolvedEnvironment,
    widgets::{
        CompositionRange, CompositionSample, PointerButton, TextEditCommand, WidgetInput,
        WidgetKey, WidgetOutput,
    },
};
use std::sync::Arc;

fn scalar_byte(text: &str, index: usize) -> Option<usize> {
    text.char_indices()
        .map(|(at, _)| at)
        .chain(std::iter::once(text.len()))
        .nth(index)
}
impl TextEditorWidget {
    pub(super) fn handle_editor_input(
        &mut self,
        bounds: Rect,
        input: WidgetInput,
        environment: &ResolvedEnvironment,
    ) -> Option<WidgetOutput> {
        if self
            .geometry
            .as_ref()
            .is_some_and(|receipt| receipt.request() != &self.layout_request(bounds, environment))
        {
            self.geometry = None;
        }
        match input {
            WidgetInput::FocusChanged(focused) => {
                self.common.state.focused = focused;
                self.common.state.pressed = false;
                if !focused && self.snapshot.is_composing() {
                    self.cancel_editor_composition()
                } else {
                    None
                }
            }
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Primary,
                modifiers,
                ..
            } if bounds.contains(position) && !self.common.state.disabled => {
                let caret = self.pointer_caret(position)?;
                if self.snapshot.is_composing() {
                    return None;
                }
                self.common.state.pressed = true;
                self.common.state.focused = true;
                self.preferred_x = None;
                self.move_caret(caret, modifiers.shift)
            }
            WidgetInput::PointerMove { position, .. } => {
                self.common.state.hovered = bounds.contains(position);
                if self.common.state.pressed && !self.snapshot.is_composing() {
                    let caret = self.pointer_caret(position)?;
                    self.preferred_x = None;
                    self.move_caret(caret, true)
                } else {
                    None
                }
            }
            WidgetInput::PointerRelease {
                button: PointerButton::Primary,
                ..
            } => {
                self.common.state.pressed = false;
                None
            }
            WidgetInput::Wheel {
                position, delta, ..
            } if bounds.contains(position) && delta.x.is_finite() && delta.y.is_finite() => {
                let previous = self.scroll;
                self.scroll.x += delta.x;
                self.scroll.y += delta.y;
                self.clamp_scroll();
                self.reveal_pending = false;
                (self.scroll != previous).then(|| WidgetOutput::typed(EditorScrollChanged))
            }
            WidgetInput::Character { character, .. }
                if self.editing_enabled() && !character.is_control() =>
            {
                self.replace_range(self.selection().range(), Arc::from(character.to_string()))
            }
            WidgetInput::TextEdit { command, .. } if self.editing_enabled() => {
                self.edit_command(command)
            }
            WidgetInput::KeyPress { key, modifiers, .. } if self.editing_enabled() => {
                let extend = modifiers.shift;
                if self.snapshot.is_composing() {
                    return if key == WidgetKey::Escape {
                        self.cancel_editor_composition()
                    } else {
                        None
                    };
                }
                match key {
                    WidgetKey::Enter => {
                        self.replace_range(self.selection().range(), Arc::from("\n"))
                    }
                    WidgetKey::Tab => self.replace_range(self.selection().range(), Arc::from("\t")),
                    WidgetKey::ArrowUp | WidgetKey::ArrowDown
                        if modifiers.control || modifiers.alt =>
                    {
                        self.move_paragraph(key == WidgetKey::ArrowDown, extend)
                    }
                    WidgetKey::ArrowUp | WidgetKey::ArrowDown if modifiers.command => self
                        .move_logical(
                            if key == WidgetKey::ArrowUp {
                                0
                            } else {
                                self.text().len()
                            },
                            extend,
                        ),
                    WidgetKey::ArrowUp => self.move_vertical(-1.0, extend),
                    WidgetKey::ArrowDown => self.move_vertical(1.0, extend),
                    WidgetKey::PageUp | WidgetKey::PageDown => {
                        let receipt = self.current_geometry()?;
                        let lines = (receipt.request().rect.height()
                            / receipt.request().line_height)
                            .floor()
                            .max(1.0);
                        self.move_vertical(
                            if key == WidgetKey::PageUp {
                                -lines
                            } else {
                                lines
                            },
                            extend,
                        )
                    }
                    WidgetKey::Home | WidgetKey::End if modifiers.command || modifiers.control => {
                        self.move_logical(
                            if key == WidgetKey::Home {
                                0
                            } else {
                                self.text().len()
                            },
                            extend,
                        )
                    }
                    WidgetKey::Home => self.move_line_end(false, extend),
                    WidgetKey::End => self.move_line_end(true, extend),
                    WidgetKey::ArrowLeft => {
                        self.edit_command(if modifiers.alt || modifiers.control {
                            TextEditCommand::MoveWordLeft {
                                extend_selection: extend,
                            }
                        } else {
                            TextEditCommand::MoveLeft {
                                extend_selection: extend,
                            }
                        })
                    }
                    WidgetKey::ArrowRight => {
                        self.edit_command(if modifiers.alt || modifiers.control {
                            TextEditCommand::MoveWordRight {
                                extend_selection: extend,
                            }
                        } else {
                            TextEditCommand::MoveRight {
                                extend_selection: extend,
                            }
                        })
                    }
                    WidgetKey::Backspace => self.edit_command(TextEditCommand::Backspace),
                    WidgetKey::Delete => self.edit_command(TextEditCommand::Delete),
                    _ => None,
                }
            }
            _ => None,
        }
    }
    fn pointer_caret(&self, position: Point) -> Option<ParagraphCaret> {
        if !position.is_finite() {
            return None;
        }
        let receipt = self.current_geometry()?;
        Some(receipt.geometry().hit_test(Point::new(
            position.x - receipt.request().rect.min.x + self.scroll.x,
            position.y - receipt.request().rect.min.y + self.scroll.y,
        )))
    }
    pub(super) fn editor_composition_context(&self) -> Option<CompositionStartContext> {
        if !self.editing_enabled() || self.snapshot.is_composing() {
            return None;
        }
        let range = self.selection().range();
        let text = self.text();
        let range = CompositionRange::new(
            text.get(..range.start)?.chars().count(),
            text.get(..range.end)?.chars().count(),
            text.chars().count(),
        )
        .ok()?;
        CompositionStartContext::new(range, range).ok()
    }
    pub(super) fn editor_composition(&mut self, sample: CompositionSample) -> Option<WidgetOutput> {
        if !self.editing_enabled() || !sample.is_valid() {
            return None;
        }
        match sample {
            CompositionSample::Start {
                replacement_range,
                selection,
                ..
            } => {
                let count = self.text().chars().count();
                if !replacement_range.is_valid_for(count) || !selection.is_valid_for(count) {
                    return None;
                }
                let range = scalar_byte(self.text(), replacement_range.start())?
                    ..scalar_byte(self.text(), replacement_range.end())?;
                let selection = TextEditorSelection {
                    anchor: scalar_byte(self.text(), selection.start())?,
                    caret: scalar_byte(self.text(), selection.end())?,
                    affinity: self.selection().affinity,
                };
                let output = self.emit(
                    TextEditorDelta::Composition(TextEditorCompositionDelta::Start { range }),
                    selection,
                );
                if output.is_some() {
                    self.hide_adornments = false;
                }
                output
            }
            CompositionSample::Update {
                preedit, selection, ..
            } => {
                if !selection.is_valid_for(preedit.chars().count()) {
                    return None;
                }
                let range = self.snapshot.composition_range()?;
                let anchor = range
                    .start
                    .checked_add(scalar_byte(&preedit, selection.start())?)?;
                let caret = range
                    .start
                    .checked_add(scalar_byte(&preedit, selection.end())?)?;
                let output = self.emit(
                    TextEditorDelta::Composition(TextEditorCompositionDelta::Update {
                        text: preedit.into(),
                    }),
                    TextEditorSelection {
                        anchor,
                        caret,
                        affinity: self.selection().affinity,
                    },
                );
                if output.is_some() {
                    self.hide_adornments = false;
                }
                output
            }
            CompositionSample::Commit { text, .. } => {
                let range = self.snapshot.composition_range()?;
                let caret = range.start.checked_add(text.len())?;
                let output = self.emit(
                    TextEditorDelta::Composition(TextEditorCompositionDelta::Commit {
                        text: text.into(),
                    }),
                    TextEditorSelection::caret(caret),
                );
                if output.is_some() {
                    self.hide_adornments = false;
                }
                output
            }
            CompositionSample::Cancel { .. } => self.cancel_editor_composition(),
        }
    }
    pub(super) fn editor_hidden_preedit(&mut self, preedit: String) -> Option<WidgetOutput> {
        if !self.editing_enabled() {
            return None;
        }
        let range = self.snapshot.composition_range()?;
        let caret = range.start.checked_add(preedit.len())?;
        let output = self.emit(
            TextEditorDelta::Composition(TextEditorCompositionDelta::Update {
                text: preedit.into(),
            }),
            TextEditorSelection::caret(caret),
        );
        if output.is_some() {
            self.hide_adornments = true;
        }
        output
    }
    fn cancel_editor_composition(&mut self) -> Option<WidgetOutput> {
        let selection = self.snapshot.composition_original_selection()?;
        let output = self.emit(
            TextEditorDelta::Composition(TextEditorCompositionDelta::Cancel),
            selection,
        );
        if output.is_some() {
            self.hide_adornments = false;
        }
        output
    }
}

/// Internal viewport output requests the existing runtime repaint path without a product delta.
#[derive(Clone)]
struct EditorScrollChanged;
