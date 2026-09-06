# Worker-side custom shader preparation measurements

Measured fixture revision: `4c499ce3e3ab6c98822cbdfd56a4539222f4f2bc`.
Production preparation code remains the validated `312a771c` implementation;
subsequent changes add test fixtures and clarify their labels.

Three fresh native test processes ran on Apple M5 Pro / Metal. Each uses one
cold device builder, three identical-key full builders, four bounded distinct
WGSL variants, and one invalid module. All nine source hashes match the retained
historical phase workload in synchronous-shader-preparation for each run.
All eight valid calls returned Ready and the invalid module returned the typed
ShaderModule failure. Every preparation call ran on a thread distinct from its
submitting test thread.

Cold worker builder wall times were 2.199750, 1.290125 and 0.977750 ms.
The historical instrumented phase totals were 2.363875, 0.884250 and 1.137000 ms.
These small samples are not a speedup comparison: the historical fixture
instruments individual phase boundaries, while the new fixture measures the
actual complete preparation entry point. Driver/thread caching and measurement
boundaries differ. Repeated identical-key samples still run the full builder;
they are not retained Radiant cache hits.

`worker_elapsed_ns` surrounds only the actual builder call, before outcome
conversion and pipeline teardown. It is wall-clock time, not thread CPU
accounting. `spawn_join_elapsed_ns` is the enclosing interval including work,
channel transfer, teardown and join; it is not isolated scheduling overhead.
Device setup is reported separately. The submitting thread is a test thread,
not the native event loop.

All three opt-in native tests passed. Strict all-target/all-feature Clippy,
315 guardrails and formatting also passed at the measured revision. The JSON,
logs and source-hash comparison are retained here.

This proves the actual native preparation call can execute on the worker with
the intended validation outcome. It does not measure foreground input latency,
GPU duration, or the complete host task scheduler. OPT-1457 remains open until
its real foreground before/after acceptance is collected.
