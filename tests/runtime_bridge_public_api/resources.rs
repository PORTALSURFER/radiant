use radiant::runtime::{ResourceLoad, ResourceLoadState, ResourceRequest, ResourceSlot};

#[test]
fn runtime_resource_slot_tracks_host_owned_background_results() {
    let mut preview = ResourceSlot::new("preview");

    preview.mark_loading();
    assert_eq!(preview.state(), ResourceLoadState::Loading);

    assert!(preview.apply(ResourceLoad::ready("preview", String::from("decoded"))));
    assert_eq!(preview.state(), ResourceLoadState::Ready);
    assert_eq!(preview.value().map(String::as_str), Some("decoded"));

    assert!(preview.apply(ResourceLoad::failed("preview", "invalid resource")));
    assert_eq!(preview.state(), ResourceLoadState::Failed);
    assert_eq!(preview.error(), Some("invalid resource"));
}

#[test]
fn runtime_resource_requests_reject_stale_same_key_results() {
    let mut preview = ResourceSlot::new("preview");

    let stale: ResourceRequest = preview.begin_load();
    let current = preview.begin_load();

    assert!(!preview.apply_for(&stale, ResourceLoad::ready("preview", String::from("old"))));
    assert_eq!(preview.state(), ResourceLoadState::Loading);
    assert!(preview.apply_for(&current, current.ready(String::from("current"))));
    assert_eq!(preview.value().map(String::as_str), Some("current"));
}

#[test]
fn runtime_resource_request_constructs_results_for_its_key() {
    let mut preview = ResourceSlot::new("preview");
    let request = preview.begin_load();

    assert!(preview.apply_for(&request, request.ready(String::from("decoded"))));
    assert_eq!(preview.value().map(String::as_str), Some("decoded"));

    let request = preview.begin_load();
    assert!(preview.apply_for(&request, request.failed("decode failed")));
    assert_eq!(preview.state(), ResourceLoadState::Failed);
    assert_eq!(preview.error(), Some("decode failed"));
}

#[test]
fn runtime_resource_slot_can_cancel_inflight_load_without_dropping_ready_value() {
    let mut preview = ResourceSlot::new("preview");
    let request = preview.begin_load();
    assert!(preview.apply_for(&request, request.ready(String::from("decoded"))));

    let stale = preview.begin_load();
    preview.cancel_load();

    assert_eq!(preview.state(), ResourceLoadState::Ready);
    assert_eq!(preview.value().map(String::as_str), Some("decoded"));
    assert!(!preview.apply_for(&stale, stale.ready(String::from("stale"))));
    assert_eq!(preview.value().map(String::as_str), Some("decoded"));
}
