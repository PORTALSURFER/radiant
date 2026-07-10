//! Passive widget chrome built through the same ViewNode API as applications.

use radiant::prelude as ui;
use radiant::runtime::canvas;

fn main() -> radiant::Result {
    radiant::window("Radiant Passive Widgets")
        .size(520, 180)
        .run(project_surface())
}

fn project_surface() -> ui::ViewNode<()> {
    ui::column([
        ui::text("Passive controls").fill_width().height(28.0),
        ui::row([
            ui::passive_button("Button")
                .id(10)
                .size(96.0, 32.0)
                .width(96.0)
                .height(32.0),
            ui::passive_toggle("Toggle", true)
                .id(11)
                .size(72.0, 28.0)
                .width(72.0)
                .height(28.0),
            ui::passive_text_input("", "Search")
                .id(12)
                .size(180.0, 32.0)
                .width(180.0)
                .height(32.0),
        ])
        .spacing(8.0)
        .fill_width()
        .height(36.0),
        ui::row([
            ui::text("Canvas slot").size(110.0, 22.0).width(110.0),
            canvas().id(13).size(120.0, 18.0).width(120.0),
            ui::spacer().id(14).width(20.0),
            ui::text("after spacer").size(120.0, 22.0).width(120.0),
        ])
        .spacing(8.0)
        .fill_width()
        .height(28.0),
        ui::row([
            ui::text("30% slot").id(15).width_percent(0.30).height(22.0),
            ui::text("remaining slot").fill_width().height(22.0),
        ])
        .spacing(8.0)
        .fill_width()
        .height(28.0),
    ])
    .style(ui::WidgetStyle::default())
    .padding(12.0)
    .spacing(10.0)
    .fill()
}

#[cfg(test)]
mod tests {
    use super::*;
    use radiant::prelude::IntoView;

    #[test]
    fn passive_widgets_lower_through_view_nodes() {
        let surface = project_surface().into_surface();
        let layout = radiant::layout::layout_tree(
            &surface.layout_node(),
            radiant::gui::types::Rect::from_min_size(
                radiant::gui::types::Point::new(0.0, 0.0),
                radiant::gui::types::Vector2::new(520.0, 180.0),
            ),
        );

        for id in 10..=15 {
            assert!(layout.rects.contains_key(&id));
        }
    }
}
