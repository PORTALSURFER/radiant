# Immutable text subtree clone evidence

Three independent release harness runs compare the unchanged 10,000-text-leaf
fixture at `ac7222355ed3a96c87cda71c62276bb91511ea34` with private shared child
storage at `a3dbf03dc63e329ef488bb0ef4d1b65b2385bc00`. Each sealed JSON pack
contains original metric rows, a digest, and matching machine/build/fixture
qualification. Each comparison selects average, p95, and p99 with a 5% tolerance.
All three clone comparisons pass.

Before-change averages span 585.290–729.800 µs; after-change averages span
0.030–0.040 µs. The optimized result is near the harness timer's floor: this
supports removal of per-leaf clone work in this fixture, not a precise speedup
ratio or an application frame latency claim. Percentiles describe batches of
eight operations, not individual native frames. No GPU or scanout was measured.

Only concrete built-in text-only subtrees qualify for sharing. Arbitrary custom
widgets preserve deep cloning and observable Clone behavior. A mutable child
borrow detaches the shared storage and conservatively retires eligibility,
preserving snapshot isolation if the caller inserts a custom widget. Tests cover
nested mutation, owned iteration, custom clone hooks and interior state, and
replacement of a previously eligible text child.

This is an incremental reconciliation prerequisite. It does not establish
component callback non-visitation, bounded layout, or native frame acceptance.

## Existing refresh controls

`refresh-controls/` retains three additional runs per revision of the existing
surface scene, refresh, and projection-refresh scenarios. Measurements used the
same shared build cache, explicitly rebuilding the library and benchmark after
switching worktrees; the cloned-tree sentinel confirmed the expected revision.
The verified candidate controls preceded the verified baseline controls. No
build or test ran concurrently with measurement.

The ordinary refresh averages overlap (before 339–392 µs, after 343–381 µs), as
do projection refresh averages (before 336–390 µs, after 339–354 µs). Paired 5%
tail/average gates fail in run 1 and pass in runs 2 and 3. These noisy controls
are inconclusive for a general refresh performance claim; all outcomes are
retained, including failures. The optimization claim is limited to immutable
text-tree cloning. Work counters remain unchanged across each scenario.
