# Text Shaping Architecture

This is the durable decision record for Radiant's future Unicode text layout.
It selects the stack and ownership boundaries; it does not claim that the
future product implementation is already shipped. The current single-line
`PaintTextRun` and `TextEditCommand` contracts remain compatible throughout the
migration.

## Decision

The only geometry authority will be a future crate-private immutable shared
paragraph snapshot. Paint, hit testing, IME, and accessibility will consume
the same snapshot rather than independently measuring or reshaping text. The
snapshot is immutable after publication and carries the source text, paragraph
style, resolved font runs, bidi runs, line records, glyph placement, and stable
logical-to-visual geometry.

The selected implementation stack is:

- `unicode-segmentation` UAX #29 extended grapheme clusters for user-visible
  boundaries and cluster construction.
- `unicode-bidi` UAX #9 paragraph levels and visual runs.
- Existing `NativeTextOptions`, `NativeFontStack`, and `skrifa` ordered
  embedded-font, path-font, environment-font, and platform-fallback policy for
  font resolution.
- `rustybuzz` 0.20.x for HarfBuzz-compatible shaping, once per resolved
  font/style/bidi run. Shaping output is retained in the paragraph snapshot;
  it is not recomputed by each consumer.
- `unicode-linebreak` 0.1.5 as the pure-Rust UAX #14 provider, pinned to its
  embedded Unicode 15.0.0 line-break tables and default Complex-Context
  Dependent (SA) to Ordinary Alphabetic (AL) tailoring. The adapter owns this
  selected ruleset and makes its decisions part of the snapshot; it does not
  delegate line breaking to a platform API or choose a provider at runtime.
- The existing Vello/`skrifa` renderer adapter, unchanged as the rendering
  boundary. It consumes snapshot glyph placements and continues to produce the
  existing text paint representation.

This is a future crate-private contract, not a new public text-layout API.

## Paragraph and line model

A paragraph snapshot contains UTF-8 source text, paragraph direction and style,
resolved font/style runs, UAX #9 levels and visual runs, UAX #29 grapheme
boundaries, UAX #14 break opportunities, and ordered lines. Each line records
its logical byte interval, visual run order, baseline and metrics, width, and
glyph placements. A glyph placement records its font/run identity, glyph ID,
cluster byte interval, advance, offset, and visual position.

The model distinguishes logical order from visual order. A line may contain
multiple bidi runs and a cluster may span multiple UTF-8 code points. Newline,
empty-paragraph, trailing-break, clipping, and single-line cases are explicit
line-policy outcomes rather than special measurements hidden in paint code.

Every grapheme, cluster, caret stop, selection range, and line has stable byte
and cluster coordinates into the immutable source snapshot. Caret geometry
stores both logical stops and their visual positions, including bidi affinity.
Range geometry is derived from the same stops and line records. Paint consumes
glyph placements; hit testing consumes visual caret/cluster geometry; IME and
accessibility consume logical byte/cluster ranges and the corresponding visual
rectangles. No consumer may establish a competing width or coordinate system.

### Line-break policy contract

The future crate-private `LineBreakPolicy` adapter is pinned to the
`unicode-linebreak` 0.1.5 provider and its embedded Unicode 15.0.0 UAX #14
ruleset with the default Complex-Context Dependent (SA) to Ordinary Alphabetic
(AL) tailoring. It accepts an immutable `LineBreakPolicyInput` containing the
paragraph's UTF-8 source, its stored `GraphemeBoundary` to `Utf8ByteOffset`
map, and an explicit wrapping mode (`NoWrap` or `SoftWrap`). It does not read
platform locale services, installed font state, or renderer state. Available
inline width and shaped cluster advances are inputs to the later deterministic
line fitter, not to the UAX #14 provider, so changing width can reuse valid
shaping and break classification.

The adapter returns a crate-private `LineBreakDecision` containing an ordered,
deduplicated list of break records. Each record contains an exact
`GraphemeBoundary`, its matching `Utf8ByteOffset`, and one of two kinds:
`Mandatory` (hard) or `Allowed` (soft). Provider byte positions are accepted
only when they map to stored UTF-8 and grapheme boundaries; an invalid or
unsupported result is rejected as one policy failure rather than published as
partial geometry. The decision always includes the terminal end-of-text
`Mandatory` opportunity; that sentinel is distinct from a trailing newline.

`Mandatory` opportunities always end the current line and cannot be suppressed
by available width. The provider's newline handling, including a CRLF pair, is
preserved as one hard-break decision, while empty paragraphs and a trailing
hard break remain explicit line-policy records. `Allowed` opportunities are
soft candidates only: `SoftWrap` mode lets the line fitter choose the last
candidate that fits before overflow, and `NoWrap` mode ignores them. Neither
mode may break inside an extended grapheme or shaping cluster; when no soft
candidate is available, the fitter keeps the unbreakable cluster intact and
uses the deterministic overflow outcome rather than inventing a scalar break.

If the selected provider is unavailable or cannot represent the input, hard
breaks remain hard and unsupported soft wrapping becomes the same complete
deterministic no-wrap fallback described in the fallback policy; no platform
provider is substituted.

## Logical coordinate compatibility bridge

The snapshot uses distinct typed logical coordinates; a scalar boundary, byte
offset, grapheme boundary, and shape cluster are not interchangeable:

- `ScalarBoundary` is an integer in `0..=scalar_count`. It identifies a
  boundary before scalar `n`, including the terminal boundary; it is not a
  code-point or byte index into the middle of a UTF-8 scalar. This is the
  coordinate currently stored by `TextInputState`.
- `Utf8ByteOffset` is an integer in `0..=text.len()` that identifies a UTF-8
  boundary. `ScalarBoundary(n)` converts by enumerating `text.char_indices()`
  and taking the `n`th scalar start, with `text.len()` for the terminal
  boundary. The reverse conversion accepts only `0`, `text.len()`, or an
  offset returned by `char_indices()`; an arbitrary interior byte is first
  normalized to the previous or next scalar boundary according to explicit
  upstream/downstream affinity, never guessed as a scalar index.
- `GraphemeBoundary` is an ordinal boundary in the UAX #29 extended grapheme
  sequence. Its snapshot record carries the corresponding UTF-8 byte interval
  `[start, end)`, obtained from `grapheme_indices(true)`; it is the user-visible
  caret and selection coordinate.
- `ShapeClusterRange` is the logical UTF-8 byte interval associated with a
  shaping cluster. Multiple glyphs may share one cluster start, and a shaping
  cluster may cover multiple scalars; a glyph-array index is never a logical
  coordinate.

The bridge first maps scalar boundaries to exact UTF-8 boundaries through
`char_indices()`, then maps byte boundaries to the stored grapheme and shape
cluster intervals. A legacy scalar boundary inside one UAX #29 extended
grapheme is canonicalized explicitly: `Upstream` (also called leading)
selects that grapheme's start, while `Downstream` (also called trailing)
selects its end. A boundary already at a grapheme edge is kept unchanged. The
same affinity is carried with a caret stop so canonicalization is deterministic
even when old scalar editing state points inside a cluster.

Selection normalization converts both legacy scalar endpoints, orders the
result by logical UTF-8 byte offset, and uses upstream/leading affinity for the
lower endpoint and downstream/trailing affinity for the upper endpoint. The
published range is therefore an ordered `[start, end)` over whole grapheme
boundaries; a reversed input has the same normalized geometry and a partial
grapheme is never exposed to a consumer.

At a bidi run edge, both possible visual caret positions remain available in
the snapshot. Upstream/leading affinity selects the trailing edge of the
preceding logical run; downstream/trailing affinity selects the leading edge of
the following logical run. At a paragraph or line edge the only available edge
is used. “Preceding” and “following” are logical-order terms, not left/right
terms, so RTL run edges resolve their visual position from the run direction
rather than from an x-coordinate guess.

Within one paragraph revision, conversions round-trip exactly at valid scalar,
grapheme, and shape-cluster boundaries: scalar -> byte -> scalar returns the
same scalar boundary, and a stored cluster interval -> byte endpoints -> stored
cluster returns the same logical range. An interior legacy scalar boundary
intentionally round-trips to the affinity-selected canonical grapheme edge,
not to the discarded interior position. A revision mismatch invalidates the
conversion instead of applying old byte or cluster offsets to new text.

OPT-1402 owns this compatibility bridge together with the shaping engine,
immutable snapshot, fallback publication, and cursor-stop mapping. OPT-1403
records the contract only; it does not change `TextInputState`, `PaintTextRun`,
or `TextEditCommand`.

## Caches and ownership

The layout cache has separate shape and width identities. A shape key includes
the relevant text/run content or revision, resolved font identity and face
instance, script/language/direction, bidi level, variation/style features, and
shaping policy. The selected line-break policy has the crate-private stable
identity
`uax14:unicode-linebreak@0.1.5:unicode@15.0.0:default-sa-to-al:v1`.
`LineBreakPolicyId` is this literal value, not a provider pointer, platform
locale, or dependency-resolution result. A width key includes the shape
identity plus available inline width, this exact break-policy identity, the
wrapping mode, paragraph spacing, and line-direction policy.

The identity is stable across supported platforms for the selected tables and
tailoring. Any provider, Unicode table, tailoring, hard/soft-break, or adapter
semantic change must issue a new policy identity and invalidate old width
entries; equal identities mean that the adapter's break classification and
hard/soft behavior are equal. The policy input/output and identity are
crate-private implementation contracts, not a new public text-layout API.

Changing width or wrapping policy reuses valid shaping; changing text, font,
style, script, language, direction, or shaping features invalidates shaping.
The UI runtime owns a bounded cache and the currently published snapshot for
each live text surface. Publication and replacement occur on the UI/runtime
ownership lane; shaping and width work may be prepared off-lane only when the
runtime can publish an immutable result fenced to the current text/style
revision. Invalidation is coalesced and bounded to affected surfaces. A stale
result is discarded, never used for paint, input, IME, or accessibility, and
cache growth is bounded by explicit entry and resource budgets.

## Fallback and diagnostics

Font resolution preserves the existing ordered policy. The candidate tiers are,
in order:

1. valid `NativeTextOptions::embedded_fonts`, in configured order;
2. valid `NativeTextOptions::font_paths`, in configured order;
3. `RADIANT_NATIVE_FONT_PATH`, when set;
4. approved platform fallback candidates.

Candidates are loaded or consulted lazily as glyph misses require them, and the
first eligible face in this order supplies the affected grapheme/run. The
environment path is therefore between configured paths and platform fallback;
it is not a replacement for either tier. This is an existing compatibility
input: when set, changing its path or font bytes can change glyph coverage,
metrics, wrapping, caret positions, and range geometry, while removing or
reordering it would be a compatibility change. For equal options, environment,
candidate bytes, and provider identity, resolution remains deterministic. A
future snapshot must capture the resolved face identity in the shape identity
and must not reread the environment or platform candidates after publication.

A missing glyph is retried through the next eligible face for the affected
grapheme/run. The unsupported behavior is selected and atomic, not merely
documented:

- If shaping is unavailable or unsupported, the engine constructs and publishes
  one complete immutable compatibility snapshot using the current deterministic
  scalar, single-line, no-wrap layout. No partially shaped paragraph, partial
  line list, or mixed shaped/fallback geometry is published.
- If the requested soft line-break policy is unsupported, hard breaks remain
  hard breaks and soft wrapping is disabled. Each hard-break-delimited segment
  uses the deterministic no-wrap fallback, so every line record and consumer
  sees the same geometry.
- For a missing glyph, ordered faces are exhausted before fallback is chosen.
  If no eligible face covers the affected logical grapheme/run, the snapshot
  emits one deterministic replacement/tofu glyph with a stable advance and one
  logical byte/scalar/cluster range for that missing content, rather than
  emitting an unbounded per-code-point sequence. The fallback glyph and its
  range are part of the same immutable snapshot.

Fallback construction is complete before publication. Paint, hit testing,
caret/selection, IME, and accessibility consume that one compatibility
snapshot; no consumer may measure again or combine fallback geometry with a
different layout. OPT-1402 owns the engine/snapshot fallback and its atomic
publication. OPT-1367 and OPT-1406 are consumers of the shared fallback
geometry for IME composition and text-range accessibility respectively.

Every fallback diagnostic is bounded in count and payload and contains the
paragraph revision, category (`ShapingUnavailable`, `UnsupportedLineBreak`, or
`MissingGlyph`), logical byte/scalar/cluster range, face identity and policy
identity, and the fallback outcome. The identity fields are stable policy or
ordered-face labels; platform-private font internals are not exposed as public
API.

## Compatibility and non-goals

The stack must remain usable on Radiant's supported platforms and build modes.
Embedded bytes and configured paths remain portable inputs; the environment
font path and platform fallback are explicit ordered inputs. `rustybuzz` 0.20.x
is MIT licensed; `unicode-linebreak` 0.1.5 is Apache-2.0 licensed;
`unicode-bidi` and `unicode-segmentation` are MIT OR Apache-2.0. Existing
`skrifa`/Vello license obligations remain subject to the repository dependency
audit; this record does not claim that a full audit was done. The checked-in
spike exercises determinism only; it is not full product shaping validation.

This decision does not replace Vello, implement a complete text engine, or ship
a full locale database. It does not change the current single-line behavior,
`PaintTextRun`, `TextEditCommand`, or logical Unicode-scalar editing contract.
It does not make shaping, multiline editing, locale services, or accessibility
integration public before their staged implementation and acceptance work.

## Follow-on work

OPT-1403 is architecture-only and does not implement any of these follow-ons.

- OPT-1402 owns the retained shaping/paragraph snapshot, scalar compatibility
  bridge, selected fallback behavior, renderer/text-layout integration, and
  cursor-stop mapping while preserving the current single-line compatibility
  surface.
- OPT-1404 implements the multiline `TextEditor` and typed-edit consumer.
- OPT-1367 implements native IME adapters and matching-key suppression, and
  consumes shared geometry, including the selected fallback, for composition
  placement.
- OPT-1406 implements text-range accessibility semantics and consumes shared
  range geometry, including the selected fallback.
- OPT-1386 supplies locale and writing-direction policy.

Each follow-on must preserve this snapshot as the only geometry authority and
must not broaden the renderer boundary or introduce a second measurement path.
Cache invalidation follows the snapshot/shape implementation owner where
appropriate.
