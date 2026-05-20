//! Resource-slot performance scenarios for host-owned background load state.

use radiant::runtime::{ResourceLoad, ResourceSlot};
use std::hint::black_box;

pub(super) fn resource_slot_stale_completions_1k() -> impl FnMut() {
    let mut slot = ResourceSlot::new("preview");
    move || {
        let mut accepted = 0usize;
        let mut rejected = 0usize;

        for index in 0..1_000 {
            let stale = slot.begin_load();
            let current = slot.begin_load();
            if slot.apply_for(
                &stale,
                ResourceLoad::ready("preview", format!("stale-{index}")),
            ) {
                accepted += 1;
            } else {
                rejected += 1;
            }
            if slot.apply_for(&current, current.ready(format!("current-{index}"))) {
                accepted += 1;
            } else {
                rejected += 1;
            }
        }

        assert_eq!(accepted, 1_000);
        assert_eq!(rejected, 1_000);
        assert_eq!(slot.value().map(String::as_str), Some("current-999"));
        black_box((accepted, rejected, slot.revision()));
    }
}
