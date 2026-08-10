use super::*;
use radiant::widgets::{CompositionPhase, CompositionRange, CompositionSample, WidgetId};
use std::{cell::RefCell, rc::Rc};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CompositionProjection {
    Both,
    OwnerOnly,
    SecondOnly,
    DisabledOwner,
    ReadOnlyOwner,
}

#[derive(Clone)]
struct PublicCompositionWidget {
    common: WidgetCommon,
    samples: Rc<RefCell<Vec<(WidgetId, CompositionPhase)>>>,
}

impl PublicCompositionWidget {
    fn new(
        widget_id: WidgetId,
        samples: Rc<RefCell<Vec<(WidgetId, CompositionPhase)>>>,
        disabled: bool,
        read_only: bool,
    ) -> Self {
        let mut common =
            WidgetCommon::new(widget_id, WidgetSizing::fixed(Vector2::new(120.0, 32.0)))
                .with_keyboard_focus();
        common.state.disabled = disabled;
        common.state.read_only = read_only;
        Self { common, samples }
    }
}

impl Widget for PublicCompositionWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, _bounds: Rect, _input: WidgetInput) -> Option<WidgetOutput> {
        None
    }

    fn accepts_composition_input(&self) -> bool {
        true
    }

    fn handle_composition_sample(&mut self, sample: CompositionSample) -> Option<WidgetOutput> {
        self.samples
            .borrow_mut()
            .push((self.common.id, sample.phase()));
        None
    }

    fn retains_managed_composition(&self) -> bool {
        true
    }

    fn append_paint(
        &self,
        _primitives: &mut Vec<PaintPrimitive>,
        _bounds: Rect,
        _layout: &radiant::layout::LayoutOutput,
        _theme: &ThemeTokens,
    ) {
    }
}

#[derive(Clone)]
struct PublicCompositionBridge {
    projection: CompositionProjection,
    samples: Rc<RefCell<Vec<(WidgetId, CompositionPhase)>>>,
}

impl Default for PublicCompositionBridge {
    fn default() -> Self {
        Self {
            projection: CompositionProjection::Both,
            samples: Rc::new(RefCell::new(Vec::new())),
        }
    }
}

impl RuntimeBridge<()> for PublicCompositionBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<()>> {
        let first = match self.projection {
            CompositionProjection::DisabledOwner => {
                PublicCompositionWidget::new(501, Rc::clone(&self.samples), true, false)
            }
            CompositionProjection::ReadOnlyOwner => {
                PublicCompositionWidget::new(501, Rc::clone(&self.samples), false, true)
            }
            CompositionProjection::Both | CompositionProjection::OwnerOnly => {
                PublicCompositionWidget::new(501, Rc::clone(&self.samples), false, false)
            }
            CompositionProjection::SecondOnly => {
                PublicCompositionWidget::new(502, Rc::clone(&self.samples), false, false)
            }
        };
        let children = match self.projection {
            CompositionProjection::OwnerOnly
            | CompositionProjection::SecondOnly
            | CompositionProjection::DisabledOwner
            | CompositionProjection::ReadOnlyOwner => vec![SurfaceChild::fill(
                SurfaceNode::custom_widget(first, WidgetMessageMapper::none()),
            )],
            CompositionProjection::Both => vec![
                SurfaceChild::fill(SurfaceNode::custom_widget(
                    first,
                    WidgetMessageMapper::none(),
                )),
                SurfaceChild::fill(SurfaceNode::custom_widget(
                    PublicCompositionWidget::new(502, Rc::clone(&self.samples), false, false),
                    WidgetMessageMapper::none(),
                )),
            ],
        };
        arc_surface(UiSurface::new(SurfaceNode::row(500, 4.0, children)))
    }
}

fn empty_start() -> CompositionSample {
    let range = CompositionRange::new(0, 0, 0).expect("empty scalar range");
    CompositionSample::start(range, range).expect("valid composition start")
}

fn update_with_two_scalars() -> CompositionSample {
    let selection = CompositionRange::new(1, 1, 2).expect("two-scalar selection");
    CompositionSample::update("あい", selection).expect("valid composition update")
}

#[test]
fn composition_runtime_pins_owner_and_never_rebinds_continuations() {
    let mut runtime = SurfaceRuntime::new(
        PublicCompositionBridge::default(),
        Vector2::new(260.0, 40.0),
    );
    assert!(runtime.focus_widget(501));
    assert_eq!(
        runtime.dispatch_composition_sample(empty_start()),
        Some(501)
    );
    assert_eq!(runtime.dispatch_composition_sample(empty_start()), None);

    assert!(runtime.focus_widget(502));
    assert_eq!(
        runtime.dispatch_composition_sample(update_with_two_scalars()),
        None
    );
    assert_eq!(
        runtime.bridge().samples.borrow().as_slice(),
        &[(501, CompositionPhase::Start)]
    );
}

#[test]
fn composition_terminal_is_owner_only_and_focus_loss_blocks_stale_samples() {
    let mut runtime = SurfaceRuntime::new(
        PublicCompositionBridge::default(),
        Vector2::new(260.0, 40.0),
    );
    assert!(runtime.focus_widget(501));
    assert_eq!(
        runtime.dispatch_composition_sample(empty_start()),
        Some(501)
    );
    assert_eq!(
        runtime.dispatch_composition_sample(CompositionSample::cancel()),
        Some(501)
    );
    assert_eq!(
        runtime.dispatch_composition_sample(update_with_two_scalars()),
        None
    );

    assert_eq!(
        runtime.bridge().samples.borrow().as_slice(),
        &[
            (501, CompositionPhase::Start),
            (501, CompositionPhase::Cancel),
        ]
    );

    assert_eq!(
        runtime.dispatch_composition_sample(empty_start()),
        Some(501)
    );
    runtime.clear_focus();
    assert_eq!(
        runtime.dispatch_composition_sample(CompositionSample::commit("late")),
        None
    );
    assert_eq!(runtime.bridge().samples.borrow().len(), 3);
}

#[test]
fn composition_refresh_preserves_exact_owner_and_blocks_removed_or_disabled_owner() {
    let mut runtime = SurfaceRuntime::new(
        PublicCompositionBridge::default(),
        Vector2::new(260.0, 40.0),
    );
    assert!(runtime.focus_widget(501));
    assert_eq!(
        runtime.dispatch_composition_sample(empty_start()),
        Some(501)
    );

    runtime.refresh();
    assert_eq!(
        runtime.dispatch_composition_sample(update_with_two_scalars()),
        Some(501)
    );

    runtime.bridge_mut().projection = CompositionProjection::OwnerOnly;
    runtime.refresh();
    assert_eq!(
        runtime.dispatch_composition_sample(update_with_two_scalars()),
        Some(501)
    );

    runtime.bridge_mut().projection = CompositionProjection::SecondOnly;
    runtime.refresh();
    assert_eq!(
        runtime.dispatch_composition_sample(CompositionSample::commit("removed")),
        None
    );

    let mut disabled_runtime = SurfaceRuntime::new(
        PublicCompositionBridge {
            projection: CompositionProjection::Both,
            samples: Rc::new(RefCell::new(Vec::new())),
        },
        Vector2::new(260.0, 40.0),
    );
    assert!(disabled_runtime.focus_widget(501));
    assert_eq!(
        disabled_runtime.dispatch_composition_sample(empty_start()),
        Some(501)
    );
    disabled_runtime.bridge_mut().projection = CompositionProjection::DisabledOwner;
    disabled_runtime.refresh();
    assert_eq!(
        disabled_runtime.dispatch_composition_sample(update_with_two_scalars()),
        None
    );
    assert_eq!(
        disabled_runtime.dispatch_composition_sample(empty_start()),
        None
    );

    let mut read_only_runtime = SurfaceRuntime::new(
        PublicCompositionBridge {
            projection: CompositionProjection::Both,
            samples: Rc::new(RefCell::new(Vec::new())),
        },
        Vector2::new(260.0, 40.0),
    );
    assert!(read_only_runtime.focus_widget(501));
    assert_eq!(
        read_only_runtime.dispatch_composition_sample(empty_start()),
        Some(501)
    );
    read_only_runtime.bridge_mut().projection = CompositionProjection::ReadOnlyOwner;
    read_only_runtime.refresh();
    assert_eq!(
        read_only_runtime.dispatch_composition_sample(update_with_two_scalars()),
        None
    );
}
