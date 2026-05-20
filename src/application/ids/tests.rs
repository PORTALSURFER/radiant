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
