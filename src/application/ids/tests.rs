use super::*;

#[test]
fn id_generator_skips_dense_reserved_runs_after_collision() {
    let reserved = (4..=10_000).collect();
    let mut ids = IdGenerator::new(reserved);

    assert_eq!(ids.next(), 1);
    assert_eq!(ids.next(), 2);
    assert_eq!(ids.next(), 3);
    assert_eq!(ids.next(), 10_001);
    assert_eq!(ids.next(), 10_002);
}

#[test]
fn id_generator_preserves_sparse_generation_before_collision() {
    let reserved = vec![8, 20];
    let mut ids = IdGenerator::new(reserved);

    assert_eq!(
        (0..7).map(|_| ids.next()).collect::<Vec<_>>(),
        (1..=7).collect::<Vec<_>>()
    );
    assert_eq!(ids.next(), 9);
}

#[test]
fn id_generator_deduplicates_reserved_ids_before_generation() {
    let mut ids = IdGenerator::new(vec![1, 1, 2, 4]);

    assert_eq!(ids.next(), 3);
    assert_eq!(ids.next(), 5);
}

#[test]
fn id_generator_uses_hashed_reserved_ids_for_large_sets() {
    let ids = IdGenerator::new((10_000..=10_512).rev().collect());

    assert!(matches!(ids.reserved, ReservedIds::Hashed(_)));
    assert_eq!(ids.reserved_range, Some((10_000, 10_512)));
}

#[test]
fn id_generator_keeps_sorted_reserved_ids_for_small_sets() {
    let ids = IdGenerator::new(vec![8, 4, 4]);

    match ids.reserved {
        ReservedIds::Sorted { ids, cursor } => {
            assert_eq!(ids, vec![4, 8]);
            assert_eq!(cursor, 0);
        }
        ReservedIds::Hashed(_) => panic!("small reserved sets should stay sorted vectors"),
    }
}

#[test]
fn id_generator_skips_probing_when_next_id_is_below_reserved_range() {
    let mut ids = IdGenerator::new((10_000..=10_512).collect());

    assert_eq!(ids.next(), 1);
    assert_eq!(ids.next(), 2);
    assert_eq!(ids.next(), 3);
}

#[test]
fn id_generator_skips_probing_after_reserved_range_is_exhausted() {
    let mut ids = IdGenerator::new((4..=512).collect());

    assert_eq!(ids.next(), 1);
    assert_eq!(ids.next(), 2);
    assert_eq!(ids.next(), 3);
    assert_eq!(ids.next(), 513);
    assert_eq!(ids.next(), 514);
}

#[test]
fn structural_identity_is_deterministic_and_role_typed() {
    let first = structural_id(
        super::super::ROOT_KEY_SCOPE,
        StructuralKind::Widget,
        StructuralRole::ContainerChild(2),
    );
    let second = structural_id(
        super::super::ROOT_KEY_SCOPE,
        StructuralKind::Widget,
        StructuralRole::ContainerChild(2),
    );
    let different_role = structural_id(
        super::super::ROOT_KEY_SCOPE,
        StructuralKind::Widget,
        StructuralRole::SceneLayer(2),
    );

    assert_eq!(first, second);
    assert_ne!(first, different_role);
}

#[test]
fn structural_generation_skips_reserved_and_duplicate_ids() {
    let scope = super::super::ROOT_KEY_SCOPE;
    let candidate = structural_id(scope, StructuralKind::Widget, StructuralRole::Root);
    let mut ids = IdGenerator::new(vec![candidate]);

    let first = ids.next_structural(scope, StructuralKind::Widget, StructuralRole::Root);
    let second = ids.next_structural(scope, StructuralKind::Widget, StructuralRole::Root);

    assert_ne!(first.id, candidate);
    assert_ne!(first.id, second.id);
    assert_eq!(first.scope, candidate);
}

#[test]
fn explicit_identity_claim_audits_duplicates() {
    let mut ids = IdGenerator::new(Vec::new());

    assert!(ids.claim_explicit(17));
    assert!(!ids.claim_explicit(17));
}
