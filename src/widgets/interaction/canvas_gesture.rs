use super::{PointerButton, PointerModifiers, WidgetInput};
use crate::gui::types::{Point, Rect, Vector2};

/// Pointer event projected into a canvas-like widget's local coordinate space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CanvasPointer {
    /// Pointer position in the widget host's logical coordinate space.
    pub position: Point,
    /// Pointer position relative to the canvas rectangle.
    pub local: Point,
    /// Local position normalized to the canvas rectangle.
    pub normalized: Vector2,
}

/// High-level canvas gesture event resolved from [`WidgetInput`].
#[derive(Clone, Debug, PartialEq)]
pub enum CanvasGestureEvent {
    /// Pointer moved without an active drag.
    Hover(CanvasPointer),
    /// Pointer button pressed.
    Press {
        /// Pointer information at press time.
        pointer: CanvasPointer,
        /// Pressed button.
        button: PointerButton,
        /// Modifier state at press time.
        modifiers: PointerModifiers,
    },
    /// Pointer moved while the same button is captured.
    Drag {
        /// Pointer information for the current move.
        pointer: CanvasPointer,
        /// Pointer information from the original press.
        origin: CanvasPointer,
        /// Drag delta in host logical coordinates.
        delta: Vector2,
        /// Captured button.
        button: PointerButton,
        /// Modifier state from the original press.
        modifiers: PointerModifiers,
    },
    /// Captured pointer button was released.
    Release {
        /// Pointer information at release time.
        pointer: CanvasPointer,
        /// Pointer information from the original press.
        origin: CanvasPointer,
        /// Release delta in host logical coordinates.
        delta: Vector2,
        /// Released button.
        button: PointerButton,
        /// Modifier state at release time.
        modifiers: PointerModifiers,
    },
    /// Pointer button was double-clicked.
    DoubleClick {
        /// Pointer information at double-click time.
        pointer: CanvasPointer,
        /// Clicked button.
        button: PointerButton,
        /// Modifier state at double-click time.
        modifiers: PointerModifiers,
    },
    /// Pointer wheel or trackpad scroll occurred.
    Wheel {
        /// Pointer information at wheel time.
        pointer: CanvasPointer,
        /// Logical scroll delta. Positive values move content right/down.
        delta: Vector2,
    },
    /// Captured pointer was dropped or canceled.
    Drop {
        /// Pointer information at drop time.
        pointer: CanvasPointer,
        /// Pointer information from the original press, when this state owned one.
        origin: Option<CanvasPointer>,
        /// Dropped button.
        button: PointerButton,
        /// Modifier state at drop time.
        modifiers: PointerModifiers,
    },
    /// Keyboard focus changed.
    FocusChanged(bool),
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ActiveCanvasPress {
    origin: CanvasPointer,
    button: PointerButton,
    modifiers: PointerModifiers,
}

/// Retained pointer gesture state for canvas-like custom widgets.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CanvasGestureState {
    active_press: Option<ActiveCanvasPress>,
}

impl CanvasGestureState {
    /// Build an idle gesture state.
    pub const fn new() -> Self {
        Self { active_press: None }
    }

    /// Return whether a pointer press is currently captured.
    pub const fn is_dragging(&self) -> bool {
        self.active_press.is_some()
    }

    /// Clear any active pointer capture.
    pub fn cancel(&mut self) {
        self.active_press = None;
    }

    /// Convert one raw widget input into a canvas gesture event.
    pub fn handle_input(
        &mut self,
        bounds: Rect,
        input: &WidgetInput,
    ) -> Option<CanvasGestureEvent> {
        match input {
            WidgetInput::PointerMove { position } => {
                let pointer = canvas_pointer(bounds, *position)?;
                Some(match self.active_press {
                    Some(active) => CanvasGestureEvent::Drag {
                        pointer,
                        origin: active.origin,
                        delta: point_delta(active.origin.position, *position),
                        button: active.button,
                        modifiers: active.modifiers,
                    },
                    None => CanvasGestureEvent::Hover(pointer),
                })
            }
            WidgetInput::PointerPress {
                position,
                button,
                modifiers,
            } => {
                let pointer = canvas_pointer(bounds, *position)?;
                self.active_press = Some(ActiveCanvasPress {
                    origin: pointer,
                    button: *button,
                    modifiers: *modifiers,
                });
                Some(CanvasGestureEvent::Press {
                    pointer,
                    button: *button,
                    modifiers: *modifiers,
                })
            }
            WidgetInput::PointerDoubleClick {
                position,
                button,
                modifiers,
            } => {
                let pointer = canvas_pointer(bounds, *position)?;
                Some(CanvasGestureEvent::DoubleClick {
                    pointer,
                    button: *button,
                    modifiers: *modifiers,
                })
            }
            WidgetInput::PointerRelease {
                position,
                button,
                modifiers,
            } => {
                let pointer = canvas_pointer(bounds, *position)?;
                let active = self.active_press.take();
                Some(CanvasGestureEvent::Release {
                    pointer,
                    origin: active.map_or(pointer, |active| active.origin),
                    delta: active.map_or(Vector2::default(), |active| {
                        point_delta(active.origin.position, *position)
                    }),
                    button: *button,
                    modifiers: *modifiers,
                })
            }
            WidgetInput::PointerDrop {
                position,
                button,
                modifiers,
            } => {
                let pointer = canvas_pointer(bounds, *position)?;
                let active = self.active_press.take();
                Some(CanvasGestureEvent::Drop {
                    pointer,
                    origin: active.map(|active| active.origin),
                    button: *button,
                    modifiers: *modifiers,
                })
            }
            WidgetInput::Wheel { position, delta } => Some(CanvasGestureEvent::Wheel {
                pointer: canvas_pointer(bounds, *position)?,
                delta: *delta,
            }),
            WidgetInput::FocusChanged(focused) => {
                if !focused {
                    self.cancel();
                }
                Some(CanvasGestureEvent::FocusChanged(*focused))
            }
            WidgetInput::KeyPress(_) | WidgetInput::Character(_) | WidgetInput::TextEdit(_) => None,
        }
    }
}

fn canvas_pointer(bounds: Rect, position: Point) -> Option<CanvasPointer> {
    let width = bounds.width();
    let height = bounds.height();
    if !position.x.is_finite()
        || !position.y.is_finite()
        || !width.is_finite()
        || !height.is_finite()
        || width <= 0.0
        || height <= 0.0
    {
        return None;
    }
    let local = Point::new(position.x - bounds.min.x, position.y - bounds.min.y);
    Some(CanvasPointer {
        position,
        local,
        normalized: Vector2::new(
            (local.x / width).clamp(0.0, 1.0),
            (local.y / height).clamp(0.0, 1.0),
        ),
    })
}

fn point_delta(origin: Point, position: Point) -> Vector2 {
    Vector2::new(position.x - origin.x, position.y - origin.y)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bounds() -> Rect {
        Rect::from_min_size(Point::new(10.0, 20.0), Vector2::new(100.0, 50.0))
    }

    #[test]
    fn canvas_gesture_state_projects_local_and_normalized_positions() {
        let mut state = CanvasGestureState::new();
        let event = state
            .handle_input(
                bounds(),
                &WidgetInput::PointerMove {
                    position: Point::new(35.0, 45.0),
                },
            )
            .unwrap();

        let CanvasGestureEvent::Hover(pointer) = event else {
            panic!("expected hover event");
        };
        assert_eq!(pointer.local, Point::new(25.0, 25.0));
        assert_eq!(pointer.normalized, Vector2::new(0.25, 0.5));
    }

    #[test]
    fn canvas_gesture_state_tracks_press_drag_and_release() {
        let mut state = CanvasGestureState::new();
        let modifiers = PointerModifiers {
            shift: true,
            ..PointerModifiers::default()
        };

        state.handle_input(
            bounds(),
            &WidgetInput::PointerPress {
                position: Point::new(20.0, 30.0),
                button: PointerButton::Primary,
                modifiers,
            },
        );
        assert!(state.is_dragging());

        let drag = state
            .handle_input(
                bounds(),
                &WidgetInput::PointerMove {
                    position: Point::new(25.0, 42.0),
                },
            )
            .unwrap();
        let CanvasGestureEvent::Drag {
            origin,
            delta,
            button,
            modifiers: drag_modifiers,
            ..
        } = drag
        else {
            panic!("expected drag event");
        };
        assert_eq!(origin.position, Point::new(20.0, 30.0));
        assert_eq!(delta, Vector2::new(5.0, 12.0));
        assert_eq!(button, PointerButton::Primary);
        assert_eq!(drag_modifiers, modifiers);

        let release = state
            .handle_input(
                bounds(),
                &WidgetInput::PointerRelease {
                    position: Point::new(30.0, 35.0),
                    button: PointerButton::Primary,
                    modifiers: PointerModifiers::default(),
                },
            )
            .unwrap();
        let CanvasGestureEvent::Release { delta, .. } = release else {
            panic!("expected release event");
        };
        assert_eq!(delta, Vector2::new(10.0, 5.0));
        assert!(!state.is_dragging());
    }

    #[test]
    fn canvas_gesture_state_clears_drag_on_focus_loss() {
        let mut state = CanvasGestureState::new();
        state.handle_input(
            bounds(),
            &WidgetInput::PointerPress {
                position: Point::new(20.0, 30.0),
                button: PointerButton::Primary,
                modifiers: PointerModifiers::default(),
            },
        );

        assert!(matches!(
            state.handle_input(bounds(), &WidgetInput::FocusChanged(false)),
            Some(CanvasGestureEvent::FocusChanged(false))
        ));
        assert!(!state.is_dragging());
    }
}
