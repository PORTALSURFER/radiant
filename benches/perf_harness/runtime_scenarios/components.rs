//! Application projection controls for a 32-component, 9,600-text-leaf tree.

use radiant::{
    application::{ComponentProjectionCounters, View, app, column, text},
    runtime::{ResolvedEnvironment, RuntimeBridge},
};
use std::{cell::Cell, rc::Rc};

fn component(value: &u64, _: &ResolvedEnvironment) -> View<()> {
    column(
        (0..300)
            .map(|index| text(format!("Row {index}: {value}")))
            .collect::<Vec<_>>(),
    )
}

pub(super) fn projection(fresh: bool) -> impl FnMut() -> crate::runner::ScenarioCounters {
    let value = Rc::new(Cell::new(0u64));
    let counters = Rc::new(Cell::new(ComponentProjectionCounters::default()));
    let observed = Rc::clone(&counters);
    let mut bridge = app(Rc::clone(&value))
        .view_with_components(
            |_| Default::default(),
            move |value, context| {
                let children = (0..32)
                    .map(|index| {
                        let input = if index == 0 { value.get() } else { 0 };
                        if fresh {
                            context.project(
                                format!("component-{index}"),
                                input,
                                move |value, environment| {
                                    std::hint::black_box(fresh);
                                    component(value, environment)
                                },
                            )
                        } else {
                            context.project(format!("component-{index}"), input, component)
                        }
                    })
                    .collect::<Vec<_>>();
                observed.set(context.counters());
                column(children)
            },
        )
        .into_bridge();
    // Populate every cache before the measured single-component edits.
    std::hint::black_box(bridge.project_surface());
    move || {
        value.set(value.get() + 1);
        std::hint::black_box(bridge.project_surface());
        let work = counters.get();
        crate::runner::ScenarioCounters::default()
            .with_application_projection_count(1)
            .with_component_projection_callback_count(work.callbacks as u64)
            .with_component_projection_cache_hit_count(work.cache_hits as u64)
    }
}

fn geometry_component(expanded: &bool, _: &ResolvedEnvironment) -> View<()> {
    column(
        (0..100)
            .map(|index| {
                text(format!("Row {index}"))
                    .width(100.0)
                    .height(if index == 0 && *expanded { 21.0 } else { 20.0 })
                    .size(100.0, if index == 0 && *expanded { 21.0 } else { 20.0 })
            })
            .collect::<Vec<_>>(),
    )
    .width(100.0)
    .height(2600.0)
}

pub(super) fn local_interaction() -> impl FnMut() -> crate::runner::ScenarioCounters {
    use radiant::{
        application::{row, view_node_from_widget},
        layout::Vector2,
        runtime::{RepaintScope, SurfaceRuntime},
        widgets::{ColorMarkerProps, ColorMarkerWidget, ColorMarkerWidgetParts, WidgetSizing},
    };
    let changed = Rc::new(Cell::new(false));
    let bridge = app(Rc::clone(&changed))
        .view_with_components(
            |_| Default::default(),
            |changed, context| {
                let mut children = (0..32)
                    .map(|index| {
                        context.project(format!("geometry-{index}"), false, geometry_component)
                    })
                    .collect::<Vec<_>>();
                children.push(
                    view_node_from_widget(ColorMarkerWidget::from_parts(ColorMarkerWidgetParts {
                        id: 19,
                        sizing: WidgetSizing::fixed(Vector2::new(20.0, 20.0)),
                        props: ColorMarkerProps::new(None),
                    }))
                    .id(19)
                    .tooltip(if changed.get() { "after" } else { "before" }),
                );
                row(children)
            },
        )
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(3400.0, 2700.0));
    runtime.refresh();
    assert!(runtime.layout().overflowed.is_empty());
    move || {
        let before = runtime.refresh_counters();
        changed.set(!changed.get());
        runtime.refresh_with_scope(RepaintScope::Projection);
        let after = runtime.refresh_counters();
        assert_eq!(
            after.application_projection - before.application_projection,
            1
        );
        assert_eq!(
            runtime
                .surface()
                .find_widget(19)
                .unwrap()
                .widget()
                .common()
                .tooltip
                .as_deref(),
            Some(if changed.get() { "after" } else { "before" })
        );
        crate::runner::ScenarioCounters::default()
            .with_application_projection_count(
                after.application_projection - before.application_projection,
            )
            .with_runtime_projection_count(after.runtime_projection - before.runtime_projection)
            .with_layout_count(after.layout - before.layout)
    }
}

pub(super) fn local_geometry() -> impl FnMut() -> crate::runner::ScenarioCounters {
    use radiant::{
        application::row,
        layout::Vector2,
        runtime::{RepaintScope, SurfaceRuntime},
    };
    let changed = Rc::new(Cell::new(false));
    let bridge = app(Rc::clone(&changed))
        .view_with_components(
            |_| Default::default(),
            |changed, context| {
                row((0..32)
                    .map(|index| {
                        context.project(
                            format!("geometry-{index}"),
                            index == 0 && changed.get(),
                            geometry_component,
                        )
                    })
                    .collect::<Vec<_>>())
            },
        )
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(3400.0, 2700.0));
    assert!(
        runtime.layout().overflowed.is_empty(),
        "fixture must fit the viewport"
    );
    move || {
        let before = runtime.refresh_counters();
        changed.set(!changed.get());
        runtime.refresh_with_scope(RepaintScope::Projection);
        let layouts = runtime.refresh_counters().layout - before.layout;
        assert_eq!(layouts, 1, "each measured edit must run layout");
        assert!(runtime.layout().overflowed.is_empty());
        crate::runner::ScenarioCounters::default()
            .with_layout_count(layouts)
            .with_layout_node_visit_count(runtime.layout().stats.laid_out_nodes as u64)
    }
}
