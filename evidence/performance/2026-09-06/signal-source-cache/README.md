# Native signal source-cache comparison

Observed 6 September 2026 on Apple M5 Pro / Metal. Both ignored native GPU tests passed; all ten complete 64x64 RGBA8 images match byte-for-byte across revisions (see pixel-parity.json). Each image is 16,384 bytes. This is an offscreen rendering/counter comparison, not a foreground frame-latency benchmark. No GPU timestamp queries or foreground window were used.

## Exact provenance

- Baseline production: a889af816a7f24c4cc442b60578027994c7da6bd; fixture commit: 74a12fa7382e6e06885f774aa69806268d3ffe74.
- Candidate production: edc0c3faffc72a3e276729635105c72bd14f266a; fixture commit: 942241d5caf69719f7fc7258b1b1b9cf39c85aa4.
- Identical fixture SHA256: 450f71122770704852548506835ad373ca8dce35b910d4a46f61775c52cc7be0. The retained source requires a cfg(test) module hook in gpu_surface.rs; the two fixture commits change only that hook and this source.

## Reproduction

Run sequentially with CARGO_INCREMENTAL=0 and the same CARGO_TARGET_DIR, touching src/lib.rs after switching worktrees. Set RADIANT_SIGNAL_EXPECT_SOURCE_CACHE=0 for baseline, 1 for candidate; RADIANT_COMPARISON_LABEL=baseline/candidate; RADIANT_COMPARISON_OUTPUT_DIR to a fresh directory. Run:

```sh
cargo test --locked --all-features --lib offscreen_signal_comparison -- --ignored --test-threads=1 --nocapture
```

## Observations

The deterministic raw fixture contains 65,536 frames and four bands. Nearby pan preserves the bucket interval: baseline rebuilds the summary and uploads immutable data; candidate has zero builds, one summary hit, and zero immutable uploads. Crossing the bucket interval or changing LOD uploads a new window while retaining the candidate summary. Replacing the source allocation or changing revision rebuilds the summary and uploads data.

The precomputed-summary fixture separately changes gain, fade, and nearby pan. Each produces changed pixels and a body render; baseline uploads immutable data for each, candidate does not. Initial and invalidating uploads contain 2,144 logical bytes. Full output parity covers raw initial/pan/interval/LOD/source/revision and summary initial/gain/fade/pan. JSON files retain the exact counters; adapter JSON retains the native adapter.

This establishes the immutable-upload and visual-equivalence acceptance for OPT-1453. Broader foreground latency and baseline packs remain separate OPT-1452/OPT-1460 work; these observations make no frame-time improvement claim.
