# Final primitive qualification controls

These three runs repeat the parent fixture at
`cb45cc431d277cba6e90fda3a2efa937724e5ed7`, after current-main integration and
restriction to verified text/button/text-input clone semantics. Exact counters
remain one enclosing projection, one component call and 31 hits for the cached
path, versus one enclosing projection and 32 component calls for fresh projection.

Cached averages span 149.310–172.720 µs and fresh averages 4251.910–4378.790 µs.
Cached p99 batch averages span 159.526–195.516 µs; fresh p99 batch averages span
4305.708–4761.297 µs. All runs are retained. The parent README describes the
fixture, ordering and measurement limits; these remain same-revision application
projection controls, with no native/frame/GPU timing claim.
