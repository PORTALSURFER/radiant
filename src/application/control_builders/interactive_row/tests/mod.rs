use super::*;
use crate::{
    application::{IntoView, ViewNode, text},
    gui::{
        list::{DenseRowMarkerParts, DenseRowMarkerStyle, DenseRowOutlineStyle, DenseRowPalette},
        types::{Point, Rect, Rgba8},
    },
    layout::Vector2,
    runtime::{PaintPrimitive, UiSurface},
    widgets::{InteractiveRowMessage, WidgetInput, WidgetOutput},
};

#[derive(Clone, Debug, PartialEq)]
enum DemoMessage {
    Activate,
    ActivateKey(String),
    ActivateWithModifiers(crate::widgets::PointerModifiers),
    ActivateWithModifiersKey(String, crate::widgets::PointerModifiers),
    DoubleActivate,
    DoubleActivateKey(String),
    DragKey(String, crate::widgets::DragHandleMessage),
    Drop,
    DropKey(String),
    HoverDrop(Point),
    HoverDropKey(String, Point),
    Secondary(Point),
    SecondaryKey(String, Point),
}

mod drag_drop;
mod primitive_input;
mod public_builder;
mod visual_state;
