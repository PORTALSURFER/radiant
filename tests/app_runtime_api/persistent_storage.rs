use super::*;
use radiant::runtime::{
    GpuPersistentStorageError as Error, GpuPersistentStoragePatch as Patch,
    GpuPersistentStorageSnapshot as Snapshot, GpuPersistentStorageStatus as Status,
    GpuPersistentStorageTarget as Target, GpuPersistentStorageUpdate as Update,
};

type Admission = Result<Option<Status>, Error>;

#[derive(Default)]
struct StorageBridge {
    admissions: Vec<Admission>,
    projections: usize,
}

impl RuntimeBridge<Admission> for StorageBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<Admission>> {
        self.projections += 1;
        arc_surface(UiSurface::new(SurfaceNode::static_widget(TextWidget::new(
            1,
            "storage",
            WidgetSizing::fixed(Vector2::new(80.0, 20.0)),
        ))))
    }

    fn update(&mut self, admission: Admission) -> Command<Admission> {
        self.admissions.push(admission);
        Command::request_paint_only()
    }
}

#[test]
fn persistent_storage_reports_ordered_admission_without_reprojection() {
    let mut runtime = SurfaceRuntime::new(StorageBridge::default(), Vector2::new(100.0, 40.0));
    let target = Target::new(1, 7, 11, 1);
    let snapshot = Snapshot::new(target, 4, 64, 4, 0, [1, 2, 3, 4]).unwrap();
    let first = runtime.execute_command(Command::update_gpu_persistent_storage(
        Update::Snapshot(snapshot),
        |result| result,
    ));
    assert!(first.paint_only_requested);
    assert_eq!(
        runtime.bridge().admissions,
        [Ok(Some(Status::Ready { revision: 0 }))]
    );
    assert_eq!(runtime.bridge().projections, 1);

    let patch = Patch::append(target, 0, 1, [5, 6, 7, 8]).unwrap();
    runtime.execute_command(Command::update_gpu_persistent_storage(
        Update::Patch(patch.clone()),
        |result| result,
    ));
    runtime.execute_command(Command::update_gpu_persistent_storage(
        Update::Patch(patch),
        |result| result,
    ));
    assert_eq!(runtime.bridge().admissions.len(), 3);
    assert_eq!(
        runtime.bridge().admissions[1],
        Ok(Some(Status::Ready { revision: 1 }))
    );
    assert_eq!(runtime.bridge().admissions[2], Err(Error::StalePatch));
    assert_eq!(
        runtime.gpu_persistent_storage_status(target),
        Some(Status::Ready { revision: 1 })
    );
    assert_eq!(runtime.bridge().projections, 1);

    runtime.execute_command(Command::update_gpu_persistent_storage(
        Update::Release(target),
        |result| result,
    ));
    assert_eq!(runtime.bridge().admissions.last(), Some(&Ok(None)));
    assert_eq!(runtime.gpu_persistent_storage_status(target), None);
}

#[test]
fn persistent_storage_gap_is_reported_and_repaired_by_snapshot() {
    let mut runtime = SurfaceRuntime::new(StorageBridge::default(), Vector2::new(100.0, 40.0));
    let target = Target::new(1, 7, 11, 1);
    runtime.execute_command(Command::update_gpu_persistent_storage(
        Update::Snapshot(Snapshot::new(target, 4, 64, 4, 0, [0; 4]).unwrap()),
        |r| r,
    ));
    runtime.execute_command(Command::update_gpu_persistent_storage(
        Update::Patch(Patch::replace(target, 2, 3, 0, [1; 4]).unwrap()),
        |r| r,
    ));
    assert_eq!(
        runtime.gpu_persistent_storage_status(target),
        Some(Status::NeedsSnapshot { revision: 0 })
    );
    runtime.execute_command(Command::update_gpu_persistent_storage(
        Update::Snapshot(Snapshot::new(target, 4, 64, 4, 3, [2; 4]).unwrap()),
        |r| r,
    ));
    assert_eq!(
        runtime.gpu_persistent_storage_status(target),
        Some(Status::Ready { revision: 3 })
    );
    assert_eq!(runtime.bridge().admissions.len(), 3);
}
