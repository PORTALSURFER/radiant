use super::SurfaceRuntime;
use crate::{
    application::{View, app, column, row, text},
    layout::{LayoutEngine, Vector2},
    runtime::{RepaintScope, ResolvedEnvironment, RuntimeBridge},
};
use std::{cell::Cell, rc::Rc};

fn component(changed: &bool, _: &ResolvedEnvironment) -> View<()> {
    column(
        (0..20)
            .map(|index| {
                text(format!("Row {index}"))
                    .width(40.0)
                    .height(if index == 0 && *changed { 11.0 } else { 10.0 })
                    .size(40.0, if index == 0 && *changed { 11.0 } else { 10.0 })
            })
            .collect::<Vec<_>>(),
    )
    .width(50.0)
    .height(350.0)
}

fn runtime(changed: Rc<Cell<bool>>, cached: bool) -> SurfaceRuntime<impl RuntimeBridge<()>, ()> {
    let bridge = app(changed)
        .view_with_components(
            |_| Default::default(),
            |changed, context| {
                row((0..3)
                    .map(|index| {
                        context.project(
                            format!("component-{index}"),
                            index == 0 && changed.get(),
                            component,
                        )
                    })
                    .collect::<Vec<_>>())
            },
        )
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(160.0, 400.0));
    runtime.layout_engine = if cached {
        LayoutEngine::with_static_geometry_fragments()
    } else {
        LayoutEngine::default()
    };
    runtime
}

#[test]
fn repeated_application_edits_preserve_all_layout_output_and_compatibility_diagnostics() {
    let changed = Rc::new(Cell::new(false));
    let mut cached = runtime(Rc::clone(&changed), true);
    let mut fresh = runtime(Rc::clone(&changed), false);
    let mut previous = None;
    for step in 0..12 {
        changed.set(step % 2 == 0);
        let cached_before = cached.refresh_counters();
        let fresh_before = fresh.refresh_counters();
        cached.refresh_with_scope(RepaintScope::Projection);
        fresh.refresh_with_scope(RepaintScope::Projection);
        assert_eq!(
            cached.refresh_counters().layout - cached_before.layout,
            1,
            "cached layout admission at step {step}"
        );
        assert_eq!(
            fresh.refresh_counters().layout - fresh_before.layout,
            1,
            "fresh layout admission at step {step}"
        );
        let mut actual = cached.layout().clone();
        let mut expected = fresh.layout().clone();
        assert_eq!(
            actual.diagnostics.len(),
            expected.diagnostics.len(),
            "step {step}"
        );
        assert!(
            actual.diagnostics == expected.diagnostics,
            "diagnostic ordering differs at step {step}"
        );
        assert!(
            !actual.diagnostics.is_empty(),
            "compatibility diagnostics remain visible"
        );
        if step > 0 {
            assert!(
                actual.stats.laid_out_nodes < expected.stats.laid_out_nodes,
                "no reuse at step {step}"
            );
            assert!(
                previous.as_ref().unwrap() != &actual.rects,
                "changed leaf geometry must publish"
            );
        }
        previous = Some(actual.rects.clone());
        assert_eq!(
            actual.stats.materialized_nodes,
            expected.stats.materialized_nodes
        );
        actual.stats = Default::default();
        expected.stats = Default::default();
        assert!(actual == expected, "layout output differs at step {step}");
    }
}
