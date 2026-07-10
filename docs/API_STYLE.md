# Radiant API Style

This document defines the preferred style for Radiant application-facing APIs,
examples, and cleanup work. It complements `docs/API.md`, which documents the
current API surface, and `docs/ARCHITECTURE.md`, which explains internal module
ownership.

The purpose is to give contributors and cleanup agents a stable target for what
"good Radiant API" means before they open implementation tickets.

## Reference Point

The Linebender Xilem examples are useful reference material because they show a
Rust GUI API that keeps small examples direct and readable:

- `https://github.com/linebender/xilem/tree/main/xilem/examples`
- `to_do_mvc.rs` demonstrates compact application structure and dynamic rows.
- `components.rs` demonstrates state modularity with lenses.
- `elm.rs` demonstrates explicit action/message mapping when a component should
  emit intent for a parent to handle.
- `lists.rs` demonstrates dynamic children without much framework ceremony.

Radiant should learn from those examples without copying Xilem's exact model.
Radiant already has its own explicit runtime commands, `UiUpdateContext`, retained
surface, custom widget, paint-plan, and host-integration contracts. The desired
outcome is the same readability: application code should reveal UI structure,
state ownership, and emitted intent without forcing readers into internals.

## North Star

Radiant application code should read as a direct declaration of UI structure and
emitted intent. Application state changes and side effects should remain
explicit in the host's update or message handler.

Good Radiant code should let a reader answer these questions quickly:

1. What is on screen?
2. How is it arranged?
3. What stable identity does dynamic UI use?
4. What message fires when the user interacts?
5. Where does durable state change?
6. Where do side effects, background work, focus changes, or repaint requests
   happen?

## Locked Decisions

These decisions define the current cleanup target for Radiant and should guide
API audits, issue creation, and implementation work:

- Radiant API compatibility may be broken while Radiant is still vendor-owned
  and only consumed by Wavecrate. Prefer one clean API over a compatibility
  layer that preserves mixed old and new styles.
- Do not add migration artifacts, aliases, or half-compatible wrappers unless
  there is a concrete short-term safety reason. When the better API breaks
  Wavecrate, update Wavecrate to the better API.
- Radiant examples are part of the public API contract. Important public API
  patterns need clear examples under `examples/`.
- Radiant is message-first. Views emit explicit messages; update handlers own
  durable state changes and side effects. Do not document direct state callbacks
  as a canonical application style.
- Normal `.view(...)` projections borrow host state immutably. Derived host
  state must be prepared before launch or in update handlers rather than during
  view construction.
- Use `on_click`, `on_activate`, `on_secondary_activate`, `on_change`,
  `on_drag`, and similar intent-oriented names for interaction hooks only when
  those hooks emit explicit messages or generic widget messages. Avoid names
  that expose low-level event mechanics in ordinary app code.
- Cleanup passes should produce the top five highest-value tickets by default,
  with strict dependency ordering when one issue unlocks another.
- Cosmetic and structural cleanliness are valid cleanup reasons when they
  affect readability or maintenance: oversized files, broad import facades,
  unclear module grouping, mixed naming, and long hard-to-scan identifiers are
  real API-quality problems.

## Public API Principles

- Prefer `radiant::prelude::*` for normal application code and examples.
- Keep the common path declarative: structure first, behavior second, styling
  third.
- Keep one mental model. Advanced modules such as `radiant::runtime`,
  `radiant::widgets`, `radiant::layout`, `radiant::theme`, and `radiant::gui`
  should extend the same API model, not form a parallel framework.
- Host applications own domain state, persistence, files, audio/plugin hosts,
  product names, and other side effects.
- Radiant owns generic GUI structure, layout, widget input, focus, stable
  identity, style resolution, invalidation, paint plans, overlays, and runtime
  coordination.
- Reusable behavior belongs in Radiant only when it is a primitive UI element or
  generic GUI building block useful beyond one host application.
- Application-specific composites belong in the host. A host may build a sample
  browser, tag library, plugin preset panel, or todo filter bar from Radiant
  primitives, but those product-shaped components should not become Radiant API.
- Keep APIs strongly typed and explicit. Avoid stringly typed routing except for
  stable widget keys or labels where a string is the natural domain value.
- Prefer narrow public exports. A type should enter `radiant::prelude` only
  when normal app code benefits from seeing it by default.
- Keep payload and customization types required by common builder signatures in
  the prelude with those builders; reserve explicit owner-module imports for
  specialist helpers and lower-level implementation APIs.

## Radiant Admission Test

Before adding a widget, helper, state model, or interaction API to Radiant, ask:

1. Could several unrelated applications use this without inheriting another
   product's vocabulary or workflow?
2. Is the API driven by host-provided data and host-owned messages rather than
   product-specific state?
3. Does it expose a reusable GUI primitive, behavior, or composition pattern
   rather than a finished product feature?
4. Can it be documented without mentioning Wavecrate, samples, tags, DAWs,
   plugins, todos, or another host domain?

If any answer is no, keep the code in the host application. Improve Radiant only
by extracting the generic lower-level pieces: layout, input routing, stable
identity, overlay placement, virtualization, paint helpers, focus behavior, or
primitive widget state.

Examples:

| Belongs in Radiant | Belongs in the host application |
| --- | --- |
| `button`, `text_input`, `slider`, `toggle` | `SampleRenameButton`, `PresetSearchBox` |
| virtualized list viewport math | sample-library filtering policy |
| generic tree row disclosure and selection | Wavecrate source-folder semantics |
| context-menu positioning and dismissal | tag-delete business rules |
| waveform-like scalar paint primitive | sample-extraction success workflow |
| status-line segments | product-specific status copy and actions |

## Canonical View Shape

For serious application surfaces, prefer this shape:

```rust
use radiant::prelude::*;

#[derive(Clone, Debug, PartialEq)]
enum Message {
    SelectRow(RowId),
    TogglePlayback,
}

fn view(state: &AppState) -> View<Message> {
    column([
        header_row(state),
        list(state.visible_rows().iter(), row_view).fill_height(),
        transport_row(state),
    ])
    .padding(12.0)
    .spacing(8.0)
}

fn row_view(row: &VisibleRow) -> View<Message> {
    list_row(
        row.id,
        [
            text(row.label.clone()).key("label").fill_width(),
            button("Select")
                .message(Message::SelectRow(row.id))
                .key("select"),
        ],
    )
}

fn update(state: &mut AppState, message: Message) {
    match message {
        Message::SelectRow(id) => state.select_row(id),
        Message::TogglePlayback => state.toggle_playback(),
    }
}
```

The exact return type may vary, but normal application-facing views should use
`View<Message>` or another message-first documented view type. The structure
should stay obvious: app state is read to build the view, interactions emit
intent, and the update path owns durable state changes.

## Message Model

Radiant's canonical application model is message-first:

- views read state and declare UI structure
- widgets emit explicit host messages or generic widget messages
- update handlers own durable state changes
- `UiUpdateContext` owns UI-safe runtime follow-up requests
- `context.business()` owns host business work that must leave the UI path
- tests can assert emitted intent separately from state mutation

Do not add or document direct state-callback APIs as the preferred style. If an
older callback-style API exists, cleanup work should replace ordinary
application usage with message-first routing instead of preserving two equal
styles.

Update handlers are UI reducers, not business executors. They may mutate host
state, apply business or platform-service results, emit messages, request
repaint/focus/timers, request typed platform services, and schedule business
work. They must not perform filesystem, database, decode/load, network,
process, cache hydration, blocking waits/joins, sleeps, thread creation, or
long CPU work directly. Examples that need those behaviors should show
`context.business()` or typed platform-service requests instead of direct
blocking calls from the handler. When the lane is static, prefer the named
business helpers such as `interactive(...)`, `background(...)`,
`blocking_io(...)`, and `idle(...)`; when host policy already produced a
`TaskPriority`, use `context.business().priority(name, priority)` so the
business boundary remains explicit without app-local lane-selection boilerplate.

Example:

```rust
button("Start")
    .primary()
    .message(LoadingMessage::Start);

radiant::app(LoadingState::default())
    .view(view)
    .handle_message(update);
```

## Layout Style

- Prefer `column([...])` and `row([...])` for short fixed child lists.
- Prefer `children().push(...).push_opt(...).push_if(...)` when optional
  branches make arrays or tuples harder to scan.
- Prefer `workspace_shell(main)` when a serious app surface has the common
  top-bar, workspace-row, optional sidebars, and status-bar shape. Keep panel
  state and product-specific region contents in the host; the Radiant builder is
  only the readable generic shell composition contract.
- Prefer `list(items.iter(), row_view)` or another documented list primitive
  for repeated homogeneous rows.
- Dynamic rows must use stable identity from host-owned IDs. Avoid index-based
  identity when rows can be reordered, filtered, inserted, or removed.
- Put layout modifiers near the container they affect. A reader should not have
  to inspect child widgets to understand row/column spacing and padding.
- Use fixed dimensions only when the surface needs stable control size, table
  alignment, custom paint bounds, or deterministic tests.
- Avoid building hidden offscreen rows for large collections. Use Radiant's
  virtual-list contracts for large lists, trees, tables, and browsers.

## Interaction Style

- Widget names should describe user intent, not implementation mechanism.
  Prefer `on_click` for literal pointer/button clicks, `on_activate` for
  keyboard-or-pointer activation, `on_secondary_activate` for secondary actions,
  `on_change` for value changes, `on_drag` for drag gestures, or `.message(...)`
  for direct message emission. These APIs should emit explicit messages rather
  than mutate host state inline. Avoid names that expose low-level event
  plumbing in ordinary app code.
- Primary activation, secondary activation, drag, drop, hover-drop, and context
  menus should be expressible through shared interaction primitives where the
  behavior is generic.
- Do not make host code hand-filter low-level widget output for common
  interactions. If several app surfaces repeat that routing, open a Radiant API
  cleanup ticket.
- Side effects belong in the host handler. Radiant widgets should emit intent
  or generic widget messages, not perform application-specific work.
- Component-owned overlays should be declared near the component that owns them.
  Root-owned layers should be reserved for deliberate app-level policy.

## State And Component Style

- Examples and application surfaces should route durable state changes through
  explicit messages and update handlers.
- Larger surfaces should split state by domain responsibility and project views
  through focused functions.
- Component extraction is desirable when it gives a named owner to state,
  messages, or reusable layout. Do not extract tiny one-off wrappers just to
  reduce line count.
- If child components need to emit parent-handled intent, prefer an explicit
  message/action type over hidden state mutation.
- Keep runtime-local GUI state in Radiant: focus, hover, pointer capture,
  scroll, invalidation, frame requests, retained surfaces, and paint caches.
- Keep durable product state in the host application.

## Example Style

Radiant examples are part of the public API contract. Each example should have
one clear reason to exist.

Good example categories:

- hello world and no-state startup
- explicit message routing
- background work with `context.business()`
- stable keyed lists
- virtualized lists
- overlays and context menus
- custom widgets and paint-plan tests
- native file drop
- multi-window or embedded host integration

Advanced synthetic domain simulations may stay under `examples/` when they
validate Radiant behavior that smaller examples cannot stress well, such as
dense custom-widget painting, high-frequency pointer drags, paint-only overlays,
retained GPU surfaces, or multi-pane workspace composition. They must be
documented as non-authoritative domain simulations, not as the default shape of
Radiant application code. The domain model should never be read as Radiant-owned
business logic.

Examples should avoid mixing too many teaching goals. If an example demonstrates
background loading, it should not also be the canonical styling, list, and
overlay example.

Every major public API family should have at least one maintained example that
shows the preferred style. An API that cannot be demonstrated clearly in an
example is not ready to be treated as a clean public API.

Use comments sparingly. Prefer names that make the structure self-explanatory.
Comments should explain why a pattern matters, not restate what each builder
call does.

## Documentation Style

`docs/API.md` should describe the current public API and the canonical way to
use it. It should not become an unstructured inventory of every helper.

When adding or changing a public API:

1. Add or update the smallest representative example.
2. Update `docs/API.md` if the public contract changed.
3. Update this document only if the preferred style changed.
4. Update `docs/ARCHITECTURE.md` if ownership or internal structure changed.
5. Add focused tests that protect the behavior.

Breaking API changes are acceptable when they remove ambiguity, simplify the
public model, or eliminate technical debt. Do not preserve a weaker old API just
to avoid updating Wavecrate call sites.

Prefer before/after examples in cleanup tickets when the issue is API
readability. A good ticket should show what app code should look like after the
cleanup, not just name an internal module to refactor.

## API Cleanup Audit Protocol

Use this protocol when creating Radiant API cleanup issues:

1. Read `docs/API_STYLE.md`, `docs/TARGET.md`, `docs/API.md`, and
   `docs/ARCHITECTURE.md`.
2. Classify the maintained examples by teaching purpose, such as no-state app,
   message-first state, `UiUpdateContext` follow-up, background work, stable
   keyed lists, virtualized lists, overlays, custom widgets, native file drop,
   and multi-window or embedded host integration.
3. Inspect current examples and Wavecrate call sites for drift from the
   message-first, primitive-boundary, and example-as-contract rules.
4. Identify the top five highest-value cleanup issues by API confusion,
   ownership drift, readability cost, and downstream cleanup leverage.
5. Search Linear for duplicates before creating issues.
6. Create implementation-ready Linear issues with concrete before/after API
   examples, affected paths, non-goals, validation, and definition of done.
7. Add strict dependencies when one issue unlocks another. Do not hide ordering
   in prose when `blockedBy` / `blocks` links should exist.

Default example classifications:

| Example | Contract |
| --- | --- |
| `hello_world.rs` | no-state startup |
| `counter.rs` | smallest message-first stateful app |
| `message_routing.rs` | message routing with `UiUpdateContext` follow-up |
| `background_loading.rs` | `context.business()` and background resources |
| `keys.rs` | stable row identity |
| `virtualized_list.rs` | large-list viewporting |
| `context_menu.rs` | context menu and overlay routing |
| `custom_widget.rs` | custom widget authoring |
| `native_file_drop.rs` | view-local native file drop |
| `multi_window_manifest.rs` | multi-window app structure |
| `plugin_panel.rs` | advanced synthetic control-panel simulation, not plugin host policy |
| `eq_editor.rs` | advanced synthetic curve-editor simulation, not DSP policy |
| `mixer_console.rs` | advanced synthetic dense-panel simulation, not mixer semantics |
| `piano_roll.rs` | advanced synthetic retained-editor simulation, not MIDI or DAW editing policy |
| `modulation_matrix.rs` | advanced synthetic matrix simulation, not synth-routing semantics |
| `arrangement_shell.rs` | advanced synthetic multi-pane workspace simulation, not arrangement or audio policy |

## Cleanup Ticket Criteria

Create a Radiant API cleanup ticket when repository evidence shows one of these
problems:

- app code must import internals for ordinary UI construction
- the same common interaction has multiple competing public patterns
- a builder chain hides emitted intent or state mutation
- an example teaches a pattern that should not be copied into real apps
- ordinary app code mutates host state inline from view callbacks instead of
  emitting explicit messages
- dynamic UI lacks stable identity
- common layout, overlay, input, or paint logic is duplicated in host code
- `docs/API.md` documents helpers without explaining the canonical path
- public names expose implementation details instead of user or app intent
- Wavecrate-specific concepts leak into Radiant
- product-shaped composite widgets or workflow state have drifted into Radiant
- Radiant examples do not cover a public feature that app code relies on
- files, modules, imports, or names are visually hard to scan and make API
  ownership difficult to understand

Cosmetic cleanup is valid when it improves readability, naming clarity, module
grouping, import hygiene, or API discoverability. Do not create tickets for
pure formatting preference alone; the issue should improve readability,
correctness, testability, API consistency, or ownership.

## Non-Goals

- Do not copy Xilem's architecture wholesale.
- Do not create a second "simple API" that diverges from Radiant's advanced
  runtime model.
- Do not move Wavecrate domain concepts into Radiant to make one app call site
  shorter.
- Do not add product-shaped composite widgets to Radiant when the right fix is
  better host composition from primitive Radiant building blocks.
- Do not hide side effects inside declarative view builders.
- Do not replace explicit Rust types with broad dynamic configuration objects
  just to reduce syntax.
- Do not add macros for ordinary UI composition unless there is clear evidence
  that builders cannot express the desired API cleanly.
