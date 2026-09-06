use std::{
    collections::{BTreeMap, BTreeSet},
    time::{Duration, Instant},
};

#[cfg(test)]
#[path = "engine/tests.rs"]
mod tests;

pub(super) const MAX_ACTIVE: usize = 256;
pub(super) const MAX_RETAINED: usize = 1024;
pub(super) const MAX_FEEDBACK_GROUPS: usize = 64;
pub(super) const MAX_FEEDBACK_INSTANCES: usize = 256;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Easing {
    Linear,
    EaseOut,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(super) struct TargetKey {
    pub owner: u64,
    pub property: u64,
}
#[derive(Clone, Copy, Debug)]
pub(super) struct Target {
    pub key: TargetKey,
    pub initial: Option<f64>,
    pub value: f64,
    pub duration: Duration,
    pub easing: Easing,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(super) struct FeedbackInstance {
    pub group: u64,
    pub owner: u64,
}
#[derive(Clone, Copy, Debug)]
pub(super) struct FeedbackTarget {
    pub instance: FeedbackInstance,
    pub period: Duration,
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct Diagnostics {
    pub active: usize,
    pub retained: usize,
    pub feedback_groups: usize,
    pub feedback_instances: usize,
    pub rejected: usize,
}
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Debug)]
pub(super) struct Frame {
    pub next_deadline: Option<Instant>,
    pub diagnostics: Diagnostics,
}

#[derive(Clone, Copy)]
struct Active {
    start: f64,
    target: f64,
    began: Instant,
    duration: Duration,
    easing: Easing,
    paused_at: Option<Instant>,
    paused: Duration,
}
#[derive(Clone, Copy)]
struct Group {
    period: Duration,
    began: Instant,
    paused_at: Option<Instant>,
    paused: Duration,
}

pub(super) struct Engine {
    active: BTreeMap<TargetKey, Active>,
    retained: BTreeMap<TargetKey, f64>,
    groups: BTreeMap<u64, Group>,
    instances: BTreeSet<FeedbackInstance>,
    rejected: usize,
    last_now: Option<Instant>,
}
impl Default for Engine {
    fn default() -> Self {
        Self {
            active: BTreeMap::new(),
            retained: BTreeMap::new(),
            groups: BTreeMap::new(),
            instances: BTreeSet::new(),
            rejected: 0,
            last_now: None,
        }
    }
}

impl Engine {
    pub(super) fn reconcile(
        &mut self,
        desired: &[Target],
        now: Instant,
        reduced: bool,
        hidden: bool,
    ) {
        let now = self.monotonic_now(now);
        if desired.len() > MAX_RETAINED {
            self.rejected = self.rejected.saturating_add(desired.len());
            self.active.clear();
            self.retained.clear();
            return;
        }
        let mut counts = BTreeMap::new();
        for target in desired {
            *counts.entry(target.key).or_insert(0usize) += 1;
        }
        let mut seen = BTreeSet::new();
        let desired_keys: BTreeSet<_> = desired.iter().map(|t| t.key).collect();
        self.active.retain(|key, _| desired_keys.contains(key));
        self.retained.retain(|key, _| desired_keys.contains(key));
        for target in desired {
            if !seen.insert(target.key) {
                self.rejected = self.rejected.saturating_add(1);
                continue;
            }
            if counts.get(&target.key) != Some(&1)
                || !target.value.is_finite()
                || target.initial.is_some_and(|value| !value.is_finite())
                || now.checked_add(target.duration).is_none()
            {
                self.active.remove(&target.key);
                self.retained.remove(&target.key);
                self.rejected = self.rejected.saturating_add(1);
                continue;
            }
            let current = self
                .value(target.key, now)
                .or(target.initial)
                .unwrap_or(target.value);
            if !self.retained.contains_key(&target.key) {
                if self.retained.len() >= MAX_RETAINED {
                    self.rejected = self.rejected.saturating_add(1);
                    continue;
                }
                self.retained.insert(target.key, current);
            }
            if reduced || target.duration.is_zero() {
                self.active.remove(&target.key);
                self.retain(target.key, target.value);
                continue;
            }
            if current == target.value {
                self.active.remove(&target.key);
                self.retain(target.key, target.value);
                continue;
            }
            if let Some(existing) = self.active.get(&target.key)
                && existing.target.to_bits() == target.value.to_bits()
                && existing.duration == target.duration
                && existing.easing == target.easing
            {
                continue;
            }
            if self.active.contains_key(&target.key) || self.active.len() < MAX_ACTIVE {
                self.active.insert(
                    target.key,
                    Active {
                        start: current,
                        target: target.value,
                        began: now,
                        duration: target.duration,
                        easing: target.easing,
                        paused_at: hidden.then_some(now),
                        paused: Duration::ZERO,
                    },
                );
            } else {
                self.rejected = self.rejected.saturating_add(1);
                self.retain(target.key, target.value);
            }
        }
        self.set_hidden(now, hidden);
    }
    pub(super) fn reconcile_feedback(
        &mut self,
        desired: &[FeedbackTarget],
        now: Instant,
        reduced: bool,
        hidden: bool,
    ) {
        let now = self.monotonic_now(now);
        if desired.len() > MAX_RETAINED {
            self.rejected = self.rejected.saturating_add(desired.len());
            self.instances.clear();
            self.groups.clear();
            return;
        }
        let instances: BTreeSet<_> = desired.iter().map(|t| t.instance).collect();
        self.instances.retain(|i| instances.contains(i));
        self.groups
            .retain(|key, _| self.instances.iter().any(|i| i.group == *key));
        if reduced {
            self.instances.clear();
            self.groups.clear();
            return;
        }
        for target in desired {
            if target.period == Duration::ZERO
                || now.checked_add(target.period).is_none()
                || self
                    .groups
                    .get(&target.instance.group)
                    .is_some_and(|group| group.period != target.period)
            {
                self.instances.remove(&target.instance);
                self.rejected = self.rejected.saturating_add(1);
                continue;
            }
            if !self.instances.contains(&target.instance)
                && self.instances.len() >= MAX_FEEDBACK_INSTANCES
            {
                self.rejected = self.rejected.saturating_add(1);
                continue;
            }
            if !self.groups.contains_key(&target.instance.group)
                && self.groups.len() >= MAX_FEEDBACK_GROUPS
            {
                self.rejected = self.rejected.saturating_add(1);
                continue;
            }
            self.instances.insert(target.instance);
            self.groups.entry(target.instance.group).or_insert(Group {
                period: target.period,
                began: now,
                paused_at: hidden.then_some(now),
                paused: Duration::ZERO,
            });
        }
        self.groups
            .retain(|key, _| self.instances.iter().any(|instance| instance.group == *key));
        self.set_hidden(now, hidden);
    }
    pub(super) fn frame(&mut self, now: Instant, reduced: bool, hidden: bool) -> Frame {
        let now = self.monotonic_now(now);
        self.set_hidden(now, hidden);
        let mut done = Vec::new();
        let mut next = None;
        if reduced {
            let completed: Vec<_> = self
                .active
                .iter()
                .map(|(&key, active)| (key, active.target))
                .collect();
            for (key, value) in completed {
                self.retain(key, value);
            }
            self.active.clear();
            self.groups.clear();
            self.instances.clear();
        } else if !hidden {
            for (&key, active) in &self.active {
                let (_, elapsed) = sample(*active, now);
                if elapsed >= active.duration {
                    done.push((key, active.target));
                } else if let Some(deadline) = now.checked_add(
                    Duration::from_nanos(16_666_667).min(active.duration.saturating_sub(elapsed)),
                ) {
                    next = Some(next.map_or(deadline, |old: Instant| old.min(deadline)));
                }
            }
        }
        for (key, value) in done {
            self.active.remove(&key);
            self.retain(key, value);
        }
        if !reduced && !hidden && !self.groups.is_empty() {
            if let Some(deadline) = now.checked_add(Duration::from_nanos(16_666_667)) {
                next = Some(next.map_or(deadline, |old| old.min(deadline)));
            }
        }
        Frame {
            next_deadline: next,
            diagnostics: self.diagnostics(),
        }
    }
    pub(super) fn feedback_phase(&self, instance: FeedbackInstance, now: Instant) -> Option<f64> {
        if !self.instances.contains(&instance) {
            return None;
        }
        let group = self.groups.get(&instance.group)?;
        let now = self.last_now.map_or(now, |last| last.max(now));
        let elapsed = group
            .paused_at
            .unwrap_or(now)
            .saturating_duration_since(group.began)
            .saturating_sub(group.paused);
        Some((elapsed.as_secs_f64() / group.period.as_secs_f64()).fract())
    }
    pub(super) fn value(&self, key: TargetKey, now: Instant) -> Option<f64> {
        let now = self.last_now.map_or(now, |last| last.max(now));
        self.active
            .get(&key)
            .map(|a| sample(*a, now).0)
            .or_else(|| self.retained.get(&key).copied())
    }
    fn retain(&mut self, key: TargetKey, value: f64) {
        if !self.retained.contains_key(&key) && self.retained.len() >= MAX_RETAINED {
            self.rejected = self.rejected.saturating_add(1);
            return;
        }
        self.retained.insert(key, value);
    }
    fn set_hidden(&mut self, now: Instant, hidden: bool) {
        for a in self.active.values_mut() {
            pause(&mut a.paused_at, &mut a.paused, now, hidden)
        }
        for g in self.groups.values_mut() {
            pause(&mut g.paused_at, &mut g.paused, now, hidden)
        }
    }
    fn diagnostics(&self) -> Diagnostics {
        Diagnostics {
            active: self.active.len(),
            retained: self.retained.len(),
            feedback_groups: self.groups.len(),
            feedback_instances: self.instances.len(),
            rejected: self.rejected,
        }
    }
    fn monotonic_now(&mut self, now: Instant) -> Instant {
        let now = self.last_now.map_or(now, |last| last.max(now));
        self.last_now = Some(now);
        now
    }
}
fn pause(slot: &mut Option<Instant>, total: &mut Duration, now: Instant, hidden: bool) {
    match (*slot, hidden) {
        (None, true) => *slot = Some(now),
        (Some(then), false) => {
            *total = total.saturating_add(now.saturating_duration_since(then));
            *slot = None
        }
        _ => {}
    }
}
fn sample(active: Active, now: Instant) -> (f64, Duration) {
    let end = active.paused_at.unwrap_or(now);
    let elapsed = end
        .saturating_duration_since(active.began)
        .saturating_sub(active.paused)
        .min(active.duration);
    let t = (elapsed.as_secs_f64() / active.duration.as_secs_f64()).clamp(0.0, 1.0);
    let t = match active.easing {
        Easing::Linear => t,
        Easing::EaseOut => 1.0 - (1.0 - t) * (1.0 - t),
    };
    let value = if active.start.signum() == active.target.signum() {
        active.start + (active.target - active.start) * t
    } else {
        active.start * (1.0 - t) + active.target * t
    };
    (
        if value.is_finite() {
            value
        } else if t >= 1.0 {
            active.target
        } else {
            active.start
        },
        elapsed,
    )
}
