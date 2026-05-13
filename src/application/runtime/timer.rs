//! Runtime-owned timer lane for delayed UI messages.

mod lane;
mod queue;
mod timing;
mod worker;

pub(super) use lane::TimerLane;
pub(super) use timing::min_timer_delay;
