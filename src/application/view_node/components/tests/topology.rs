//! Runtime continuity when component topology deliberately selects full refresh.

use super::{changing_button_component, input_component};
use crate::{
    application::{View, app, column, text},
    layout::Vector2,
    runtime::{Event, RepaintScope, ResolvedEnvironment, RuntimeBridge, SurfaceRuntime},
    widgets::{CompositionRange, CompositionSample, TextInputWidget},
};
use std::{cell::RefCell, rc::Rc};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Item {
    Button,
    Editor,
    Label(u64),
}

fn component(item: &Item, environment: &ResolvedEnvironment) -> View<()> {
    let view = match item {
        Item::Button => changing_button_component(&false, environment),
        Item::Editor => input_component(&(), environment),
        Item::Label(id) => column([text(format!("Label {id}")).id(100 + id)]),
    };
    view.width(180.0).height(32.0)
}

fn bridge(state: Rc<RefCell<Vec<Item>>>, fresh: bool) -> impl RuntimeBridge<()> {
    app(state)
        .view_with_components(
            |_| Default::default(),
            move |state, context| {
                column(
                    state
                        .borrow()
                        .iter()
                        .map(|item| {
                            let key = format!("{item:?}");
                            if fresh {
                                context.project(key, *item, move |input, environment| {
                                    std::hint::black_box(fresh);
                                    component(input, environment)
                                })
                            } else {
                                context.project(key, *item, component)
                            }
                        })
                        .collect::<Vec<_>>(),
                )
                .id(1)
            },
        )
        .into_bridge()
}

fn assert_parity<Bridge: RuntimeBridge<()>>(
    cached: &SurfaceRuntime<Bridge, ()>,
    fresh: &SurfaceRuntime<Bridge, ()>,
) {
    assert_eq!(cached.layout(), fresh.layout());
    assert_eq!(cached.focused_widget(), fresh.focused_widget());
    assert_eq!(cached.pointer_capture(), fresh.pointer_capture());
    assert_eq!(
        cached.paint_plan(&Default::default()),
        fresh.paint_plan(&Default::default())
    );
    for id in [8, 9, 101, 102] {
        match (
            cached.surface().find_widget(id),
            fresh.surface().find_widget(id),
        ) {
            (Some(actual), Some(expected)) => {
                assert_eq!(
                    actual.widget().common().state,
                    expected.widget().common().state
                );
                assert_eq!(
                    actual.widget().automation_semantics(),
                    expected.widget().automation_semantics()
                );
            }
            (None, None) => {}
            _ => panic!("different membership for {id}"),
        }
    }
}

fn topology_edits() -> [Vec<Item>; 4] {
    use Item::{Button, Editor, Label};
    [
        vec![Label(1), Button, Editor],
        vec![Label(2), Label(1), Button, Editor],
        vec![Label(2), Button, Editor],
        vec![Editor, Button, Label(2)],
    ]
}

#[test]
fn component_reorder_insert_remove_preserves_capture_then_retires_before_reuse() {
    use Item::{Button, Editor, Label};
    let state = Rc::new(RefCell::new(vec![Button, Editor, Label(1)]));
    let mut cached = SurfaceRuntime::new(bridge(state.clone(), false), Vector2::new(200.0, 300.0));
    let mut fresh = SurfaceRuntime::new(bridge(state.clone(), true), Vector2::new(200.0, 300.0));
    cached.refresh();
    fresh.refresh();
    let press = Event::primary_press(cached.layout().rects[&8].center());
    cached.dispatch_event(press);
    fresh.dispatch_event(press);
    assert_eq!(cached.pointer_capture(), Some(8));
    for items in topology_edits() {
        *state.borrow_mut() = items;
        let before = cached.refresh_counters();
        cached.refresh_with_scope(RepaintScope::Projection);
        fresh.refresh_with_scope(RepaintScope::Projection);
        assert_eq!(
            cached.refresh_counters().runtime_projection,
            before.runtime_projection + 1
        );
        assert_eq!(
            cached.refresh_counters().reconciliation_applied,
            before.reconciliation_applied
        );
        assert_eq!(cached.pointer_capture(), Some(8));
        assert!(
            cached
                .surface()
                .find_widget(8)
                .unwrap()
                .widget()
                .common()
                .state
                .pressed
        );
        assert_parity(&cached, &fresh);
    }
    *state.borrow_mut() = vec![Editor, Label(2)];
    cached.refresh();
    fresh.refresh();
    assert_eq!(cached.pointer_capture(), None);
    assert_parity(&cached, &fresh);
    *state.borrow_mut() = vec![Button, Editor, Label(2)];
    cached.refresh();
    fresh.refresh();
    assert!(
        !cached
            .surface()
            .find_widget(8)
            .unwrap()
            .widget()
            .common()
            .state
            .pressed
    );
    let release = Event::primary_release(cached.layout().rects[&8].center());
    cached.dispatch_event(release);
    fresh.dispatch_event(release);
    assert_eq!(cached.pointer_capture(), None);
    assert_parity(&cached, &fresh);
}

#[test]
fn component_topology_preserves_ime_and_removed_editor_rejects_stale_commit() {
    use Item::{Button, Editor, Label};
    let state = Rc::new(RefCell::new(vec![Button, Editor, Label(1)]));
    let mut cached = SurfaceRuntime::new(bridge(state.clone(), false), Vector2::new(200.0, 300.0));
    let mut fresh = SurfaceRuntime::new(bridge(state.clone(), true), Vector2::new(200.0, 300.0));
    let range = CompositionRange::new(0, 0, 4).unwrap();
    let selected = CompositionRange::new(1, 1, 1).unwrap();
    for runtime in [&mut cached, &mut fresh] {
        runtime.refresh();
        assert!(runtime.focus_widget(9));
        assert_eq!(
            runtime.dispatch_composition_sample(CompositionSample::start(range, range).unwrap()),
            Some(9)
        );
        assert_eq!(
            runtime.dispatch_composition_sample(CompositionSample::update("あ", selected).unwrap()),
            Some(9)
        );
    }
    for items in topology_edits() {
        *state.borrow_mut() = items;
        cached.refresh();
        fresh.refresh();
        assert!(
            cached
                .surface()
                .find_widget(9)
                .unwrap()
                .widget()
                .retains_managed_composition()
        );
        assert_eq!(cached.focused_widget(), Some(9));
        assert_parity(&cached, &fresh);
    }
    *state.borrow_mut() = vec![Button, Label(2)];
    cached.refresh();
    fresh.refresh();
    assert_parity(&cached, &fresh);
    *state.borrow_mut() = vec![Button, Editor, Label(2)];
    for runtime in [&mut cached, &mut fresh] {
        runtime.refresh();
        assert!(runtime.focus_widget(9));
        assert_eq!(
            runtime.dispatch_composition_sample(CompositionSample::commit("stale")),
            None
        );
        let widget = runtime.surface().find_widget(9).unwrap().widget();
        assert!(!widget.retains_managed_composition());
        assert_eq!(
            widget
                .as_any()
                .downcast_ref::<TextInputWidget>()
                .unwrap()
                .state
                .value,
            "seed"
        );
    }
    assert_parity(&cached, &fresh);
}
