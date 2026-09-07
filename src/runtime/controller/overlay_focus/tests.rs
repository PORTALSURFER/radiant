use super::*;
use crate::{
    application::{IntoView, column, text, text_input},
    gui::{automation::AutomationNodeId, types::Vector2},
    runtime::{
        FocusDirection, FocusTraversal, LayerKind, OverlayFocusOwner, SurfaceLayer, SurfaceNode,
    },
};
use std::sync::Arc;

struct Bridge {
    depth: usize,
    owners: [OverlayFocusOwner; 2],
    empty: bool,
    base_present: bool,
    policy: OverlayFocusPolicy,
}

impl Default for Bridge {
    fn default() -> Self {
        Self {
            depth: 0,
            owners: [OverlayFocusOwner::new(), OverlayFocusOwner::new()],
            empty: false,
            base_present: true,
            policy: OverlayFocusPolicy::Modal,
        }
    }
}

fn inputs(root: u64, first: u64) -> SurfaceNode<()> {
    column([
        text_input("")
            .message(|_| ())
            .id(first)
            .width(100.0)
            .height(28.0),
        text_input("")
            .message(|_| ())
            .id(first + 1)
            .width(100.0)
            .height(28.0),
    ])
    .id(root)
    .fill()
    .into_node()
}

impl RuntimeBridge<()> for Bridge {
    fn project_surface(&mut self) -> Arc<UiSurface<()>> {
        let base = if self.base_present {
            inputs(10, 1)
        } else {
            inputs(20, 3)
        };
        let layers = (0..self.depth)
            .map(|index| {
                let body = if self.empty {
                    text::<()>("empty modal").id(100 + index as u64).into_node()
                } else {
                    inputs(100 + index as u64, 11 + index as u64 * 10)
                };
                SurfaceLayer::new(LayerKind::Modal, body)
                    .focus_owner(self.owners[index].clone(), self.policy)
            })
            .collect();
        crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::scene(900, base, layers)))
    }
}

fn runtime() -> SurfaceRuntime<Bridge, ()> {
    SurfaceRuntime::new(Bridge::default(), Vector2::new(240.0, 160.0))
}

#[test]
fn opening_modal_activates_and_traps_all_focus_entry_points() {
    let mut runtime = runtime();
    assert!(runtime.focus_widget(2));
    let old_target = runtime.focus_target(1).unwrap();
    runtime.bridge_mut().depth = 1;
    runtime.refresh();
    assert_eq!(runtime.focused_widget(), Some(11));
    assert!(!runtime.focus_widget(1));
    assert!(runtime.focus_target(1).is_none());
    assert!(
        runtime
            .semantic_action_target(&AutomationNodeId("1".into()))
            .is_none()
    );
    assert_eq!(
        runtime.transfer_focus(&old_target),
        FocusTransferOutcome::Stale
    );
    assert_eq!(
        runtime.traverse_focus_explicit(FocusTraversal::Forward),
        FocusTransferOutcome::Admitted(12)
    );
    assert_eq!(
        runtime.traverse_focus_explicit(FocusTraversal::Forward),
        FocusTransferOutcome::Admitted(11)
    );
    assert_eq!(
        runtime.traverse_focus_explicit(FocusTraversal::Backward),
        FocusTransferOutcome::Admitted(12)
    );
    assert_eq!(
        runtime.traverse_focus_spatial(FocusDirection::Up),
        FocusTransferOutcome::Admitted(11)
    );
}

#[test]
fn nested_modal_close_restores_inner_then_outer_bookmark() {
    let mut runtime = runtime();
    assert!(runtime.focus_widget(2));
    runtime.bridge_mut().depth = 1;
    runtime.refresh();
    assert!(runtime.focus_widget(12));
    runtime.bridge_mut().depth = 2;
    runtime.refresh();
    assert_eq!(runtime.focused_widget(), Some(21));
    runtime.bridge_mut().depth = 1;
    runtime.refresh();
    assert_eq!(runtime.focused_widget(), Some(12));
    runtime.bridge_mut().depth = 0;
    runtime.refresh();
    assert_eq!(runtime.focused_widget(), Some(2));
}

#[test]
fn closing_both_modals_uses_outermost_surviving_bookmark() {
    let mut runtime = runtime();
    assert!(runtime.focus_widget(2));
    runtime.bridge_mut().depth = 1;
    runtime.refresh();
    runtime.bridge_mut().depth = 2;
    runtime.refresh();
    runtime.bridge_mut().depth = 0;
    runtime.refresh();
    assert_eq!(runtime.focused_widget(), Some(2));
}

#[test]
fn empty_modal_clears_base_focus_without_allowing_escape() {
    let mut runtime = runtime();
    assert!(runtime.focus_widget(1));
    runtime.bridge_mut().empty = true;
    runtime.bridge_mut().depth = 1;
    runtime.refresh();
    assert_eq!(runtime.focused_widget(), None);
    assert!(!runtime.focus_widget(1));
    assert_eq!(
        runtime.traverse_focus_explicit(FocusTraversal::Forward),
        FocusTransferOutcome::NoDestination
    );
    runtime.bridge_mut().depth = 0;
    runtime.refresh();
    assert_eq!(runtime.focused_widget(), Some(1));
}

#[test]
fn nonmodal_restore_policy_preserves_explicit_base_focus_choice() {
    let mut runtime = runtime();
    assert!(runtime.focus_widget(1));
    runtime.bridge_mut().policy = OverlayFocusPolicy::Restore;
    runtime.bridge_mut().depth = 1;
    runtime.refresh();
    assert_eq!(runtime.focused_widget(), Some(1));
    assert!(runtime.focus_widget(2));
    runtime.bridge_mut().depth = 0;
    runtime.refresh();
    assert_eq!(runtime.focused_widget(), Some(2));
}

#[test]
fn retired_prior_owner_uses_current_fallback_on_close() {
    let mut runtime = runtime();
    assert!(runtime.focus_widget(1));
    runtime.bridge_mut().depth = 1;
    runtime.refresh();
    runtime.bridge_mut().base_present = false;
    runtime.refresh();
    runtime.bridge_mut().depth = 0;
    runtime.refresh();
    assert_eq!(runtime.focused_widget(), Some(3));
}

#[test]
fn initially_projected_modal_receives_focus_after_startup() {
    let runtime = SurfaceRuntime::new(
        Bridge {
            depth: 1,
            ..Bridge::default()
        },
        Vector2::new(240.0, 160.0),
    );
    assert_eq!(runtime.focused_widget(), Some(11));
}

use crate::{
    gui::types::Rect,
    layout::LayoutOutput,
    runtime::{PaintPrimitive, WidgetMessageMapper},
    theme::ThemeTokens,
    widgets::{FocusBehavior, Widget, WidgetCommon, WidgetInput, WidgetOutput},
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

#[derive(Clone)]
struct VetoWidget {
    common: WidgetCommon,
    decision: Rc<Cell<FocusLossDecision>>,
    probes: Rc<Cell<usize>>,
    changes: Rc<RefCell<Vec<bool>>>,
    emit_loss: bool,
}
impl Widget for VetoWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }
    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }
    fn prepare_focus_loss(&mut self) -> FocusLossDecision {
        self.probes.set(self.probes.get() + 1);
        self.decision.get()
    }
    fn handle_input(&mut self, _bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        if let WidgetInput::FocusChanged(focused) = input {
            self.common.state.focused = focused;
            self.changes.borrow_mut().push(focused);
            if !focused && self.emit_loss {
                return Some(WidgetOutput::typed(()));
            }
        }
        None
    }
    fn append_paint(
        &self,
        _: &mut Vec<PaintPrimitive>,
        _: Rect,
        _: &LayoutOutput,
        _: &ThemeTokens,
    ) {
    }
}
struct VetoBridge {
    widget: VetoWidget,
    open: bool,
    owner: OverlayFocusOwner,
}
impl RuntimeBridge<()> for VetoBridge {
    fn reduce_message(&mut self, _: ()) {
        self.open = false;
    }
    fn project_surface(&mut self) -> Arc<UiSurface<()>> {
        let layers = if self.open {
            vec![
                SurfaceLayer::new(LayerKind::Modal, inputs(100, 11))
                    .focus_owner(self.owner.clone(), OverlayFocusPolicy::Modal),
            ]
        } else {
            vec![]
        };
        crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::scene(
            900,
            SurfaceNode::widget(self.widget.clone(), WidgetMessageMapper::typed(|()| ())),
            layers,
        )))
    }
}
#[test]
fn modal_open_veto_preserves_complete_surface_and_allow_is_probed_once() {
    let decision = Rc::new(Cell::new(FocusLossDecision::Veto));
    let probes = Rc::new(Cell::new(0));
    let changes = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = SurfaceRuntime::new(
        VetoBridge {
            widget: VetoWidget {
                common: WidgetCommon::fixed(1, 100.0, 28.0).with_focus(FocusBehavior::Keyboard),
                decision: decision.clone(),
                probes: probes.clone(),
                changes: changes.clone(),
                emit_loss: false,
            },
            open: false,
            owner: OverlayFocusOwner::new(),
        },
        Vector2::new(240.0, 160.0),
    );
    assert!(runtime.focus_widget(1));
    changes.borrow_mut().clear();
    runtime.bridge_mut().open = true;
    runtime.refresh();
    assert_eq!(probes.get(), 1);
    assert_eq!(runtime.focused_widget(), Some(1));
    assert!(runtime.surface().find_widget(11).is_none());
    assert!(changes.borrow().is_empty());
    decision.set(FocusLossDecision::Allow);
    runtime.refresh();
    assert_eq!(
        probes.get(),
        2,
        "preflight approval must not call the widget twice"
    );
    assert_eq!(runtime.focused_widget(), Some(11));
    assert_eq!(*changes.borrow(), vec![false]);
    runtime.bridge_mut().open = false;
    runtime.refresh();
    assert_eq!(runtime.focused_widget(), Some(1));
}

#[test]
fn focus_loss_reentrant_close_keeps_successor_focus_and_surface() {
    let changes = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = SurfaceRuntime::new(
        VetoBridge {
            widget: VetoWidget {
                common: WidgetCommon::fixed(1, 100.0, 28.0).with_focus(FocusBehavior::Keyboard),
                decision: Rc::new(Cell::new(FocusLossDecision::Allow)),
                probes: Rc::new(Cell::new(0)),
                changes: changes.clone(),
                emit_loss: true,
            },
            open: false,
            owner: OverlayFocusOwner::new(),
        },
        Vector2::new(240.0, 160.0),
    );
    assert!(runtime.focus_widget(1));
    changes.borrow_mut().clear();
    runtime.bridge_mut().open = true;
    runtime.refresh();
    assert!(!runtime.bridge().open);
    assert!(runtime.surface().find_widget(11).is_none());
    assert_eq!(runtime.focused_widget(), Some(1));
    assert!(!runtime.has_modal_focus_scope());
    assert_eq!(
        *changes.borrow(),
        vec![false, true],
        "old publication must not resend focus after nested close"
    );
}

#[test]
fn modal_semantics_disable_base_actions_and_restore_them_on_close() {
    let mut runtime = runtime();
    runtime.bridge_mut().depth = 1;
    runtime.refresh();
    let snapshot = runtime.automation_target_snapshot();
    let base = snapshot
        .targets
        .iter()
        .find(|target| target.id.0 == "1")
        .unwrap();
    assert!(!base.enabled);
    assert!(base.available_actions.is_empty());
    let modal = snapshot
        .targets
        .iter()
        .find(|target| target.id.0 == "11")
        .unwrap();
    assert!(modal.enabled);
    runtime.bridge_mut().depth = 0;
    runtime.refresh();
    let snapshot = runtime.automation_target_snapshot();
    let base = snapshot
        .targets
        .iter()
        .find(|target| target.id.0 == "1")
        .unwrap();
    assert!(base.enabled);
    assert!(!base.available_actions.is_empty());
}

struct EscapeBridge {
    depth: usize,
    closed: Vec<usize>,
    non_dismissible_inner: bool,
}
impl RuntimeBridge<usize> for EscapeBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<usize>> {
        use crate::application::{Layer, scene};
        let mut root = scene(text::<usize>("base").id(2));
        if self.depth > 0 {
            let mut body = scene(text("outer").id(3).width(70.0).height(30.0));
            if self.depth > 1 {
                let mut inner = Layer::modal(text("inner").id(4).width(50.0).height(20.0))
                    .dismiss_on_outside_click(1);
                if !self.non_dismissible_inner {
                    inner = inner.dismiss_on_escape(1);
                }
                body = body.layer(inner);
            }
            root = root.layer(
                Layer::modal(body.into_view())
                    .dismiss_on_escape(0)
                    .dismiss_on_outside_click(0),
            );
        }
        crate::runtime::test_arc_surface(root.into_view().into_surface())
    }
    fn reduce_message(&mut self, depth: usize) {
        self.closed.push(depth);
        self.depth = depth;
    }
}
fn escape_runtime() -> SurfaceRuntime<EscapeBridge, usize> {
    SurfaceRuntime::new(
        EscapeBridge {
            depth: 2,
            closed: vec![],
            non_dismissible_inner: false,
        },
        Vector2::new(240.0, 160.0),
    )
}
fn escape_event(repeat: bool) -> crate::runtime::Event {
    crate::runtime::Event::KeyPress {
        key: crate::widgets::WidgetKey::Escape,
        modifiers: Default::default(),
        repeat,
        timestamp: None,
    }
}
#[test]
fn escape_dismisses_only_top_overlay_and_repeat_does_not_cascade() {
    let mut runtime = escape_runtime();
    assert!(runtime.dispatch_keyboard_event(escape_event(false)));
    assert_eq!(runtime.bridge().closed, vec![1]);
    assert!(runtime.dispatch_keyboard_event(escape_event(true)));
    assert_eq!(runtime.bridge().closed, vec![1]);
    assert!(runtime.dispatch_keyboard_event(escape_event(false)));
    assert_eq!(runtime.bridge().closed, vec![1, 0]);
}
#[test]
fn non_dismissible_top_overlay_does_not_dismiss_parent() {
    let mut runtime = escape_runtime();
    runtime.bridge_mut().non_dismissible_inner = true;
    runtime.refresh();
    assert!(!runtime.dispatch_keyboard_event(escape_event(false)));
    assert!(runtime.bridge().closed.is_empty());
}
#[test]
fn outside_pointer_dismissal_uses_same_nested_render_order() {
    let mut runtime = escape_runtime();
    runtime.dispatch_primary_click(crate::gui::types::Point::new(220.0, 140.0));
    assert_eq!(runtime.bridge().closed, vec![1]);
    runtime.dispatch_primary_click(crate::gui::types::Point::new(220.0, 140.0));
    assert_eq!(runtime.bridge().closed, vec![1, 0]);
}
#[test]
fn native_compatibility_key_route_dismisses_top_overlay() {
    let mut runtime = escape_runtime();
    assert!(runtime.dispatch_key_press_with_timestamp(
        crate::gui::input::KeyPress {
            key: crate::gui::input::KeyCode::Escape,
            command: false,
            control: false,
            shift: false,
            alt: false
        },
        Some(crate::widgets::WidgetKey::Escape),
        crate::gui::focus::FocusSurface::None,
        Default::default(),
        None,
        false,
    ));
    assert_eq!(runtime.bridge().closed, vec![1]);
}

#[test]
fn recycled_prior_widget_id_cannot_reclaim_retired_bookmark() {
    let mut runtime = runtime();
    assert!(runtime.focus_widget(2));
    runtime.bridge_mut().depth = 1;
    runtime.refresh();
    runtime.bridge_mut().base_present = false;
    runtime.refresh();
    runtime.bridge_mut().base_present = true;
    runtime.refresh();
    runtime.bridge_mut().depth = 0;
    runtime.refresh();
    assert_eq!(
        runtime.focused_widget(),
        Some(1),
        "fallback must not restore recycled id 2"
    );
}

#[test]
fn modal_close_retires_composition_before_same_id_reopens() {
    use crate::widgets::{CompositionRange, CompositionSample};
    let mut runtime = runtime();
    runtime.bridge_mut().depth = 1;
    runtime.refresh();
    let range = CompositionRange::new(0, 0, 0).unwrap();
    assert_eq!(
        runtime.dispatch_composition_sample(CompositionSample::start(range, range).unwrap()),
        Some(11)
    );
    let selected = CompositionRange::new(1, 1, 1).unwrap();
    assert_eq!(
        runtime.dispatch_composition_sample(CompositionSample::update("あ", selected).unwrap()),
        Some(11)
    );
    runtime.bridge_mut().depth = 0;
    runtime.refresh();
    runtime.bridge_mut().depth = 1;
    runtime.refresh();
    assert_eq!(runtime.focused_widget(), Some(11));
    assert_eq!(
        runtime.dispatch_composition_sample(CompositionSample::commit("stale")),
        None
    );
    assert!(
        !runtime
            .surface()
            .find_widget(11)
            .unwrap()
            .widget()
            .retains_managed_composition()
    );
}

#[test]
fn modal_close_retires_pointer_capture_before_body_reopens() {
    let mut runtime = runtime();
    runtime.bridge_mut().depth = 1;
    runtime.refresh();
    let position = runtime.layout().rects[&11].center();
    runtime.dispatch_event(crate::runtime::Event::primary_press(position));
    assert_eq!(runtime.pointer_capture(), Some(11));
    runtime.bridge_mut().depth = 0;
    runtime.refresh();
    assert_eq!(runtime.pointer_capture(), None);
    runtime.bridge_mut().depth = 1;
    runtime.refresh();
    runtime.dispatch_event(crate::runtime::Event::primary_release(position));
    assert_eq!(runtime.pointer_capture(), None);
}
