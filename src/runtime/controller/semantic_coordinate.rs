//! Runtime-owned destination context and custom semantic-coordinate admission.
//!
//! This module is intentionally private. The public resolver receives only the
//! immutable request vocabulary from `runtime::virtual_layout`; this module
//! owns exact context validation, panic/reentry containment, clipping, and the
//! private witness carried into publication.

use super::SurfaceRuntime;
use crate::{
    gui::{
        automation::{AutomationNodeId, AutomationNodeSnapshot, GuiAutomationSnapshot},
        layout_core::{VirtualLayoutSemanticRejectedReason, VirtualLayoutSemanticTransformWitness},
        types::Rect,
    },
    layout::{NodeId, VirtualLayoutPolicyIdentity},
    runtime::{
        RuntimeBridge, VirtualLayoutRevisions,
        virtual_layout::{
            VirtualLayoutSemanticCoordinateTransformInvocation,
            VirtualLayoutSemanticCoordinateTransformInvoker,
            VirtualLayoutSemanticCoordinateTransformRequest,
        },
    },
};
use std::collections::BTreeMap;
use std::rc::Rc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RectBits {
    min_x: u32,
    min_y: u32,
    max_x: u32,
    max_y: u32,
}

impl RectBits {
    fn from_rect(rect: Rect) -> Self {
        Self {
            min_x: rect.min.x.to_bits(),
            min_y: rect.min.y.to_bits(),
            max_x: rect.max.x.to_bits(),
            max_y: rect.max.y.to_bits(),
        }
    }
}

/// Exact destination context evidence retained by a semantic provider fence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct SemanticCoordinateContextFence {
    anchor: RectBits,
    destination_clip: RectBits,
    surface_viewport: RectBits,
}

impl SemanticCoordinateContextFence {
    fn new(anchor: Rect, destination_clip: Rect, surface_viewport: Rect) -> Self {
        Self {
            anchor: RectBits::from_rect(anchor),
            destination_clip: RectBits::from_rect(destination_clip),
            surface_viewport: RectBits::from_rect(surface_viewport),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct SemanticCoordinateDestination {
    anchor: Rect,
    destination_clip: Rect,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SemanticCoordinateContextError {
    InvalidSurfaceViewport,
    MissingAnchor,
    DuplicateAnchor,
    InvalidAnchor,
    MissingClipAncestor,
    InvalidClip,
    EmptyClip,
}

/// Immutable runtime-owned context captured for one explicit semantic turn.
#[derive(Clone, Debug)]
pub(in crate::runtime) struct SemanticCoordinateContext {
    surface_viewport: Rect,
    destinations:
        BTreeMap<NodeId, Result<SemanticCoordinateDestination, SemanticCoordinateContextError>>,
}

impl SemanticCoordinateContext {
    #[cfg(test)]
    pub(super) fn empty() -> Self {
        Self {
            surface_viewport: Rect::from_xy_size(0.0, 0.0, 1.0, 1.0),
            destinations: BTreeMap::new(),
        }
    }

    pub(super) fn for_runtime<Bridge, Message>(
        runtime: &SurfaceRuntime<Bridge, Message>,
        ordinary: &GuiAutomationSnapshot,
    ) -> Self
    where
        Bridge: RuntimeBridge<Message>,
    {
        let surface_viewport = runtime.viewport;
        let surface_valid = valid_rect(surface_viewport);
        let mut destinations = BTreeMap::new();
        for registration in &runtime.traversal.containers.virtual_layout_registrations {
            let container_id = registration.container_id;
            let result = if !surface_valid {
                Err(SemanticCoordinateContextError::InvalidSurfaceViewport)
            } else {
                let automation_id = AutomationNodeId::new(container_id.to_string());
                match count_automation_nodes(&ordinary.root, &automation_id) {
                    0 => Err(SemanticCoordinateContextError::MissingAnchor),
                    1 => {
                        let anchor = runtime
                            .layout
                            .rects
                            .get(&container_id)
                            .copied()
                            .ok_or(SemanticCoordinateContextError::MissingAnchor)
                            .and_then(|anchor| {
                                if valid_rect(anchor) {
                                    Ok(anchor)
                                } else {
                                    Err(SemanticCoordinateContextError::InvalidAnchor)
                                }
                            });
                        anchor.and_then(|anchor| {
                            effective_clip(runtime, container_id, surface_viewport, anchor).map(
                                |destination_clip| SemanticCoordinateDestination {
                                    anchor,
                                    destination_clip,
                                },
                            )
                        })
                    }
                    _ => Err(SemanticCoordinateContextError::DuplicateAnchor),
                }
            };
            destinations.insert(container_id, result);
        }
        Self {
            surface_viewport,
            destinations,
        }
    }

    pub(super) fn fence_for(
        &self,
        container_id: NodeId,
    ) -> Result<SemanticCoordinateContextFence, SemanticCoordinateContextError> {
        let destination = self.destination_for(container_id)?;
        Ok(SemanticCoordinateContextFence::new(
            destination.anchor,
            destination.destination_clip,
            self.surface_viewport,
        ))
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn resolve(
        &self,
        container_id: NodeId,
        source_rect: Rect,
        revisions: VirtualLayoutRevisions,
        identity: &VirtualLayoutPolicyIdentity,
        transform_revision: u64,
        transform_generation: u64,
        resolver_token: usize,
        resolver: &Rc<dyn VirtualLayoutSemanticCoordinateTransformInvoker>,
    ) -> Result<(Rect, VirtualLayoutSemanticTransformWitness), VirtualLayoutSemanticRejectedReason>
    {
        if !valid_rect(source_rect) {
            return Err(VirtualLayoutSemanticRejectedReason::NonFiniteBounds);
        }
        let destination = self.destination_for(container_id).map_err(|_| {
            VirtualLayoutSemanticRejectedReason::CoordinateTransformContextUnavailable
        })?;
        if transform_generation == 0 || resolver_token == 0 {
            return Err(VirtualLayoutSemanticRejectedReason::CoordinateTransformContextUnavailable);
        }
        let request = VirtualLayoutSemanticCoordinateTransformRequest::new(
            source_rect,
            destination.anchor,
            destination.destination_clip,
            revisions,
            transform_revision,
        );
        let output = match resolver.invoke(&request) {
            VirtualLayoutSemanticCoordinateTransformInvocation::Found(rect) => rect,
            VirtualLayoutSemanticCoordinateTransformInvocation::Unsupported => {
                return Err(VirtualLayoutSemanticRejectedReason::CoordinateTransformUnsupported);
            }
            VirtualLayoutSemanticCoordinateTransformInvocation::Singular => {
                return Err(VirtualLayoutSemanticRejectedReason::CoordinateTransformSingular);
            }
            VirtualLayoutSemanticCoordinateTransformInvocation::Ambiguous => {
                return Err(VirtualLayoutSemanticRejectedReason::CoordinateTransformAmbiguous);
            }
            VirtualLayoutSemanticCoordinateTransformInvocation::Panic => {
                return Err(VirtualLayoutSemanticRejectedReason::CoordinateTransformPanic);
            }
            VirtualLayoutSemanticCoordinateTransformInvocation::Reentrant => {
                return Err(VirtualLayoutSemanticRejectedReason::CoordinateTransformReentrant);
            }
        };
        if !output.is_finite() {
            return Err(VirtualLayoutSemanticRejectedReason::CoordinateTransformOverflow);
        }
        if output.min.x > output.max.x || output.min.y > output.max.y {
            return Err(VirtualLayoutSemanticRejectedReason::CoordinateTransformInvalidOutput);
        }
        let clipped = output
            .intersection(destination.destination_clip)
            .ok_or(VirtualLayoutSemanticRejectedReason::CoordinateTransformOutsideClip)?;
        if !valid_rect(clipped) {
            return Err(VirtualLayoutSemanticRejectedReason::CoordinateTransformInvalidOutput);
        }
        let witness = VirtualLayoutSemanticTransformWitness::new(
            identity.clone(),
            transform_revision,
            transform_generation,
            resolver_token,
            source_rect,
            destination.anchor,
            destination.destination_clip,
        );
        Ok((clipped, witness))
    }

    fn destination_for(
        &self,
        container_id: NodeId,
    ) -> Result<SemanticCoordinateDestination, SemanticCoordinateContextError> {
        self.destinations
            .get(&container_id)
            .ok_or(SemanticCoordinateContextError::MissingAnchor)?
            .to_owned()
    }
}

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(in crate::runtime) fn semantic_coordinate_context(
        &self,
        ordinary: &GuiAutomationSnapshot,
    ) -> SemanticCoordinateContext {
        SemanticCoordinateContext::for_runtime(self, ordinary)
    }
}

fn valid_rect(rect: Rect) -> bool {
    rect.is_finite() && rect.min.x <= rect.max.x && rect.min.y <= rect.max.y
}

fn count_automation_nodes(node: &AutomationNodeSnapshot, wanted: &AutomationNodeId) -> usize {
    usize::from(node.id == *wanted)
        + node
            .children
            .iter()
            .map(|child| count_automation_nodes(child, wanted))
            .sum::<usize>()
}

fn effective_clip<Bridge, Message>(
    runtime: &SurfaceRuntime<Bridge, Message>,
    container_id: NodeId,
    surface_viewport: Rect,
    anchor: Rect,
) -> Result<Rect, SemanticCoordinateContextError>
where
    Bridge: RuntimeBridge<Message>,
{
    let mut clip = intersect_valid(surface_viewport, anchor)?;
    if let Some(own_viewport) = runtime.layout.viewport_bounds.get(&container_id).copied() {
        clip = intersect_valid(clip, own_viewport)?;
    }
    if let Some(ancestors) = runtime
        .traversal
        .containers
        .clip_ancestors
        .get(&container_id)
    {
        for ancestor in ancestors.as_slice() {
            let ancestor_clip = runtime
                .layout
                .viewport_bounds
                .get(ancestor)
                .copied()
                .or_else(|| runtime.layout.rects.get(ancestor).copied())
                .ok_or(SemanticCoordinateContextError::MissingClipAncestor)?;
            clip = intersect_valid(clip, ancestor_clip)?;
        }
    }
    Ok(clip)
}

fn intersect_valid(left: Rect, right: Rect) -> Result<Rect, SemanticCoordinateContextError> {
    if !valid_rect(right) {
        return Err(SemanticCoordinateContextError::InvalidClip);
    }
    let intersection = left
        .intersection(right)
        .ok_or(SemanticCoordinateContextError::EmptyClip)?;
    if valid_rect(intersection) {
        Ok(intersection)
    } else {
        Err(SemanticCoordinateContextError::EmptyClip)
    }
}
