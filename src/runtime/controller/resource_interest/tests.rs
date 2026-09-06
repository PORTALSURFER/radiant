use super::*;
use crate::{
    application::{
        column, text, DeclarativeEffectOwner, IntoView, ResourceInterest, ResourceInterestError,
        ResourceInterestKind, SharedResourceTasks,
    },
    gui::types::Vector2,
    runtime::{
        Command, DeclarativeOwnedCommandRuntimeBridge, EffectOwner, ResourceInterestEffect,
        RuntimeBridge,
    },
};
use std::{cell::Cell, rc::Rc};

fn runtime_with_owners(
    first: DeclarativeEffectOwner,
    second: DeclarativeEffectOwner,
) -> (
    SurfaceRuntime<impl RuntimeBridge<()>, ()>,
    Rc<Cell<(bool, bool)>>,
) {
    let visible = Rc::new(Cell::new((true, true)));
    let projection_visible = Rc::clone(&visible);
    let bridge = DeclarativeOwnedCommandRuntimeBridge::new(
        (),
        move |_| match projection_visible.get() {
            (true, true) => column([
                text::<()>("first").key("first").effect_owner(first),
                text::<()>("second").key("second").effect_owner(second),
            ])
            .into_surface(),
            (true, false) => {
                column([text::<()>("first").key("first").effect_owner(first)]).into_surface()
            }
            (false, true) => {
                column([text::<()>("second").key("second").effect_owner(second)]).into_surface()
            }
            (false, false) => column([text::<()>("empty").key("empty")]).into_surface(),
        },
        |_, ()| Command::none(),
    );
    (
        SurfaceRuntime::new(bridge, Vector2::new(240.0, 80.0)),
        visible,
    )
}

fn interest_effect(
    tasks: &SharedResourceTasks,
    key: &str,
    owner: EffectOwner,
    interest_id: u64,
    kind: ResourceInterestKind,
) -> ResourceInterestEffect<()> {
    ResourceInterestEffect {
        tasks: tasks.clone(),
        key: key.to_owned().into(),
        owner,
        interest_id,
        kind,
        on_completed: Box::new(|_| ()),
    }
}

fn admit<Bridge: RuntimeBridge<()>>(
    runtime: &mut SurfaceRuntime<Bridge, ()>,
    tasks: &SharedResourceTasks,
    key: &str,
    owner: EffectOwner,
    interest_id: u64,
    kind: ResourceInterestKind,
) -> Result<ResourceInterest, ResourceInterestError> {
    runtime.admit_resource_interest(&interest_effect(tasks, key, owner, interest_id, kind))
}

#[test]
fn retired_owner_releases_only_its_shared_interest() {
    let first = DeclarativeEffectOwner::new();
    let second = DeclarativeEffectOwner::new();
    let (mut runtime, visible) = runtime_with_owners(first, second);
    let tasks = SharedResourceTasks::new();
    let first_interest = admit(
        &mut runtime,
        &tasks,
        "shared",
        EffectOwner::Declarative(first),
        1,
        ResourceInterestKind::Visible,
    )
    .expect("first owner is accepted");
    let second_interest = admit(
        &mut runtime,
        &tasks,
        "shared",
        EffectOwner::Declarative(second),
        1,
        ResourceInterestKind::Prefetch,
    )
    .expect("second owner is accepted");

    visible.set((false, true));
    runtime.refresh();
    assert!(!first_interest.is_live());
    assert!(second_interest.is_live());
    assert_eq!(tasks.interest_count(), 1);

    visible.set((false, false));
    runtime.refresh();
    assert!(!second_interest.is_live());
    assert_eq!(tasks.interest_count(), 0);
}

#[test]
fn class_change_and_duplicate_admission_keep_one_runtime_lease() {
    let first = DeclarativeEffectOwner::new();
    let second = DeclarativeEffectOwner::new();
    let (mut runtime, _) = runtime_with_owners(first, second);
    let tasks = SharedResourceTasks::new();
    let initial = admit(
        &mut runtime,
        &tasks,
        "deduplicated",
        EffectOwner::Declarative(first),
        9,
        ResourceInterestKind::Visible,
    )
    .expect("initial interest");
    let duplicate = admit(
        &mut runtime,
        &tasks,
        "deduplicated",
        EffectOwner::Declarative(first),
        9,
        ResourceInterestKind::Prefetch,
    )
    .expect("same interest deduplicates");

    assert!(initial.set_kind(ResourceInterestKind::Persistent));
    assert_eq!(duplicate.kind(), Some(ResourceInterestKind::Persistent));
    assert_eq!(tasks.interest_count(), 1);
    assert_eq!(runtime.resource_interests.entries.len(), 1);
}

#[test]
fn unavailable_owner_does_not_bind_or_leak_the_broker() {
    let present = DeclarativeEffectOwner::new();
    let absent = DeclarativeEffectOwner::new();
    let other = DeclarativeEffectOwner::new();
    let (mut rejected_runtime, _) = runtime_with_owners(present, other);
    let tasks = SharedResourceTasks::new();

    assert!(matches!(
        admit(
            &mut rejected_runtime,
            &tasks,
            "unavailable",
            EffectOwner::Declarative(absent),
            1,
            ResourceInterestKind::Visible,
        ),
        Err(ResourceInterestError::OwnerUnavailable)
    ));
    assert_eq!(tasks.interest_count(), 0);

    let ambiguous = DeclarativeEffectOwner::new();
    let (mut ambiguous_runtime, _) = runtime_with_owners(ambiguous, ambiguous);
    assert!(matches!(
        admit(
            &mut ambiguous_runtime,
            &tasks,
            "ambiguous",
            EffectOwner::Declarative(ambiguous),
            1,
            ResourceInterestKind::Visible,
        ),
        Err(ResourceInterestError::OwnerUnavailable)
    ));
    assert_eq!(tasks.interest_count(), 0);

    let (mut accepted_runtime, _) = runtime_with_owners(present, other);
    assert!(admit(
        &mut accepted_runtime,
        &tasks,
        "unavailable",
        EffectOwner::Application,
        1,
        ResourceInterestKind::Visible,
    )
    .is_ok());
}

#[test]
fn retired_lease_cannot_retire_a_reinserted_owner_generation() {
    let owner = DeclarativeEffectOwner::new();
    let other = DeclarativeEffectOwner::new();
    let (mut runtime, visible) = runtime_with_owners(owner, other);
    let tasks = SharedResourceTasks::new();
    let old = admit(
        &mut runtime,
        &tasks,
        "reinserted",
        EffectOwner::Declarative(owner),
        3,
        ResourceInterestKind::Visible,
    )
    .expect("old owner admitted");

    visible.set((false, true));
    runtime.refresh();
    assert!(!old.is_live());
    visible.set((true, true));
    runtime.refresh();
    let current = admit(
        &mut runtime,
        &tasks,
        "reinserted",
        EffectOwner::Declarative(owner),
        3,
        ResourceInterestKind::Visible,
    )
    .expect("reinserted owner admitted");

    assert!(!old.release());
    assert!(current.is_live());
}

#[test]
fn discarded_admission_callback_releases_the_returned_interest() {
    let owner = DeclarativeEffectOwner::new();
    let other = DeclarativeEffectOwner::new();
    let (mut runtime, _) = runtime_with_owners(owner, other);
    let tasks = SharedResourceTasks::new();
    let effect = interest_effect(
        &tasks,
        "discarded",
        EffectOwner::Declarative(owner),
        1,
        ResourceInterestKind::Visible,
    );

    let result = runtime.admit_resource_interest(&effect);
    (effect.on_completed)(result);
    assert_eq!(tasks.interest_count(), 0);
}

#[test]
fn closing_runtime_retires_all_registered_interests() {
    let owner = DeclarativeEffectOwner::new();
    let other = DeclarativeEffectOwner::new();
    let (mut runtime, _) = runtime_with_owners(owner, other);
    let tasks = SharedResourceTasks::new();
    let interest = admit(
        &mut runtime,
        &tasks,
        "closing",
        EffectOwner::Declarative(owner),
        1,
        ResourceInterestKind::Visible,
    )
    .expect("owner admitted");

    assert!(runtime.begin_closing());
    assert!(!interest.is_live());
    assert_eq!(tasks.interest_count(), 0);
}

#[test]
fn dropped_command_never_admits_an_interest() {
    let tasks = SharedResourceTasks::new();
    let command = tasks.interest(
        "dropped-command",
        EffectOwner::Application,
        1,
        ResourceInterestKind::Visible,
        |_| (),
    );
    drop(command);
    assert_eq!(tasks.interest_count(), 0);
}

#[test]
fn aggregate_runtime_capacity_deduplicates_without_binding_or_leaking_rejected_brokers() {
    let first = DeclarativeEffectOwner::new();
    let second = DeclarativeEffectOwner::new();
    let (mut runtime, _) = runtime_with_owners(first, second);
    let full = SharedResourceTasks::new();
    let mut retained = Vec::new();

    // Fill the controller-wide registry exactly: 16 resource keys with the
    // ledger's 64 distinct interests per key. Application ownership keeps this
    // independent of declarative projection while exercising the same runtime
    // aggregate guard used by all brokers.
    for key_index in 0..16 {
        let key = format!("aggregate-{key_index}");
        for interest_id in 0..64 {
            retained.push(
                admit(
                    &mut runtime,
                    &full,
                    &key,
                    EffectOwner::Application,
                    interest_id,
                    ResourceInterestKind::Visible,
                )
                .expect("aggregate capacity admits exactly 1,024 interests"),
            );
        }
    }
    assert_eq!(full.interest_count(), 1024);
    assert_eq!(runtime.resource_interests.entries.len(), 1024);

    let duplicate = admit(
        &mut runtime,
        &full,
        "aggregate-0",
        EffectOwner::Application,
        0,
        ResourceInterestKind::Persistent,
    )
    .expect("exact duplicate remains admitted at capacity");
    assert_eq!(full.interest_count(), 1024);
    assert_eq!(runtime.resource_interests.entries.len(), 1024);
    drop(duplicate);

    let rejected = SharedResourceTasks::new();
    assert!(matches!(
        admit(
            &mut runtime,
            &rejected,
            "rejected-fresh-broker",
            EffectOwner::Application,
            1,
            ResourceInterestKind::Visible,
        ),
        Err(ResourceInterestError::RuntimeCapacity)
    ));
    assert_eq!(rejected.interest_count(), 0);

    // A capacity-rejected broker has no accepted runtime attachment and can
    // therefore be admitted by a different runtime.
    let (mut other_runtime, _) = runtime_with_owners(first, second);
    assert!(admit(
        &mut other_runtime,
        &rejected,
        "rejected-fresh-broker",
        EffectOwner::Application,
        1,
        ResourceInterestKind::Visible,
    )
    .is_ok());

    drop(retained);
}
