# Radiant Target Alignment

| Overall measure | Estimate |
| --- | ---: |
| Generic architecture-sequence completion | ~97% (92–99%, medium confidence) |
| Broad end-to-end target coverage | ~78.82% (867 / 11) |

| Category | Estimate |
| --- | ---: |
| Public API and module boundaries | 83% |
| Declarative model, identity, reconciliation | 70% |
| Input, provenance, and edit lifecycle | 91% |
| Layout, composition, virtualization | 96% |
| Text, focus, and selection | 69% |
| Numeric controls | 72% |
| Runtime, effects, and scheduling | 96% |
| Rendering, invalidation, retained GPU surfaces | 78% |
| Platform, windowing, and host boundaries | 70% |
| Diagnostics, profiling, and performance validation | 66% |
| Examples, documentation, and CI guardrails | 76% |

The broad estimate is the unweighted mean of the category rows:
`(83 + 70 + 91 + 96 + 69 + 72 + 96 + 78 + 70 + 66 + 76) / 11 = 78.8181...%`,
reported as approximately `78.82%`.
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
current source has no composition event or state. Complete-mode NumericInput
PointerScrub consumption is now shipped for the explicitly configured
primary-plus-Alt/Option path, including managed capture, bounded output, typed
failures, and exact rollback/teardown. Wheel adjustment remains unimplemented
as a NumericInput consumer, but the backend-neutral wheel-sample and managed
wheel-sequence routing kernel is now shipped. Numeric accessibility actions are
contract-defined but
unimplemented. Slider/Knob, platform, scheduler, renderer, and product policy
remain out of scope for this slice.
The three remaining numeric consumers—IME/composition, NumericInput wheel, and
accessibility—remain target-only and unimplemented; generic PointerScrub and
wheel routing foundations are shipped.
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

The generic pointer-press admission/capture kernel is now shipped as the next
qualified controller foundation. `radiant::widgets::PointerPressAdmission`
provides default-compatible Legacy, managed exact-widget/exact-button capture,
and Blocked admission; the controller validates focus and continuation authority
at dispatch, cancellation, and refresh boundaries and keeps bounded
button-specific orphan-release suppression. Scrollbar/layout precedence,
legacy capture and mapper ownership, pointer metadata, and all existing widget
contracts remain unchanged. This foundation earned no PointerScrub credit by
itself; the later NumericInput consumer below now supplies the bounded
PointerScrub behavior.
The exact-head accepted NumericInput PointerScrub consumer now uses that kernel
for explicitly configured complete-mode widgets. Its 11 deterministic fixtures,
public API and runtime managed-capture coverage, source guardrails, full
all-target/all-feature validation, and pointer-motion performance evidence cover
policy selection, finite in-bounds normalization without clamping, anchor and
step-change reprojection, no-op accumulation, lifecycle rollback, typed initial
and update failures, exact metadata, fixed-capacity output, and wheel fall-through.
This slice updates the estimates by one point in Input/provenance/edit lifecycle,
one point in Text/focus/selection, and five points in Numeric controls:
`860 + 1 + 1 + 5 = 867`, `867 / 11 = 78.8181...%`, reported as approximately
`78.82%`; the generic estimate remains about 97% and the other target gaps
remain unchanged.
The accepted generic wheel-routing foundation adds qualified exact line/pixel
samples, default-compatible widget hooks, owner-first managed continuity, and a
bounded Idle/Active/Blocked lifecycle with conservative refresh, focus, removal,
replacement, authority, disabled, and read-only handling. Legacy phase-less hit
testing remains metadata-free while exact samples preserve metadata. Its focused
runtime/custom-widget/public API and guardrail coverage, native compatibility
regression coverage, full library validation, and current-head performance
evidence are shipped. This foundation earns zero NumericInput wheel-consumer
credit; the broad estimate remains `867 / 11 = 78.8181...%` (approximately
`78.82%`) and the generic estimate remains about 97%.
The numeric interaction output mapping is now shipped for the TextEdit and
complete-mode keyboard lifecycles. It fixes the exact `on_interaction` mapper
type and associated error order, one interaction batch/mapper/host dispatch per
input or teardown boundary, nested TextEdit terminal shapes, bounded keyboard
edit and failure shapes, compatibility-only `on_edit`, mapper exclusivity,
validator acceptance, retiring-mapper mode selection, and generic
host-first/owner-first focused-key routing. The earlier bounded keyboard
consumer slice moved the estimates from 854 to 860; the PointerScrub consumer
above records the current 867 total.
