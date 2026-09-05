//! Observe admitted pointer phases through the ordinary application update path.

use radiant::application::render_canvas_pointer;
use radiant::gui::pointer_ingress::PointerEvent;
use radiant::prelude::*;
use radiant::runtime::RenderCanvasContent;
use std::sync::Arc;

struct State {
    samples: Arc<[f32]>,
    last: Option<PointerEvent>,
    deliveries: usize,
}

impl Default for State {
    fn default() -> Self {
        Self {
            samples: (0..256).map(|index| (index as f32 * 0.1).sin()).collect(),
            last: None,
            deliveries: 0,
        }
    }
}

fn view(state: &State) -> View<PointerEvent> {
    column([
        text("Press and drag across the signal"),
        render_canvas_pointer(
            7,
            0,
            RenderCanvasContent::SignalBands {
                frames: 256,
                band_count: 1,
                frame_range: [0.0, 256.0],
                samples: Arc::clone(&state.samples),
            },
            |event| event,
        )
        .id(7)
        .size(480.0, 180.0),
        text(state.last.map_or_else(
            || "No pointer delivery yet".to_owned(),
            |event| {
                format!(
                    "{:?}: {:?} at {:?}; {} deliveries",
                    event.kind(),
                    event.phase(),
                    event.logical_position(),
                    state.deliveries
                )
            },
        )),
        text(
            "Touch, pen and gesture recognition are explicitly unsupported in this ingress slice.",
        )
        .wrap(),
    ])
    .padding(16.0)
    .spacing(8.0)
}

fn update(state: &mut State, event: PointerEvent) {
    state.last = Some(event);
    state.deliveries += 1;
}

fn main() -> radiant::Result {
    radiant::app(State::default())
        .title("Typed pointer input")
        .size(520, 340)
        .view(view)
        .update(update)
        .run()
}

#[cfg(test)]
mod tests {
    use super::*;
    use radiant::gui::{pointer_ingress::*, types::Point};
    use radiant::runtime::SurfaceRuntime;
    use radiant::widgets::{PointerButton, PointerModifiers};

    #[test]
    fn public_pointer_builder_delivers_one_sequence_and_rejects_replayed_terminal() {
        let mut runtime = SurfaceRuntime::new(
            radiant::app(State::default())
                .view(view)
                .update(update)
                .into_bridge(),
            Vector2::new(520.0, 340.0),
        );
        let bounds = runtime.layout().rects[&7];
        let position = Point::new(bounds.min.x + 20.0, bounds.min.y + 20.0);
        let device = InputDeviceId::new(1).unwrap();
        let contact = PointerContactId::new(1).unwrap();
        let start = PointerIngress::new(
            DeviceKind::Mouse,
            device,
            contact,
            PointerPhase::Started {
                button: PointerButton::Primary,
            },
            position,
            PointerButtons::PRIMARY,
            PointerModifiers::default(),
            None,
            None,
            None,
            None,
        )
        .unwrap();
        let admission = runtime.dispatch_pointer_ingress_with_admission(start);
        assert_eq!(
            admission.disposition(),
            PointerIngressDisposition::RoutedWidget(7)
        );
        let token = admission.sequence_token().unwrap();
        let terminal = PointerIngress::from_runtime(
            DeviceKind::Mouse,
            device,
            contact,
            PointerPhase::Ended {
                button: PointerButton::Primary,
            },
            position,
            PointerButtons::empty(),
            PointerModifiers::default(),
            None,
            None,
            None,
            None,
            token,
        )
        .unwrap();
        assert_eq!(
            runtime.dispatch_pointer_ingress(terminal),
            PointerIngressDisposition::RoutedWidget(7)
        );
        assert_eq!(
            runtime.dispatch_pointer_ingress(terminal),
            PointerIngressDisposition::Stale
        );
        assert!(runtime.paint_plan(&Default::default()).primitives.iter().any(|primitive| {
            matches!(primitive, radiant::runtime::PaintPrimitive::Text(run) if run.text.as_str().contains("2 deliveries"))
        }));
    }
}
