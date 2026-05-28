//! Focused SVG icon rendering example.

use radiant::prelude::*;

const PLAY_ICON: &str = r##"
<svg viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg">
  <polygon fill="#eeeeee" points="4,3 13,8 4,13" />
</svg>
"##;

const STOP_ICON: &str = r##"
<svg viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg">
  <rect fill="#eeeeee" x="4" y="4" width="8" height="8" />
</svg>
"##;

const LOOP_ICON: &str = r##"
<svg viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg">
  <path fill="#ffa052" d="M4 3h5.4V1.5L14 5l-4.6 3.5V7H4.2C3 7 2 8 2 9.2V10H.5v-.8C.5 5.8 2 3 4 3z"/>
  <path fill="#ffa052" d="M12 13H6.6v1.5L2 11l4.6-3.5V9H12c1.2 0 2-1 2-2.2V6h1.5v.8C15.5 10.2 14 13 12 13z"/>
</svg>
"##;

fn main() -> radiant::Result {
    radiant::window("Radiant SVG")
        .size(260, 120)
        .min_size(220, 96)
        .run(svg_example_view())
}

fn svg_example_view() -> ViewNode<()> {
    column([
        text("SVG icons").height(24.0).fill_width(),
        row([
            svg_icon_button(10, LOOP_ICON, true),
            svg_icon_button(11, PLAY_ICON, false),
            svg_icon_button(12, STOP_ICON, false),
        ])
        .spacing(8.0)
        .height(40.0),
    ])
    .style(WidgetStyle::default())
    .padding(16.0)
    .spacing(10.0)
    .fill()
}

fn svg_icon_button(id: u64, svg: &str, active: bool) -> ViewNode<()> {
    icon_button(SvgIcon::from_svg(svg).expect("example SVG icon should parse"))
        .active(active)
        .message(())
        .id(id)
        .size(40.0, 32.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use radiant::{
        gui::types::{Point, Rect, Vector2},
        prelude::IntoView,
        runtime::PaintPrimitive,
        theme::ThemeTokens,
    };

    #[test]
    fn svg_example_paints_retained_svg_icons() {
        let surface = svg_example_view().into_surface();
        let frame = surface.frame(
            Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(260.0, 120.0)),
            &ThemeTokens::default(),
        );
        let svg_count = frame
            .paint_plan
            .primitives
            .iter()
            .filter(|primitive| matches!(primitive, PaintPrimitive::Svg(_)))
            .count();

        assert_eq!(svg_count, 3);
    }
}
