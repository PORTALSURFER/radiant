# Bounded signal detail evidence — OPT-1458

The offscreen Metal fixtures ran from compiled source
`adc50e2ff2aa5f0eb70be2d13215320c8fa1154a` on Apple M5 Pro. They create no
foreground window. The later `c2e6e954` change updates an architecture guard to
require bounded worker construction; it does not change production rendering.
The subsequent parent-branch refresh adds previously retained evidence only.

## Observed results

- 141 signal tests passed, with five native tests explicitly ignored by that run.
- Those five native tests then passed separately: three bounded/source-retirement
  fixtures and two precise-window reference fixtures.
- Strict all-target/all-feature Clippy, 315 architecture guardrails, and formatting
  passed. Logs retain the revision distinctions above.
- A 65,536-frame one-band source renders a coarse overview, then a completed
  detail tile. The detail pixels exactly match the legacy full-pyramid rendering;
  the overview and detail images differ, so this exercises a real detail transition.
- A nearby pan reuses the immutable GPU tile buffer with zero immutable upload
  operations. The real upload-plan path also verifies warm reuse after frame reset.
- Source-end pixels match the legacy reference. A new actual WGPU device and queue
  reconstruct the same pixels from retained CPU detail without another worker job.
- Under a calibrated GPU budget, a replacement uses committed incomplete fallback,
  retires stale ownership, then succeeds on retry within the same budget. The older
  source-replacement fixture also verifies CPU leases retire with all GPU owners.
- All 23 precise-window buffers match the previously merged OPT-1454 reference
  byte-for-byte, including origins 0, 2^24, and 2^40, gain, slide, pan, and sub-bucket zoom.

`comparison.json` contains SHA-256 checksums for all 26 retained RGBA buffers.
They are 64 × 64 `Rgba8Unorm`, 16,384 bytes each. The native recorder's separately
labelled `DefaultHasher` values are auxiliary within-build checks, not SHA-256.

## Memory and upload interpretation

The measured close-view state retains 262,144 logical source bytes, 131,048 CPU
summary/detail bytes, and 82,304 logical GPU bytes. For this exact source shape,
the legacy full-pyramid topology contains 131,071 eight-byte buckets, or 1,048,568
summary bytes. That legacy number is a topology calculation, not a process-RSS
measurement. The CPU summary reduction excludes the unchanged raw source allocation.

The first coarse upload is 32 bytes; the first complete detail tile uploads 65,520
bytes. Retaining a full detail page enables the observed zero-upload nearby pan,
but does not establish lower cold-upload cost than a viewport-only buffer.

The independent production limits are 256 MiB retained raw sources, 16 MiB
coarse overviews within a 64 MiB CPU summary/detail pool, and 128 MiB logical
signal GPU residency. Active jobs, queued jobs, retained sources, target interests,
and tile products are separately bounded. CPU cancellation and retained-owner
pressure cases are covered by the focused tests.

These captures establish reference pixels, logical accounting, and bounded
admission behavior. They do not establish foreground latency, GPU duration,
physical driver residency, or a speedup. Fence-safe physical reclamation remains
part of OPT-1374; the broader foreground comparison remains in OPT-1452.
