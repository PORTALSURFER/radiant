# Radiant Target Alignment

| Overall measure | Estimate |
| --- | ---: |
| Generic architecture-sequence completion | ~97% (92–99%, medium confidence) |
| Broad end-to-end target coverage | ~78.18% (860 / 11) |

| Category | Estimate |
| --- | ---: |
| Public API and module boundaries | 83% |
| Declarative model, identity, reconciliation | 70% |
| Input, provenance, and edit lifecycle | 90% |
| Layout, composition, virtualization | 96% |
| Text, focus, and selection | 68% |
| Numeric controls | 67% |
| Runtime, effects, and scheduling | 96% |
| Rendering, invalidation, retained GPU surfaces | 78% |
| Platform, windowing, and host boundaries | 70% |
| Diagnostics, profiling, and performance validation | 66% |
| Examples, documentation, and CI guardrails | 76% |

The broad estimate is the unweighted mean of the category rows:
`(83 + 70 + 90 + 96 + 68 + 67 + 96 + 78 + 70 + 66 + 76) / 11 = 78.1818...%`,
reported as approximately `78.18%`.
The generic architecture-sequence estimate remains about 97%; this consumer
adds executable evidence without claiming completion of the remaining
consumer-, platform-, scheduler-, renderer-, or product-policy boundaries.
The generic widget interaction teardown seam is now executable: an additive
defaulted `Widget` hook can terminate retiring local state, old-surface mappers
collect ordered UI-local output before discard, and the existing deferred
command path reduces it after installation. Conservative removal, incompatible
replacement, authority/disabled/read-only loss, compatible preservation, old
mapper ownership, unmapped cleanup, ordering, exactly-once behavior, and
non-reentrant projection are covered by private runtime fixtures. These results
do not change the estimates above.

The current numeric evidence is a public generic `numeric_input(value, codec,
adjustment)` builder with typed construction failures and the shipped
fixed-capacity `NumericInputEditBatch<T>` bounded incremental carrier. The
carrier accepts exactly `[Update]`, `[Commit]`, `[Cancel]`, `[Begin, Update]`,
`[Begin, Commit]`, and `[Begin, Cancel]` in private inline capacity-two
storage. The shipped text-first widget emits only `[Begin, Commit]` and
`[Begin, Cancel]`, including `[Begin, Cancel]` when replacement teardown retires
an active edit; this carrier is storage and shape validation foundation only. The
consumer formats the initial value through the application codec,
validates the adjustment inverse, preserves verbatim drafts, caches draft
classification for the synchronous allocation-free focus-loss veto seam,
commits valid Enter/focus-loss edits, cancels active Escape edits, and retains
draft/caret/session state only for an actually active edit during same-ID
reprojection. Qualified exports remain outside the common prelude.
The qualified `NumericStepAttempt`, `NumericInputInteraction<T, StepError,
FormatError>`, and `NumericInputInteractionBatch<T, StepError, FormatError>`
are now shipped as a fixed-capacity keyboard envelope and complete TextEdit
output contract. The batch validates successful keyboard edit fragments,
TextEdit terminal fragments, and typed initial or rollback-before-repeat
failures. Complete mode consumes an explicitly attached step policy for
effective ArrowUp/ArrowDown transactions, typed failures, rollback, capture,
and teardown through the selected complete mapper; compatibility mode remains
inert.
The crate-private shared numeric interaction gate is now shipped for TextEdit
admission, no-op cleanup, terminal cleanup, replacement teardown, and compatible
active reprojection. `NumericInputWidget` consumes the generic
`Widget::prepare_replacement` seam: an exact same-ID, same-value, enabled,
non-read-only successor preserves the active session for normal synchronization;
every other replacement boundary publishes one rollback through the retiring
widget's selected mapper mode, restores the value/draft/caret/selection
snapshot, and releases TextEdit ownership. A mode change cannot inherit the
active session. Invalid, incomplete, and out-of-range drafts use the existing
cancel path without consulting codec or adjustment policy.
Normalized `KeyRelease` plumbing is now shipped across native input, runtime
events, and focused widget dispatch; complete-mode semantic keyboard
adjustment is now shipped for explicitly configured numeric step policies. The
backend-neutral IME/composition lifecycle is now defined but unimplemented; the
current source has no composition event or state. Pointer scrubbing is
contract-defined but unimplemented. Wheel adjustment is now contract-defined but
unimplemented. Numeric accessibility actions are contract-defined but
unimplemented. Slider/Knob, platform, scheduler, renderer, and product policy
remain out of scope for this slice.
The four other shared-gate consumers—IME/composition, pointer, wheel, and
accessibility—remain target-only and unimplemented.
The public `KeyboardModifier`/`NumericStepModifiers` selector and
`NumericInputBuilder::step_modifiers(...)` attachment are now the explicit
complete-mode keyboard consumer policy. The selector evaluates lossless
`KeyboardModifiers` samples without allocation or retained state; the widget
stores `None` when unconfigured or exactly `Some(policy)` when attached and
recomputes the selected step for every sample. No automatic platform policy is
introduced; compatibility mode and an unconfigured complete widget remain
inert.
The native keyboard boundary now also ships a lossless widget-modifier
projection alongside the unchanged host-shortcut `KeyPress` projection:
Linux/Windows Control remains host `command` for shortcut resolution but reaches
an unhandled focused widget as `control`, while Super/Meta reaches it as
`command`; combined and Shift/Alt states remain independent across press,
repeat, and release. Host handling remains first, and handled shortcuts do not
reach widgets. This prerequisite correction has zero impact on the estimates
and does not ship numeric stepping, capture, transactions, or a numeric
consumer.
The generic metadata-aware focused-key routing kernel is now shipped as the
backend-neutral routing authority consumed by complete-mode `KeyboardAdjustment`.
It adds
defaulted object-safe widget opt-in/captured-key queries, one private fixed-size
controller capture record, host-first uncaptured-initial routing, owner-first
continuations and cancellation, stale/orphan/competing ignore behavior, exact
metadata preservation, and conservative refresh reconciliation. Native Vello
normalizes evidence and delegates to the same controller authority; synthetic,
backend-neutral, and native/direct fixtures cover equivalent decisions. Existing
widgets retain the key-only `preempts_host_shortcut_key` compatibility path.
The kernel itself remains generic: the numeric widget supplies the step,
transaction, output, and typed-failure semantics.
The numeric interaction output mapping is now shipped for the TextEdit and
complete-mode keyboard lifecycles. It fixes the exact `on_interaction` mapper
type and associated error order, one interaction batch/mapper/host dispatch per
input or teardown boundary, nested TextEdit terminal shapes, bounded keyboard
edit and failure shapes, compatibility-only `on_edit`, mapper exclusivity,
validator acceptance, retiring-mapper mode selection, and generic
host-first/owner-first focused-key routing. This bounded consumer slice updates
the estimates by one point in Input/provenance/edit lifecycle, one point in
Text/focus/selection, and four points in Numeric controls:
`854 + 1 + 1 + 4 = 860`, `860 / 11 = 78.1818...%`, reported as approximately
`78.18%`; the generic estimate remains about 97% and the other target gaps
remain unchanged.
