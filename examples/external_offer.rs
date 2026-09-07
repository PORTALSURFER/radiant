//! Deterministic owned-offer admission and worker decoding; no native window or I/O.
use radiant::{
    application::{DeclarativeEffectOwner, app, button},
    gui::types::{Point, Vector2},
    runtime::{
        ExternalDropTarget, ExternalOfferAdmission, ExternalOfferData, ExternalOfferFormat,
        OwnedExternalOffer, testing::DeterministicHost,
    },
};
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

type Decoded = Result<u32, &'static str>;

fn exercise() {
    let owner = DeclarativeEffectOwner::new();
    let decoded = Arc::new(AtomicUsize::new(0));
    let worker_counter = Arc::clone(&decoded);
    let received = Rc::new(RefCell::new(Vec::<Rc<Decoded>>::new()));
    let observed = Rc::clone(&received);
    let target = ExternalDropTarget::new(
        owner,
        ExternalOfferFormat::text(),
        move |offer| {
            worker_counter.fetch_add(1, Ordering::Relaxed);
            match offer.data() {
                ExternalOfferData::Text(value) => value
                    .trim()
                    .parse::<u32>()
                    .map_err(|_| "expected an unsigned integer"),
                _ => Err("unsupported representation"),
            }
        },
        Rc::new, // UI-local message; it never crosses the worker boundary.
    );
    let bridge = app(())
        .view(move |_| {
            button("Import a number")
                .filter_mapped(|_| None::<Rc<Decoded>>)
                .width(160.0)
                .height(40.0)
                .external_drop_target(target.clone())
                .key("number-import")
        })
        .update(move |_, result| observed.borrow_mut().push(result))
        .into_bridge();
    let mut host = DeterministicHost::with_default_config(bridge, Vector2::new(200.0, 80.0))
        .expect("headless host");
    let offer = OwnedExternalOffer::try_new(ExternalOfferData::Text(" 42 ".into()))
        .expect("bounded transport data");
    assert_eq!(
        host.dispatch_external_offer(Point::new(20.0, 20.0), offer)
            .expect("offer admission"),
        ExternalOfferAdmission::Accepted
    );
    assert_eq!(decoded.load(Ordering::Relaxed), 0);
    assert!(received.borrow().is_empty());
    let pending = host.pending_worker_tasks();
    assert_eq!(pending.len(), 1);
    host.complete_worker(pending[0].id)
        .expect("worker decoding");
    assert_eq!(decoded.load(Ordering::Relaxed), 1);
    assert!(received.borrow().is_empty());
    host.turn().expect("UI result turn");
    assert_eq!(
        received
            .borrow()
            .iter()
            .map(|value| **value)
            .collect::<Vec<_>>(),
        [Ok(42)]
    );
}

fn main() {
    exercise();
    println!("Owned offer decoded on the worker lane; UI received 42 on the next turn.");
}

#[cfg(test)]
mod tests {
    #[test]
    fn owned_offer_decodes_before_ui_mapping() {
        super::exercise();
    }
}
