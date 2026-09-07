use radiant::runtime::{
    ExternalDropTarget, ExternalOfferAdmission, ExternalOfferData, ExternalOfferFormat,
    ExternalOfferKind, MAX_EXTERNAL_OFFER_TEXT_BYTES, OwnedExternalOffer,
    declarative_owned_runtime_bridge,
};
use radiant::{
    layout::{Point, Vector2},
    prelude::{self as ui, IntoView},
    runtime::testing::DeterministicHost,
};
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

#[test]
fn owned_external_offer_is_available_from_the_public_runtime_api() {
    let offer = OwnedExternalOffer::try_new(ExternalOfferData::Text(String::from("untrusted")))
        .expect("bounded text offer");

    assert_eq!(offer.metadata().kind(), ExternalOfferKind::Text);
    assert_eq!(offer.metadata().item_count(), 1);
    assert_eq!(offer.metadata().bytes(), "untrusted".len());
    assert_eq!(MAX_EXTERNAL_OFFER_TEXT_BYTES, 1024 * 1024);
}

#[derive(Clone)]
enum OfferMessage {
    Decoded(Rc<str>),
    Disable,
    Enable,
    ReplaceDecoder,
}

type DeliveredOffers = Rc<RefCell<Vec<Rc<str>>>>;

struct OfferState {
    enabled: bool,
    wrong_owner: bool,
    target: ExternalDropTarget<OfferMessage>,
    delivered: DeliveredOffers,
}

fn offer_host(
    decode_count: Arc<AtomicUsize>,
    wrong_owner: bool,
) -> (
    DeterministicHost<impl radiant::runtime::RuntimeBridge<OfferMessage>, OfferMessage>,
    DeliveredOffers,
) {
    let owner = radiant::application::DeclarativeEffectOwner::new();
    let target = ExternalDropTarget::new(
        owner,
        ExternalOfferFormat::text(),
        move |offer| {
            decode_count.fetch_add(1, Ordering::SeqCst);
            match offer.data() {
                ExternalOfferData::Text(text) => text.clone(),
                _ => String::new(),
            }
        },
        |text| OfferMessage::Decoded(Rc::from(text)),
    );
    let delivered = Rc::new(RefCell::new(Vec::new()));
    let bridge = declarative_owned_runtime_bridge(
        OfferState {
            enabled: true,
            wrong_owner,
            target,
            delivered: Rc::clone(&delivered),
        },
        |state| {
            let view = ui::button_message("offer target", OfferMessage::Disable)
                .width(100.0)
                .height(100.0);
            if state.enabled {
                let view = view.external_drop_target(state.target.clone());
                if state.wrong_owner {
                    view.effect_owner(radiant::application::DeclarativeEffectOwner::new())
                        .key("offer-target")
                        .into_surface()
                } else {
                    view.key("offer-target").into_surface()
                }
            } else {
                view.into_surface()
            }
        },
        |state, message| match message {
            OfferMessage::Decoded(text) => state.delivered.borrow_mut().push(text),
            OfferMessage::Disable => state.enabled = false,
            OfferMessage::Enable => state.enabled = true,
            OfferMessage::ReplaceDecoder => {
                state.target = ExternalDropTarget::new(
                    state.target.owner(),
                    ExternalOfferFormat::text(),
                    |offer| match offer.data() {
                        ExternalOfferData::Text(value) => format!("B:{value}"),
                        _ => unreachable!("approved text format"),
                    },
                    |value| OfferMessage::Decoded(Rc::from(value)),
                );
            }
        },
    );
    (
        DeterministicHost::with_default_config(bridge, Vector2::new(120.0, 120.0))
            .expect("deterministic host"),
        delivered,
    )
}

#[test]
fn external_offer_decodes_only_after_worker_completion_and_maps_ui_local_messages() {
    let decoded = Arc::new(AtomicUsize::new(0));
    let (mut host, delivered) = offer_host(Arc::clone(&decoded), false);
    let rejected =
        OwnedExternalOffer::try_new(ExternalOfferData::Urls(vec!["https://example.test".into()]))
            .expect("bounded URL offer");
    assert_eq!(
        host.dispatch_external_offer(Point::new(10.0, 10.0), rejected)
            .expect("rejected offer dispatch"),
        ExternalOfferAdmission::Rejected
    );
    assert_eq!(decoded.load(Ordering::SeqCst), 0);
    let offer = OwnedExternalOffer::try_new(ExternalOfferData::Text("later".into())).unwrap();

    assert_eq!(
        host.dispatch_external_offer(Point::new(10.0, 10.0), offer)
            .expect("offer dispatch"),
        ExternalOfferAdmission::Accepted
    );
    assert_eq!(decoded.load(Ordering::SeqCst), 0);
    let worker = host.pending_worker_tasks().pop().expect("worker").id;
    host.complete_worker(worker).expect("complete decoder");
    assert_eq!(decoded.load(Ordering::SeqCst), 1);
    host.turn().expect("delivery turn");
    assert_eq!(delivered.borrow().as_slice(), [Rc::<str>::from("later")]);
}

#[test]
fn retired_keyed_owner_suppresses_late_external_offer_mapper() {
    let decoded = Arc::new(AtomicUsize::new(0));
    let (mut host, delivered) = offer_host(Arc::clone(&decoded), false);
    let offer = OwnedExternalOffer::try_new(ExternalOfferData::Text("stale".into())).unwrap();
    assert_eq!(
        host.dispatch_external_offer(Point::new(10.0, 10.0), offer)
            .expect("offer dispatch"),
        ExternalOfferAdmission::Accepted
    );
    let worker = host.pending_worker_tasks().pop().expect("worker").id;
    host.complete_worker(worker).expect("complete decoder");
    assert_eq!(decoded.load(Ordering::SeqCst), 1);
    host.dispatch_message(OfferMessage::Disable)
        .expect("retire owner");
    host.turn().expect("delivery turn");
    assert_eq!(decoded.load(Ordering::SeqCst), 1);
    assert!(delivered.borrow().is_empty());
}

#[test]
fn external_offers_are_independent_one_shot_workers() {
    let decoded = Arc::new(AtomicUsize::new(0));
    let (mut host, delivered) = offer_host(Arc::clone(&decoded), false);
    for text in ["first", "second"] {
        let offer = OwnedExternalOffer::try_new(ExternalOfferData::Text(text.into())).unwrap();
        assert_eq!(
            host.dispatch_external_offer(Point::new(10.0, 10.0), offer)
                .expect("offer dispatch"),
            ExternalOfferAdmission::Accepted
        );
    }
    let workers = host.pending_worker_tasks();
    assert_eq!(workers.len(), 2);
    for worker in workers {
        host.complete_worker(worker.id).expect("complete decoder");
    }
    host.turn().expect("delivery turn");
    assert_eq!(decoded.load(Ordering::SeqCst), 2);
    assert_eq!(delivered.borrow().len(), 2);
}

#[test]
fn worker_capacity_rejects_without_decoding() {
    let decoded = Arc::new(AtomicUsize::new(0));
    let (mut wrong_owner, _) = offer_host(Arc::clone(&decoded), true);
    let offer = OwnedExternalOffer::try_new(ExternalOfferData::Text("wrong".into())).unwrap();
    assert_eq!(
        wrong_owner
            .dispatch_external_offer(Point::new(10.0, 10.0), offer)
            .unwrap(),
        ExternalOfferAdmission::Rejected
    );
    assert_eq!(decoded.load(Ordering::SeqCst), 0);

    let (mut host, _) = offer_host(Arc::clone(&decoded), false);
    for _ in 0..64 {
        let offer = OwnedExternalOffer::try_new(ExternalOfferData::Text("queued".into())).unwrap();
        assert_eq!(
            host.dispatch_external_offer(Point::new(10.0, 10.0), offer)
                .unwrap(),
            ExternalOfferAdmission::Accepted
        );
    }
    let offer = OwnedExternalOffer::try_new(ExternalOfferData::Text("full".into())).unwrap();
    assert_eq!(
        host.dispatch_external_offer(Point::new(10.0, 10.0), offer)
            .unwrap(),
        ExternalOfferAdmission::Rejected
    );
    assert_eq!(decoded.load(Ordering::SeqCst), 0);
}

#[test]
fn target_wrapper_preserves_fixed_and_fill_parent_slots() {
    for fill in [false, true] {
        let target = ui::button_message("target", ()).width(60.0).height(40.0);
        let target = if fill { target.fill_width() } else { target };
        let target = target.external_drop_target(fixture_target()).key("target");
        let sibling = ui::button_message("sibling", ()).width(20.0).id(903);
        let mut host = fixture_host(ui::row([target, sibling]).spacing(0.0));
        let expected = if fill { 100.0 } else { 60.0 };
        assert_eq!(host.runtime().layout().rects[&903].min.x, expected);
        assert_eq!(
            host.dispatch_external_offer(Point::new(expected - 1.0, 10.0), text_offer("inside"))
                .unwrap(),
            ExternalOfferAdmission::Accepted
        );
        assert_eq!(
            host.dispatch_external_offer(Point::new(expected + 1.0, 10.0), text_offer("sibling"))
                .unwrap(),
            ExternalOfferAdmission::NoTarget
        );
    }
}

fn text_offer(value: &str) -> OwnedExternalOffer {
    OwnedExternalOffer::try_new(ExternalOfferData::Text(value.into())).unwrap()
}

#[test]
fn accepted_offer_keeps_mapper_snapshot_across_compatible_refresh() {
    let (mut host, delivered) = offer_host(Arc::new(AtomicUsize::new(0)), false);
    assert_eq!(
        host.dispatch_external_offer(Point::new(10.0, 10.0), text_offer("old"))
            .unwrap(),
        ExternalOfferAdmission::Accepted
    );
    host.dispatch_message(OfferMessage::ReplaceDecoder).unwrap();
    assert_eq!(
        host.dispatch_external_offer(Point::new(10.0, 10.0), text_offer("new"))
            .unwrap(),
        ExternalOfferAdmission::Accepted
    );
    for worker in host.pending_worker_tasks() {
        host.complete_worker(worker.id).unwrap();
    }
    host.turn().unwrap();
    assert_eq!(
        delivered
            .borrow()
            .iter()
            .map(|value| value.as_ref())
            .collect::<Vec<_>>(),
        ["old", "B:new"]
    );
}

#[test]
fn reopened_same_key_does_not_revive_queued_import() {
    let decoded = Arc::new(AtomicUsize::new(0));
    let (mut host, delivered) = offer_host(Arc::clone(&decoded), false);
    assert_eq!(
        host.dispatch_external_offer(Point::new(10.0, 10.0), text_offer("old"))
            .unwrap(),
        ExternalOfferAdmission::Accepted
    );
    host.complete_worker(host.pending_worker_tasks()[0].id)
        .unwrap();
    host.dispatch_message(OfferMessage::Disable).unwrap();
    host.dispatch_message(OfferMessage::Enable).unwrap();
    host.turn().unwrap();
    assert_eq!(decoded.load(Ordering::SeqCst), 1);
    assert!(delivered.borrow().is_empty());
}

fn fixture_target() -> ExternalDropTarget<()> {
    ExternalDropTarget::new(
        radiant::application::DeclarativeEffectOwner::new(),
        ExternalOfferFormat::text(),
        |_| (),
        |_| (),
    )
}

fn fixture_host(
    view: radiant::application::ViewNode<()>,
) -> DeterministicHost<impl radiant::runtime::RuntimeBridge<()>, ()> {
    let bridge =
        declarative_owned_runtime_bridge(view.into_surface(), |surface| surface.clone(), |_, _| {});
    DeterministicHost::with_default_config(bridge, Vector2::new(120.0, 120.0)).unwrap()
}

#[test]
fn absent_or_ambiguous_keyed_owner_cannot_admit_a_decoder() {
    for ambiguous in [false, true] {
        let target = fixture_target();
        let owner = target.owner();
        let target_view = ui::button_message("target", ())
            .width(60.0)
            .height(60.0)
            .external_drop_target(target);
        let view = if ambiguous {
            ui::row([
                target_view.key("target"),
                ui::button_message("other", ())
                    .effect_owner(owner)
                    .key("other"),
            ])
        } else {
            target_view
        };
        let mut host = fixture_host(view);
        assert_eq!(
            host.dispatch_external_offer(Point::new(10.0, 10.0), text_offer("reject"))
                .unwrap(),
            ExternalOfferAdmission::Rejected
        );
        assert!(host.pending_worker_tasks().is_empty());
    }
}

#[test]
fn modal_and_occluding_layers_block_background_external_targets() {
    for modal in [false, true] {
        let target = ui::button_message("target", ())
            .fill()
            .external_drop_target(fixture_target())
            .key("target");
        let overlay = ui::button_message("occluder", ()).fill();
        let layer = if modal {
            radiant::Layer::modal(overlay).block_input()
        } else {
            radiant::Layer::floating(overlay)
        };
        let view = radiant::application::scene(target).layer(layer).into_view();
        let mut host = fixture_host(view);
        assert_eq!(
            host.dispatch_external_offer(Point::new(10.0, 10.0), text_offer("reject"))
                .unwrap(),
            ExternalOfferAdmission::NoTarget
        );
        assert!(host.pending_worker_tasks().is_empty());
    }
}

#[test]
fn reserved_scroll_gutter_and_padding_are_outside_external_target_clip() {
    use radiant::layout::{ScrollPolicy, ScrollbarPlacement};
    let policy = ScrollPolicy::default().scrollbar_placement(ScrollbarPlacement::Reserved);
    let view = ui::scroll(
        ui::button_message("wide tall target", ())
            .width(200.0)
            .height(240.0)
            .external_drop_target(fixture_target())
            .key("target"),
    )
    .id(900)
    .width(100.0)
    .height(100.0)
    .padding(10.0)
    .scroll_policy(policy);
    let mut host = fixture_host(ui::column([view]));
    let viewport = host.runtime().layout().viewport_bounds[&900];
    let outer = host.runtime().layout().rects[&900];
    assert!(viewport.max.x < outer.max.x && viewport.min.x > outer.min.x);
    let inside = Point::new(viewport.min.x + 1.0, viewport.min.y + 1.0);
    assert_eq!(
        host.dispatch_external_offer(inside, text_offer("inside"))
            .unwrap(),
        ExternalOfferAdmission::Accepted
    );
    for outside in [
        Point::new(1.0, inside.y),
        Point::new(viewport.max.x + 1.0, inside.y),
        Point::new(inside.x, viewport.max.y + 1.0),
    ] {
        assert_eq!(
            host.dispatch_external_offer(outside, text_offer("outside"))
                .unwrap(),
            ExternalOfferAdmission::NoTarget
        );
    }
    assert_eq!(host.pending_worker_tasks().len(), 1);
}
