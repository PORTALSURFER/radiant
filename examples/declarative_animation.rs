//! Deterministic custom-container animation without per-frame messages.
use radiant::{
    animation::{
        Animatable, AnimationImpact, AnimationPaintContext, AnimationTarget, AnimationValues,
        Transition,
    },
    application::{IntoView, column, layout},
    gui::types::{Rect, Rgba8, Vector2},
    layout::{Constraints, LayoutPolicy, MeasureChildren, PlaceChildren, SizeHint},
    runtime::{
        Command, PaintFillRect, PaintPrimitive, RuntimeBridge, UiSurface,
        testing::DeterministicHost,
    },
};
use std::{rc::Rc, sync::Arc, time::Duration};

struct EmptyPolicy;
impl LayoutPolicy for EmptyPolicy {
    fn measure(&self, _: &mut MeasureChildren<'_>, _: Constraints) -> SizeHint {
        SizeHint::preferred(Vector2::new(120.0, 40.0))
    }
    fn place(&self, _: &mut PlaceChildren<'_>, _: Rect) {}
}
struct Meter {
    target: [AnimationTarget; 1],
}
impl Animatable for Meter {
    fn targets(&self) -> &[AnimationTarget] {
        &self.target
    }
    fn append_paint(
        &self,
        values: AnimationValues<'_>,
        context: AnimationPaintContext<'_>,
        output: &mut Vec<PaintPrimitive>,
    ) {
        let width = values.get(1).unwrap_or(0.0) as f32;
        output.push(PaintPrimitive::FillRect(PaintFillRect {
            widget_id: 902,
            rect: context.bounds,
            color: Rgba8::new(0, 0, 0, 0),
        }));
        output.push(PaintPrimitive::FillRect(PaintFillRect {
            widget_id: 901,
            rect: Rect::from_min_size(context.bounds.min, Vector2::new(width, 20.0)),
            color: Rgba8::new(40, 160, 220, 255),
        }));
    }
    fn measure(
        &self,
        values: AnimationValues<'_>,
        _: &dyn LayoutPolicy,
        _: &mut MeasureChildren<'_>,
        _: Constraints,
    ) -> SizeHint {
        SizeHint::preferred(Vector2::new(
            120.0,
            values.get(1).unwrap_or(0.0) as f32 + 20.0,
        ))
    }
}
struct Model {
    target: f64,
    visible: bool,
    updates: usize,
    projections: usize,
}
enum Message {
    Target(f64),
    Remove,
    Show,
}
impl RuntimeBridge<Message> for Model {
    #[allow(clippy::arc_with_non_send_sync)]
    fn project_surface(&mut self) -> Arc<UiSurface<Message>> {
        self.projections += 1;
        let children = if self.visible {
            vec![
                layout(EmptyPolicy, Vec::new())
                    .animatable(Rc::new(Meter {
                        target: [AnimationTarget::new(
                            1,
                            0.0,
                            self.target,
                            Transition::linear(Duration::from_secs(1)),
                            AnimationImpact::Geometry,
                        )
                        .expect("finite target")],
                    }))
                    .key("meter"),
            ]
        } else {
            Vec::new()
        };
        Arc::new(column(children).into_surface())
    }
    fn update(&mut self, message: Message) -> Command<Message> {
        self.updates += 1;
        match message {
            Message::Target(value) => self.target = value,
            Message::Remove => self.visible = false,
            Message::Show => self.visible = true,
        };
        Command::none()
    }
}
fn host() -> DeterministicHost<Model, Message> {
    DeterministicHost::with_default_config(
        Model {
            target: 100.0,
            visible: true,
            updates: 0,
            projections: 0,
        },
        Vector2::new(240.0, 180.0),
    )
    .expect("headless host")
}
fn width(host: &DeterministicHost<Model, Message>) -> f32 {
    host.paint_plan()
        .primitives
        .iter()
        .find_map(|p| match p {
            PaintPrimitive::FillRect(rect) if rect.widget_id == 901 => Some(rect.rect.width()),
            _ => None,
        })
        .unwrap_or(0.0)
}
fn main() {
    let mut host = host();
    host.advance_time(Duration::from_millis(500))
        .expect("half time");
    println!(
        "{{\"phase\":\"half\",\"width\":{},\"active\":{}}}",
        width(&host),
        host.runtime().declarative_animation_status().active
    );
    host.execute_command(Command::message(Message::Target(150.0)))
        .expect("retarget");
    host.advance_time(Duration::from_secs(1)).expect("complete");
    assert_eq!(width(&host), 150.0);
    assert_eq!(host.runtime().declarative_animation_status().active, 0);
    println!(
        "{{\"phase\":\"complete\",\"width\":150,\"updates\":{}}}",
        host.bridge().updates
    );
    host.execute_command(Command::message(Message::Remove))
        .expect("remove");
    assert_eq!(host.runtime().declarative_animation_status().retained, 0);
    host.execute_command(Command::message(Message::Show))
        .expect("show new owner");
    assert_eq!(width(&host), 0.0);
    host.execute_command(Command::message(Message::Remove))
        .expect("remove again");
    println!("{{\"phase\":\"removed\",\"active\":0}}");
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn runtime_values_change_paint_without_application_projection() {
        let mut host = host();
        let projections = host.bridge().projections;
        host.advance_time(Duration::from_millis(500)).unwrap();
        assert!((width(&host) - 50.0).abs() < 0.1);
        assert_eq!(host.bridge().updates, 0);
        assert_eq!(host.bridge().projections, projections);
        host.advance_time(Duration::from_millis(501)).unwrap();
        assert_eq!(width(&host), 100.0);
        assert_eq!(host.runtime().declarative_animation_status().active, 0);
    }
    #[test]
    fn headless_lifecycle_example() {
        main();
    }

    #[test]
    fn hidden_and_reduced_motion_use_runtime_policy() {
        let mut host = host();
        host.advance_time(Duration::from_millis(250)).unwrap();
        assert_eq!(width(&host), 25.0);
        host.set_animation_hidden(true).unwrap();
        host.advance_time(Duration::from_secs(1)).unwrap();
        assert_eq!(width(&host), 25.0);
        host.set_animation_hidden(false).unwrap();
        host.advance_time(Duration::from_millis(250)).unwrap();
        assert_eq!(width(&host), 50.0);
        let old = host.runtime().window_environment();
        host.set_window_environment(radiant::runtime::WindowEnvironment::new(
            old.display_scale(),
            old.color_scheme(),
            old.contrast(),
            true,
        ))
        .unwrap();
        host.advance_time(Duration::ZERO).unwrap();
        assert_eq!(width(&host), 100.0);
        assert_eq!(host.runtime().declarative_animation_status().active, 0);
    }
    #[test]
    fn geometry_samples_relayout_current_surface() {
        fn height(host: &DeterministicHost<Model, Message>) -> f32 {
            host.paint_plan()
                .primitives
                .iter()
                .find_map(|p| match p {
                    PaintPrimitive::FillRect(r) if r.widget_id == 902 => Some(r.rect.height()),
                    _ => None,
                })
                .unwrap()
        }
        let mut host = host();
        let initial = height(&host);
        host.advance_time(Duration::from_millis(500)).unwrap();
        assert!(height(&host) > initial, "geometry sample must reach layout");
    }
}

#[cfg(test)]
mod feedback_tests {
    use super::*;
    use radiant::animation::FeedbackAnimation;
    struct Phase {
        declaration: [FeedbackAnimation; 1],
    }
    impl Animatable for Phase {
        fn targets(&self) -> &[AnimationTarget] {
            &[]
        }
        fn feedback(&self) -> &[FeedbackAnimation] {
            &self.declaration
        }
        fn append_paint(
            &self,
            values: AnimationValues<'_>,
            context: AnimationPaintContext<'_>,
            output: &mut Vec<PaintPrimitive>,
        ) {
            output.push(PaintPrimitive::FillRect(PaintFillRect {
                widget_id: 903,
                rect: Rect::from_min_size(
                    context.bounds.min,
                    Vector2::new(values.get(1).unwrap_or(0.75) as f32 * 100.0, 10.0),
                ),
                color: Rgba8::new(20, 80, 120, 255),
            }));
        }
    }
    struct PhaseModel;
    impl RuntimeBridge<()> for PhaseModel {
        #[allow(clippy::arc_with_non_send_sync)]
        fn project_surface(&mut self) -> Arc<UiSurface<()>> {
            Arc::new(
                column([])
                    .animatable(Rc::new(Phase {
                        declaration: [
                            FeedbackAnimation::new(1, 7, Duration::from_secs(1), 0.75).unwrap()
                        ],
                    }))
                    .into_surface(),
            )
        }
    }
    fn phase_width(host: &DeterministicHost<PhaseModel, ()>) -> f32 {
        host.paint_plan()
            .primitives
            .iter()
            .find_map(|p| match p {
                PaintPrimitive::FillRect(r) if r.widget_id == 903 => Some(r.rect.width()),
                _ => None,
            })
            .unwrap()
    }
    #[test]
    fn shared_phase_pauses_and_reduced_motion_can_resume() {
        let mut host =
            DeterministicHost::with_default_config(PhaseModel, Vector2::new(240.0, 180.0)).unwrap();
        assert_eq!(
            host.runtime()
                .declarative_animation_status()
                .feedback_groups,
            1
        );
        host.advance_time(Duration::from_millis(250)).unwrap();
        assert_eq!(phase_width(&host), 25.0);
        host.set_animation_hidden(true).unwrap();
        host.advance_time(Duration::from_secs(1)).unwrap();
        assert_eq!(phase_width(&host), 25.0);
        host.set_animation_hidden(false).unwrap();
        let old = host.runtime().window_environment();
        host.set_window_environment(radiant::runtime::WindowEnvironment::new(
            old.display_scale(),
            old.color_scheme(),
            old.contrast(),
            true,
        ))
        .unwrap();
        assert_eq!(phase_width(&host), 75.0);
        assert_eq!(
            host.runtime()
                .declarative_animation_status()
                .feedback_groups,
            0
        );
        host.set_window_environment(old).unwrap();
        assert_eq!(
            host.runtime()
                .declarative_animation_status()
                .feedback_groups,
            1
        );
        host.advance_time(Duration::from_millis(250)).unwrap();
        assert_eq!(phase_width(&host), 25.0);
    }
}
