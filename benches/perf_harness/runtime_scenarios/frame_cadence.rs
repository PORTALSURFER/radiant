//! Runtime frame-cadence policy performance scenarios.

use crate::runner::ScenarioCounters;
use radiant::runtime::{RuntimeAnimationActivity, RuntimeAnimationDemand};
use std::hint::black_box;

const FRAME_CADENCE_STEPS: u64 = 1_024;
const NATIVE_TARGET_FPS: u32 = 120;

pub(super) fn animation_frame_cadence_1k() -> impl FnMut() -> ScenarioCounters {
    let mut bench = AnimationFrameCadenceBench::new();
    move || bench.step()
}

struct AnimationFrameCadenceBench {
    tick: u64,
}

impl AnimationFrameCadenceBench {
    fn new() -> Self {
        Self { tick: 0 }
    }

    fn step(&mut self) -> ScenarioCounters {
        let mut due = 0_u64;
        let mut wait = 0_u64;
        let mut paint_only = 0_u64;
        let mut checksum = 0_u64;

        for offset in 0..FRAME_CADENCE_STEPS {
            let activity = activity_for(self.tick.wrapping_add(offset));
            let target_fps = target_fps_for(activity);
            let interval_us = 1_000_000_u64 / u64::from(target_fps);
            let elapsed_us = self
                .tick
                .wrapping_add(offset)
                .wrapping_mul(137)
                .wrapping_rem(50_000);

            if activity.needs_animation() && elapsed_us >= interval_us {
                due += 1;
            } else if activity.needs_animation() {
                wait += 1;
            }
            if activity.needs_animation() && !activity.needs_frame_message() {
                paint_only += 1;
            }

            checksum ^= u64::from(target_fps)
                ^ elapsed_us
                ^ bool_counter(activity.needs_frame_message())
                ^ bool_counter(activity.needs_animation());
        }

        self.tick = self.tick.wrapping_add(1);
        assert!(due > 0);
        assert!(wait > 0);
        assert!(paint_only > 0);
        black_box((self.tick, checksum));

        ScenarioCounters::default()
            .with_frame_cadence_due_count(due)
            .with_frame_cadence_wait_count(wait)
            .with_paint_only_count(paint_only)
            .with_allocation_sensitive_work_count(FRAME_CADENCE_STEPS)
    }
}

fn bool_counter(value: bool) -> u64 {
    if value { 1 } else { 0 }
}

fn activity_for(index: u64) -> RuntimeAnimationActivity {
    match index % 6 {
        0 => RuntimeAnimationActivity::from_demand(RuntimeAnimationDemand::Idle),
        1 => RuntimeAnimationActivity::paint_only(),
        2 => RuntimeAnimationActivity::paint_only_at(30),
        3 => RuntimeAnimationActivity::frame_messages(),
        4 => RuntimeAnimationActivity::frame_messages_at(48),
        _ => RuntimeAnimationActivity::paint_only_at(240)
            .merge(RuntimeAnimationActivity::frame_messages_at(24)),
    }
}

fn target_fps_for(activity: RuntimeAnimationActivity) -> u32 {
    match activity.target_fps() {
        Some(target_fps) => target_fps.clamp(1, NATIVE_TARGET_FPS),
        None if activity.needs_animation() => NATIVE_TARGET_FPS,
        None => NATIVE_TARGET_FPS,
    }
}
