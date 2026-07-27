use crate::runtime::WorkerEffectSink;

pub(crate) trait CloseLatest {
    fn close_latest(&self);
}

impl CloseLatest for WorkerEffectSink {
    fn close_latest(&self) {
        WorkerEffectSink::close_latest(self);
    }
}

pub(crate) struct LatestStreamCloseGuard<S: CloseLatest> {
    sink: Option<S>,
}

impl<S: CloseLatest> LatestStreamCloseGuard<S> {
    pub(crate) fn new(sink: S) -> Self {
        Self { sink: Some(sink) }
    }

    pub(crate) fn close(mut self) {
        self.close_inner();
    }

    fn close_inner(&mut self) {
        if let Some(sink) = self.sink.take() {
            sink.close_latest();
        }
    }
}

impl<S: CloseLatest> Drop for LatestStreamCloseGuard<S> {
    fn drop(&mut self) {
        self.close_inner();
    }
}

#[cfg(test)]
mod tests {
    use super::LatestStreamCloseGuard;
    use crate::runtime::WorkerEffectSink;
    use std::any::Any;
    use std::{
        panic::{AssertUnwindSafe, catch_unwind},
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
    };

    #[test]
    fn latest_stream_close_guard_closes_when_work_unwinds() {
        let close_count = Arc::new(AtomicUsize::new(0));
        let close_count_for_sink = Arc::clone(&close_count);
        let sink = WorkerEffectSink::new_latest(
            |_: Box<dyn Any + Send>| true,
            |_: Box<dyn Any + Send>| true,
            move || {
                close_count_for_sink.fetch_add(1, Ordering::AcqRel);
            },
        );

        let result = catch_unwind(AssertUnwindSafe(|| {
            let _guard = LatestStreamCloseGuard::new(sink);
            panic!("stream work failed");
        }));

        assert!(result.is_err());
        assert_eq!(close_count.load(Ordering::Acquire), 1);
    }

    #[test]
    fn latest_stream_close_guard_explicit_close_is_not_repeated_on_drop() {
        let close_count = Arc::new(AtomicUsize::new(0));
        let close_count_for_sink = Arc::clone(&close_count);
        let sink = WorkerEffectSink::new_latest(
            |_: Box<dyn Any + Send>| true,
            |_: Box<dyn Any + Send>| true,
            move || {
                close_count_for_sink.fetch_add(1, Ordering::AcqRel);
            },
        );

        LatestStreamCloseGuard::new(sink).close();

        assert_eq!(close_count.load(Ordering::Acquire), 1);
    }
}
