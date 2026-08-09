# Radiant Target Alignment

| Overall measure | Estimate |
| --- | ---: |
| Generic architecture-sequence completion | ~97% (92–99%, medium confidence) |
| Broad end-to-end target coverage | ~77.00% (847 / 11) |

| Category | Estimate |
| --- | ---: |
| Public API and module boundaries | 82% |
| Declarative model, identity, reconciliation | 70% |
| Input, provenance, and edit lifecycle | 86% |
| Layout, composition, virtualization | 96% |
| Text, focus, and selection | 66% |
| Numeric controls | 61% |
| Runtime, effects, and scheduling | 96% |
| Rendering, invalidation, retained GPU surfaces | 78% |
| Platform, windowing, and host boundaries | 70% |
| Diagnostics, profiling, and performance validation | 66% |
| Examples, documentation, and CI guardrails | 76% |

The broad estimate is the unweighted mean of the category rows:
`(82 + 70 + 86 + 96 + 66 + 61 + 96 + 78 + 70 + 66 + 76) / 11 = 77.00%`.
The generic architecture-sequence estimate remains about 97%; this consumer
adds executable evidence without claiming completion of the remaining
consumer-, platform-, scheduler-, renderer-, or product-policy boundaries.

The current numeric evidence is a public generic `numeric_input(value, codec,
adjustment)` builder with typed construction failures and a fixed-capacity
`Begin` plus terminal edit batch. It formats the initial value through the
application codec, validates the adjustment inverse, preserves verbatim drafts,
caches draft classification for the synchronous allocation-free focus-loss
veto seam, commits valid Enter/focus-loss edits, cancels active Escape edits,
and retains draft/caret/session state only for an actually active edit during
same-ID reprojection. Qualified exports remain outside the common prelude.
Semantic keyboard adjustment is contract-defined but unimplemented. Pointer/
wheel scrubbing, IME, accessibility, Slider/Knob, platform, scheduler,
renderer, and product policy remain out of scope for this slice.
