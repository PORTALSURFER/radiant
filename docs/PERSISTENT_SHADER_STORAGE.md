# Persistent shader storage updates

Persistent storage is an opt-in companion to a custom shader's existing bulk
storage descriptor. It supports edits and appends without republishing a whole
buffer. Presentation uniforms remain replaceable snapshots; storage patches
are dependent data updates with explicit admission feedback.

## Identity and layout

`GpuPersistentStorageTarget` identifies a widget, canvas key, storage identity,
and storage generation. These match the painted widget/key and descriptor
`storage_identity`/`storage_revision`. The generation fences one allocation;
its dynamic data revision advances independently with each patch. Existing
`GpuShaderSurfaceDescriptor` and parts fields retain their meanings and shape.

Start with `GpuPersistentStorageSnapshot::new`. Specify a positive element
stride divisible by four, capacity, logical byte length, revision, and exactly
that many initial bytes. Capacity and logical length must be element aligned.
Unused capacity is zero initialized. The painted descriptor's storage length
must equal capacity. The shader may read allocated capacity; applications
publish logical length through their own ordinary uniforms when needed.

Each resource is limited to 16 MiB. A runtime retains at most 32 resources and
64 MiB of CPU shadows. These limits describe logical CPU storage, separately
from renderer resource budgets and physical driver residency. Resources remain
owned until explicit `Release(target)`, replacement, or runtime destruction.

## Ordered admission

Use `GpuPersistentStoragePatch::replace` for a range within logical length or
`append` to extend logical length within capacity. Each patch carries the exact
base revision and its checked successor. Payloads are nonempty, at most 256 KiB,
and element aligned. There is no hidden read-modify-write alignment expansion.

Queue `GpuPersistentStorageUpdate` through
`UiUpdateContext::update_gpu_persistent_storage` or
`Command::update_gpu_persistent_storage`. The required UI-local completion
mapper receives `Result<Option<GpuPersistentStorageStatus>,
GpuPersistentStorageError>`. This reports CPU admission, not GPU presentation.
A successful release returns `None`.

Stale patches do not change accepted contents. A missing base revision marks
the resource `NeedsSnapshot`; subsequent deltas cannot repair the gap. Supply
a newer full snapshot. Invalid ranges and capacity exhaustion leave existing
bytes intact. Patches admitted in order update the CPU shadow directly, and
later overlapping edits win. A new snapshot is an explicit replacement.

## Uploads and recovery

The CPU shadow retains at most 64 revision/range journal entries. A current GPU
cursor needs no storage upload. A lagging cursor within retained history uses
sorted, merged overlapping or touching ranges from the latest CPU contents.
Coalescing preserves final bytes without retaining copies of every patch.
A fresh binding, replaced snapshot, or cursor older than retained history uses
one full capacity upload. History eviction does not discard accepted data.
No frame scans all buffer contents to find differences.

Persistent range writes use encoder staging. A binding's pending revision is
acknowledged only after the whole native present succeeds. Failed or abandoned
frames retain the committed cursor for retry. A device rebuild starts without
GPU cursors and reconstructs from the runtime's retained CPU shadow. CPU state
is independent of adapter resources.

Changing the descriptor fence selects a different resource. An absent matching
persistent resource leaves the existing bulk descriptor path available. Storage
updates do not change shader language, expose native handles, or take ownership
of consumer audio queues.

## Evidence

Focused CPU, public API, native rendering, upload accounting, and recovery
validation are required before this implementation is considered complete.
Measured results and source revisions are recorded separately under
`evidence/performance/`; this document does not assert latency or memory-speedup
measurements.
