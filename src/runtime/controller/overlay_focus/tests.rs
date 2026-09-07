use super::*;
use crate::{
    application::{IntoView, column, text, text_input},
    gui::{
        automation::AutomationNodeId,
        types::{Rect, Vector2},
    },
    runtime::{
        FocusDirection, FocusTraversal, LayerKind, OverlayFocusOwner, SurfaceLayer, SurfaceNode,
    },
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
};

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
    layout::LayoutOutput,
    runtime::{PaintPrimitive, WidgetMessageMapper},
    theme::ThemeTokens,
    widgets::{FocusBehavior, Widget, WidgetCommon, WidgetInput, WidgetOutput},
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

#[test]
fn initially_open_modal_without_prior_focus_closes_to_first_base_target() {
    let mut runtime = SurfaceRuntime::new(
        Bridge {
            depth: 1,
            ..Bridge::default()
        },
        Vector2::new(240.0, 160.0),
    );
    assert_eq!(runtime.focused_widget(), Some(11));
    runtime.bridge_mut().depth = 0;
    runtime.refresh();
    assert_eq!(runtime.focused_widget(), Some(1));
}

#[test]
fn nonmodal_close_preserves_explicitly_cleared_focus() {
    let mut runtime = runtime();
    assert!(runtime.focus_widget(1));
    runtime.bridge_mut().policy = OverlayFocusPolicy::Restore;
    runtime.bridge_mut().depth = 1;
    runtime.refresh();
    runtime.clear_focus();
    runtime.bridge_mut().depth = 0;
    runtime.refresh();
    assert_eq!(runtime.focused_widget(), None);
}

#[test]
fn closing_runtime_does_not_admit_overlay_escape() {
    let mut runtime = escape_runtime();
    assert!(runtime.begin_closing());
    assert!(!runtime.dispatch_keyboard_event(escape_event(false)));
    assert!(runtime.bridge().closed.is_empty());
}

struct AnchorBridge {
    open: bool,
    trigger: bool,
    duplicate: bool,
    veto: Option<VetoWidget>,
}
impl RuntimeBridge<()> for AnchorBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<()>> {
        use crate::application::{Layer, scene, spacer};
        let base = if let Some(veto) = &self.veto {
            crate::application::widget(crate::application::MappedWidget::new(
                veto.clone(),
                WidgetMessageMapper::typed(|()| ()),
            ))
            .id(1)
            .width(100.0)
            .height(28.0)
        } else {
            text_input("")
                .message(|_| ())
                .id(1)
                .width(100.0)
                .height(28.0)
        };
        let mut children = vec![base, spacer().height(100.0)];
        if self.trigger {
            children.push(text("trigger").id(90).width(30.0).height(20.0));
        }
        if self.duplicate {
            children.push(text("duplicate").id(90).width(30.0).height(20.0));
        }
        let mut root = scene(column(children).fill());
        if self.open {
            root = root.layer(
                Layer::modal(text_input("").message(|_| ()).id(11).fill())
                    .block_input()
                    .anchored_to(
                        crate::layout::OverlayAnchor::below(90, Vector2::new(80.0, 40.0)).gap(4.0),
                    ),
            );
        }
        crate::runtime::test_arc_surface(root.into_view().into_surface())
    }
}
fn anchor_runtime() -> SurfaceRuntime<AnchorBridge, ()> {
    let mut runtime = SurfaceRuntime::new(
        AnchorBridge {
            open: false,
            trigger: true,
            duplicate: false,
            veto: None,
        },
        Vector2::new(240.0, 220.0),
    );
    assert!(runtime.focus_widget(1));
    runtime.bridge_mut().open = true;
    runtime.refresh();
    assert_eq!(runtime.focused_widget(), Some(11));
    runtime
}

#[test]
fn anchored_modal_tracks_resize_and_restores_focus_when_trigger_is_clipped() {
    let mut runtime = anchor_runtime();
    let trigger = runtime.layout().rects[&90];
    assert_eq!(runtime.layout().rects[&11].min.y, trigger.max.y + 4.0);
    runtime.set_viewport(Vector2::new(240.0, 160.0));
    assert_eq!(runtime.layout().rects[&11].max.y, trigger.min.y - 4.0);
    runtime.set_viewport(Vector2::new(240.0, 80.0));
    assert!(!runtime.layout().rects.contains_key(&11));
    assert_eq!(runtime.focused_widget(), Some(1));
    assert!(!runtime.has_modal_focus_scope());
    assert!(
        !runtime
            .surface()
            .find_widget(11)
            .unwrap()
            .widget()
            .common()
            .state
            .focused
    );
    runtime.set_viewport(Vector2::new(240.0, 220.0));
    assert_eq!(runtime.focused_widget(), Some(11));
    assert_eq!(runtime.layout().rects[&11].min.y, trigger.max.y + 4.0);
}

#[test]
fn anchored_modal_missing_trigger_omits_shield_and_semantics() {
    let mut runtime = anchor_runtime();
    runtime.bridge_mut().trigger = false;
    runtime.refresh();
    assert!(!runtime.layout().rects.contains_key(&11));
    assert_eq!(runtime.focused_widget(), Some(1));
    let base_position = runtime.layout().rects[&1].center();
    runtime.dispatch_event(crate::runtime::Event::primary_press(base_position));
    assert_eq!(runtime.pointer_capture(), Some(1));
    assert!(
        runtime
            .automation_target_snapshot()
            .targets
            .iter()
            .all(|target| target.id.0 != "11")
    );
}

#[test]
fn visible_anchored_modal_noop_refresh_keeps_focus_and_close_restores_base() {
    let mut runtime = anchor_runtime();
    assert_eq!(runtime.focused_widget(), Some(11));
    runtime.refresh();
    assert_eq!(runtime.focused_widget(), Some(11));
    assert!(runtime.has_modal_focus_scope());

    runtime.bridge_mut().open = false;
    runtime.refresh();
    assert_eq!(runtime.focused_widget(), Some(1));
    assert!(!runtime.has_modal_focus_scope());
}

#[test]
fn unresolved_anchored_modal_does_not_probe_a_vetoing_focused_editor() {
    let decision = Rc::new(Cell::new(FocusLossDecision::Veto));
    let probes = Rc::new(Cell::new(0));
    let mut runtime = SurfaceRuntime::new(
        AnchorBridge {
            open: false,
            trigger: false,
            duplicate: false,
            veto: Some(VetoWidget {
                common: WidgetCommon::fixed(1, 100.0, 28.0).with_focus(FocusBehavior::Keyboard),
                decision,
                probes: probes.clone(),
                changes: Rc::new(RefCell::new(Vec::new())),
                emit_loss: false,
            }),
        },
        Vector2::new(240.0, 220.0),
    );
    assert!(runtime.focus_widget(1));
    runtime.bridge_mut().open = true;
    runtime.refresh();

    assert_eq!(probes.get(), 0);
    assert_eq!(runtime.focused_widget(), Some(1));
    assert!(!runtime.has_modal_focus_scope());
    assert!(!runtime.layout().rects.contains_key(&11));
}

#[test]
fn visible_anchored_modal_veto_omits_group_and_allow_is_probed_once() {
    let decision = Rc::new(Cell::new(FocusLossDecision::Veto));
    let probes = Rc::new(Cell::new(0));
    let mut runtime = SurfaceRuntime::new(
        AnchorBridge {
            open: false,
            trigger: true,
            duplicate: false,
            veto: Some(VetoWidget {
                common: WidgetCommon::fixed(1, 100.0, 28.0).with_focus(FocusBehavior::Keyboard),
                decision: decision.clone(),
                probes: probes.clone(),
                changes: Rc::new(RefCell::new(Vec::new())),
                emit_loss: false,
            }),
        },
        Vector2::new(240.0, 220.0),
    );
    assert!(runtime.focus_widget(1));
    runtime.bridge_mut().open = true;
    runtime.refresh();
    assert_eq!(probes.get(), 1);
    assert_eq!(runtime.focused_widget(), Some(1));
    assert!(!runtime.layout().rects.contains_key(&11));
    assert!(!runtime.has_modal_focus_scope());

    decision.set(FocusLossDecision::Allow);
    runtime.set_viewport(Vector2::new(240.0, 221.0));
    assert_eq!(probes.get(), 2);
    assert_eq!(runtime.focused_widget(), Some(11));
    assert!(runtime.layout().rects.contains_key(&11));
}

#[test]
fn anchored_modal_resize_retires_capture_and_composition_before_reappearance() {
    use crate::widgets::{CompositionRange, CompositionSample};
    let mut runtime = anchor_runtime();
    let point = runtime.layout().rects[&11].center();
    runtime.dispatch_event(crate::runtime::Event::primary_press(point));
    assert_eq!(runtime.pointer_capture(), Some(11));
    let range = CompositionRange::new(0, 0, 0).unwrap();
    assert_eq!(
        runtime.dispatch_composition_sample(CompositionSample::start(range, range).unwrap()),
        Some(11)
    );
    runtime.set_viewport(Vector2::new(240.0, 80.0));
    assert_eq!(runtime.pointer_capture(), None);
    runtime.set_viewport(Vector2::new(240.0, 220.0));
    assert_eq!(
        runtime.dispatch_composition_sample(CompositionSample::commit("stale")),
        None
    );
    runtime.dispatch_event(crate::runtime::Event::primary_release(point));
    assert_eq!(runtime.pointer_capture(), None);
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
fn anchored_modal_reactivation_veto_keeps_current_base_geometry() {
    let decision = Rc::new(Cell::new(FocusLossDecision::Veto));
    let probes = Rc::new(Cell::new(0));
    let mut runtime = SurfaceRuntime::new(
        AnchorBridge {
            open: true,
            trigger: true,
            duplicate: false,
            veto: Some(VetoWidget {
                common: WidgetCommon::fixed(1, 100.0, 28.0).with_focus(FocusBehavior::Keyboard),
                decision: decision.clone(),
                probes: probes.clone(),
                changes: Rc::new(RefCell::new(Vec::new())),
                emit_loss: false,
            }),
        },
        Vector2::new(240.0, 80.0),
    );
    assert!(runtime.focus_widget(1));
    assert_eq!(
        probes.get(),
        0,
        "offscreen anchor must not probe focus loss"
    );
    assert!(!runtime.layout().rects.contains_key(&11));
    runtime.set_viewport(Vector2::new(240.0, 220.0));
    assert_eq!(probes.get(), 1);
    assert_eq!(runtime.focused_widget(), Some(1));
    assert!(!runtime.layout().rects.contains_key(&11));
    assert!(!runtime.has_modal_focus_scope());
    assert_eq!(runtime.context().viewport.height(), 220.0);
    decision.set(FocusLossDecision::Allow);
    runtime.set_viewport(Vector2::new(240.0, 221.0));
    assert_eq!(probes.get(), 2);
    assert_eq!(runtime.focused_widget(), Some(11));
}

#[test]
fn anchored_modal_scale_change_uses_logical_geometry_once() {
    let mut runtime = anchor_runtime();
    let previous = runtime.layout().rects[&11];
    runtime.set_window_environment(crate::runtime::WindowEnvironment::new(
        crate::theme::DpiScale::new(2.0),
        None,
        false,
        false,
    ));
    runtime.relayout_current_surface();
    assert_eq!(runtime.layout().rects[&11], previous);
    assert_eq!(runtime.focused_widget(), Some(11));
}

struct NestedAnchorBridge {
    child_anchor: u64,
}
impl RuntimeBridge<()> for NestedAnchorBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<()>> {
        use crate::application::{Layer, scene};
        use crate::layout::OverlayAnchor;
        let input = |id| {
            text_input("")
                .message(|_| ())
                .id(id)
                .width(80.0)
                .height(24.0)
        };
        let inner = Layer::popover(input(21))
            .focus_policy(OverlayFocusPolicy::Modal)
            .block_input()
            .anchored_to(OverlayAnchor::below(
                self.child_anchor,
                Vector2::new(80.0, 30.0),
            ));
        let outer = scene(column([input(11), text("inner trigger").id(91).height(20.0)]).fill())
            .layer(inner)
            .into_view()
            .key("outer");
        let root = scene(column([input(1), text("outer trigger").id(90).height(20.0)]).fill())
            .layer(
                Layer::modal(outer)
                    .block_input()
                    .anchored_to(OverlayAnchor::below(90, Vector2::new(120.0, 100.0))),
            );
        crate::runtime::test_arc_surface(root.into_view().into_surface())
    }
}
#[test]
fn nested_anchored_modal_uses_current_parent_geometry_and_restores_base_on_omission() {
    let mut runtime = SurfaceRuntime::new(
        NestedAnchorBridge { child_anchor: 91 },
        Vector2::new(240.0, 220.0),
    );
    assert_eq!(runtime.focused_widget(), Some(21));
    assert_eq!(
        runtime.layout().rects[&21].min.y,
        runtime.layout().rects[&91].max.y
    );
    assert!(!runtime.focus_widget(11));
    runtime.set_viewport(Vector2::new(240.0, 20.0));
    assert!(!runtime.layout().rects.contains_key(&11));
    assert!(!runtime.layout().rects.contains_key(&21));
    assert_eq!(runtime.focused_widget(), Some(1));
    runtime.set_viewport(Vector2::new(240.0, 220.0));
    assert_eq!(runtime.focused_widget(), Some(21));
}

#[test]
fn nested_overlay_anchored_to_base_is_omitted_with_its_parent() {
    let mut runtime = SurfaceRuntime::new(
        NestedAnchorBridge { child_anchor: 1 },
        Vector2::new(240.0, 220.0),
    );
    assert_eq!(runtime.focused_widget(), Some(21));
    runtime.set_viewport(Vector2::new(240.0, 20.0));
    assert!(runtime.layout().rects.contains_key(&1));
    assert!(!runtime.layout().rects.contains_key(&11));
    assert!(!runtime.layout().rects.contains_key(&21));
    assert_eq!(runtime.focused_widget(), Some(1));
    runtime.dispatch_event(crate::runtime::Event::primary_press(
        runtime.layout().rects[&1].center(),
    ));
    assert_eq!(runtime.pointer_capture(), Some(1));
}

#[derive(Clone)]
struct WheelProbe {
    common: crate::widgets::WidgetCommon,
    phases: Rc<RefCell<Vec<crate::widgets::WheelPhase>>>,
}

impl crate::widgets::Widget for WheelProbe {
    fn common(&self) -> &crate::widgets::WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut crate::widgets::WidgetCommon {
        &mut self.common
    }

    fn handle_input(
        &mut self,
        _bounds: Rect,
        _input: crate::widgets::WidgetInput,
    ) -> Option<crate::widgets::WidgetOutput> {
        None
    }

    fn handle_wheel_sample(
        &mut self,
        _bounds: Rect,
        _position: crate::gui::types::Point,
        sample: crate::widgets::WheelSample,
    ) -> Option<crate::widgets::WidgetOutput> {
        if let Some(phase) = sample.phase() {
            self.phases.borrow_mut().push(phase);
        }
        None
    }

    fn accepts_wheel_input(&self) -> bool {
        true
    }

    fn retains_managed_wheel_sequence(&self) -> bool {
        true
    }

    fn append_paint(
        &self,
        _primitives: &mut Vec<crate::runtime::PaintPrimitive>,
        _bounds: Rect,
        _layout: &crate::layout::LayoutOutput,
        _theme: &crate::theme::ThemeTokens,
    ) {
    }
}

struct WheelProbeBridge {
    probe: WheelProbe,
    owner: OverlayFocusOwner,
}

impl RuntimeBridge<()> for WheelProbeBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<()>> {
        crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::scene(
            900,
            SurfaceNode::text(
                1,
                "base",
                crate::widgets::WidgetSizing::fixed(Vector2::new(120.0, 80.0)),
            ),
            vec![
                SurfaceLayer::new(
                    LayerKind::Popover,
                    SurfaceNode::widget(
                        self.probe.clone(),
                        crate::runtime::WidgetMessageMapper::typed(|()| ()),
                    ),
                )
                .focus_owner(self.owner.clone(), OverlayFocusPolicy::Restore),
            ],
        )))
    }
}

#[test]
fn omitted_nonfocusable_overlay_wheel_owner_receives_one_cancelled_terminal() {
    let phases = Rc::new(RefCell::new(Vec::new()));
    let probe = WheelProbe {
        common: crate::widgets::WidgetCommon::fixed(11, 100.0, 40.0),
        phases: Rc::clone(&phases),
    };
    let mut runtime = SurfaceRuntime::new(
        WheelProbeBridge {
            probe,
            owner: OverlayFocusOwner::new(),
        },
        Vector2::new(160.0, 100.0),
    );
    let bounds = runtime.layout().rects[&11];
    assert!(
        runtime.wheel_or_scroll_at_with_sample(
            bounds.center(),
            crate::widgets::WheelSample::started(
                crate::widgets::WheelDelta::Pixels(Vector2::new(0.0, 1.0)),
                crate::widgets::PointerModifiers::default(),
            )
            .expect("finite wheel start"),
        )
    );
    let owners = runtime.capture_overlay_input_owners();
    runtime.layout.rects.remove(&11);
    let _ = runtime.retire_omitted_overlay_input(owners);
    assert_eq!(
        *phases.borrow(),
        vec![
            crate::widgets::WheelPhase::Started,
            crate::widgets::WheelPhase::Cancelled,
        ]
    );
    assert!(matches!(
        runtime.interaction.wheel.managed_sequence,
        super::super::interaction_state::RuntimeManagedWheelSequenceState::Blocked
    ));
}

#[test]
fn geometry_materialization_qualifies_the_accepted_overlay_membership() {
    let mut runtime = runtime();
    runtime.bridge_mut().depth = 1;
    runtime.refresh();
    assert_eq!(runtime.focused_widget(), Some(11));
    let transition = runtime.capture_overlay_relayout();
    // The virtual geometry path can accept new concrete surfaces between
    // capturing old input owners and publishing final layout, without asking
    // for another application projection.
    runtime.bridge_mut().depth = 2;
    runtime.surface = runtime.bridge_mut().project_surface().as_ref().clone();
    let projection = runtime.surface.runtime_projection();
    runtime.replace_layout_root(projection.layout_root);
    runtime.relayout_with_traversal(projection.traversal);
    runtime.finish_overlay_relayout(transition);
    assert_eq!(runtime.focused_widget(), Some(21));
    assert!(!runtime.focus_widget(11));
}
