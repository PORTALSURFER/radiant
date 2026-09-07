# Feedback and notifications

Feedback builders project application-owned state. `spinner().label(...)`, `skeleton().label(...).size(...)`, `status_badge(StatusSemantic::Success).label(...).value(...)`, and `inline_error(...).retry(message)` use ordinary text, badge, and button semantics. Labels and values retain at most 240 Unicode scalar values. Spinner and skeleton paint share one bounded central feedback clock; reduced motion uses static readable paint. They do not start tasks or choose retry policy. Existing `progress_bar` and `resource(snapshot)` compose with these builders.

`NoticeQueue` is application-owned data for at most 64 notices. `Notice::new(id, severity, message)` validates a 4096-byte message bound. An optional action has a 128-byte label and application command ID. Pushing the same ID replaces it in its insertion position, invalidates old action and dismissal tokens, and restarts timeout admission. Critical notices persist by default; other severities default to five seconds. Explicit positive timeouts are bounded to one day; `None` makes any notice persistent.

```rust
use radiant::application::{Notice, NoticeQueue, NoticeSeverity};
let mut queue = NoticeQueue::new();
queue.push(Notice::new(1, NoticeSeverity::Success, "Saved")?)?;
# Ok::<(), radiant::application::NoticeError>(())
```

Project `notifications(queue.snapshot()).on_dismiss(Message::Dismiss).on_action(Message::Action).layer()` into an ordinary `scene`. Apply dismissal with `queue.dismiss(&event)` and consume actions with `queue.take_action(&event)`. Both reject stale, duplicate, and foreign-queue tokens. Callbacks map events into ordinary application updates. Snapshot creation starts no timers and does not keep the queue alive.

The default visible limit is four, configurable from one through eight. Visible admission preserves insertion order so incoming notices cannot displace an existing actionable card. Severity controls the presentation. Waiting notices receive their full timeout only when admitted into the accepted visible projection. Logical corner placement follows normal layout direction. The floating layer sits below modals, adds no full-window input blocker, and never acquires focus on appearance.

One central runtime deadline tracks the accepted notice set. Timeout accounting pauses for hidden/occluded windows, omitted or offscreen cards, pointer hover, keyboard focus inside a card, and accepted modal content. Resume preserves the remaining duration. Projection removal retires timing; reappearance starts fresh admission. Expiry sends one typed dismissal request through the normal update path. If the application ignores it, the runtime does not repeatedly send the same expiry. Without an `on_dismiss` callback, notices remain persistent and have no dismiss control. Reduced motion changes feedback painting, not notice timeout durations.

Run `cargo run --example feedback_notifications` for a deterministic headless lifecycle fixture. Its tests cover timeout delivery, ignored dismissals, hidden/modal/hover/focus pauses, replacement, removal/reappearance, queued admission, and semantic text parity.
