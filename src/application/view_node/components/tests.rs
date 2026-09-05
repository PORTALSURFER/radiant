use super::*;
use crate::application::{column, text};
use std::{cell::Cell, rc::Rc};

fn environment() -> ResolvedEnvironment {
    ResolvedEnvironment::from_window_environment(Default::default())
}

#[derive(PartialEq)]
struct LabelInput(String);

fn label(input: &LabelInput, _: &ResolvedEnvironment) -> ViewNode<()> {
    text(input.0.clone())
}

#[test]
fn exact_inputs_skip_callbacks_and_removed_keys_retire() {
    let mut cache = ComponentProjectionCache::<()>::default();
    for expected in [0, 1] {
        let mut context = cache.begin(environment());
        let _ = context.project("a", LabelInput("unchanged".to_owned()), label);
        assert_eq!(context.counters().cache_hits, expected);
        assert_eq!(context.counters().callbacks, 1 - expected);
        context.finish();
    }
    cache.begin(environment()).finish();
    let mut context = cache.begin(environment());
    let _ = context.project("a", LabelInput("unchanged".to_owned()), label);
    assert_eq!(context.counters().callbacks, 1);
}

#[test]
fn changed_input_and_capturing_projectors_are_fresh() {
    let mut cache = ComponentProjectionCache::<()>::default();
    let calls = Rc::new(Cell::new(0));
    for input in ["first", "second", "second"] {
        let mut context = cache.begin(environment());
        let called = Rc::clone(&calls);
        let _ = context.project("a", input.to_owned(), move |input, _| {
            called.set(called.get() + 1);
            text(input.clone())
        });
        assert_eq!(context.counters().callbacks, 1);
        assert_eq!(context.counters().cache_hits, 0);
        context.finish();
    }
    assert_eq!(calls.get(), 3);
}

#[test]
fn projector_panic_clears_previous_cache() {
    let mut cache = ComponentProjectionCache::<()>::default();
    let mut context = cache.begin(environment());
    let _ = context.project("a", LabelInput("value".to_owned()), label);
    context.finish();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut context = cache.begin(environment());
        let _ = context.project("b", (), |_, _| panic!("provider failed"));
    }));
    assert!(result.is_err());
    assert!(cache.entries.is_empty());
}

#[test]
fn node_budget_does_not_retain_oversized_components() {
    let mut cache = ComponentProjectionCache::<()>::default();
    let mut context = cache.begin(environment());
    let _ = context.project("large", (), |_, _| {
        column(
            (0..MAX_RETAINED_NODES)
                .map(|_| text("leaf"))
                .collect::<Vec<_>>(),
        )
    });
    assert_eq!(context.counters().retained_nodes, 0);
}

#[test]
fn application_environment_change_invalidates_unchanged_inputs() {
    let mut cache = ComponentProjectionCache::<()>::default();
    let mut context = cache.begin(environment());
    let _ = context.project("a", LabelInput("same".to_owned()), label);
    context.finish();
    let changed = ResolvedEnvironment::from_snapshots(
        Default::default(),
        std::sync::Arc::new(crate::application::ApplicationEnvironment::new(
            crate::application::LocaleId::new("fr").unwrap(),
        )),
    );
    let mut context = cache.begin(changed);
    let _ = context.project("a", LabelInput("same".to_owned()), label);
    assert_eq!(context.counters().callbacks, 1);
}

#[test]
fn capacity_overflow_projects_without_retaining_and_duplicate_keys_fail_closed() {
    let mut cache = ComponentProjectionCache::<()>::default();
    let mut context = cache.begin(environment());
    for index in 0..MAX_COMPONENTS + 1 {
        let _ = context.project(format!("key-{index}"), LabelInput("same".to_owned()), label);
    }
    assert_eq!(context.cache.entries.len(), MAX_COMPONENTS);
    assert_eq!(context.counters().callbacks, MAX_COMPONENTS + 1);
    context.finish();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut context = cache.begin(environment());
        let _ = context.project("key-0", LabelInput("same".to_owned()), label);
        let _ = context.project("key-0", LabelInput("same".to_owned()), label);
    }));
    assert!(result.is_err());
    assert!(cache.entries.is_empty());
}

fn changed_component(input: &LabelInput, _: &ResolvedEnvironment) -> ViewNode<()> {
    crate::application::row([text(input.0.clone()).id(7)])
}

fn stable_component(_: &(), _: &ResolvedEnvironment) -> ViewNode<()> {
    use crate::{
        layout::Vector2,
        widgets::{ButtonWidget, WidgetSizing},
    };
    crate::application::row(
        [crate::application::view_node_from_widget(ButtonWidget::new(
            8,
            "Stable",
            WidgetSizing::fixed(Vector2::new(100.0, 28.0)),
        ))
        .id(8)],
    )
}

#[test]
fn production_component_reuse_matches_fresh_projection_during_capture() {
    use crate::{
        application::app,
        layout::Vector2,
        runtime::{Event, RepaintScope, SurfaceRuntime},
        widgets::{PointerButton, PointerModifiers},
    };
    let state = Rc::new(Cell::new(0u32));
    let counts = Rc::new(std::cell::RefCell::new(Vec::new()));
    let observed = Rc::clone(&counts);
    let mut cached = SurfaceRuntime::new(
        app(Rc::clone(&state))
            .view_with_components(
                |_| Default::default(),
                move |state, context| {
                    let changed = context.project(
                        "changed",
                        LabelInput(state.get().to_string()),
                        changed_component,
                    );
                    let stable = context.project("stable", (), stable_component);
                    observed.borrow_mut().push(context.counters());
                    column([changed, stable]).id(1)
                },
            )
            .into_bridge(),
        Vector2::new(240.0, 100.0),
    );
    let mut fresh = SurfaceRuntime::new(
        app(Rc::clone(&state))
            .view_with_components(
                |_| Default::default(),
                |state, context| {
                    let force_fresh = true;
                    let changed = context.project(
                        "changed",
                        LabelInput(state.get().to_string()),
                        move |input, environment| {
                            std::hint::black_box(force_fresh);
                            changed_component(input, environment)
                        },
                    );
                    let stable = context.project("stable", (), move |input, environment| {
                        std::hint::black_box(force_fresh);
                        stable_component(input, environment)
                    });
                    column([changed, stable]).id(1)
                },
            )
            .into_bridge(),
        Vector2::new(240.0, 100.0),
    );
    let bounds = cached.layout().rects[&8];
    let press = Event::PointerPress {
        position: crate::gui::types::Point::new(bounds.min.x + 5.0, bounds.min.y + 5.0),
        button: PointerButton::Primary,
        modifiers: PointerModifiers::default(),
        timestamp: None,
    };
    cached.dispatch_event(press);
    fresh.dispatch_event(press);
    assert_eq!(cached.pointer_capture(), Some(8));
    for value in 1..4 {
        state.set(value);
        cached.refresh_with_scope(RepaintScope::Projection);
        fresh.refresh_with_scope(RepaintScope::Projection);
        let counters = *counts.borrow().last().unwrap();
        assert_eq!(counters.callbacks, 1);
        assert_eq!(counters.cache_hits, 1);
        assert_eq!(
            cached
                .surface()
                .find_widget(7)
                .unwrap()
                .widget()
                .as_any()
                .downcast_ref::<crate::widgets::TextWidget>()
                .unwrap()
                .text
                .as_str(),
            value.to_string()
        );
        assert_eq!(cached.pointer_capture(), Some(8));
        assert_eq!(cached.pointer_capture(), fresh.pointer_capture());
        assert_eq!(cached.focused_widget(), fresh.focused_widget());
        assert_eq!(cached.layout(), fresh.layout());
        assert_eq!(
            cached.paint_plan(&Default::default()),
            fresh.paint_plan(&Default::default())
        );
        for id in [7, 8] {
            assert_eq!(
                cached
                    .surface()
                    .find_widget(id)
                    .unwrap()
                    .widget()
                    .automation_semantics(),
                fresh
                    .surface()
                    .find_widget(id)
                    .unwrap()
                    .widget()
                    .automation_semantics()
            );
        }
    }
}

#[test]
fn later_environment_source_override_reaches_components_and_runtime() {
    use crate::{
        application::{ApplicationEnvironment, LocaleId, app},
        layout::Vector2,
        runtime::SurfaceRuntime,
    };
    let seen = Rc::new(std::cell::RefCell::new(Vec::new()));
    let observed = Rc::clone(&seen);
    let runtime = SurfaceRuntime::new(
        app(())
            .view_with_components(
                |_| ApplicationEnvironment::new(LocaleId::new("en").unwrap()),
                move |_, context| {
                    observed
                        .borrow_mut()
                        .push(context.environment().locale().cloned());
                    context.project("a", (), |_, environment| {
                        text(environment.locale().unwrap().as_str().to_owned())
                    })
                },
            )
            .application_environment(|_| ApplicationEnvironment::new(LocaleId::new("fr").unwrap()))
            .into_bridge(),
        Vector2::new(100.0, 100.0),
    );
    assert_eq!(seen.borrow()[0].as_ref().unwrap().as_str(), "fr");
    assert_eq!(
        runtime.context().application_environment().fallback_chain()[0].as_str(),
        "fr"
    );
}

fn input_component(_: &(), _: &ResolvedEnvironment) -> ViewNode<()> {
    use crate::{
        layout::Vector2,
        widgets::{TextInputWidget, WidgetSizing},
    };
    crate::application::row([
        crate::application::view_node_from_widget(TextInputWidget::new(
            9,
            "seed",
            WidgetSizing::fixed(Vector2::new(160.0, 28.0)),
        ))
        .id(9),
    ])
}

fn input_bridge(state: Rc<Cell<u32>>, fresh: bool) -> impl crate::runtime::RuntimeBridge<()> {
    crate::application::app((state, fresh))
        .view_with_components(
            |_| Default::default(),
            |(state, fresh), context| {
                let changed = context.project(
                    "changed",
                    LabelInput(state.get().to_string()),
                    changed_component,
                );
                let editor = if *fresh {
                    let force = *fresh;
                    context.project("editor", (), move |input, environment| {
                        std::hint::black_box(force);
                        input_component(input, environment)
                    })
                } else {
                    context.project("editor", (), input_component)
                };
                column([changed, editor]).id(1)
            },
        )
        .into_bridge()
}

#[test]
fn unchanged_component_preserves_ime_during_unrelated_projection() {
    use crate::{
        layout::Vector2,
        runtime::{RepaintScope, SurfaceRuntime},
        widgets::{CompositionRange, CompositionSample},
    };
    let state = Rc::new(Cell::new(0));
    let mut cached = SurfaceRuntime::new(
        input_bridge(Rc::clone(&state), false),
        Vector2::new(240.0, 100.0),
    );
    let mut fresh = SurfaceRuntime::new(
        input_bridge(Rc::clone(&state), true),
        Vector2::new(240.0, 100.0),
    );
    let range = CompositionRange::new(0, 0, 4).unwrap();
    let preedit_range = CompositionRange::new(1, 1, 1).unwrap();
    for runtime in [&mut cached, &mut fresh] {
        assert!(runtime.focus_widget(9));
        assert_eq!(
            runtime.dispatch_composition_sample(CompositionSample::start(range, range).unwrap()),
            Some(9)
        );
        assert_eq!(
            runtime.dispatch_composition_sample(
                CompositionSample::update("あ", preedit_range).unwrap()
            ),
            Some(9)
        );
    }
    state.set(1);
    for runtime in [&mut cached, &mut fresh] {
        runtime.refresh_with_scope(RepaintScope::Projection);
        assert!(
            runtime
                .surface()
                .find_widget(9)
                .unwrap()
                .widget()
                .retains_managed_composition()
        );
        assert_eq!(runtime.focused_widget(), Some(9));
    }
    assert_eq!(
        cached.paint_plan(&Default::default()),
        fresh.paint_plan(&Default::default())
    );
    for runtime in [&mut cached, &mut fresh] {
        assert_eq!(
            runtime.dispatch_composition_sample(CompositionSample::commit("い")),
            Some(9)
        );
    }
    assert_eq!(
        cached
            .surface()
            .find_widget(9)
            .unwrap()
            .widget()
            .automation_semantics(),
        fresh
            .surface()
            .find_widget(9)
            .unwrap()
            .widget()
            .automation_semantics()
    );
}

#[test]
fn native_window_snapshot_change_invalidates_component_cache() {
    let mut cache = ComponentProjectionCache::<()>::default();
    let mut context = cache.begin(environment());
    let _ = context.project("a", LabelInput("same".to_owned()), label);
    context.finish();
    let window = crate::runtime::WindowEnvironment::new(
        crate::theme::DpiScale::new(2.0),
        Some(crate::runtime::WindowColorScheme::Dark),
        false,
        true,
    );
    let mut context = cache.begin(ResolvedEnvironment::from_window_environment(window));
    let _ = context.project("a", LabelInput("same".to_owned()), label);
    assert_eq!(context.counters().callbacks, 1);
    assert_eq!(context.counters().cache_hits, 0);
}

#[derive(Clone)]
struct ObservedInput {
    revision: u32,
    callbacks: Rc<Cell<usize>>,
    lowered_widgets: Rc<Cell<usize>>,
}

impl PartialEq for ObservedInput {
    fn eq(&self, other: &Self) -> bool {
        self.revision == other.revision
            && Rc::ptr_eq(&self.callbacks, &other.callbacks)
            && Rc::ptr_eq(&self.lowered_widgets, &other.lowered_widgets)
    }
}

struct ObservedWidgetView(Rc<Cell<usize>>);

impl crate::application::WidgetView<()> for ObservedWidgetView {
    fn default_sizing(&self) -> crate::widgets::WidgetSizing {
        crate::widgets::WidgetSizing::fixed(crate::layout::Vector2::new(100.0, 20.0))
    }

    fn into_surface_node(
        self: Box<Self>,
        context: crate::application::WidgetViewContext,
    ) -> SurfaceNode<()> {
        self.0.set(self.0.get() + 1);
        let mut widget =
            crate::widgets::TextWidget::new(context.id, "Observed", self.default_sizing());
        context.apply_to(&mut widget);
        SurfaceNode::static_widget(widget)
    }
}

fn observed_component(input: &ObservedInput, _: &ResolvedEnvironment) -> ViewNode<()> {
    input.callbacks.set(input.callbacks.get() + 1);
    column(
        (0..32)
            .map(|_| {
                crate::application::view_node_from_widget(ObservedWidgetView(Rc::clone(
                    &input.lowered_widgets,
                )))
            })
            .collect::<Vec<_>>(),
    )
}

#[test]
fn actual_component_and_widget_lowering_callbacks_skip_unchanged_sibling() {
    let mut cache = ComponentProjectionCache::<()>::default();
    let mut changing = ObservedInput {
        revision: 0,
        callbacks: Rc::new(Cell::new(0)),
        lowered_widgets: Rc::new(Cell::new(0)),
    };
    let stable = ObservedInput {
        revision: 0,
        callbacks: Rc::new(Cell::new(0)),
        lowered_widgets: Rc::new(Cell::new(0)),
    };
    for revision in 0..3 {
        changing.revision = revision;
        let mut context = cache.begin(environment());
        let _ = context.project("changing", changing.clone(), observed_component);
        let _ = context.project("stable", stable.clone(), observed_component);
        context.finish();
    }
    assert_eq!(changing.callbacks.get(), 3);
    assert_eq!(changing.lowered_widgets.get(), 96);
    assert_eq!(stable.callbacks.get(), 1);
    assert_eq!(stable.lowered_widgets.get(), 32);
}

struct StatefulCloneWidget {
    common: crate::widgets::WidgetCommon,
    generation: u32,
}

impl Clone for StatefulCloneWidget {
    fn clone(&self) -> Self {
        Self {
            common: self.common.clone(),
            generation: self.generation + 1,
        }
    }
}

impl crate::widgets::Widget for StatefulCloneWidget {
    fn common(&self) -> &crate::widgets::WidgetCommon {
        &self.common
    }
    fn common_mut(&mut self) -> &mut crate::widgets::WidgetCommon {
        &mut self.common
    }
    fn handle_input(
        &mut self,
        _: crate::gui::types::Rect,
        _: crate::widgets::WidgetInput,
    ) -> Option<crate::widgets::WidgetOutput> {
        None
    }
    fn append_paint(
        &self,
        _: &mut Vec<crate::runtime::PaintPrimitive>,
        _: crate::gui::types::Rect,
        _: &crate::layout::LayoutOutput,
        _: &crate::theme::ThemeTokens,
    ) {
    }
}

fn stateful_clone_component(_: &(), _: &ResolvedEnvironment) -> ViewNode<()> {
    crate::application::view_node_from_widget(StatefulCloneWidget {
        common: crate::widgets::WidgetCommon::fixed(8, 20.0, 20.0),
        generation: 0,
    })
}

#[test]
fn custom_clone_behavior_falls_back_without_reconstructing_from_cached_widget() {
    let mut cache = ComponentProjectionCache::<()>::default();
    for _ in 0..3 {
        let mut context = cache.begin(environment());
        let view = context.project("custom", (), stateful_clone_component);
        assert_eq!(context.counters().callbacks, 1);
        assert_eq!(context.counters().cache_hits, 0);
        assert_eq!(context.counters().retained_nodes, 0);
        let root = view.into_surface().into_root();
        let SurfaceNode::Widget(widget) = root else {
            panic!("expected custom widget");
        };
        assert_eq!(
            widget
                .widget()
                .as_any()
                .downcast_ref::<StatefulCloneWidget>()
                .unwrap()
                .generation,
            0
        );
        context.finish();
    }
}
