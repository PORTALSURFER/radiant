# Persistent storage qualification

Source: `08fccffc4016109ab16b7fd40298f3c3ce958802`.
Base: `b45e74ada6248d5cba1540f2da267b9eb553189f`.
The offscreen device was Apple M5 Pro / Metal, using WGPU default limits.

| Operation | Observed storage upload bytes |
| --- | ---: |
| Initial allocation | 65,536 |
| Four-byte replacement | 4 |
| Unchanged frame | 0 |
| Fresh-device reconstruction | 65,536 |

The three retained 64 × 64 Rgba8Unorm buffers contain the complete output.
Initial pixels equal the CPU reference (64, 64, 64, 255); updated and recovered
pixels equal (191, 191, 191, 255) and are byte-identical. `comparison.json`
contains SHA-256 checksums. The native recorder's separately named
DefaultHasher values are diagnostic identities, not SHA-256 hashes.

The three native tests additionally verify an append plus overlapping edits
coalescing to eight bytes, replay after an unsubmitted encoder is discarded,
full replay after release/recreation with the same external fence, three
visible ordered persistent/bulk/persistent occurrences sharing a surface key,
and same-device recovery after a submitted frame loses acknowledgement.
A conservative uncertain-submission retry uploads the full 65,536-byte shadow.

Validation: 251 GPU-surface tests at `61b0718c`, followed by 11 focused storage
tests, two public command/API tests, all 315 guardrails, strict all-target and
all-feature Clippy, no-default-features library check, formatting, and three
native tests at the final source revision. The later production change only
replaces private panic shortcuts with typed error handling. The ordered native
fixture was corrected to expect its middle bulk region's distinct color.

These are logical transfer counts and offscreen correctness observations.
They do not establish input latency, GPU duration, physical driver residency,
or foreground host acceptance. The fixed CPU shadow and journal limits are
specified in `docs/PERSISTENT_SHADER_STORAGE.md`; general renderer budgets
remain separate work.
