# Radiant Platform Acceptance and Evidence Policy

This is the authoritative policy for deciding what Radiant platform and feature
claims are supported by evidence. It defines the evidence lanes, outcome states,
platform/session prerequisites, release gates, artifact contract, and ownership
for the target in `docs/TARGET.md`. It does not add runtime behavior, change a
public API, create a native adapter, or replace an existing acceptance
procedure.

`docs/TARGET.md` remains the source of target requirements and completion
criteria. `docs/DESIGN_DIRECTION.md` remains the normative architecture
contract, `docs/API.md` remains the current application-facing API contract, and
`docs/ARCHITECTURE.md` remains the contributor ownership map. This policy is the
single interpretation layer for acceptance claims across those documents. A
lane/outcome ID uses the form `LANE/OUTCOME`, for example `A/PASS` or
`M/NOT_RUN`.

## Authority and claim rules

- Only canonical merged source can support a shipped or target-complete claim.
  Branch, draft, acceptance-only, and unverified evidence do not change shipped
  status or scorecard estimates.
- A claim is scoped to one capability, one platform baseline, one evidence lane,
  and one source commit. Evidence from one platform or lane is not silently
  transferred to another platform or lane.
- A documented command, expected result, source-level test, or configured CI job
  describes a procedure or available evidence source. It is not a `PASS` until
  the required lane runs and its manifest is retained.
- A lane proves only the boundary named in the lane table. Compilation cannot
  prove a native window; a deterministic host cannot prove a compositor, IME,
  accessibility adapter, GPU, or screen reader; headless host evidence cannot
  prove logged-in desktop automation or manual hardware use.
- Every reported result records the source commit, lane/outcome ID, platform and
  session, command or harness, capability/criteria covered, and artifact
  manifest. A result without that identity is not release evidence.
- The current evidence inventory below is a bounded description of repository
  configuration and documented acceptance boundaries. It is not a second
  per-run ledger. Run-specific detail belongs in the retained CI/release
  artifact manifest.
- A policy or documentation change does not earn completion-criterion credit and
  does not revise `docs/TARGET_ALIGNMENT_STATUS.md` or its estimates.
- X11 sessions, a direct X11 backend, product-specific behavior, VST/plugin SDK
  integration inside Radiant, and claims about unlisted operating-system
  versions are outside this policy's acceptance scope.

## Evidence lanes

The lanes are non-substitutable. A stronger-looking result in the wrong lane is
still the wrong evidence.

| Lane | Name | Proves | Does not prove |
| --- | --- | --- | --- |
| C | Static/build/cross-target/compile | Formatting, static checks, tests that do not require a native session, target compilation, dependency boundaries, example compilation, documentation builds, and configured CI build/performance smoke commands. | Runtime behavior, a real event loop or compositor, native presentation, IME, accessibility, screen-reader behavior, GPU behavior, or human/hardware acceptance. |
| A | Automated deterministic/core, including `DeterministicHost` | Production core/runtime behavior with fixed inputs, virtual time, task completions, structured snapshots, lifecycle transitions, and deterministic replay-style assertions. | A native window, compositor, GPU, native text service, IME, accessibility consumer, screen reader, or physical device. |
| H | Headless native host with a real event loop/compositor | Native host lifecycle, event-loop routing, surface/window creation, resize, compositor-backed presentation smoke, and host-boundary behavior in a real native host environment. | Logged-in desktop automation, human interaction, screen-reader use, physical hardware, or a capability not exercised by the headless host. A fake event loop or synthetic compositor is not H evidence. |
| N | Logged-in live native desktop automation | A fresh native application in a logged-in desktop session, with app-level native automation such as AppKit/AX, Windows UI Automation, or an equivalent supported desktop automation path. It can prove the native adapter behavior that the automation actually exercises. | Human/manual usability, screen-reader acceptance, physical keyboard/mouse/IME behavior not exercised by the automation, or GPU/timing behavior not observed by the run. |
| M | Manual native/hardware | Human acceptance using the native desktop, physical or assigned input devices, native IME workflows, screen readers, and hardware-dependent presentation or recovery. | Deterministic repeatability, unattended CI, or a capability that the operator did not actually exercise. |

Each evidence item declares exactly one primary lane. A run may attach
supporting artifacts from other lanes, but the primary claim remains in its
declared lane and every required lane still has to be satisfied.

## Outcome states

Outcome states are terminal for the reported evidence item until a new run
produces a new manifest. They are not interchangeable labels for missing
information.

| Outcome | Meaning | Gate treatment |
| --- | --- | --- |
| PASS | The required assertions ran successfully in the declared lane, platform, session, and source scope, and the manifest is complete. | Satisfies that scoped requirement only. It never supplies another lane or platform. |
| FAIL | The lane ran and an assertion failed, the evidence was stale or malformed, or the manifest could not establish the claimed scope. | Blocks every gate that requires the item until a new qualifying run passes. |
| UNSUPPORTED | The declared platform or implementation explicitly does not provide the capability. | Allowed only where the capability matrix or criteria map says the capability is conditional. It is never converted to `PASS` and is not an excuse for an unconditional target requirement. |
| UNAVAILABLE | The lane was attempted, but the required runner, session, device, permission, compositor, or other environment capability was unavailable. | Blocks a required gate unless the row explicitly allows an unavailable or capability-conditional outcome. The manifest must identify the missing prerequisite. |
| NOT_RUN | No qualifying attempt has produced evidence for the item. | Never satisfies a gate. A documented procedure remains `NOT_RUN` until executed. |
| NOT_APPLICABLE | The matrix explicitly excludes the item for this platform, lane, or change scope. | It is excluded only when the policy row says so; it is not a synonym for `UNSUPPORTED`, `UNAVAILABLE`, or `PASS`. |

The only permitted state changes are explicit and manifest-backed:

- `NOT_RUN` may become `PASS` or `FAIL` only after a qualifying run. It may
  become `UNAVAILABLE` only after setup was attempted and the missing
  prerequisite is recorded; it may not silently become `PASS`.
- `UNAVAILABLE` may become `PASS` or `FAIL` only through a new run with the
  required environment. It may not silently become `NOT_RUN`.
- `UNSUPPORTED` may remain an allowed conditional result, but it can become
  `PASS` only after the capability is implemented or enabled and a new run is
  performed; the old result is never relabeled.
- `FAIL` becomes `PASS` only through a new run at the corrected source and
  required environment. Rewording a report or replacing an artifact does not
  convert it.
- `NOT_APPLICABLE` is set by the matrix before execution. It cannot be used to
  hide a missing required lane and cannot be converted to a general pass.

## Platform baselines and session requirements

The supported target matrix is fixed by the current target documents:

| Platform ID | Baseline | Required session and native prerequisites |
| --- | --- | --- |
| macOS | The current supported macOS release on the current physical M5 Pro development host. The exact OS version/build and host details belong in each manifest. | H requires a real native event loop and WindowServer/compositor-backed surface. N and M require a logged-in interactive desktop session; M also requires the native input, IME, screen-reader, or GPU device named by the capability. |
| Windows | Windows 11 25H2. The exact image/build, toolchain, and device details belong in each manifest. | H requires a real Windows native event loop and compositor-backed presentation environment. N and M require a logged-in interactive desktop session; M also requires the named native input, screen-reader, or GPU device. |
| Ubuntu Wayland | Ubuntu 26.04 LTS Desktop with its default GNOME Wayland session. | H requires a real Wayland event loop and compositor, including a real headless compositor when the host is intentionally headless. N and M require a logged-in GNOME Wayland desktop session and the named native service or device. X11 is not a substitute. |

C does not require an interactive desktop and A uses the fixed deterministic
host contract. Neither lane may be used to imply that a native session exists.
H requires the native event loop and compositor even when no human is present.
N requires a logged-in desktop and a fresh application/bundle attachment.
M requires a human-operated native session and the physical or assigned
hardware/service needed by the capability. If a prerequisite is absent, record
`H/UNAVAILABLE`, `N/UNAVAILABLE`, or `M/UNAVAILABLE` rather than silently
falling back to C or A.

The exact OS version/build, runner image, display/compositor, account/session
type, hardware or virtual device, permissions, toolchain, and application
artifact are required manifest fields. A runner label alone is not proof of a
session or capability.

## Capability acceptance matrix

The matrix gives the minimum lane set for each capability family. `Required`
means a missing required outcome blocks the target or release gate. `Conditional`
means that only the named capability portion may end in the explicitly recorded
conditional outcome; it does not make an unconditional requirement optional.

| Capability | Acceptance claim | Required lanes | Applicable platforms | Gate status | Allowed unavailable/capability-conditional outcome |
| --- | --- | --- | --- | --- | --- |
| Core/lifecycle | Core view, layout, input, focus, invalidation, state, and lifecycle behavior follows the backend-neutral contract. | C + A | macOS, Windows 11 25H2, Ubuntu Wayland | Required | None. `UNAVAILABLE` or `UNSUPPORTED` cannot satisfy this row. |
| Deterministic host | `DeterministicHost` drives one production runtime with fixed viewport/environment/time, bounded work, and structured deterministic observations. | C + A | macOS, Windows 11 25H2, Ubuntu Wayland | Required | None for the host contract. Native-only consumers remain outside A and must use their own lane. |
| Native window/presentation | Native windows, event-loop ownership, surface creation, resize, presentation, and multi-window lifecycle work on the baseline host. | C + H + N | macOS, Windows 11 25H2, Ubuntu Wayland | Required | `H/UNAVAILABLE` is an environment failure, not acceptance. `UNSUPPORTED` is allowed only for a separately named optional host capability. |
| IME | The platform IME path preserves composition, preedit, candidate, commit, cancel, caret, and focus-loss behavior for the supported text control. | C + N + M | macOS, Windows 11 25H2, Ubuntu Wayland | Required when the baseline provides the IME service | `UNSUPPORTED` or `UNAVAILABLE` is allowed only for a named platform/IME capability with the prerequisite and owner recorded; it never passes an available baseline. |
| Accessibility/native adapters | Native accessibility projection, identity, focus, actions, lifecycle, and adapter failure behavior remain consistent with the backend-neutral semantics. | C + H + N | macOS, Windows 11 25H2, Ubuntu Wayland | Required | A missing adapter is `UNSUPPORTED` or `UNAVAILABLE` only while explicitly tracked as a conditional capability; it is not a native acceptance pass. |
| Screen-reader | A real screen reader can discover, focus, announce, and operate the required native semantics in the baseline desktop session. | C + N + M | macOS, Windows 11 25H2, Ubuntu Wayland | Required for screen-reader claims | `UNSUPPORTED` or `UNAVAILABLE` is allowed only when the baseline screen-reader capability is explicitly absent and recorded. No H or A result substitutes for M. |
| GPU/recovery/timing capability | The adapter's GPU resource, loss/recovery, timing, and explicit unavailable-outcome behavior is observed without fabricating unsupported GPU data. | C + H + N + M | macOS, Windows 11 25H2, Ubuntu Wayland | Conditional on the adapter capability | `UNSUPPORTED` or `UNAVAILABLE` is allowed for a named adapter/device/timestamp capability with manifest evidence. Existing scheduler/performance contracts supply thresholds; this policy adds none. |
| Performance/fairness | Declared workloads report the existing counters and trend data, and native multi-window fairness is checked against the existing scheduler contract. | C + A + N | macOS, Windows 11 25H2, Ubuntu Wayland | Required for performance-sensitive completion; trend evidence otherwise | Machine-specific performance or native fairness evidence may be `UNAVAILABLE` only with an explicit environment record. It is not a portable PASS and does not waive a required release check. |

## Target completion criteria map

This table is a one-to-one map of the bullets currently under
`docs/TARGET.md` `## Completion Criteria`. The criterion text is copied as one
normalized line where the source bullet wraps. IDs, lanes, platforms, gate
status, and owners are policy metadata; they do not alter the target wording.

| ID | Completion criterion from `docs/TARGET.md` | Required lanes | Applicable platforms | Gate status | Owner | Allowed unavailable/capability-conditional outcome |
| --- | --- | --- | --- | --- | --- | --- |
| TC-01 | A cleaner public API | C + A | macOS, Windows 11 25H2, Ubuntu Wayland | Required target gate | API maintainers | None; unavailable evidence does not pass. |
| TC-02 | One unified API surface instead of fragmented simple/advanced APIs | C + A | macOS, Windows 11 25H2, Ubuntu Wayland | Required target gate | API maintainers | None; unavailable evidence does not pass. |
| TC-03 | A more declarative usage model | C + A | macOS, Windows 11 25H2, Ubuntu Wayland | Required target gate | UI/runtime maintainers | None; unavailable evidence does not pass. |
| TC-04 | Strong independence from any single application domain | C + A | macOS, Windows 11 25H2, Ubuntu Wayland | Required target gate | Radiant maintainers | None; product-specific evidence is not applicable to this criterion. |
| TC-05 | Vello-based rendering for standard UI widgets | C + A + H | macOS, Windows 11 25H2, Ubuntu Wayland | Required target gate | Rendering maintainers | `UNSUPPORTED` is not allowed for the standard renderer; a missing native run remains unavailable. |
| TC-06 | Direct WGPU/custom shader rendering for specialized GPU-heavy widgets where useful | C + H + N | macOS, Windows 11 25H2, Ubuntu Wayland | Conditional GPU target gate | GPU/runtime maintainers | `UNSUPPORTED` or `UNAVAILABLE` only for the named adapter capability, with no unconditional credit. |
| TC-07 | Clean integration between Vello-rendered UI and direct-WGPU custom surfaces | C + A + H + N | macOS, Windows 11 25H2, Ubuntu Wayland | Conditional GPU target gate | Rendering/runtime maintainers | `UNSUPPORTED` or `UNAVAILABLE` only for an explicitly named GPU capability; core integration remains required. |
| TC-08 | No unnecessary leakage of Vello or WGPU internals into normal application code | C + A | macOS, Windows 11 25H2, Ubuntu Wayland | Required boundary gate | API/rendering maintainers | None; a capability exception cannot excuse a boundary violation. |
| TC-09 | Rendering architecture that can evolve later without requiring a public API rewrite | C + A | macOS, Windows 11 25H2, Ubuntu Wayland | Required architecture gate | Architecture maintainers | None; unavailable evidence does not pass. |
| TC-10 | Native macOS, Windows, and Linux/Wayland support without unnecessary platform-specific assumptions in core code | C + H + N + M | macOS, Windows 11 25H2, Ubuntu Wayland | Required three-platform gate | Platform/runtime maintainers | `UNAVAILABLE` blocks the affected platform; `UNSUPPORTED` is not allowed for an in-scope baseline. |
| TC-11 | GitHub Actions portable/build/compile/check evidence and, where runners permit, the required Linux/Windows integration and headless Wayland/native-host lanes, plus native M5 Pro acceptance for macOS | C + H + N + M | macOS, Windows 11 25H2, Ubuntu Wayland | Required evidence gate | Platform and release maintainers | Runner limits may be recorded as `UNAVAILABLE` for a conditional lane, but cannot silently become PASS or erase the macOS/native requirement. |
| TC-12 | No direct VST SDK integration inside Radiant | C | macOS, Windows 11 25H2, Ubuntu Wayland | Required boundary invariant | Radiant maintainers | None; no platform capability exception. |
| TC-13 | A plugin-friendly GUI architecture that can be integrated by application/plugin frameworks | C + A + H + N | macOS, Windows 11 25H2, Ubuntu Wayland | Required target gate | API/runtime maintainers | Native adapter portions may be capability-conditional only when named; generic GUI evidence remains required. |
| TC-14 | Clean internal module structure | C | macOS, Windows 11 25H2, Ubuntu Wayland | Required source-quality gate | Architecture maintainers | None; unavailable evidence does not pass. |
| TC-15 | Small, focused files | C | macOS, Windows 11 25H2, Ubuntu Wayland | Required source-quality gate | All subsystem maintainers | None; unavailable evidence does not pass. |
| TC-16 | Small, focused functions | C | macOS, Windows 11 25H2, Ubuntu Wayland | Required source-quality gate | All subsystem maintainers | None; unavailable evidence does not pass. |
| TC-17 | Clear structs and traits | C | macOS, Windows 11 25H2, Ubuntu Wayland | Required source-quality gate | API maintainers | None; unavailable evidence does not pass. |
| TC-18 | Reduced code smells | C + A | macOS, Windows 11 25H2, Ubuntu Wayland | Required source-quality gate | All subsystem maintainers | None; unavailable evidence does not pass. |
| TC-19 | Strong rendering and layout performance | C + A + N | macOS, Windows 11 25H2, Ubuntu Wayland | Required performance gate | Rendering/layout maintainers | Machine-specific timing may be `UNAVAILABLE` only with a manifest; the existing performance contract remains authoritative. |
| TC-20 | Strong support for modern CPU/GPU performance techniques | C + A + H + N | macOS, Windows 11 25H2, Ubuntu Wayland | Conditional performance/GPU gate | Runtime/performance maintainers | GPU portions may be `UNSUPPORTED` or `UNAVAILABLE` only for a named capability; no new threshold is created here. |
| TC-21 | Multi-threading support where useful | C + A | macOS, Windows 11 25H2, Ubuntu Wayland | Required target gate | Runtime maintainers | None; unavailable evidence does not pass. |
| TC-22 | SIMD-friendly internals where useful | C + A | macOS, Windows 11 25H2, Ubuntu Wayland | Conditional performance gate | Performance maintainers | `UNSUPPORTED` is allowed only for a named hardware optimization; generic correctness remains required. |
| TC-23 | GPU acceleration and compute-shader paths where useful | C + H + N | macOS, Windows 11 25H2, Ubuntu Wayland | Conditional GPU gate | GPU/rendering maintainers | `UNSUPPORTED` or `UNAVAILABLE` only for the named adapter/device capability, with explicit manifest evidence. |
| TC-24 | Clean widget, layout, style, event, input, focus, and state systems | C + A + H + N + M | macOS, Windows 11 25H2, Ubuntu Wayland | Required target gate | UI/runtime maintainers | A platform-only capability may be conditional as recorded in the matrix; core behavior cannot be waived. |
| TC-25 | Text/font handling designed as a first-class concern | C + A + N + M | macOS, Windows 11 25H2, Ubuntu Wayland | Required text gate | Text/runtime maintainers | `UNSUPPORTED` or `UNAVAILABLE` only for a named native text/IME capability; backend-neutral text behavior remains required. |
| TC-26 | Maintained examples and sandboxes covering major systems | C + A | macOS, Windows 11 25H2, Ubuntu Wayland | Required example gate | Example maintainers | Native-only example steps may be `NOT_APPLICABLE` for a non-native build, but compilation and documented boundaries remain required. |
| TC-27 | Tests that validate important behavior without locking in incidental implementation details | C + A | macOS, Windows 11 25H2, Ubuntu Wayland | Required test gate | Test maintainers | None; unavailable evidence does not pass. |
| TC-28 | Benchmarks or profiling tools for important hot paths | C + A | macOS, Windows 11 25H2, Ubuntu Wayland | Required tooling gate | Performance maintainers | Machine-specific execution may be `UNAVAILABLE` with a manifest; tool existence and structural coverage remain required. |
| TC-29 | Clear documentation | C | macOS, Windows 11 25H2, Ubuntu Wayland | Required documentation gate | Documentation maintainers | None; unavailable evidence does not pass. |
| TC-30 | A clear distinction between library code, examples, and application-specific code | C + A | macOS, Windows 11 25H2, Ubuntu Wayland | Required boundary gate | Architecture/documentation maintainers | None; product-specific behavior remains out of scope. |

## Release gates and cadence

The release decision is the conjunction of the applicable gates below. A gate
is green only when every required item is `PASS`, or when the table explicitly
permits `NOT_APPLICABLE` or a recorded capability-conditional terminal state.
`FAIL`, `NOT_RUN`, and unallowed `UNAVAILABLE` or `UNSUPPORTED` block the gate.

| Gate | Required evidence | Decision |
| --- | --- | --- |
| Source and contract | C for the edited boundary, documentation guardrails, and a complete diff from canonical merged source. | Required on every change that affects a documented contract. |
| Deterministic/core | A for the affected production runtime behavior, including `DeterministicHost` where it is the host under test. | Required before a core or backend-neutral feature is called complete. |
| Native host | H for each applicable baseline, with the real event loop and compositor. | Required before claiming native lifecycle/window/presentation or native-host smoke. |
| Live desktop | N for each applicable native adapter/capability that requires a logged-in desktop. | Required before claiming live native desktop automation or adapter acceptance. |
| Manual/hardware | M for IME, screen-reader, physical input, GPU, recovery, or other human/hardware claims. | Required before claiming the corresponding user or hardware behavior. |
| Performance/fairness | C/A trend and counter evidence plus the existing native workload contract when the change affects scheduling or performance. | Required for performance-sensitive completion; timing remains machine/capability scoped and uses existing thresholds only. |
| Release artifact | A complete manifest, raw outputs, checksums, and retention links for every required result. | Required for a release candidate and for any published acceptance claim. |

Cadence is event-based so an old result cannot silently survive a changed
boundary:

- Every pull request runs the applicable C and A checks and the documentation
  guardrail. A change touching native/platform code also identifies the H/N/M
  evidence it needs; a missing live environment is recorded, not waived.
- Main-branch CI runs the configured repository quality/cross-target/performance
  and compile lanes. These runs are evidence for their declared C scope only.
- A release candidate refreshes all required platform and capability artifacts
  at the candidate source commit. Native evidence is refreshed after changes to
  the relevant adapter, event-loop, presentation, text, accessibility, GPU,
  recovery, or scheduler boundary.
- A failed or unavailable run stays visible until a new manifest supersedes it.
  A procedure may be repeated at any time, but repetition without a new
  qualifying artifact does not change its outcome.

## Artifact schema and retention

Every retained evidence artifact has one manifest. The manifest is the durable
identity for the run; screenshots, videos, logs, JSONL, benchmark output, and
test reports are payloads referenced by it rather than independent claims.

The manifest schema is versioned and contains at least:

| Field | Required content |
| --- | --- |
| `schema` | Manifest schema version. A schema change creates a new manifest rather than changing an old result. |
| `policy` | `Radiant Platform Acceptance and Evidence Policy`. |
| `evidence_id` | Stable unique ID containing the lane, platform, source commit, and run identity. |
| `source_commit` | Full canonical source commit SHA used by the run. |
| `generated_at_utc` | UTC generation time. |
| `lane` and `outcome` | One of `C`, `A`, `H`, `N`, `M` and one of the six policy outcomes, exposed together as the lane/outcome ID. |
| `capability` and `criteria` | Capability matrix row and every covered `TC-01` through `TC-30` ID. |
| `platform` and `session` | Baseline, OS version/build, runner or host, session type, compositor, account/permission context, and device details where applicable. |
| `command_or_harness` | Exact command, workflow job, example, test, or manual procedure. |
| `assertions` | Structured assertion names and their result, including explicit unavailable/unsupported reasons. |
| `toolchain` | Rust/toolchain and relevant dependency or driver identity. |
| `payloads` and `sha256` | Relative artifact names, media/log type, and checksum for each payload. |
| `owner`, `supersedes`, and `notes` | Responsible owner, any replaced evidence ID, limitations, and redacted-environment notes. |

Retention rules:

- Keep the manifest and all payloads together in the CI/release evidence store
  or the linked review/release artifact location. Do not add a per-run ledger to
  the source tree and do not treat a local unlinked file as release evidence.
- Retain every release-candidate manifest for the lifetime of the release and
  retain the latest passing manifest for each active platform/capability until a
  newer passing manifest supersedes it. Keep failed and unavailable manifests
  until the replacement is linked so the state transition is auditable.
- Never overwrite an evidence ID or edit a manifest to change its outcome. A
  rerun creates a new ID and links `supersedes`; the old result remains
  traceable.
- Redact credentials, tokens, personal data, private window contents, and
  unrelated desktop information before retention. Redaction must not remove the
  platform/session facts or assertion output needed to verify the claim.
- A release or review may link a compact summary, but the manifest and raw
  payloads remain the authoritative artifact. If a payload is lost or its
  checksum no longer matches, the result is `FAIL`, not `NOT_RUN` or `PASS`.

## Current evidence inventory

This inventory records what the current repository configures or explicitly
documents. It does not assert a fresh run in this change and does not change
scorecard estimates. It is not a second evidence ledger; run-specific detail
belongs in the retained manifest and payloads described below.

| Current source or procedure | Lane/outcome ID | Evidence currently recorded or configured | Boundary |
| --- | --- | --- | --- |
| `.github/workflows/ci.yml` `quality` job on `macos-15-intel` | `C/NOT_RUN` for this policy snapshot | macOS quality, cross-target, and performance commands are configured: formatting, Clippy, library/integration tests, examples, docs/doctests, Linux/macOS no-default-features checks, and perf-harness baseline smoke. | Workflow configuration is C evidence scope; no native host, IME, accessibility, screen-reader, GPU, or manual acceptance is inferred without a run manifest. |
| `.github/workflows/ci.yml` `windows-compile` job on `windows-2025` | `C/NOT_RUN` for this policy snapshot | Windows compile-only evidence is configured through `cargo check --locked --all-targets --all-features`. | Compile-only does not prove a Windows native event loop, presentation, IME, accessibility, GPU, or performance. |
| `radiant::runtime::testing::DeterministicHost` and its API contract | `A/NOT_RUN` for this policy snapshot | The production deterministic/headless host and structured snapshot boundary are documented and available to automated tests. | It has no native window, real compositor, GPU rendering, IME, accessibility consumer, or production scheduler-policy claim. |
| `macos_frame_profile_acceptance` procedure in `docs/API.md` | `N/NOT_RUN` | A macOS live frame-profile procedure and expected bounded output are documented. | The procedure is not a live result and does not claim Linux/Windows presentation or GPU timestamp support. |
| `macos_devtools_acceptance` procedure in `docs/API.md` | `N/NOT_RUN` | A macOS live devtools-overlay procedure is documented. | It does not establish other-platform native acceptance or an automation result without a manifest. |
| `macos_external_drag_acceptance` procedure in `docs/API.md` | `M/NOT_RUN` | A manual native Finder/file-receiver procedure is documented. | The API documentation explicitly does not claim that a live Finder run has been performed. |
| `macos_numeric_accessibility_acceptance` bounded AppKit/Computer Use result | `N/NOT_RUN` for current policy-compliant evidence | A historical bounded AppKit/Computer Use run provides bounded N evidence for numeric action, set-value, and restart exposure. | It is explicitly not VoiceOver or release evidence; no complete policy manifest is recorded, so it is not a current `PASS`. |
| `macos_text_input_ime_acceptance` procedure in `docs/API.md` | `M/NOT_RUN` | A manual Japanese IME procedure covering preedit, candidate, commit, cancel, and focus-loss behavior is documented. | The automated projection checks and the native live IME procedure are separate; native Japanese IME acceptance remains unperformed. |
| Named macOS scheduler/fairness workload in `docs/DESIGN_DIRECTION.md` | `N/NOT_RUN` | The workload contract and assertions are normative, including stale redraws, input, close, recovery, and fairness-boundary checks. | Diagnostics remain non-authoritative until wired to the scheduler contract; no native fairness result is claimed. |
| Native-host CI coverage for Ubuntu Wayland and Windows | `H/NOT_RUN` | The target requires future headless Wayland and Windows native-host smoke lanes where runners permit. | No current CI native-host, compositor, IME, accessibility, or presentation acceptance is established. |

The current CI therefore has macOS quality/cross-target/performance evidence
sources and Windows compile-only evidence, all within C. It does not have
native host acceptance, native IME acceptance, accessibility acceptance, GPU
acceptance, screen-reader acceptance, or manual/hardware acceptance. A future
run may replace an inventory state with a manifest-backed outcome, but the
lane cannot be changed by rewording the inventory.

The current explicit non-goals remain X11/direct X11 hosting, product-specific
behavior, VST/plugin SDK integration in Radiant, and any platform or native
capability not named by the baseline matrix. This policy also does not add a
new timing threshold, native adapter, acceptance harness, or runtime behavior.

## Downstream ticket map

Ownership below follows the ticket and boundary references already present in
the normative documents. No new ticket ID or scorecard estimate is invented by
this policy. Native CI/desktop evidence gaps that have no named downstream
ticket remain owned by Radiant platform/release maintainers until separately
assigned.

| Ticket or boundary | Owner | Policy responsibility and dependency |
| --- | --- | --- |
| OPT-1420 | Radiant maintainers | Own this canonical policy, its structural guardrail, and the evidence vocabulary. This ticket does not add runtime behavior or native acceptance. |
| OPT-1371 | Platform adapter maintainers | Consume the policy for platform-adapter claims; adapter evidence depends on the lane, baseline, session, and complete manifest required here. |
| OPT-1372 | Platform adapter maintainers | Consume the policy for platform-adapter claims; adapter evidence depends on the lane, baseline, session, and complete manifest required here. |
| OPT-1373 | Accessibility maintainers | Consume the policy for native accessibility claims; adapter and consumer evidence must satisfy the applicable lane requirements and complete manifest rule. |
| OPT-1377 | Accessibility maintainers | Consume the policy for accessibility acceptance claims; evidence remains scoped to the exercised capability and requires a complete manifest for `PASS`. |
| OPT-1376 | CI and release maintainers | Consume the policy for CI evidence and release-gate classification; configured jobs describe available evidence sources until a qualifying manifest is retained. |
| OPT-1378 | Text and platform maintainers | Consume the policy for IME claims; deterministic or documented procedures do not replace the required native IME evidence and complete manifest. |
| OPT-1375 | Performance maintainers | Consume the policy for performance claims; existing workload and threshold contracts remain authoritative, with results classified only from their applicable lane and manifest. |
| OPT-1418 | Runtime and performance maintainers | Consume the policy for fairness claims; native multi-window fairness remains scoped to the scheduler contract, applicable environment, and complete manifest. |
| OPT-1381 | Platform and release maintainers | Perform the final policy audit; audit conclusions depend on the mapped evidence being correctly scoped and manifest-backed. |
| OPT-1417 | Platform adapter maintainers | Consume the policy for external-drag claims; documented procedures remain non-PASS until the applicable native result has a complete manifest. |
| OPT-1384 | Runtime maintainers | Separate trace consumer; deterministic A evidence must not be described as trace replay until this boundary exists. |
| OPT-1385 | Runtime/platform maintainers | Later environment expansion; native capability claims must continue to record the exact environment fields available at the time. |
| OPT-1386 | Text/platform maintainers | Locale and writing-direction services; text/IME claims remain limited to the currently shipped capability until this work lands. |
| OPT-1402 | Text/rendering maintainers | Bidi and complex shaping; text/font completion cannot use current Unicode-scalar evidence as proof of this future capability. |
| OPT-1362, then OPT-1400, OPT-1398, OPT-1397, OPT-1399, and OPT-1401 | Virtual-layout/runtime/accessibility maintainers | First-class virtual collection, focus, accessibility, and production-consumer sequence; private bridge evidence remains scoped and does not satisfy those later native claims. |
| OPT-1407 | GPU/API maintainers | Compatibility decision for the target `CanvasProgram`/`CanvasGraph` boundary; do not infer a new public GPU acceptance surface. |
| OPT-1408 | GPU/runtime maintainers | Implementation of the target canvas program/graph boundary; GPU rows remain capability-conditional until the adapter can produce the required evidence. |
| Unnamed Ubuntu Wayland and Windows H/N/M lanes | Platform and release maintainers | Create or route the future infrastructure work when appropriate. Until then, record `H/NOT_RUN`, `N/NOT_RUN`, or `M/NOT_RUN`/`UNAVAILABLE` with the missing prerequisite rather than inventing a ticket or claiming acceptance. |

The map is an ownership aid, not a second issue tracker. Ticket state, review
state, and release state remain in their authoritative systems.
