//! Dense plugin-style control panel built from generic Radiant widgets.

use radiant::prelude as ui;

#[derive(Clone, Debug)]
struct PanelState {
    enabled: bool,
    linked: bool,
    drive: i32,
    width: i32,
    mix: i32,
}

impl Default for PanelState {
    fn default() -> Self {
        Self {
            enabled: true,
            linked: false,
            drive: 42,
            width: 65,
            mix: 80,
        }
    }
}

fn main() -> radiant::Result {
    radiant::app(PanelState::default())
        .title("Radiant Plugin Panel")
        .size(620, 360)
        .min_size(440, 260)
        .view(project_surface)
        .run()
}

fn project_surface(state: &mut PanelState) -> ui::StateView<PanelState> {
    ui::column([
        ui::row([
            ui::text("Plugin Panel").height(30.0).fill_width(),
            ui::toggle("Enabled", state.enabled)
                .on_change(|state: &mut PanelState, enabled| state.enabled = enabled)
                .size(118.0, 30.0),
            ui::toggle("Link", state.linked)
                .on_change(|state: &mut PanelState, linked| state.linked = linked)
                .size(86.0, 30.0),
        ])
        .fill_width()
        .spacing(10.0),
        ui::grid_with_gaps(
            [
                parameter_tile("Drive", state.drive, dec_drive, inc_drive),
                parameter_tile("Width", state.width, dec_width, inc_width),
                parameter_tile("Mix", state.mix, dec_mix, inc_mix),
            ],
            3,
            10.0,
            10.0,
        )
        .fill_width()
        .style(ui::WidgetStyle::default())
        .padding(10.0),
        ui::row([
            ui::button("A")
                .primary()
                .on_click(|state: &mut PanelState| {
                    state.drive = 42;
                    state.width = 65;
                    state.mix = 80;
                }),
            ui::button("B").subtle().on_click(|state: &mut PanelState| {
                state.drive = 28;
                state.width = 90;
                state.mix = 55;
            }),
            ui::text(status_text(state)).fill_width().height(32.0),
        ])
        .fill_width()
        .spacing(10.0),
    ])
    .style(ui::WidgetStyle::default())
    .padding(16.0)
    .spacing(12.0)
}

fn parameter_tile(
    label: &'static str,
    value: i32,
    dec: fn(&mut PanelState),
    inc: fn(&mut PanelState),
) -> ui::StateView<PanelState> {
    ui::column([
        ui::text(label).height(22.0).fill_width(),
        ui::text(format!("{value:03}"))
            .height(32.0)
            .fill_width()
            .baseline(22.0),
        ui::row([
            ui::button("-").subtle().on_click(dec).size(42.0, 30.0),
            ui::button("+").primary().on_click(inc).size(42.0, 30.0),
        ])
        .spacing(8.0),
    ])
    .style(ui::WidgetStyle {
        tone: ui::WidgetTone::Neutral,
        prominence: ui::WidgetProminence::Subtle,
    })
    .padding(10.0)
    .spacing(6.0)
    .fill_width()
    .height(124.0)
}

fn status_text(state: &PanelState) -> String {
    let link = if state.linked {
        "linked"
    } else {
        "independent"
    };
    let power = if state.enabled { "active" } else { "bypassed" };
    format!("{power} / {link} / mix {}%", state.mix)
}

fn dec_drive(state: &mut PanelState) {
    state.drive = (state.drive - 1).clamp(0, 100);
}

fn inc_drive(state: &mut PanelState) {
    state.drive = (state.drive + 1).clamp(0, 100);
}

fn dec_width(state: &mut PanelState) {
    state.width = (state.width - 1).clamp(0, 100);
}

fn inc_width(state: &mut PanelState) {
    state.width = (state.width + 1).clamp(0, 100);
}

fn dec_mix(state: &mut PanelState) {
    state.mix = (state.mix - 1).clamp(0, 100);
}

fn inc_mix(state: &mut PanelState) {
    state.mix = (state.mix + 1).clamp(0, 100);
}
