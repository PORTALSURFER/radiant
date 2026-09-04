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
  embedded-font, path-font, and platform-fallback policy for font resolution.
- `rustybuzz` 0.20.x for HarfBuzz-compatible shaping, once per resolved
  font/style/bidi run. Shaping output is retained in the paragraph snapshot;
  it is not recomputed by each consumer.
- An explicit UAX #14 line-break policy adapter. The adapter owns the selected
  break policy and makes its decisions part of the snapshot; line breaking is
  not an incidental renderer behavior.
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

## Caches and ownership

The layout cache has separate shape and width identities. A shape key includes
the relevant text/run content or revision, resolved font identity and face
instance, script/language/direction, bidi level, variation/style features, and
shaping policy. A width key includes the shape identity plus available inline
width, break policy identity, paragraph spacing, and line-direction policy.

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

Font resolution follows the existing ordered policy: embedded fonts, configured
font paths, then approved platform fallback. A missing glyph is retried through
the next eligible face for the affected grapheme/run; fallback never silently
changes the paragraph's geometry authority. If no face covers the content, the
snapshot records a deterministic diagnostic and replacement/tofu outcome with
stable geometry. Diagnostics identify the paragraph revision, run, code-point
range, and fallback decision without exposing platform-specific font internals
as public API. Unsupported shaping or line-break input is reported as a
bounded diagnostic and uses the documented deterministic fallback policy.

## Compatibility and non-goals

The stack must remain usable on Radiant's supported platforms and build modes.
Embedded bytes and configured paths remain portable inputs; platform fallback is
an explicit last-resort policy. `rustybuzz` 0.20.x is MIT licensed;
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

- OPT-1402 integrates the retained shaping/paragraph snapshot with
  renderer/text-layout and cursor-stop mapping while preserving the current
  single-line compatibility surface.
- OPT-1404 implements the multiline `TextEditor` and typed-edit consumer.
- OPT-1367 implements native IME adapters and matching-key suppression, and
  consumes shared geometry for composition placement.
- OPT-1406 implements text-range accessibility semantics and consumes shared
  range geometry.
- OPT-1386 supplies locale and writing-direction policy.

Each follow-on must preserve this snapshot as the only geometry authority and
must not broaden the renderer boundary or introduce a second measurement path.
Cache invalidation follows the snapshot/shape implementation owner where
appropriate.
