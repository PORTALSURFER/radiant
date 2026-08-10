# Radiant Target Alignment

| Overall measure | Estimate |
| --- | ---: |
| Generic architecture-sequence completion | ~97% (92–99%, medium confidence) |
| Broad end-to-end target coverage | ~81.09% (892 / 11) |

| Category | Estimate |
| --- | ---: |
| Public API and module boundaries | 83% |
| Declarative model, identity, reconciliation | 70% |
| Input, provenance, and edit lifecycle | 96% |
| Layout, composition, virtualization | 96% |
| Text, focus, and selection | 73% |
| Numeric controls | 87% |
| Runtime, effects, and scheduling | 96% |
| Rendering, invalidation, retained GPU surfaces | 78% |
| Platform, windowing, and host boundaries | 71% |
| Diagnostics, profiling, and performance validation | 66% |
| Examples, documentation, and CI guardrails | 76% |

The broad estimate is the unweighted mean of the category rows:
`(83 + 70 + 96 + 96 + 73 + 87 + 96 + 78 + 71 + 66 + 76) / 11 = 81.0909...%`,
reported as approximately `81.09%`.
The generic architecture-sequence estimate remains about 97%; this consumer
adds executable evidence without claiming completion of the remaining runtime
accessibility-dispatch, native-adapter, scheduler-, renderer-, or
product-policy boundaries.
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
`[Begin, Commit]`, `[Begin, Cancel]`, and `[Begin, Update, Commit]` in private
inline capacity-three storage. The shipped text-first widget emits only `[Begin, Commit]` and
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
qualified backend-neutral IME/composition foundation, the single-line
`TextInputWidget` consumer, and the `NumericInputWidget` consumer are now
shipped: validated Unicode-scalar `CompositionRange`/`CompositionSample`
values, default-compatible object-safe widget hooks, erased surface dispatch,
a private fixed-size focused `Idle`/`Active`/`Blocked` ownership kernel, and
widget-local start/update/commit/cancel state with revision-aware
reprojection. Numeric preedit remains visible local text without parse or
publication; a valid commit reuses text sanitization and the numeric codec for
one terminal `[Begin, Commit]` batch, while invalid commits remain correctable
and cancel/focus-loss restores the captured edit. The source deliberately
leaves native IME adapters, matching-key suppression, candidate windows,
runtime accessibility dispatch, and product behavior unshipped. Complete-mode
NumericInput
PointerScrub consumption is now shipped for the explicitly configured
primary-plus-Alt/Option path, including managed capture, bounded output, typed
failures, and exact rollback/teardown. Complete-mode NumericInput wheel
consumption is also shipped through the explicit `NumericWheelPolicy`
attachment: exact samples preserve line/pixel units and phaseful provenance,
use the fixed 40-pixel line equivalence, retain pending/active ownership, emit
bounded `[Begin, Update, Commit]` or incremental fragments, report typed
adjustment and format failures, roll back before update diagnostics, and cancel
the incumbent owner directly before a superseding `Started` sample is routed.
Phase-less and `Discrete` samples are bounded atomic gestures; orphan phaseful
`Changed`, `Ended`, and `Cancelled` samples are ignored and remain available to
fallback routing.
Legacy vector dispatch remains metadata-preserving after metadata-neutral hit
testing. The typed, widget-local NumericInput accessibility policy now ships
the neutral Increment/Decrement/SetValueText vocabulary, Base-step and
complete-text semantics, atomic Accessibility provenance, typed failures, and
incumbent-owner blocking. Runtime target resolution, focus admission, stale or
unmaterialized classification, dispatch, and native adapters remain separate
unshipped boundaries.
Slider/Knob, platform, scheduler, renderer, and product policy remain out of
scope for this slice.
The Input evidence moves from 95% to 96%, Numeric controls from 82% to 87%,
and Text remains 73% because runtime focus/selection admission is separate.
The evidence-backed total for this consumer moves from `886` to `892`:
`886 + 1 + 5 = 892`, `892 / 11 = 81.0909...%`, reported as approximately
`81.09%`. Runtime accessibility dispatch remains the remaining numeric
consumer boundary; native adapters, matching-key suppression, and candidate
windows also remain separate boundaries. The generic composition foundation,
the single-line text consumer, the NumericInput consumer, and the other
previously shipped routing foundations remain distinct from that remaining
runtime boundary.
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
remain unchanged at that earlier cycle.
The accepted generic wheel-routing foundation adds qualified exact line/pixel
samples, default-compatible widget hooks, owner-first managed continuity, and a
bounded Idle/Active/Blocked lifecycle with conservative refresh, focus, removal,
replacement, authority, disabled, and read-only handling. Legacy phase-less hit
testing remains metadata-free while exact samples preserve metadata. Its focused
runtime/custom-widget/public API and guardrail coverage, native compatibility
regression coverage, full library validation, and current-head performance
evidence are shipped. The later accepted NumericInput wheel consumer adds one
point in Input/provenance/edit lifecycle, one point in Text/focus/selection, and
five points in Numeric controls. The resulting evidence-backed total is
`867 + 1 + 1 + 5 = 874`, `874 / 11 = 79.4545...%`, reported as approximately
`79.45%`; the generic estimate remains about 97%.
The numeric interaction output mapping is now shipped for the TextEdit and
complete-mode keyboard lifecycles. It fixes the exact `on_interaction` mapper
type and associated error order, one interaction batch/mapper/host dispatch per
input or teardown boundary, nested TextEdit terminal shapes, bounded keyboard
edit and failure shapes, compatibility-only `on_edit`, mapper exclusivity,
validator acceptance, retiring-mapper mode selection, and generic
host-first/owner-first focused-key routing. The earlier bounded keyboard
consumer slice moved the estimates from 854 to 860; the PointerScrub consumer
above records 867, and the accepted NumericInput wheel consumer records 874.
The accepted native wheel adapter slice now maps winit line and pixel deltas to
validated backend-neutral `WheelSample` values, preserves DPI-adjusted sign,
TouchPhase lifecycle, modifiers, timestamps, and sequence ranges, and routes
phaseful events through the exact wheel seam. Explicit lifecycle boundaries
flush pending phase-less compatibility input; phaseful samples bypass the
vector-only GPU/scroll queues so units and phase cannot be erased, while exact
hit testing remains metadata-neutral. Native Vello and auxiliary-window paths
share the adapter, and focused conversion, routing, full-library, integration,
guardrail, cross-target, and remote CI evidence are shipped. Phase-less callers
retain the existing compatibility/coalescing path, and malformed exact input
retains the sanitized fallback without constructing an invalid exact sample.
This platform slice moves the evidence-backed total from 874 to 875:
`874 + 1 = 875`, `875 / 11 = 79.5454...%`, reported as approximately
`79.55%`; the generic estimate remains about 97%.
