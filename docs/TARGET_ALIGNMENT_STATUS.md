# Radiant Target-Alignment Status

This document tracks Radiant's implementation alignment with the normative
target. It is a durable progress snapshot, not a measure of line count,
documentation completeness, or workflow activity. GitHub remains the delivery
record for individual slices and reviews.

## Snapshot

- Snapshot date: **2026-08-05**
- Canonical main: **`b6991a3a`**
- Overall estimate: **~63%**
- Working range: **56–70%**
- Confidence: **medium**

The estimate reflects the combination of shipped contract, implementation,
tests, documentation, and integration evidence. Target-only prose, diagrams,
or examples do not count as shipped implementation; they establish the
contract against which progress is measured.

## Alignment by category

| Category | Alignment | Evidence / status |
| --- | ---: | --- |
| Public API and module boundaries | 80% | Explicit public/module boundaries and prelude hygiene are shipped; the full target surface is not. |
| Declarative model, identity, reconciliation | 70% | Stable identity, revision, and continuity foundations are shipped; complete production reconciliation remains. |
| Input, provenance, and edit lifecycle | 80% | Shared provenance and `EditEvent` lifecycle are adopted by `Slider`, `Knob`, and `PanelResizeState`; broader consumers remain. |
| Layout, composition, virtualization | 60% | Core ownership direction is established; generic `LayoutInteraction`/`split_pane` and full virtualization proof remain. |
| Text, focus, and selection | 60% | Focus and selection foundations exist; richer multiline/IME/composition editing and native accessibility remain. |
| Numeric controls | 35% | Finite linear/log `ValueMapping` and deterministic allocation-free `ValueFormat` are shipped; edit-session and control integration are not. |
| Runtime, effects, and scheduling | 65% | Runtime controller and host-facing lifecycle foundations exist; the complete effects/scheduling target is not wired. |
| Rendering, invalidation, retained GPU surfaces | 60% | Revision/damage direction is present; production render-boundary and cache-admission wiring remains. |
| Platform, windowing, and host boundaries | 60% | macOS-first host-facing boundaries are established; broader Linux/Windows runtime validation remains. |
| Diagnostics, profiling, and performance validation | 50% | Bounded diagnostics and validation foundations exist; first-class profiling/debug inspection and broader proof remain. |
| Examples, documentation, and CI guardrails | 75% | Normative docs, API references, tests, and CI guardrails are substantial; target-only examples do not substitute for integration evidence. |

## Shipped foundations

The current foundation includes:

- explicit public/module boundaries and prelude hygiene;
- stable identity, revision, and continuity;
- interaction provenance and the shared `EditEvent` lifecycle adopted by
  `Slider`, `Knob`, and `PanelResizeState`;
- host-facing resize state;
- finite linear/log `ValueMapping`; and
- deterministic, allocation-free `ValueFormat`.

These foundations make later slices safer and more composable. They do not
mean that every target consumer, runtime path, platform, or integration is
complete.

## Remaining gaps, ordered by leverage

1. **Numeric edit session (next recommended slice; not shipped).** Add a
   parser-agnostic `NumericEditSession` with a runtime-local draft and typed
   commit/cancel semantics. This is the next dependency-correct gap.
2. **Numeric and input integration.** Complete numeric attachment,
   `numeric_input`, widget/runtime/focus integration, and the pointer,
   keyboard, and accessibility domain contract around those paths.
3. **Generic layout interaction.** Add the reusable `LayoutInteraction` and
   `split_pane` contract.
4. **Richer text editing.** Complete multiline editing, IME/composition, and
   native accessibility semantics.
5. **Production frame wiring.** Complete reconciliation, damage propagation,
   render-boundary selection, and retained-surface cache admission in the
   production runtime path.
6. **Profiling and performance proof.** Add first-class `ProfilingMode`,
   `FrameProfile`, a debug inspector, and broader performance validation.
7. **Platform expansion.** Broaden Linux/Windows runtime validation and
   platform implementation behind the existing boundaries.

dB, tempo, and other custom numeric formats remain later work after the
parser-agnostic edit-session and generic numeric integration contracts are
established.

## Evidence map

The target and implementation evidence for this snapshot is mapped here:

- [Project target](../docs/TARGET.md)
- [Normative design direction](../docs/DESIGN_DIRECTION.md)
- [Architecture and ownership map](../docs/ARCHITECTURE.md)
- [Current public API](../docs/API.md)
- [Edit lifecycle and provenance](../src/widgets/interaction/edit.rs)
- [Value mapping](../src/widgets/interaction/value.rs)
- [Value formatting](../src/widgets/interaction/format.rs)
- [Widget revision contract](../src/widgets/contract/revision.rs)
- [Refresh, identity, and damage controller](../src/runtime/controller/refresh.rs)

`ARCHITECTURE.md` warns that target-state boundaries are not proof of
implementation. The evidence map therefore distinguishes normative contracts
from concrete shipped source, tests, and integration evidence; a target API or
example is not promoted to shipped status without that supporting proof.

## Update protocol

After each merged alignment slice:

1. update the snapshot date and canonical main SHA;
2. update the overall estimate and working range;
3. update the affected category score and evidence/status;
4. move an item only when code, tests, documentation, and integration justify
   the change;
5. record the next dependency-correct gap; and
6. keep GitHub as the delivery record rather than duplicating its workflow
   ledger here.

### Initial entry

| Date | Canonical main | Overall | Note |
| --- | --- | ---: | --- |
| 2026-08-05 | `b6991a3a` | ~63% (56–70%, medium confidence) | Initial durable target-alignment snapshot; the next recommended gap is the unshipped parser-agnostic `NumericEditSession`. |
