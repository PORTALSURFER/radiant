//! Bounded detail products. Each product pins its already-accounted source overview.
use super::*;
use crate::runtime::{BoundedSignalError, BoundedSignalTileRequest, build_bounded_tile};

const MAX_TILES: usize = 192;
const MAX_TILE_BYTES: usize = 256 * 1024;
const MAX_RECENT_PER_SOURCE: usize = 8;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(super) struct TileSpec {
    first: usize,
    width: usize,
    count: usize,
    wrap: bool,
}

impl TileSpec {
    /// Quantized pages keep nearby viewports on the same immutable product.
    pub(super) fn for_view(
        frames: usize,
        bands: usize,
        range: [f32; 2],
        slide: i64,
        overview_width: usize,
    ) -> Option<Self> {
        if frames == 0 || bands == 0 || overview_width <= 1 {
            return None;
        }
        let start = f64::from(range[0]);
        let span = f64::from(range[1]) - start;
        if !start.is_finite()
            || !span.is_finite()
            || start < 0.0
            || span <= 0.0
            || span > frames as f64
        {
            return None;
        }
        let capacity = (MAX_TILE_BYTES
            / std::mem::size_of::<crate::runtime::GpuSignalSummaryBucket>()
            / bands)
            .min(8192);
        // Reserve room for page quantization and interpolation guards.
        let columns = capacity.checked_div(2)?.checked_sub(2)?;
        if columns == 0 {
            return None;
        }
        let wanted = (span / columns as f64).ceil().max(1.0) as usize;
        let width = wanted.checked_next_power_of_two()?;
        if width >= overview_width {
            return None;
        }
        let integral = start.floor();
        let physical = ((integral as i128 - i128::from(slide)).rem_euclid(frames as i128)) as usize;
        let page = columns.checked_mul(width)?;
        let first = (physical / page * page).saturating_sub(width);
        // Use a fixed two-page extent, not the exact viewport end; otherwise
        // one-frame pans would manufacture a new product key on every request.
        let count = columns.checked_mul(2)?.checked_add(2)?;
        let wrap = physical as f64 + (start - integral) + span > frames as f64;
        let count = if wrap {
            count
        } else {
            count.min((frames - first).div_ceil(width))
        };
        if count == 0 || count > capacity {
            return None;
        }
        first.checked_add(count.checked_mul(width)?)?;
        Some(Self {
            first,
            width,
            count,
            wrap,
        })
    }

    fn bytes(self, bands: usize) -> Option<usize> {
        self.count
            .checked_mul(bands)?
            .checked_mul(std::mem::size_of::<crate::runtime::GpuSignalSummaryBucket>())
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct TileKey {
    source: SourceKey,
    spec: TileSpec,
}

enum TileState {
    Queued,
    Active {
        id: u64,
        cancel: Arc<AtomicBool>,
    },
    Ready {
        tile: Arc<BoundedSignalTile>,
        token: Arc<RetentionToken>,
    },
    Failed,
}
struct TileEntry {
    owner: PreparedSummary,
    bytes: usize,
    interests: usize,
    last_used: u64,
    retired: bool,
    state: TileState,
}
struct TileCompletion {
    key: TileKey,
    id: u64,
    result: Result<BoundedSignalTile, BoundedSignalError>,
}

pub(in crate::gui_runtime::native_vello::generic_runtime) struct TileDispatch {
    id: u64,
    key: TileKey,
    owner: PreparedSummary,
    cancel: Arc<AtomicBool>,
    sender: SyncSender<TileCompletion>,
    wake: Arc<dyn RepaintSignal>,
}
impl TileDispatch {
    pub(super) fn id(&self) -> u64 {
        self.id
    }
    pub(super) fn run(self) {
        let started = std::time::Instant::now();
        let spec = self.key.spec;
        let result = catch_unwind(AssertUnwindSafe(|| {
            build_bounded_tile(
                &self.owner._source,
                self.key.source.effective_frames,
                self.key.source.effective_bands,
                BoundedSignalTileRequest {
                    first_frame: spec.first,
                    bucket_frames: spec.width,
                    bucket_count: spec.count,
                    wrap: spec.wrap,
                },
                || self.cancel.load(Ordering::Acquire),
            )
        }))
        .unwrap_or(Err(BoundedSignalError::InvalidShape));
        if tracing::enabled!(target: "radiant::signal_summary_prepare", tracing::Level::DEBUG) {
            let (state, ready_tile_bytes) = match &result {
                Ok(tile) => (
                    "ready",
                    tile.buckets.len()
                        * std::mem::size_of::<crate::runtime::GpuSignalSummaryBucket>(),
                ),
                Err(BoundedSignalError::Cancelled) => ("cancelled", 0),
                Err(_) => ("failed", 0),
            };
            tracing::debug!(target: "radiant::signal_summary_prepare",
                job = self.id, result = state,
                preparation_elapsed_us = started.elapsed().as_micros() as u64,
                first_frame = spec.first, bucket_frames = spec.width,
                bucket_count = spec.count, ready_tile_bytes,
                "raw signal detail worker terminal");
        }
        // The source owner stays alive until the terminal is enqueued.
        let _ = self.sender.send(TileCompletion {
            key: self.key,
            id: self.id,
            result,
        });
        drop(self.owner);
        self.wake.request_repaint();
    }
}

pub(super) struct TileCache {
    entries: HashMap<TileKey, TileEntry>,
    interests: HashMap<SummaryTargetId, TileKey>,
    queue: VecDeque<TileKey>,
    sender: SyncSender<TileCompletion>,
    receiver: Receiver<TileCompletion>,
    wake: Arc<dyn RepaintSignal>,
    clock: u64,
    pub(super) active: usize,
    pub(super) bytes: usize,
}
impl TileCache {
    pub(super) fn new(wake: Arc<dyn RepaintSignal>) -> Self {
        let (sender, receiver) = sync_channel(MAX_ACTIVE);
        Self {
            entries: HashMap::new(),
            interests: HashMap::new(),
            queue: VecDeque::new(),
            sender,
            receiver,
            wake,
            clock: 0,
            active: 0,
            bytes: 0,
        }
    }
    pub(super) fn queued(&self) -> usize {
        self.queue.len()
    }
    pub(super) fn len(&self) -> usize {
        self.entries.len()
    }

    /// False leaves the caller's bounded, payload-free demand available for retry.
    pub(super) fn request(
        &mut self,
        target: SummaryTargetId,
        mut owner: PreparedSummary,
        spec: TileSpec,
        queue_slots: usize,
        max_bytes: usize,
    ) -> bool {
        let key = TileKey {
            source: owner.key,
            spec,
        };
        if self.interests.get(&target) == Some(&key) {
            return true;
        }
        self.release(target);
        self.clock = self.clock.saturating_add(1);
        if let Some(entry) = self.entries.get_mut(&key) {
            if entry.retired {
                let TileState::Ready { token, .. } = &entry.state else {
                    return false;
                };
                // A retained immutable product can regain interest without
                // another allocation; cancelled active work still awaits terminal.
                entry.retired = false;
                token.retired.store(false, Ordering::Release);
            }
            entry.interests += 1;
            entry.last_used = self.clock;
            self.interests.insert(target, key);
            return true;
        }
        let Some(bytes) = spec
            .bytes(key.source.effective_bands)
            .filter(|b| *b <= MAX_TILE_BYTES)
        else {
            return false;
        };
        if queue_slots == 0 {
            return false;
        }
        self.maintain();
        self.make_room(bytes, max_bytes);
        if self.entries.len() >= MAX_TILES
            || self
                .bytes
                .checked_add(bytes)
                .is_none_or(|total| total > max_bytes)
            || queue_slots == 0
        {
            return false;
        }
        // Never retain another tile through a source owner: that would form a chain.
        owner.tile = None;
        owner._tile_lease = None;
        self.entries.insert(
            key,
            TileEntry {
                owner,
                bytes,
                interests: 1,
                last_used: self.clock,
                retired: false,
                state: TileState::Queued,
            },
        );
        self.interests.insert(target, key);
        self.queue.push_back(key);
        self.bytes += bytes;
        true
    }

    pub(super) fn attach(&self, target: SummaryTargetId, owner: &mut PreparedSummary) {
        let Some(key) = self
            .interests
            .get(&target)
            .filter(|key| key.source == owner.key)
        else {
            return;
        };
        let Some(TileEntry {
            state: TileState::Ready { tile, token },
            retired: false,
            ..
        }) = self.entries.get(key)
        else {
            return;
        };
        owner.tile = Some(Arc::clone(tile));
        owner._tile_lease = Some(SummaryRetentionLease(Some(Arc::clone(token))));
    }

    pub(super) fn dispatch(&mut self, id: u64) -> Option<TileDispatch> {
        while let Some(key) = self.queue.pop_front() {
            let Some(entry) = self.entries.get_mut(&key) else {
                continue;
            };
            if entry.retired || entry.interests == 0 || !matches!(entry.state, TileState::Queued) {
                continue;
            }
            let cancel = Arc::new(AtomicBool::new(false));
            entry.state = TileState::Active {
                id,
                cancel: Arc::clone(&cancel),
            };
            self.active += 1;
            return Some(TileDispatch {
                id,
                key,
                owner: entry.owner.clone(),
                cancel,
                sender: self.sender.clone(),
                wake: Arc::clone(&self.wake),
            });
        }
        None
    }

    pub(super) fn reject(&mut self, id: u64) -> bool {
        for entry in self.entries.values_mut() {
            if matches!(entry.state, TileState::Active { id: active, .. } if active == id) {
                entry.state = TileState::Failed;
                self.active -= 1;
                return true;
            }
        }
        false
    }

    pub(super) fn drain(&mut self) -> Vec<SummaryTargetId> {
        let mut notify = Vec::new();
        while let Ok(completion) = self.receiver.try_recv() {
            let Some(entry) = self.entries.get_mut(&completion.key) else {
                continue;
            };
            if !matches!(entry.state, TileState::Active { id, .. } if id == completion.id) {
                continue;
            }
            self.active -= 1;
            entry.state = match completion.result {
                Ok(tile) => TileState::Ready {
                    tile: Arc::new(tile),
                    token: Arc::new(RetentionToken {
                        wake: Arc::clone(&self.wake),
                        retired: Arc::new(AtomicBool::new(entry.retired)),
                    }),
                },
                Err(_) => TileState::Failed,
            };
            notify.extend(
                self.interests
                    .iter()
                    .filter_map(|(target, key)| (*key == completion.key).then_some(*target)),
            );
        }
        self.maintain();
        notify
    }

    pub(super) fn release(&mut self, target: SummaryTargetId) {
        let Some(key) = self.interests.remove(&target) else {
            return;
        };
        let Some(entry) = self.entries.get_mut(&key) else {
            return;
        };
        entry.interests -= 1;
        if entry.interests == 0 && !matches!(entry.state, TileState::Ready { .. }) {
            Self::retire(entry);
            self.queue.retain(|queued| *queued != key);
        }
    }

    pub(super) fn retire_source(&mut self, source: SourceKey) {
        for (key, entry) in &mut self.entries {
            if key.source == source {
                Self::retire(entry);
            }
        }
        self.queue.retain(|key| key.source != source);
    }
    fn retire(entry: &mut TileEntry) {
        entry.retired = true;
        match &entry.state {
            TileState::Active { cancel, .. } => cancel.store(true, Ordering::Release),
            TileState::Ready { token, .. } => token.retired.store(true, Ordering::Release),
            _ => {}
        }
    }

    fn make_room(&mut self, incoming: usize, max_bytes: usize) {
        let mut candidates: Vec<_> = self
            .entries
            .iter()
            .filter(|(_, entry)| entry.interests == 0 && !entry.retired)
            .map(|(key, entry)| (*key, entry.last_used))
            .collect();
        candidates.sort_unstable_by_key(|(_, age)| *age);
        for (key, _) in candidates {
            if self.entries.len() < MAX_TILES
                && self
                    .bytes
                    .checked_add(incoming)
                    .is_some_and(|total| total <= max_bytes)
            {
                break;
            }
            if let Some(entry) = self.entries.get_mut(&key) {
                Self::retire(entry);
            }
            self.maintain();
        }
    }

    pub(super) fn maintain(&mut self) {
        // Keep a small recent set per live source. Retained GPU leases remain charged.
        let mut recent: HashMap<SourceKey, Vec<(TileKey, u64)>> = HashMap::new();
        for (key, entry) in &self.entries {
            if entry.interests == 0 && !entry.retired {
                recent
                    .entry(key.source)
                    .or_default()
                    .push((*key, entry.last_used));
            }
        }
        for entries in recent.values_mut() {
            entries.sort_unstable_by_key(|(_, age)| std::cmp::Reverse(*age));
            for (key, _) in entries.iter().skip(MAX_RECENT_PER_SOURCE) {
                if let Some(entry) = self.entries.get_mut(key) {
                    Self::retire(entry);
                }
            }
        }
        self.entries.retain(|_, entry| {
            let removable = entry.retired
                && match &entry.state {
                    TileState::Active { .. } => false,
                    TileState::Ready { token, .. } => Arc::strong_count(token) == 1,
                    _ => true,
                };
            if removable {
                self.bytes -= entry.bytes;
            }
            !removable
        });
    }
}

impl Drop for TileCache {
    fn drop(&mut self) {
        for entry in self.entries.values() {
            if let TileState::Active { cancel, .. } = &entry.state {
                cancel.store(true, Ordering::Release);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Wake;
    impl RepaintSignal for Wake {
        fn request_repaint(&self) {}
    }
    fn target() -> SummaryTargetId {
        SummaryTargetId::new(
            WindowId::dummy(),
            NativeAdapterGeneration::from_test_serial(1),
            NativeTargetGeneration::from_test_serial(1),
            1,
        )
        .unwrap()
    }
    fn owner() -> PreparedSummary {
        let samples: Arc<[f32]> = (0..16384).map(|i| (i % 17) as f32 / 17.0).collect();
        let request = SummaryRequest::new(samples, 16384, 1, 1);
        let overview =
            crate::runtime::build_bounded_overview(&request.samples, 16384, 1, || false).unwrap();
        PreparedSummary {
            key: request.key,
            _source: request.samples,
            summary: Arc::new(GpuSignalSummary {
                frames: overview.frames,
                band_count: overview.band_count,
                levels: overview.levels,
            }),
            _lease: SummaryRetentionLease(Some(Arc::new(RetentionToken {
                wake: Arc::new(Wake),
                retired: Arc::new(AtomicBool::new(false)),
            }))),
            tile: None,
            _tile_lease: None,
            gpu_budget: Arc::new(SignalGpuBudget::default()),
        }
    }
    fn spec(first: usize) -> TileSpec {
        TileSpec {
            first,
            width: 1,
            count: 16,
            wrap: false,
        }
    }
    #[test]
    fn coalesced_tile_keeps_source_and_retired_bytes_until_last_owner_drops() {
        let mut cache = TileCache::new(Arc::new(Wake));
        let source = owner();
        let a = target();
        let b = target();
        assert!(cache.request(a, source.clone(), spec(0), 8, 1024));
        assert!(cache.request(b, source.clone(), spec(0), 8, 1024));
        assert_eq!(cache.len(), 1);
        cache.dispatch(1).unwrap().run();
        assert_eq!(cache.active, 1);
        assert_eq!(cache.drain().len(), 2);
        let mut retained = source.clone();
        cache.attach(a, &mut retained);
        assert!(retained.tile().is_some());
        cache.release(a);
        cache.release(b);
        cache.retire_source(source.key);
        cache.maintain();
        assert_eq!(cache.bytes, 128);
        drop(retained);
        cache.maintain();
        assert_eq!(cache.bytes, 0);
    }
    #[test]
    fn returning_interest_reuses_retired_detail_still_held_by_renderer() {
        let mut cache = TileCache::new(Arc::new(Wake));
        let source = owner();
        let a = target();
        assert!(cache.request(a, source.clone(), spec(0), 8, 1024));
        cache.dispatch(1).unwrap().run();
        cache.drain();
        let mut previous = source.clone();
        cache.attach(a, &mut previous);
        cache.release(a);
        cache.retire_source(source.key);
        cache.maintain();
        let b = target();
        assert!(cache.request(b, source.clone(), spec(0), 0, 0));
        let mut current = source;
        cache.attach(b, &mut current);
        assert_eq!(previous.asset_key(), current.asset_key());
        assert!(cache.dispatch(2).is_none());
        assert_eq!(cache.bytes, 128);
    }

    #[test]
    fn cancelled_active_reservation_is_released_only_after_terminal() {
        let mut cache = TileCache::new(Arc::new(Wake));
        let source = owner();
        let a = target();
        assert!(cache.request(a, source.clone(), spec(0), 8, 1024));
        let dispatch = cache.dispatch(1).unwrap();
        cache.release(a);
        cache.maintain();
        assert_eq!(cache.bytes, 128);
        assert_eq!(cache.active, 1);
        dispatch.run();
        cache.drain();
        assert_eq!(cache.bytes, 0);
        assert_eq!(cache.active, 0);
        assert!(cache.request(a, source, spec(0), 8, 1024));
    }
    #[test]
    fn pressure_evicts_recent_unreferenced_tile_but_cannot_discard_gpu_lease() {
        let mut cache = TileCache::new(Arc::new(Wake));
        let source = owner();
        let a = target();
        assert!(cache.request(a, source.clone(), spec(0), 8, 128));
        cache.dispatch(1).unwrap().run();
        cache.drain();
        let mut retained = source.clone();
        cache.attach(a, &mut retained);
        cache.release(a);
        assert!(!cache.request(a, source.clone(), spec(16), 8, 128));
        assert_eq!(cache.bytes, 128);
        drop(retained);
        assert!(cache.request(a, source, spec(16), 8, 128));
        assert_eq!(cache.len(), 1);
    }
    #[test]
    fn detail_picker_handles_extreme_slide_and_reuses_nearby_page() {
        let a = TileSpec::for_view(100_000, 4, [10.25, 50.25], i64::MIN, 32).unwrap();
        assert!(a.count <= 8192);
        let physical = ((10_i128 - i128::from(i64::MIN)).rem_euclid(100_000)) as usize;
        assert!(a.first <= physical);
        assert!(a.first + a.count * a.width >= physical + 41);
        let b = TileSpec::for_view(100_000, 4, [11.25, 51.25], i64::MIN, 32).unwrap();
        assert_eq!(a, b);
    }
}
