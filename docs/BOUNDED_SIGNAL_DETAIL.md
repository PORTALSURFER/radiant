# Bounded signal detail preparation

The native signal preparation broker builds private, bounded products for raw
`GpuSurfaceContent::SignalBands`. This keeps large retained sample sources out
of a full level-zero summary pyramid while preserving the existing public
summary API.

## Products and selection

An overview scans raw samples directly into a power-of-two base level. Its base
has at most 4,096 buckets and its complete retained pyramid is at most 256 KiB.
It does not build a full level-zero pyramid and then slice it.

When an overview is ready, the broker can request a finer immutable detail tile
for the current raw frame range. A tile has at most 8,192 buckets and at most
256 KiB of min/max data. The picker quantizes the range into pages so nearby
pans reuse a tile instead of continuously creating products. If no CPU tile is
ready, CPU admission is unavailable, or the tile does not cover the request,
rendering uses the overview. GPU admission can instead produce the explicit
incomplete-surface fallback. It runs before transactional writes, and releasing
GPU capacity requests a retry for current targets.

The raw route has no slide or gain presentation fields: `SignalBands` contains
only frames, band count, frame range, and interleaved samples. Caller-owned
`SignalSummaryBands` remains its separate compatibility route.

## Ownership and limits

Source and revision identity form the shared key for prepared overviews and
tiles. A tile holds an already-accounted overview owner while it is queued or
active. Ready products use retention leases; retirement cancels active work and
keeps charged bytes until the final external lease drops.

The shared broker admits at most two active jobs, eight queued jobs, 64 sources,
and 128 targets. Its independent logical CPU limits are:

- raw retained samples: 256 MiB
- overview allocation: 16 MiB
- summary and detail pool: 64 MiB

Raw retention is accounted separately from the 64 MiB summary/detail pool.
Logical native GPU handle residency is limited to 128 MiB. This includes
signal buffers, body textures, and their composite uniforms. A composite binding
shares the body reservation while it retains that texture view. Replacement is
charged alongside the prior handles until committed cleanup releases them.

The 4,096-bucket overview base covers ordinary viewport widths while keeping
four-band overview storage below 256 KiB. Detail pages reserve room for nearby
scrolling and interpolation guards within the same per-product byte ceiling.
Band counts above four reduce the available bucket count instead of increasing
the allocation ceiling.

These are logical accounting limits. They exclude driver allocation size,
submission completion, and GPU fences; those physical-residency concerns remain
tracked by OPT-1374.

## Compatibility boundary

The public full-pyramid `GpuSignalSummary` constructors are unchanged. The
bounded overview and tile builders are private runtime implementation details.
The exact external-window presentation route introduced by OPT-1454 is a
separate path and is not described or configured by this broker.

This document records the implemented ownership and budget boundaries. It does
not claim completed performance characterization or verification.
