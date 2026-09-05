//! Exact-input component reuse with ordinary pure component functions.

use radiant::application::ComponentProjectionContext;
use radiant::prelude::*;
use radiant::runtime::ResolvedEnvironment;

#[derive(Clone)]
enum Message {
    Increment,
}

fn counter(value: &u32, _: &ResolvedEnvironment) -> View<Message> {
    column([
        text(format!("Count: {value}")).id(101),
        button("Increment").message(Message::Increment).id(102),
    ])
}

fn explanation(_: &(), _: &ResolvedEnvironment) -> View<Message> {
    column([
        text("This sibling's component function is reused while its inputs remain equal."),
        text("Window and application environment changes invalidate reuse."),
    ])
}

fn view(value: &u32, context: &mut ComponentProjectionContext<'_, Message>) -> View<Message> {
    let counter = context.project("counter", *value, counter);
    let explanation = context.project("explanation", (), explanation);
    let work = context.counters();
    column([
        counter,
        explanation,
        text(format!(
            "Component callbacks: {}; cache hits: {}",
            work.callbacks, work.cache_hits
        )),
    ])
    .padding(12.0)
}

fn main() -> radiant::application::Result {
    radiant::app(0u32)
        .title("Component projection")
        .size(640, 240)
        .view_with_components(|_| Default::default(), view)
        .update(|value, Message::Increment| *value += 1)
        .run()
}

#[cfg(test)]
mod tests {
    use super::*;
    use radiant::runtime::{Event, SurfaceRuntime};
    use radiant::widgets::{PointerButton, PointerModifiers};
    use std::{cell::RefCell, rc::Rc};

    #[test]
    fn public_component_example_skips_unchanged_sibling_after_click() {
        let work = Rc::new(RefCell::new(Vec::new()));
        let observed = Rc::clone(&work);
        let mut runtime = SurfaceRuntime::new(
            radiant::app(0u32)
                .view_with_components(
                    |_| Default::default(),
                    move |value, context| {
                        let result = view(value, context);
                        observed.borrow_mut().push(context.counters());
                        result
                    },
                )
                .update(|value, Message::Increment| *value += 1)
                .into_bridge(),
            radiant::layout::Vector2::new(640.0, 240.0),
        );
        let bounds = runtime.layout().rects[&102];
        let position = radiant::gui::types::Point::new(bounds.min.x + 5.0, bounds.min.y + 5.0);
        runtime.dispatch_event(Event::PointerPress {
            position,
            button: PointerButton::Primary,
            modifiers: PointerModifiers::default(),
            timestamp: None,
        });
        runtime.dispatch_event(Event::PointerRelease {
            position,
            button: PointerButton::Primary,
            modifiers: PointerModifiers::default(),
            timestamp: None,
        });
        let work = work.borrow();
        assert!(
            work.iter()
                .any(|work| work.callbacks == 2 && work.cache_hits == 0)
        );
        assert!(
            work.iter()
                .any(|work| work.callbacks == 1 && work.cache_hits == 1)
        );
    }
}
