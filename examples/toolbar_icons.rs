//! Horizontal SVG-icon toolbar with state-driven toggle buttons.

use radiant::gui::{
    svg::rasterize_svg_icon,
    types::{ImageRgba, Rect, Rgba8},
};
use radiant::layout::{LayoutOutput, Point, Vector2};
use radiant::prelude::*;
use radiant::runtime::{PaintFillRect, PaintImage, PaintPrimitive, PaintStrokeRect};
use radiant::theme::ThemeTokens;
use radiant::widgets::{
    FocusBehavior, PointerButton, WidgetCommon, WidgetInput, WidgetSizing, WidgetState,
    WidgetStyle, WidgetTone, resolve_widget_visual_tokens,
};
use std::sync::Arc;

const ICON_SIZE: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ToolId {
    Select,
    Brush,
    Erase,
    Snap,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ToolMessage {
    Toggle(ToolId),
}

#[derive(Clone, Debug)]
struct ToolbarState {
    select: bool,
    brush: bool,
    erase: bool,
    snap: bool,
    icons: ToolbarIcons,
}

impl Default for ToolbarState {
    fn default() -> Self {
        Self {
            select: true,
            brush: false,
            erase: false,
            snap: true,
            icons: ToolbarIcons::new(&ThemeTokens::default()),
        }
    }
}

impl ToolbarState {
    fn active(&self, tool: ToolId) -> bool {
        match tool {
            ToolId::Select => self.select,
            ToolId::Brush => self.brush,
            ToolId::Erase => self.erase,
            ToolId::Snap => self.snap,
        }
    }

    fn toggle(&mut self, tool: ToolId) {
        match tool {
            ToolId::Select => self.select = !self.select,
            ToolId::Brush => self.brush = !self.brush,
            ToolId::Erase => self.erase = !self.erase,
            ToolId::Snap => self.snap = !self.snap,
        }
    }

    fn summary(&self) -> String {
        let active = [
            (self.select, "Select"),
            (self.brush, "Brush"),
            (self.erase, "Erase"),
            (self.snap, "Snap"),
        ]
        .into_iter()
        .filter_map(|(active, label)| active.then_some(label))
        .collect::<Vec<_>>();
        if active.is_empty() {
            "No tools active".to_string()
        } else {
            format!("Active: {}", active.join(", "))
        }
    }
}

#[derive(Clone, Debug)]
struct ToolbarIcons {
    select: ToolbarIcon,
    brush: ToolbarIcon,
    erase: ToolbarIcon,
    snap: ToolbarIcon,
}

impl ToolbarIcons {
    fn new(theme: &ThemeTokens) -> Self {
        Self {
            select: ToolbarIcon::new(SELECT_ICON, theme.highlight_cyan, theme.text_muted),
            brush: ToolbarIcon::new(BRUSH_ICON, theme.accent_mint, theme.text_muted),
            erase: ToolbarIcon::new(ERASE_ICON, theme.accent_warning, theme.text_muted),
            snap: ToolbarIcon::new(SNAP_ICON, theme.accent_copper, theme.text_muted),
        }
    }

    fn icon(&self, tool: ToolId) -> ToolbarIcon {
        match tool {
            ToolId::Select => self.select.clone(),
            ToolId::Brush => self.brush.clone(),
            ToolId::Erase => self.erase.clone(),
            ToolId::Snap => self.snap.clone(),
        }
    }
}

#[derive(Clone, Debug)]
struct ToolbarIcon {
    active: Arc<ImageRgba>,
    inactive: Arc<ImageRgba>,
}

impl ToolbarIcon {
    fn new(svg: &str, active: Rgba8, inactive: Rgba8) -> Self {
        Self {
            active: toolbar_icon_image(svg, active),
            inactive: toolbar_icon_image(svg, inactive),
        }
    }
}

#[derive(Clone, Debug)]
struct IconToggleButton {
    common: WidgetCommon,
    tool: ToolId,
    icon: ToolbarIcon,
    active: bool,
}

impl IconToggleButton {
    fn new(tool: ToolId, icon: ToolbarIcon, active: bool) -> Self {
        let mut common = WidgetCommon::new(0, WidgetSizing::fixed(Vector2::new(42.0, 36.0)));
        common.focus = FocusBehavior::Keyboard;
        common.state.active = active;
        common.style = WidgetStyle {
            tone: WidgetTone::Accent,
            ..WidgetStyle::default()
        };
        Self {
            common,
            tool,
            icon,
            active,
        }
    }
}

impl Widget for IconToggleButton {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        match input {
            WidgetInput::PointerMove { position } => {
                self.common.state.hovered = bounds.contains(position);
                None
            }
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Primary,
            } => {
                self.common.state.pressed = bounds.contains(position);
                None
            }
            WidgetInput::PointerRelease {
                position,
                button: PointerButton::Primary,
            } => {
                let should_toggle = self.common.state.pressed && bounds.contains(position);
                self.common.state.pressed = false;
                should_toggle.then(|| WidgetOutput::custom(ToolMessage::Toggle(self.tool)))
            }
            WidgetInput::KeyPress(radiant::widgets::WidgetKey::Enter)
            | WidgetInput::KeyPress(radiant::widgets::WidgetKey::Space)
                if self.common.state.focused =>
            {
                Some(WidgetOutput::custom(ToolMessage::Toggle(self.tool)))
            }
            WidgetInput::FocusChanged(focused) => {
                self.common.state.focused = focused;
                None
            }
            _ => None,
        }
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        let tokens =
            resolve_widget_visual_tokens(theme, self.common.style, toolbar_state(&self.common));
        primitives.push(PaintPrimitive::FillRect(PaintFillRect {
            widget_id: self.common.id,
            rect: bounds,
            color: tokens.fill,
        }));
        primitives.push(PaintPrimitive::StrokeRect(PaintStrokeRect {
            widget_id: self.common.id,
            rect: bounds,
            color: tokens.border,
            width: 1.0,
        }));
        if self.common.state.focused {
            primitives.push(PaintPrimitive::StrokeRect(PaintStrokeRect {
                widget_id: self.common.id,
                rect: Rect::from_min_max(
                    Point::new(bounds.min.x - 1.0, bounds.min.y - 1.0),
                    Point::new(bounds.max.x + 1.0, bounds.max.y + 1.0),
                ),
                color: tokens.emphasis,
                width: 1.0,
            }));
        }
        if self.active {
            primitives.push(PaintPrimitive::FillRect(PaintFillRect {
                widget_id: self.common.id,
                rect: Rect::from_min_max(
                    Point::new(bounds.min.x + 7.0, bounds.max.y - 4.0),
                    Point::new(bounds.max.x - 7.0, bounds.max.y - 2.0),
                ),
                color: tokens.emphasis,
            }));
        }
        let icon_side = 20.0;
        let icon_rect = Rect::from_min_size(
            Point::new(
                bounds.min.x + (bounds.width() - icon_side) * 0.5,
                bounds.min.y + (bounds.height() - icon_side) * 0.5 - 1.0,
            ),
            Vector2::new(icon_side, icon_side),
        );
        primitives.push(PaintPrimitive::Image(PaintImage {
            widget_id: self.common.id,
            source_rect: None,
            rect: icon_rect,
            image: if self.active {
                Arc::clone(&self.icon.active)
            } else {
                Arc::clone(&self.icon.inactive)
            },
        }));
    }
}

fn toolbar_state(common: &WidgetCommon) -> WidgetState {
    common.state
}

fn main() -> radiant::Result {
    radiant::app(ToolbarState::default())
        .title("Radiant Toolbar Icons")
        .size(360, 150)
        .min_size(300, 120)
        .view(project_surface)
        .update(|state, message| match message {
            ToolMessage::Toggle(tool) => state.toggle(tool),
        })
        .run()
}

fn project_surface(state: &mut ToolbarState) -> View<ToolMessage> {
    column([
        text("Icon Toolbar").height(28.0).fill_width(),
        toolbar(state),
        text(state.summary()).height(28.0).fill_width(),
    ])
    .padding(16.0)
    .spacing(12.0)
    .fill()
}

fn toolbar(state: &ToolbarState) -> View<ToolMessage> {
    row([
        toolbar_button(state, ToolId::Select).id(10),
        toolbar_button(state, ToolId::Brush).id(11),
        toolbar_button(state, ToolId::Erase).id(12),
        toolbar_button(state, ToolId::Snap).id(13),
    ])
    .spacing(8.0)
    .height(42.0)
    .fill_width()
}

fn toolbar_button(state: &ToolbarState, tool: ToolId) -> View<ToolMessage> {
    custom_widget_mapped(
        IconToggleButton::new(tool, state.icons.icon(tool), state.active(tool)),
        |message: ToolMessage| message,
    )
}

fn toolbar_icon_image(svg: &str, color: Rgba8) -> Arc<ImageRgba> {
    Arc::new(rasterize_svg_icon(svg, ICON_SIZE, color).expect("toolbar icon SVG should rasterize"))
}

const SELECT_ICON: &str = r#"
<svg viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
  <polygon points="5,3 18,13 12,14 15,21 12,22 9,15 5,19" />
</svg>
"#;

const BRUSH_ICON: &str = r#"
<svg viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
  <path d="M 15 3 L 21 9 L 12 18 L 6 12 Z" />
  <path d="M 5 13 L 11 19 L 8 22 L 2 22 L 2 16 Z" />
</svg>
"#;

const ERASE_ICON: &str = r#"
<svg viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
  <path d="M 8 4 L 21 17 L 15 23 L 2 10 Z" />
  <rect x="7" y="16" width="11" height="4" />
</svg>
"#;

const SNAP_ICON: &str = r#"
<svg viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
  <rect x="4" y="3" width="6" height="12" />
  <rect x="14" y="3" width="6" height="12" />
  <rect x="4" y="17" width="16" height="4" />
</svg>
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use radiant::runtime::{PaintPrimitive, SurfaceRuntime};

    #[test]
    fn svg_toolbar_icons_rasterize_active_and_inactive_variants() {
        let icons = ToolbarIcons::new(&ThemeTokens::default());

        assert_eq!(icons.select.active.width, ICON_SIZE);
        assert_eq!(icons.select.inactive.height, ICON_SIZE);
        assert_ne!(icons.select.active.pixels, icons.select.inactive.pixels);
        assert!(
            icons
                .select
                .active
                .pixels
                .iter()
                .any(|channel| *channel != 0)
        );
    }

    #[test]
    fn toolbar_button_routes_toggle_through_runtime() {
        let bridge = radiant::app(ToolbarState::default())
            .view(project_surface)
            .update(|state, message| match message {
                ToolMessage::Toggle(tool) => state.toggle(tool),
            })
            .into_bridge();
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(360.0, 150.0));

        assert!(runtime.dispatch_input(
            11,
            WidgetInput::PointerPress {
                position: Point::new(70.0, 60.0),
                button: PointerButton::Primary,
            },
        ));
        assert!(runtime.dispatch_input(
            11,
            WidgetInput::PointerRelease {
                position: Point::new(70.0, 60.0),
                button: PointerButton::Primary,
            },
        ));

        let brush_active = runtime
            .surface()
            .find_widget(11)
            .and_then(|widget| {
                widget
                    .widget_object()
                    .as_any()
                    .downcast_ref::<IconToggleButton>()
            })
            .map(|button| button.active);
        assert_eq!(brush_active, Some(true));
    }

    #[test]
    fn toolbar_button_paints_icon_image_and_active_marker() {
        let button = IconToggleButton::new(
            ToolId::Select,
            ToolbarIcons::new(&ThemeTokens::default()).select,
            true,
        );
        let mut primitives = Vec::new();
        button.append_paint(
            &mut primitives,
            Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(42.0, 36.0)),
            &LayoutOutput::default(),
            &ThemeTokens::default(),
        );

        assert!(
            primitives
                .iter()
                .any(|primitive| matches!(primitive, PaintPrimitive::Image(_)))
        );
        assert!(primitives.len() >= 4);
    }
}
