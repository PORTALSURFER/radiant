use super::*;
use radiant::widgets::{CompositionPhase, CompositionRange, CompositionSample, WidgetId};
use std::{cell::RefCell, rc::Rc};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CompositionHostMessage {
    Cancel { owner_id: WidgetId },
}

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
    composition_active: Rc<RefCell<bool>>,
    mapped_cancel_output: bool,
}

impl PublicCompositionWidget {
    fn new(
        widget_id: WidgetId,
        samples: Rc<RefCell<Vec<(WidgetId, CompositionPhase)>>>,
        composition_active: Rc<RefCell<bool>>,
        mapped_cancel_output: bool,
        disabled: bool,
        read_only: bool,
    ) -> Self {
        let mut common =
            WidgetCommon::new(widget_id, WidgetSizing::fixed(Vector2::new(120.0, 32.0)))
                .with_keyboard_focus();
        common.state.disabled = disabled;
        common.state.read_only = read_only;
        Self {
            common,
            samples,
            composition_active,
            mapped_cancel_output,
        }
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
        let phase = sample.phase();
        self.samples.borrow_mut().push((self.common.id, phase));
        *self.composition_active.borrow_mut() =
            matches!(phase, CompositionPhase::Start | CompositionPhase::Update);
        (self.mapped_cancel_output && phase == CompositionPhase::Cancel).then(|| {
            WidgetOutput::typed(CompositionHostMessage::Cancel {
                owner_id: self.common.id,
            })
        })
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
    composition_active: Rc<RefCell<bool>>,
    mapped_cancel_output: bool,
    host_messages: Rc<RefCell<Vec<CompositionHostMessage>>>,
    projected_active_states: Rc<RefCell<Vec<bool>>>,
}

impl Default for PublicCompositionBridge {
    fn default() -> Self {
        Self {
            projection: CompositionProjection::Both,
            samples: Rc::new(RefCell::new(Vec::new())),
            composition_active: Rc::new(RefCell::new(false)),
            mapped_cancel_output: false,
            host_messages: Rc::new(RefCell::new(Vec::new())),
            projected_active_states: Rc::new(RefCell::new(Vec::new())),
        }
    }
}

impl PublicCompositionBridge {
    fn mapped_cancel_reprojection() -> Self {
        Self {
            mapped_cancel_output: true,
            ..Self::default()
        }
    }

    fn composition_mapper(&self) -> WidgetMessageMapper<CompositionHostMessage> {
        if self.mapped_cancel_output {
            WidgetMessageMapper::dynamic(|output| output.typed_copied::<CompositionHostMessage>())
        } else {
            WidgetMessageMapper::none()
        }
    }

    fn composition_widget(
        &self,
        widget_id: WidgetId,
        disabled: bool,
        read_only: bool,
    ) -> PublicCompositionWidget {
        PublicCompositionWidget::new(
            widget_id,
            Rc::clone(&self.samples),
            Rc::clone(&self.composition_active),
            self.mapped_cancel_output,
            disabled,
            read_only,
        )
    }
}

impl RuntimeBridge<CompositionHostMessage> for PublicCompositionBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<CompositionHostMessage>> {
        self.projected_active_states
            .borrow_mut()
            .push(*self.composition_active.borrow());
        let first = match self.projection {
            CompositionProjection::DisabledOwner => self.composition_widget(501, true, false),
            CompositionProjection::ReadOnlyOwner => self.composition_widget(501, false, true),
            CompositionProjection::Both | CompositionProjection::OwnerOnly => {
                self.composition_widget(501, false, false)
            }
            CompositionProjection::SecondOnly => self.composition_widget(502, false, false),
        };
        let children = match self.projection {
            CompositionProjection::OwnerOnly
            | CompositionProjection::SecondOnly
            | CompositionProjection::DisabledOwner
            | CompositionProjection::ReadOnlyOwner => vec![SurfaceChild::fill(
                SurfaceNode::custom_widget(first, self.composition_mapper()),
            )],
            CompositionProjection::Both => vec![
                SurfaceChild::fill(SurfaceNode::custom_widget(first, self.composition_mapper())),
                SurfaceChild::fill(SurfaceNode::custom_widget(
                    self.composition_widget(502, false, false),
                    self.composition_mapper(),
                )),
            ],
        };
        arc_surface(UiSurface::new(SurfaceNode::row(500, 4.0, children)))
    }

    fn update(&mut self, message: CompositionHostMessage) -> Command<CompositionHostMessage> {
        self.host_messages.borrow_mut().push(message);
        self.projection = CompositionProjection::SecondOnly;
        Command::repaint(RepaintScope::Projection)
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

fn invalid_update() -> CompositionSample {
    CompositionSample::Update {
        preedit: String::from("あ"),
        selection: CompositionRange::new(0, 0, 2).expect("mismatched scalar evidence"),
        timestamp: None,
    }
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
fn invalid_composition_cancels_live_owner_and_fences_continuation() {
    let bridge = PublicCompositionBridge::default();
    let composition_active = Rc::clone(&bridge.composition_active);
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(260.0, 40.0));

    assert!(runtime.focus_widget(501));
    assert_eq!(
        runtime.dispatch_composition_sample(empty_start()),
        Some(501)
    );
    assert!(*composition_active.borrow());

    assert_eq!(runtime.dispatch_composition_sample(invalid_update()), None);
    assert!(!*composition_active.borrow());
    assert_eq!(
        runtime.bridge().samples.borrow().as_slice(),
        &[
            (501, CompositionPhase::Start),
            (501, CompositionPhase::Cancel),
        ]
    );

    assert!(runtime.focus_widget(502));
    assert_eq!(
        runtime.dispatch_composition_sample(update_with_two_scalars()),
        None
    );
    assert!(!*composition_active.borrow());
    assert_eq!(
        runtime.bridge().samples.borrow().as_slice(),
        &[
            (501, CompositionPhase::Start),
            (501, CompositionPhase::Cancel),
        ]
    );
}

#[test]
fn invalid_composition_maps_one_owner_cancel_and_reprojects_without_rebinding() {
    let bridge = PublicCompositionBridge::mapped_cancel_reprojection();
    let composition_active = Rc::clone(&bridge.composition_active);
    let host_messages = Rc::clone(&bridge.host_messages);
    let projected_active_states = Rc::clone(&bridge.projected_active_states);
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(260.0, 40.0));

    assert!(runtime.focus_widget(501));
    assert_eq!(
        runtime.dispatch_composition_sample(empty_start()),
        Some(501)
    );
    assert!(*composition_active.borrow());

    assert_eq!(runtime.dispatch_composition_sample(invalid_update()), None);
    assert_eq!(
        runtime.bridge().samples.borrow().as_slice(),
        &[
            (501, CompositionPhase::Start),
            (501, CompositionPhase::Cancel),
        ]
    );
    assert_eq!(
        host_messages.borrow().as_slice(),
        &[CompositionHostMessage::Cancel { owner_id: 501 }]
    );
    assert_eq!(projected_active_states.borrow().last(), Some(&false));
    assert!(!*composition_active.borrow());
    assert!(runtime.surface().find_widget(501).is_none());
    assert!(runtime.surface().find_widget(502).is_some());

    assert!(runtime.focus_widget(502));
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
        host_messages.borrow().as_slice(),
        &[CompositionHostMessage::Cancel { owner_id: 501 }]
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
            ..PublicCompositionBridge::default()
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
            ..PublicCompositionBridge::default()
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
