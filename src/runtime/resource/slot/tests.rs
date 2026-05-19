use super::{ResourceLoad, ResourceLoadState, ResourceSlot};

#[test]
fn resource_slot_tracks_loading_success_failure_and_revision() {
    let mut slot = ResourceSlot::new("preview");

    assert_eq!(slot.state(), ResourceLoadState::Idle);
    assert_eq!(slot.revision(), 0);

    slot.mark_loading();
    assert!(slot.is_loading());
    assert_eq!(slot.error(), None);

    assert!(slot.apply(ResourceLoad::ready("preview", "pixels")));
    assert_eq!(slot.state(), ResourceLoadState::Ready);
    assert_eq!(slot.value(), Some(&"pixels"));
    assert_eq!(slot.revision(), 1);

    assert!(slot.apply(ResourceLoad::failed("preview", "decode failed")));
    assert_eq!(slot.state(), ResourceLoadState::Failed);
    assert_eq!(slot.value(), None);
    assert_eq!(slot.error(), Some("decode failed"));
    assert_eq!(slot.revision(), 2);
}

#[test]
fn resource_slot_ignores_results_for_other_keys() {
    let mut slot = ResourceSlot::<String>::new("preview");

    assert!(!slot.apply(ResourceLoad::ready("other", String::from("stale"))));
    assert_eq!(slot.state(), ResourceLoadState::Idle);
    assert_eq!(slot.revision(), 0);
}

#[test]
fn resource_slot_rejects_stale_request_results_for_same_key() {
    let mut slot = ResourceSlot::new("preview");

    let stale = slot.begin_load();
    let current = slot.begin_load();

    assert_eq!(stale.key().as_str(), "preview");
    assert!(current.generation() > stale.generation());
    assert!(!slot.apply_for(&stale, ResourceLoad::ready("preview", "old pixels")));
    assert_eq!(slot.state(), ResourceLoadState::Loading);
    assert_eq!(slot.revision(), 0);

    assert!(slot.apply_for(&current, ResourceLoad::ready("preview", "new pixels")));
    assert_eq!(slot.state(), ResourceLoadState::Ready);
    assert_eq!(slot.value(), Some(&"new pixels"));
    assert_eq!(slot.revision(), 1);
}

#[test]
fn resource_request_builds_keyed_success_and_failure_results() {
    let mut slot = ResourceSlot::new("preview");
    let request = slot.begin_load();

    assert!(slot.apply_for(&request, request.ready("pixels")));
    assert_eq!(slot.value(), Some(&"pixels"));

    let request = slot.begin_load();
    assert!(slot.apply_for(&request, request.failed("decode failed")));
    assert_eq!(slot.state(), ResourceLoadState::Failed);
    assert_eq!(slot.error(), Some("decode failed"));
}

#[test]
fn resource_slot_clear_invalidates_in_flight_request() {
    let mut slot = ResourceSlot::new("preview");

    let request = slot.begin_load();
    slot.clear();

    assert!(!slot.apply_for(&request, ResourceLoad::ready("preview", "pixels")));
    assert_eq!(slot.state(), ResourceLoadState::Idle);
    assert_eq!(slot.value(), None);
    assert_eq!(slot.revision(), 1);
}

#[test]
fn resource_slot_cancel_load_preserves_last_ready_value() {
    let mut slot = ResourceSlot::new("preview");
    let initial = slot.begin_load();
    assert!(slot.apply_for(&initial, initial.ready("pixels")));

    let stale = slot.begin_load();
    assert_eq!(slot.state(), ResourceLoadState::Loading);

    slot.cancel_load();

    assert_eq!(slot.state(), ResourceLoadState::Ready);
    assert_eq!(slot.value(), Some(&"pixels"));
    assert_eq!(slot.error(), None);
    assert_eq!(slot.revision(), 2);
    assert!(!slot.apply_for(&stale, stale.ready("stale")));
    assert_eq!(slot.value(), Some(&"pixels"));
}
