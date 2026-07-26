# Timer host API migration

Timer scheduling is now identity-only across the host boundary. Custom
`RuntimeTaskHost` implementations must replace `schedule_message(Duration,
Message)` with `schedule_timer(Duration, RuntimeTimerWake)`. The host stores or
forwards the opaque wake and must not construct, transport, reduce, or run a
mapper on its timer thread. The UI runtime receives the wake during its normal
drain turn, preserves FIFO order, validates its owner and generation/epoch, and
invokes the registered mapper before reducing any resulting message.

Custom bridges must also expose the opaque wake through `RuntimeQueueHost`:
`take_runtime_timer_wakes` (or the equivalent drain path) is the only timer
ingress consumed by `SurfaceRuntime`; pair it with
`map_runtime_timer_wake` for application-owned wakes. The bridge must preserve
wake FIFO and return controller-owned wakes without mapping them on the host
thread. A retained wake beyond the per-turn budget keeps
`runtime_work_remaining` and requests the next repaint; omitting the wake
ingress makes delayed commands silently disappear.

`Command::after`, `UiUpdateContext::after`, `UiUpdateContext::after_latest`,
and `Subscription::interval` call shapes remain unchanged for applications.
`LatestTask` remains application-owned UI state: custom hosts may use the same
ticket contract, but only the UI owner may validate a ticket, invoke its
mapper, or reduce its message.
