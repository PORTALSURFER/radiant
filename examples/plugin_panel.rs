//! Advanced synthetic control-panel simulation built from generic Radiant widgets.
//!
//! This example validates compact control-panel layout, toggles, focus, and
//! message-first updates. Plugin SDK integration, preset management, and host
//! lifecycle policy remain outside Radiant.

use radiant::prelude as ui;

#[derive(Clone, Debug, PartialEq, Eq)]
enum PanelMessage {
    SetEnabled(bool),
    SetLinked(bool),
    ApplyPresetA,
    ApplyPresetB,
    AdjustDrive(i32),
    AdjustWidth(i32),
    AdjustMix(i32),
}

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
        .update(update)
        .run()
}

fn project_surface(state: &PanelState) -> ui::View<PanelMessage> {
    ui::column([
        ui::row([
            ui::text("Plugin Panel").height(30.0).fill_width(),
            ui::toggle("Enabled", state.enabled)
                .message(PanelMessage::SetEnabled)
                .size(118.0, 30.0),
            ui::toggle("Link", state.linked)
                .message(PanelMessage::SetLinked)
                .size(86.0, 30.0),
        ])
        .fill_width()
        .spacing(10.0),
        ui::grid_with_gaps(
            [
                parameter_tile(
                    "Drive",
                    state.drive,
                    PanelMessage::AdjustDrive(-1),
                    PanelMessage::AdjustDrive(1),
                ),
                parameter_tile(
                    "Width",
                    state.width,
                    PanelMessage::AdjustWidth(-1),
                    PanelMessage::AdjustWidth(1),
                ),
                parameter_tile(
                    "Mix",
                    state.mix,
                    PanelMessage::AdjustMix(-1),
                    PanelMessage::AdjustMix(1),
                ),
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
                .message(PanelMessage::ApplyPresetA),
            ui::button("B").subtle().message(PanelMessage::ApplyPresetB),
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
    dec: PanelMessage,
    inc: PanelMessage,
) -> ui::View<PanelMessage> {
    ui::column([
        ui::text(label).height(22.0).fill_width(),
        ui::text(format!("{value:03}"))
            .height(32.0)
            .fill_width()
            .baseline(22.0),
        ui::row([
            ui::button("-").subtle().message(dec).size(42.0, 30.0),
            ui::button("+").primary().message(inc).size(42.0, 30.0),
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

fn update(state: &mut PanelState, message: PanelMessage) {
    match message {
        PanelMessage::SetEnabled(enabled) => state.enabled = enabled,
        PanelMessage::SetLinked(linked) => state.linked = linked,
        PanelMessage::ApplyPresetA => {
            state.drive = 42;
            state.width = 65;
            state.mix = 80;
        }
        PanelMessage::ApplyPresetB => {
            state.drive = 28;
            state.width = 90;
            state.mix = 55;
        }
        PanelMessage::AdjustDrive(delta) => {
            state.drive = (state.drive + delta).clamp(0, 100);
        }
        PanelMessage::AdjustWidth(delta) => {
            state.width = (state.width + delta).clamp(0, 100);
        }
        PanelMessage::AdjustMix(delta) => {
            state.mix = (state.mix + delta).clamp(0, 100);
        }
    }
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
