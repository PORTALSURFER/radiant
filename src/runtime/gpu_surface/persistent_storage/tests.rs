use super::*;

fn target(identity: u64, generation: u64) -> GpuPersistentStorageTarget {
    GpuPersistentStorageTarget::new(7, CanvasKey::new(11), identity, generation)
}

fn snapshot(
    target: GpuPersistentStorageTarget,
    revision: u64,
    bytes: &[u8],
) -> GpuPersistentStorageSnapshot {
    GpuPersistentStorageSnapshot::new(target, 4, 64, bytes.len(), revision, bytes).unwrap()
}

#[test]
fn stale_gap_and_recovery_never_mutate_the_accepted_shadow() {
    let target = target(1, 3);
    let mut store = GpuPersistentStorageStore::default();
    assert_eq!(
        store
            .apply_snapshot(snapshot(target, 4, &[1, 2, 3, 4]))
            .unwrap(),
        GpuPersistentStorageStatus::Ready { revision: 4 }
    );
    let stale = GpuPersistentStoragePatch::replace(target, 3, 4, 0, [9, 9, 9, 9]).unwrap();
    assert_eq!(
        store.apply_patch(stale),
        Err(GpuPersistentStorageError::StalePatch)
    );
    let gap = GpuPersistentStoragePatch::replace(target, 6, 7, 0, [9, 9, 9, 9]).unwrap();
    assert_eq!(
        store.apply_patch(gap).unwrap(),
        GpuPersistentStorageStatus::NeedsSnapshot { revision: 4 }
    );
    let blocked = GpuPersistentStoragePatch::replace(target, 4, 5, 0, [8, 8, 8, 8]).unwrap();
    assert_eq!(
        store.apply_patch(blocked).unwrap(),
        GpuPersistentStorageStatus::NeedsSnapshot { revision: 4 }
    );
    assert_eq!(store.entry(target).unwrap().bytes()[..4], [1, 2, 3, 4]);
    assert_eq!(
        store
            .apply_snapshot(snapshot(target, 8, &[5, 6, 7, 8]))
            .unwrap(),
        GpuPersistentStorageStatus::Ready { revision: 8 }
    );
}

#[test]
fn replace_append_and_overlaps_coalesce_without_shadow_copying() {
    let target = target(1, 3);
    let mut store = GpuPersistentStorageStore::default();
    store
        .apply_snapshot(snapshot(target, 0, &[0, 0, 0, 0]))
        .unwrap();
    store
        .apply_patch(GpuPersistentStoragePatch::append(target, 0, 1, [1, 1, 1, 1]).unwrap())
        .unwrap();
    store
        .apply_patch(GpuPersistentStoragePatch::replace(target, 1, 2, 0, [2, 2, 2, 2]).unwrap())
        .unwrap();
    store
        .apply_patch(GpuPersistentStoragePatch::replace(target, 2, 3, 4, [3, 3, 3, 3]).unwrap())
        .unwrap();
    let entry = store.entry(target).unwrap();
    assert_eq!(entry.bytes()[..8], [2, 2, 2, 2, 3, 3, 3, 3]);
    assert_eq!(entry.logical_len(), 8);
    assert_eq!(
        entry.uploads_since(Some(0)),
        PersistentStorageUploads::Ranges {
            revision: 3,
            ranges: vec![PersistentStorageUploadRange { start: 0, end: 8 }],
        }
    );
}

#[test]
fn generation_replacement_is_deterministic_and_release_requires_exact_fence() {
    let old = target(1, 1);
    let replacement = target(2, 1);
    let mut store = GpuPersistentStorageStore::default();
    store
        .apply_snapshot(snapshot(old, 9, &[1, 1, 1, 1]))
        .unwrap();
    assert_eq!(
        store.apply_snapshot(snapshot(target(1, 0), 10, &[9, 9, 9, 9])),
        Err(GpuPersistentStorageError::StaleSnapshot)
    );
    assert_eq!(
        store.apply_snapshot(snapshot(old, 9, &[9, 9, 9, 9])),
        Err(GpuPersistentStorageError::StaleSnapshot)
    );
    store
        .apply_snapshot(snapshot(replacement, 0, &[2, 2, 2, 2]))
        .unwrap();
    assert!(store.entry(old).is_none());
    assert_eq!(store.entry(replacement).unwrap().revision(), 0);
    assert!(!store.release(old));
    assert!(store.release(replacement));
}

#[test]
fn reincarnation_changes_when_a_released_fence_is_recreated() {
    let target = target(1, 1);
    let mut store = GpuPersistentStorageStore::default();
    store
        .apply_snapshot(snapshot(target, 0, &[1, 1, 1, 1]))
        .unwrap();
    let first = store.entry(target).unwrap().incarnation();
    assert!(store.release(target));
    store
        .apply_snapshot(snapshot(target, 0, &[2, 2, 2, 2]))
        .unwrap();
    assert!(store.entry(target).unwrap().incarnation() > first);
}

#[test]
fn patch_operation_exposes_only_validated_payloads() {
    let target = target(1, 1);
    let patch = GpuPersistentStoragePatch::replace(target, 2, 3, 4, [1, 2, 3, 4]).unwrap();
    assert_eq!(
        patch.operation(),
        GpuPersistentStoragePatchOperation::Replace
    );
    assert_eq!(patch.byte_offset(), Some(4));
    assert_eq!(patch.bytes(), [1, 2, 3, 4]);
}

#[test]
fn history_lag_requests_full_capacity_and_equal_cursor_is_empty() {
    let target = target(1, 1);
    let mut store = GpuPersistentStorageStore::default();
    store
        .apply_snapshot(snapshot(target, 0, &[0, 0, 0, 0]))
        .unwrap();
    for revision in 0..65 {
        store
            .apply_patch(
                GpuPersistentStoragePatch::replace(
                    target,
                    revision,
                    revision + 1,
                    0,
                    [revision as u8; 4],
                )
                .unwrap(),
            )
            .unwrap();
    }
    let entry = store.entry(target).unwrap();
    assert_eq!(
        entry.uploads_since(Some(0)),
        PersistentStorageUploads::Full {
            revision: 65,
            range: PersistentStorageUploadRange { start: 0, end: 64 },
        }
    );
    assert_eq!(
        entry.uploads_since(Some(65)),
        PersistentStorageUploads::Empty { revision: 65 }
    );
    assert_eq!(
        entry.uploads_since(None),
        PersistentStorageUploads::Full {
            revision: 65,
            range: PersistentStorageUploadRange { start: 0, end: 64 },
        }
    );
}

#[test]
fn deterministic_reference_sequence_matches_mutable_shadow() {
    let target = target(1, 1);
    let mut store = GpuPersistentStorageStore::default();
    let mut reference = vec![0; 64];
    store
        .apply_snapshot(snapshot(target, 0, &reference))
        .unwrap();
    let mut state = 0x42_u32;
    for revision in 0..48 {
        state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let offset = usize::from((state % 16) as u8) * 4;
        let bytes = state.to_le_bytes();
        reference[offset..offset + 4].copy_from_slice(&bytes);
        store
            .apply_patch(
                GpuPersistentStoragePatch::replace(target, revision, revision + 1, offset, bytes)
                    .unwrap(),
            )
            .unwrap();
    }
    assert_eq!(store.entry(target).unwrap().bytes(), reference);
}

#[test]
fn layout_and_shadow_capacity_are_charged_by_allocated_capacity() {
    let target = target(1, 1);
    assert_eq!(
        GpuPersistentStorageSnapshot::new(target, 2, 64, 4, 0, [0; 4]),
        Err(GpuPersistentStorageError::InvalidElementStride)
    );
    assert_eq!(
        GpuPersistentStorageSnapshot::new(target, 4, 66, 4, 0, [0; 4]),
        Err(GpuPersistentStorageError::InvalidCapacity)
    );
    assert_eq!(
        GpuPersistentStoragePatch::replace(target, 0, 1, 2, [0; 4]),
        Err(GpuPersistentStorageError::InvalidPatchRange)
    );
    let mut store = GpuPersistentStorageStore::default();
    for index in 0..MAX_RESOURCES {
        let unique =
            GpuPersistentStorageTarget::new(index as u64, CanvasKey::new(index as u64), 1, 1);
        store
            .apply_snapshot(snapshot(unique, 0, &[0, 0, 0, 0]))
            .unwrap();
    }
    let extra = GpuPersistentStorageTarget::new(99, CanvasKey::new(99), 1, 1);
    assert_eq!(
        store.apply_snapshot(snapshot(extra, 0, &[0, 0, 0, 0])),
        Err(GpuPersistentStorageError::ResourceLimit)
    );
}
