//! Runtime-owned timer lane for delayed UI messages.

mod lane;
mod queue;
mod registry;
mod timing;
mod worker;

pub(super) use lane::TimerLane;
pub(super) use queue::{TimerIdentity, TimerSink, TimerWake, timer_sink};
pub(super) use registry::TimerRegistry;
pub(super) use timing::min_timer_delay;
