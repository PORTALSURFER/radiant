# Precise signal viewport reference evidence

OPT-1454 uses additive exact-origin CPU coordinates and a bounded summary-window
producer through the existing CustomShader content variant. The legacy signal
API and its exhaustive content enum remain source compatible.

Native capture revision: `1463a4056b67ca075f3172b9fdf488ed7194f7b9`.
Final source revision: `d954bd13` changes only the equivalent range-containment
expression in the legacy f32 adapter after capture. No shader or precise-window
rendering code changed after capture.

Observed adapter: Apple M5 Pro, Metal. Each reference is an actual 64x64
Rgba8Unorm offscreen render, with 16,384 tightly packed bytes. Per-image JSON
retains adapter, source revision and format. `comparison.json` adds SHA-256
hashes and the comparison inventory. These captures do not measure foreground
input latency, GPU duration or throughput.

All required comparisons are byte-for-byte identical, with no tolerance:

- Six scenarios at origins 0, 2^24 and 2^40: fractional base view, adjacent pan,
  anchored fractional zoom, positive slide, negative slide, and a gain/fade
  selection partially overlapping the viewport.
- The precise near base, current legacy path, and frozen pre-extraction legacy
  shader produce identical pixels.
- A four-frame bucket with a two-frame viewport matches both its far-origin
  equivalent and the legacy smoothing reference.

The immutable fixture has 64 buckets and four bands. Far fixtures declare a
logical source of `origin + 64` frames and retain only the nearby window; they
never allocate a 2^40-frame source. Every non-base scenario is also asserted to
change the synthetic output. There are 23 retained pixel buffers in total.

The frozen shader is the header/summary/fragment concatenation from main
`26102a7b04cfcd33ca89efd82ff77a540bc3d12f`, plus one trailing blank line.
It is rendered with the original signal binding layout and pipeline settings.
Its file SHA-256 is
`c40708c73d8c53d4a83fb780bc14e542b560ed8d19d611e33357aa73eb453d65`.

Validation:

| Check | Revision | Result |
| --- | --- | --- |
| Native references | 1463a405 | 2 passed |
| Broader signal suite | d954bd13 | 91 passed, 2 opt-in tests ignored |
| Strict all-target/all-feature Clippy | d954bd13 | Passed |
| Architecture/source guardrails | d954bd13 | 315 passed |
| Formatting | d954bd13 | Passed |
| Public precise_signal_window example | aa5d1ad4 | Passed; exact hit 1099511627792 + 0.25 |

The broader final signal suite includes the numeric and bounded-window tests.
Those cover malformed/overflowing positions, integer translation, missing
windows, modulo slide, truncated buckets, local gain arithmetic, and shared
immutable allocation identity. Independent CPU and shader reviews found no
remaining correctness issue. CI supplies the wider public/example/doc and
portable compilation checks before merge.

This is the focused old/new rendering baseline for the precision correctness
change. OPT-1452's broader foreground cold/warm performance pack remains a
separate unfinished work item; these pixel comparisons do not substitute for it.
