use crate::{
    application::{IntoView, ViewNode, app, row, view_node_from_widget},
    layout::Vector2,
    runtime::{Event, RepaintScope, SurfaceRuntime},
    widgets::{ButtonWidget, PointerButton, PointerModifiers, WidgetSizing},
};
use std::{cell::Cell, rc::Rc};

fn view(secondary: &Rc<Cell<bool>>) -> ViewNode<()> {
    let mut button = ButtonWidget::new(7, "Open", WidgetSizing::fixed(Vector2::new(100.0, 28.0)));
    if secondary.get() {
        button = button.with_secondary_click();
    }
    row([
        view_node_from_widget(button).id(7),
        view_node_from_widget(ButtonWidget::new(
            8,
            "Stable",
            WidgetSizing::fixed(Vector2::new(100.0, 28.0)),
        ))
        .id(8),
    ])
    .id(1)
}

#[test]
fn application_bridge_supplies_exact_leaf_evidence_and_matches_full_projection() {
    let state = Rc::new(Cell::new(false));
    let mut exact = SurfaceRuntime::new(
        app(Rc::clone(&state)).view(view).into_bridge(),
        Vector2::new(240.0, 100.0),
    );
    let mut full = SurfaceRuntime::new(
        app(Rc::clone(&state))
            .view(|state| view(state).into_projection())
            .into_bridge(),
        Vector2::new(240.0, 100.0),
    );
    let bounds = exact.layout().rects[&8];
    let press = Event::PointerPress {
        position: crate::gui::types::Point::new(bounds.min.x + 10.0, bounds.min.y + 10.0),
        button: PointerButton::Primary,
        modifiers: PointerModifiers::default(),
        timestamp: None,
    };
    exact.dispatch_event(press);
    full.dispatch_event(press);
    assert_eq!(exact.pointer_capture(), Some(8));
    for secondary in [true, false, true] {
        let before = exact.refresh_counters();
        state.set(secondary);
        exact.refresh_with_scope(RepaintScope::Projection);
        full.refresh_with_scope(RepaintScope::Projection);
        let after = exact.refresh_counters();
        assert_eq!(
            after.application_projection,
            before.application_projection + 1
        );
        assert_eq!(after.runtime_projection, before.runtime_projection);
        assert_eq!(after.layout, before.layout);
        assert_eq!(exact.pointer_capture(), Some(8));
        assert_eq!(exact.pointer_capture(), full.pointer_capture());
        assert_eq!(exact.focused_widget(), full.focused_widget());
        assert_eq!(exact.layout(), full.layout());
        assert_eq!(
            exact.paint_plan(&Default::default()),
            full.paint_plan(&Default::default())
        );
        for id in [7, 8] {
            assert_eq!(
                exact
                    .surface()
                    .find_widget(id)
                    .unwrap()
                    .widget()
                    .automation_semantics(),
                full.surface()
                    .find_widget(id)
                    .unwrap()
                    .widget()
                    .automation_semantics(),
            );
        }
    }
}

#[test]
fn geometry_change_falls_back_then_recovers_exact_interaction_updates() {
    let width = Rc::new(Cell::new(100.0));
    let secondary = Rc::new(Cell::new(false));
    let bridge = app((Rc::clone(&width), Rc::clone(&secondary)))
        .view(|(width, secondary)| {
            let mut widget = ButtonWidget::new(
                7,
                "Open",
                WidgetSizing::fixed(Vector2::new(width.get(), 28.0)),
            );
            if secondary.get() {
                widget = widget.with_secondary_click();
            }
            row([view_node_from_widget(widget).id(7)]).id(1)
        })
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(300.0, 100.0));
    let before = runtime.refresh_counters();
    width.set(150.0);
    runtime.refresh_with_scope(RepaintScope::Projection);
    assert!(runtime.refresh_counters().runtime_projection > before.runtime_projection);
    assert_eq!(runtime.layout().rects[&7].width(), 150.0);
    let before = runtime.refresh_counters();
    secondary.set(true);
    runtime.refresh_with_scope(RepaintScope::Projection);
    assert_eq!(
        runtime.refresh_counters().runtime_projection,
        before.runtime_projection
    );
    assert_eq!(runtime.refresh_counters().layout, before.layout);
}
