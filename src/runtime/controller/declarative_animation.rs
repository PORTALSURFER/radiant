//! One accepted-owner animation set per runtime.
mod engine;

use super::SurfaceRuntime;
use crate::{
    animation::{AnimationEasing, AnimationTarget, FeedbackAnimation},
    layout::NodeId,
    runtime::{RuntimeBridge, surface::ProjectedAnimationIdentity},
};
use std::{collections::HashMap, time::Instant};

/// Observational counters for the bounded declarative animator.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DeclarativeAnimationStatus {
    /// Currently interpolating finite properties.
    pub active: usize,
    /// Current accepted finite properties, including completed values.
    pub retained: usize,
    /// Shared indeterminate clocks.
    pub feedback_groups: usize,
    /// Accepted feedback consumers.
    pub feedback_instances: usize,
    /// Rejected admissions, including invalid source declarations.
    pub rejected: usize,
    /// Frames on which sampled presentation changed.
    pub changed_frames: u64,
    /// Whether the host has suspended animation while hidden or occluded.
    pub hidden: bool,
}
struct Owner {
    token: u64,
    node: NodeId,
    targets: Vec<AnimationTarget>,
    feedback: Vec<FeedbackAnimation>,
}
pub(super) struct DeclarativeAnimator {
    engine: engine::Engine,
    owners: HashMap<ProjectedAnimationIdentity, Owner>,
    next_owner: Option<u64>,
    deadline: Option<Instant>,
    status: DeclarativeAnimationStatus,
    applying: bool,
    samples: Vec<(NodeId, u64, f64)>,
}
impl Default for DeclarativeAnimator {
    fn default() -> Self {
        Self {
            engine: Default::default(),
            owners: HashMap::new(),
            next_owner: Some(1),
            deadline: None,
            status: Default::default(),
            applying: false,
            samples: Vec::new(),
        }
    }
}
impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    /// Observe animator admission and frame demand without advancing time.
    pub fn declarative_animation_status(&self) -> DeclarativeAnimationStatus {
        self.declarative_animation.status
    }

    /// Suspend animation while a host window is hidden or occluded.
    /// Values freeze until visibility resumes; reduced motion still snaps to a
    /// readable static presentation. No application messages are emitted.
    pub fn set_animation_hidden(&mut self, hidden: bool) -> bool {
        if self.declarative_animation.status.hidden == hidden {
            return false;
        }
        self.declarative_animation.status.hidden = hidden;
        self.update_notice_pause(self.timed_repaint_now());
        if self.lifecycle_accepts_work() {
            self.advance_declarative_animation(self.timed_repaint_now());
        }
        true
    }
    pub(super) fn install_declarative_animations(&mut self) {
        if !self.lifecycle_accepts_work() || self.declarative_animation.applying {
            return;
        }
        let now = self.timed_repaint_now();
        let reduced = self.surface.resolved_environment().reduced_motion();
        let hidden = self.declarative_animation.status.hidden;
        let desired = self.surface.animation_descriptors();
        let animator = &mut self.declarative_animation;
        let desired = match desired {
            Some(v) => v,
            None => {
                animator.status.rejected = animator.status.rejected.saturating_add(1);
                Vec::new()
            }
        };
        animator
            .owners
            .retain(|id, _| desired.iter().any(|d| &d.identity == id));
        let mut targets = Vec::new();
        let mut feedback = Vec::new();
        for descriptor in desired {
            let owner = if let Some(owner) = animator.owners.get_mut(&descriptor.identity) {
                owner
            } else {
                let Some(token) = animator.next_owner else {
                    animator.status.rejected = animator.status.rejected.saturating_add(1);
                    continue;
                };
                animator.next_owner = token.checked_add(1);
                animator.owners.entry(descriptor.identity).or_insert(Owner {
                    token,
                    node: descriptor.node_id,
                    targets: Vec::new(),
                    feedback: Vec::new(),
                })
            };
            owner.node = descriptor.node_id;
            owner.targets = descriptor.targets;
            owner.feedback = descriptor.feedback;
            for target in &owner.targets {
                targets.push(engine::Target {
                    key: engine::TargetKey {
                        owner: owner.token,
                        property: target.property(),
                    },
                    initial: Some(target.initial()),
                    value: target.target(),
                    duration: target.transition().duration(),
                    easing: match target.transition().easing() {
                        AnimationEasing::Linear => engine::Easing::Linear,
                        AnimationEasing::EaseOut => engine::Easing::EaseOut,
                    },
                });
            }
            for item in &owner.feedback {
                if self.layout.is_omitted(owner.node)
                    || !self
                        .layout
                        .rects
                        .get(&owner.node)
                        .is_some_and(|bounds| bounds.overlaps(self.viewport))
                {
                    continue;
                }
                feedback.push(engine::FeedbackTarget {
                    instance: engine::FeedbackInstance {
                        owner: owner.token,
                        group: item.group(),
                    },
                    period: item.period(),
                });
            }
        }
        animator.engine.reconcile(&targets, now, reduced, hidden);
        animator
            .engine
            .reconcile_feedback(&feedback, now, reduced, hidden);
        self.advance_declarative_animation(now);
    }
    pub(super) fn declarative_animation_deadline(&self) -> Option<Instant> {
        self.declarative_animation.deadline
    }
    pub(super) fn clear_declarative_animations(&mut self) {
        self.declarative_animation = Default::default();
    }
    pub(super) fn advance_declarative_animation(&mut self, now: Instant) -> bool {
        if !self.lifecycle_accepts_work() || self.declarative_animation.applying {
            return false;
        }
        let reduced = self.surface.resolved_environment().reduced_motion();
        let animator = &mut self.declarative_animation;
        let frame = animator.engine.frame(now, reduced, animator.status.hidden);
        animator.deadline = frame.next_deadline;
        animator.status.active = frame.diagnostics.active;
        animator.status.retained = frame.diagnostics.retained;
        animator.status.feedback_groups = frame.diagnostics.feedback_groups;
        animator.status.feedback_instances = frame.diagnostics.feedback_instances;
        animator.status.rejected = animator.status.rejected.max(frame.diagnostics.rejected);
        let mut samples = std::mem::take(&mut animator.samples);
        samples.clear();
        for owner in animator.owners.values() {
            for target in &owner.targets {
                let value = animator
                    .engine
                    .value(
                        engine::TargetKey {
                            owner: owner.token,
                            property: target.property(),
                        },
                        now,
                    )
                    .unwrap_or(target.target());
                samples.push((owner.node, target.property(), value));
            }
            for item in &owner.feedback {
                let value = if reduced {
                    item.static_value()
                } else {
                    animator
                        .engine
                        .feedback_phase(
                            engine::FeedbackInstance {
                                owner: owner.token,
                                group: item.group(),
                            },
                            now,
                        )
                        .unwrap_or(item.static_value())
                };
                samples.push((owner.node, item.property(), value));
            }
        }
        let impact = self.surface.apply_animation_samples(&samples);
        self.declarative_animation.samples = samples;
        if !impact.changed {
            return false;
        }
        self.declarative_animation.status.changed_frames = self
            .declarative_animation
            .status
            .changed_frames
            .saturating_add(1);
        self.base_paint_plan_reuse_eligible = false;
        self.repaint_requested = true;
        if impact.geometry {
            self.declarative_animation.applying = true;
            self.external_layout_dirty = true;
            self.relayout();
            self.declarative_animation.applying = false;
        }
        true
    }
}
