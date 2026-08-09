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
adjustment)` builder with typed construction failures and the shipped
fixed-capacity `NumericInputEditBatch<T>` bounded incremental carrier. The
carrier accepts exactly `[Update]`, `[Commit]`, `[Cancel]`, `[Begin, Update]`,
`[Begin, Commit]`, and `[Begin, Cancel]` in private inline capacity-two
storage. The shipped text-first widget still emits only `[Begin, Commit]` and
`[Begin, Cancel]`; this carrier is storage and shape validation foundation
only. The consumer formats the initial value through the application codec,
validates the adjustment inverse, preserves verbatim drafts, caches draft
classification for the synchronous allocation-free focus-loss veto seam,
commits valid Enter/focus-loss edits, cancels active Escape edits, and retains
draft/caret/session state only for an actually active edit during same-ID
reprojection. Qualified exports remain outside the common prelude.
The qualified `NumericStepAttempt`, `NumericInputInteraction<T, StepError,
FormatError>`, and `NumericInputInteractionBatch<T, StepError, FormatError>`
are now shipped as a fixed-capacity keyboard output-envelope foundation. The
batch validates successful keyboard edit fragments and typed initial or
rollback-before-repeat failures, but it is behaviorally unconsumed: no current
widget or runtime produces or consumes these parts, and semantic keyboard
adjustment remains unimplemented. This public storage/validation foundation has
zero impact on the estimates.
The crate-private shared numeric interaction gate is now shipped for TextEdit
admission, no-op cleanup, terminal cleanup, and compatible active reprojection.
Normalized `KeyRelease` plumbing is now shipped across native input, runtime
events, and focused widget dispatch; semantic keyboard adjustment remains
contract-defined but unimplemented. The
backend-neutral IME/composition lifecycle is now defined but unimplemented; the
current source has no composition event or state. Pointer scrubbing is
contract-defined but unimplemented. Wheel adjustment is now contract-defined but
unimplemented. Numeric accessibility actions are contract-defined but
unimplemented. Slider/Knob, platform, scheduler, renderer, and product policy
remain out of scope for this slice.
The five other shared-gate consumers—IME/composition, keyboard adjustment,
pointer, wheel, and accessibility—remain target-only and unimplemented.
The public `KeyboardModifier`/`NumericStepModifiers` selector and
`NumericInputBuilder::step_modifiers(...)` attachment are now shipped as a
pure configuration foundation. The selector evaluates lossless
`KeyboardModifiers` samples without allocation or retained state; the widget
stores `None` when unconfigured or exactly `Some(policy)` when attached, but no
current producer or consumer reads it. Semantic stepping, adjustment calls,
capture, transactions, and numeric output remain unimplemented. This storage
foundation has zero impact on the estimates.
The native keyboard boundary now also ships a lossless widget-modifier
projection alongside the unchanged host-shortcut `KeyPress` projection:
Linux/Windows Control remains host `command` for shortcut resolution but reaches
an unhandled focused widget as `control`, while Super/Meta reaches it as
`command`; combined and Shift/Alt states remain independent across press,
repeat, and release. Host handling remains first, and handled shortcuts do not
reach widgets. This prerequisite correction has zero impact on the estimates
and does not ship numeric stepping, capture, transactions, or a numeric
consumer.
