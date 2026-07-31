//! Allocation-free observational paint-segment evidence.
//!
//! This module deliberately models evidence only.  It does not describe
//! renderer encoding, cache ownership, redraw policy, or partial rendering.

use super::{PaintPrimitive, SurfacePaintPlan};
use crate::runtime::surface::{SurfaceDamage, ViewDeltaDiagnostics, ViewDeltaEffect};
use crate::widgets::WidgetId;

const MAX_PAINT_SEGMENTS: usize = 64;

/// Stable identity of one GPU render-canvas boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PaintSegmentAnchor {
    pub(crate) widget_id: WidgetId,
    pub(crate) key: u64,
}

/// Stable identity derived from the exact neighboring render-canvas anchors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PaintSegmentIdentity {
    pub(crate) preceding: Option<PaintSegmentAnchor>,
    pub(crate) following: Option<PaintSegmentAnchor>,
}

/// One ordinary contiguous primitive run observed in a materialized paint plan.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PaintSegment {
    pub(crate) identity: PaintSegmentIdentity,
    pub(crate) owner: Option<WidgetId>,
    pub(crate) revision: u64,
    pub(crate) implicated: bool,
}

/// Fixed-capacity observation returned by [`PaintSegmentObserver`].
///
/// `conservative` means that stable identity evidence was unavailable or
/// ambiguous.  `all_implicated` is an evidence mask, not a redraw instruction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PaintSegmentObservation {
    pub(crate) segments: [Option<PaintSegment>; MAX_PAINT_SEGMENTS],
    pub(crate) segment_count: u8,
    pub(crate) conservative: bool,
    pub(crate) all_implicated: bool,
}

impl PaintSegmentObservation {
    /// Evidence is unavailable until a materialized paint plan has been
    /// observed. This is deliberately conservative for private consumers.
    pub(crate) const fn unavailable() -> Self {
        Self {
            segments: [None; MAX_PAINT_SEGMENTS],
            segment_count: 0,
            conservative: true,
            all_implicated: true,
        }
    }

    pub(crate) const fn empty() -> Self {
        Self {
            segments: [None; MAX_PAINT_SEGMENTS],
            segment_count: 0,
            conservative: false,
            all_implicated: false,
        }
    }
}

impl Default for PaintSegmentObservation {
    fn default() -> Self {
        Self::empty()
    }
}

#[derive(Clone, Copy)]
struct RetainedSegment {
    identity: PaintSegmentIdentity,
    revision: u64,
}

/// Fixed-capacity observer for ordinary paint segments.
///
/// The observer owns no renderer state and performs no allocations while
/// scanning a plan.  Retained identities remain in the bounded table after a
/// segment disappears so a later reappearance cannot reset its revision.
#[derive(Clone, Copy)]
pub(crate) struct PaintSegmentObserver {
    retained: [Option<RetainedSegment>; MAX_PAINT_SEGMENTS],
    retained_count: u8,
    observation: PaintSegmentObservation,
}

impl PaintSegmentObserver {
    pub(crate) const fn new() -> Self {
        Self {
            retained: [None; MAX_PAINT_SEGMENTS],
            retained_count: 0,
            observation: PaintSegmentObservation::empty(),
        }
    }

    /// Observe one already-materialized base plan and private refresh evidence.
    ///
    /// Reusing a plan with empty damage preserves observations.  Any non-empty
    /// damage paired with reuse is inconsistent evidence and widens all
    /// ordinary segments conservatively.
    pub(crate) fn observe(
        &mut self,
        plan: &SurfacePaintPlan,
        diagnostics: &ViewDeltaDiagnostics,
        plan_reused: bool,
    ) -> PaintSegmentObservation {
        let mut current = [None; MAX_PAINT_SEGMENTS];
        let (current_count, malformed) = collect_segments(&plan.primitives, &mut current);
        if malformed || current_count == 0 && has_ordinary_primitive(&plan.primitives) {
            self.observation = PaintSegmentObservation {
                segments: [None; MAX_PAINT_SEGMENTS],
                segment_count: 0,
                conservative: true,
                all_implicated: true,
            };
            return self.observation;
        }

        let conservative = !diagnostics.classified;
        let inconsistent_reuse = plan_reused && !damage_is_empty(&diagnostics.damage);
        let all_implicated = conservative
            || inconsistent_reuse
            || diagnostics.damage.full_viewport
            || diagnostics.damage.candidate_count > MAX_PAINT_SEGMENTS as u8;

        let mut observation = PaintSegmentObservation {
            segments: [None; MAX_PAINT_SEGMENTS],
            segment_count: current_count,
            conservative,
            all_implicated,
        };

        for (index, entry) in current.iter().enumerate().take(usize::from(current_count)) {
            let Some((identity, owner)) = *entry else {
                continue;
            };
            let (retained_index, newly_inserted) = match self.find_retained(identity) {
                Some(index) => (index, false),
                None => {
                    if usize::from(self.retained_count) >= MAX_PAINT_SEGMENTS {
                        let saturated = PaintSegmentObservation {
                            segments: [None; MAX_PAINT_SEGMENTS],
                            segment_count: 0,
                            conservative: true,
                            all_implicated: true,
                        };
                        self.observation = saturated;
                        return saturated;
                    }
                    let retained_index = usize::from(self.retained_count);
                    self.retained[retained_index] = Some(RetainedSegment {
                        identity,
                        revision: 1,
                    });
                    self.retained_count = self.retained_count.saturating_add(1);
                    (retained_index, true)
                }
            };

            let implicated = observation.all_implicated
                || segment_implicated(identity, owner, &diagnostics.damage, &plan.primitives);
            let Some(retained) = self.retained[retained_index] else {
                let inconsistent = PaintSegmentObservation {
                    segments: [None; MAX_PAINT_SEGMENTS],
                    segment_count: 0,
                    conservative: true,
                    all_implicated: true,
                };
                self.observation = inconsistent;
                return inconsistent;
            };
            let revision = if implicated && !newly_inserted {
                let next = retained.revision.saturating_add(1);
                self.retained[retained_index] = Some(RetainedSegment {
                    identity,
                    revision: next,
                });
                next
            } else {
                retained.revision
            };
            observation.segments[index] = Some(PaintSegment {
                identity,
                owner,
                revision,
                implicated,
            });
        }
        self.observation = observation;
        observation
    }

    fn find_retained(&self, identity: PaintSegmentIdentity) -> Option<usize> {
        self.retained[..usize::from(self.retained_count)]
            .iter()
            .position(|entry| entry.is_some_and(|entry| entry.identity == identity))
    }
}

impl Default for PaintSegmentObserver {
    fn default() -> Self {
        Self::new()
    }
}

fn collect_segments(
    primitives: &[PaintPrimitive],
    output: &mut [Option<(PaintSegmentIdentity, Option<WidgetId>)>; MAX_PAINT_SEGMENTS],
) -> (u8, bool) {
    let mut segment_count = 0usize;
    let mut start = 0usize;
    let mut preceding = None;
    let mut seen_anchors = [None; MAX_PAINT_SEGMENTS];
    let mut anchor_count = 0usize;
    let mut malformed = false;

    for (index, primitive) in primitives.iter().enumerate() {
        let Some(surface) = primitive.gpu_surface() else {
            continue;
        };
        let anchor = PaintSegmentAnchor {
            widget_id: surface.widget_id,
            key: surface.key,
        };
        if anchor_count >= seen_anchors.len()
            || seen_anchors[..anchor_count].contains(&Some(anchor))
        {
            malformed = true;
        } else {
            seen_anchors[anchor_count] = Some(anchor);
            anchor_count += 1;
        }
        if start < index {
            if segment_count >= output.len() {
                malformed = true;
            } else {
                let owner = ordinary_owner(&primitives[start..index]);
                output[segment_count] = Some((
                    PaintSegmentIdentity {
                        preceding,
                        following: Some(anchor),
                    },
                    owner,
                ));
                segment_count += 1;
            }
        }
        preceding = Some(anchor);
        start = index + 1;
    }
    if start < primitives.len() {
        if segment_count >= output.len() {
            malformed = true;
        } else {
            output[segment_count] = Some((
                PaintSegmentIdentity {
                    preceding,
                    following: None,
                },
                ordinary_owner(&primitives[start..]),
            ));
            segment_count += 1;
        }
    }
    (segment_count as u8, malformed)
}

fn ordinary_owner(primitives: &[PaintPrimitive]) -> Option<WidgetId> {
    let mut owner = None;
    for primitive in primitives {
        let Some(widget_id) = primitive.widget_id() else {
            continue;
        };
        if primitive.gpu_surface().is_some() {
            continue;
        }
        if owner.is_some_and(|existing| existing != widget_id) {
            return None;
        }
        owner = Some(widget_id);
    }
    owner
}

fn has_ordinary_primitive(primitives: &[PaintPrimitive]) -> bool {
    primitives
        .iter()
        .any(|primitive| primitive.gpu_surface().is_none())
}

fn damage_is_empty(damage: &SurfaceDamage) -> bool {
    !damage.full_viewport && damage.candidate_count == 0
}

fn segment_implicated(
    _identity: PaintSegmentIdentity,
    owner: Option<WidgetId>,
    damage: &SurfaceDamage,
    primitives: &[PaintPrimitive],
) -> bool {
    if damage.full_viewport {
        return true;
    }
    if damage.candidate_count == 0 {
        return false;
    }
    for candidate in damage.candidates.iter().flatten() {
        if candidate.old_bounds.is_none() || candidate.new_bounds.is_none() {
            return true;
        }
        if candidate.effect != ViewDeltaEffect::Paint {
            return true;
        }
        if owner == Some(candidate.node_id) {
            return true;
        }
        let candidate_is_ordinary = primitives.iter().any(|primitive| {
            primitive.gpu_surface().is_none() && primitive.widget_id() == Some(candidate.node_id)
        });
        if candidate_is_ordinary {
            // Exact paint evidence for another known owner does not implicate
            // this segment. A mixed-owner segment has no safe correlation.
            if owner.is_none() {
                return true;
            }
            continue;
        }
        let candidate_is_gpu_only = primitives.iter().any(|primitive| {
            primitive
                .gpu_surface()
                .is_some_and(|surface| surface.widget_id == candidate.node_id)
        });
        if !candidate_is_gpu_only {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::types::{Point, Rect, Rgba8};
    use crate::runtime::{
        GpuSurfaceCapabilities, GpuSurfaceContent, PaintFillRect, PaintGpuSurface,
    };
    use std::sync::Arc;

    fn fill(widget_id: u64) -> PaintPrimitive {
        PaintPrimitive::FillRect(PaintFillRect {
            widget_id,
            rect: Rect::from_min_size(Point::default(), crate::gui::types::Vector2::new(1.0, 1.0)),
            color: Rgba8::new(255, 255, 255, 255),
        })
    }

    fn gpu(widget_id: u64, key: u64) -> PaintPrimitive {
        PaintPrimitive::GpuSurface(PaintGpuSurface {
            widget_id,
            key,
            revision: 1,
            rect: Rect::from_min_size(Point::default(), crate::gui::types::Vector2::new(1.0, 1.0)),
            content: GpuSurfaceContent::CustomShader {
                descriptor: Arc::new(crate::runtime::GpuShaderSurfaceDescriptor::new("test")),
            },
            capabilities: GpuSurfaceCapabilities::default(),
            overlays: Vec::new(),
        })
    }

    fn diagnostics(damage: SurfaceDamage) -> ViewDeltaDiagnostics {
        ViewDeltaDiagnostics {
            classified: true,
            damage,
            ..ViewDeltaDiagnostics::default()
        }
    }

    fn empty_damage() -> SurfaceDamage {
        SurfaceDamage::empty(Rect::from_min_size(
            Point::default(),
            crate::gui::types::Vector2::new(1.0, 1.0),
        ))
    }

    #[test]
    fn segments_use_exact_canvas_anchors_and_ignore_empty_runs() {
        let plan = SurfacePaintPlan {
            clear_color: Rgba8::new(255, 255, 255, 255),
            primitives: vec![fill(1), gpu(20, 4), gpu(21, 5), fill(2)],
        };
        let mut observer = PaintSegmentObserver::new();
        observer.observe(&plan, &diagnostics(empty_damage()), false);
        let observation = observer.observation;
        assert_eq!(observation.segment_count, 2);
        assert_eq!(observation.segments[0].unwrap().identity.preceding, None);
        assert_eq!(
            observation.segments[0].unwrap().identity.following,
            Some(PaintSegmentAnchor {
                widget_id: 20,
                key: 4
            })
        );
        assert_eq!(
            observation.segments[1].unwrap().identity,
            PaintSegmentIdentity {
                preceding: Some(PaintSegmentAnchor {
                    widget_id: 21,
                    key: 5
                }),
                following: None,
            }
        );
    }

    #[test]
    fn unavailable_snapshot_is_distinct_from_valid_empty_observation() {
        let observation = PaintSegmentObservation::unavailable();
        assert_eq!(observation.segment_count, 0);
        assert!(observation.conservative);
        assert!(observation.all_implicated);
        assert!(observation.segments.iter().all(Option::is_none));
        assert_ne!(observation, PaintSegmentObservation::empty());
    }

    #[test]
    fn paint_damage_targets_owner_and_gpu_only_damage_is_empty() {
        let plan = SurfacePaintPlan {
            clear_color: Rgba8::new(255, 255, 255, 255),
            primitives: vec![fill(1), gpu(20, 4), fill(2)],
        };
        let mut observer = PaintSegmentObserver::new();
        observer.observe(&plan, &diagnostics(empty_damage()), false);
        let mut damage = empty_damage();
        damage.candidates[0] = Some(crate::runtime::surface::SurfaceDamageCandidate {
            node_id: 1,
            old_bounds: Some(Rect::default()),
            new_bounds: Some(Rect::default()),
            effect: ViewDeltaEffect::Paint,
        });
        damage.candidate_count = 1;
        observer.observe(&plan, &diagnostics(damage), false);
        assert!(observer.observation.segments[0].unwrap().implicated);
        assert!(!observer.observation.segments[1].unwrap().implicated);
        damage.candidates[0] = Some(crate::runtime::surface::SurfaceDamageCandidate {
            node_id: 20,
            old_bounds: Some(Rect::default()),
            new_bounds: Some(Rect::default()),
            effect: ViewDeltaEffect::Paint,
        });
        observer.observe(&plan, &diagnostics(damage), false);
        assert!(!observer.observation.segments[0].unwrap().implicated);
        assert!(!observer.observation.segments[1].unwrap().implicated);
    }

    #[test]
    fn duplicate_anchor_and_reused_inconsistent_damage_are_conservative() {
        let plan = SurfacePaintPlan {
            clear_color: Rgba8::new(255, 255, 255, 255),
            primitives: vec![fill(1), gpu(20, 4), fill(2), gpu(20, 4), fill(3)],
        };
        let mut observer = PaintSegmentObserver::new();
        observer.observe(&plan, &diagnostics(empty_damage()), false);
        assert!(observer.observation.conservative);

        let valid = SurfacePaintPlan {
            clear_color: Rgba8::new(255, 255, 255, 255),
            primitives: vec![fill(1)],
        };
        observer.observe(&valid, &diagnostics(empty_damage()), false);
        let mut damage = empty_damage();
        damage.full_viewport = true;
        observer.observe(&valid, &diagnostics(damage), true);
        assert!(observer.observation.all_implicated);
        assert!(observer.observation.segments[0].unwrap().implicated);
    }

    #[test]
    fn empty_damage_preserves_revision_and_exact_paint_increments_it() {
        let plan = SurfacePaintPlan {
            clear_color: Rgba8::new(255, 255, 255, 255),
            primitives: vec![fill(1)],
        };
        let mut observer = PaintSegmentObserver::new();
        observer.observe(&plan, &diagnostics(empty_damage()), false);
        assert_eq!(observer.observation.segments[0].unwrap().revision, 1);
        observer.observe(&plan, &diagnostics(empty_damage()), false);
        assert_eq!(observer.observation.segments[0].unwrap().revision, 1);

        let mut damage = empty_damage();
        damage.candidates[0] = Some(crate::runtime::surface::SurfaceDamageCandidate {
            node_id: 1,
            old_bounds: Some(Rect::default()),
            new_bounds: Some(Rect::default()),
            effect: ViewDeltaEffect::Paint,
        });
        damage.candidate_count = 1;
        observer.observe(&plan, &diagnostics(damage), false);
        assert_eq!(observer.observation.segments[0].unwrap().revision, 2);
        observer.observe(&plan, &diagnostics(empty_damage()), false);
        assert_eq!(observer.observation.segments[0].unwrap().revision, 2);
    }

    #[test]
    fn retained_identity_saturation_returns_empty_conservative_observation() {
        let mut observer = PaintSegmentObserver::new();
        for key in 0..MAX_PAINT_SEGMENTS as u64 {
            let plan = SurfacePaintPlan {
                clear_color: Rgba8::new(255, 255, 255, 255),
                primitives: vec![gpu(20, key), fill(2)],
            };
            observer.observe(&plan, &diagnostics(empty_damage()), false);
            assert_eq!(observer.observation.segment_count, 1);
        }

        let plan = SurfacePaintPlan {
            clear_color: Rgba8::new(255, 255, 255, 255),
            primitives: vec![gpu(20, MAX_PAINT_SEGMENTS as u64), fill(2)],
        };
        let observation = observer.observe(&plan, &diagnostics(empty_damage()), false);
        assert_eq!(observation.segment_count, 0);
        assert!(observation.conservative);
        assert!(observation.all_implicated);
        assert!(observation.segments.iter().all(Option::is_none));
    }
}
