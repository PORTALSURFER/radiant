# Glyph-fallback font fixtures

`primary.ttf` and `secondary.ttf` are deterministic test-only subsets of
Sometype Mono Regular. The source is the existing Toybox asset at
`assets/Sometype_Mono/static/SometypeMono-Regular.ttf`, from the Sometype Mono
Project Authors (2018), licensed under the SIL Open Font License 1.1:
<https://openfontlicense.org>. The redistributed license text is in
`OFL.txt` beside these fixtures.

The subsets were generated with FontTools 4.x using:

```text
python3 -m fontTools.subset SometypeMono-Regular.ttf --unicodes='U+003F,U+0041' --output-file=primary.ttf
python3 -m fontTools.subset SometypeMono-Regular.ttf --unicodes='U+003F,U+03A9' --output-file=secondary.ttf
python3 -m fontTools.subset SometypeMono-Regular.ttf --unicodes='U+0041' --output-file=no_question.ttf
python3 -c "from fontTools.ttLib import TTFont; p='secondary.ttf'; f=TTFont(p); _, lsb=f['hmtx']['uni03A9']; f['hmtx']['uni03A9']=(900, lsb); f.save(p)"
```

The primary face contains `?` and `A` but not `Ω`; the secondary face contains
`?` and `Ω` but not `A`. The fixtures are not product assets and are retained
only to make fallback order, per-face metrics, and true-missing diagnostics
deterministic in tests. `no_question.ttf` contains `A` but no replacement glyph.
