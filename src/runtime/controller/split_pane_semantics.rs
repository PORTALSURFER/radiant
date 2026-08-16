//! Pure backend-neutral semantic publication for admitted split dividers.

use super::split_pane_separator::SplitPaneSeparatorProjection;
use crate::{
    gui::{
        automation::{
            AutomationBounds, AutomationNodeId, AutomationNodeSemantics, AutomationNodeSnapshot,
            AutomationRole, GuiAutomationSnapshot,
        },
        layout_core::SPLIT_PANE_DIVIDER_REGION_ID,
        panel::SplitPaneAxis,
    },
    layout::NodeId,
};
use std::collections::HashSet;

const SPLIT_PANE_SEMANTIC_SNAPSHOT_SCHEMA: u32 = 3;

struct SeparatorPlan {
    anchor_id: AutomationNodeId,
    node: AutomationNodeSnapshot,
}

/// Publish one semantic separator for every valid admitted projection.
///
/// The input is an already-staged ordinary snapshot and an immutable projection
/// slice. Every projection is preflighted before the ordinary tree is cloned for
/// insertion. Any malformed or ambiguous evidence returns the unchanged
/// ordinary snapshot, so a caller never observes a partial separator set.
pub(super) fn compose_split_pane_automation_snapshot(
    ordinary: &GuiAutomationSnapshot,
    projections: &[SplitPaneSeparatorProjection],
) -> GuiAutomationSnapshot {
    if projections.is_empty() || ordinary.schema_version < SPLIT_PANE_SEMANTIC_SNAPSHOT_SCHEMA {
        return ordinary.clone();
    }

    let Some(plans) = preflight_separator_plans(ordinary, projections) else {
        return ordinary.clone();
    };

    let mut staged = ordinary.clone();
    for plan in &plans {
        if insert_separator(&mut staged.root, plan) != 1 {
            return ordinary.clone();
        }
    }

    let mut final_ids = HashSet::new();
    if !audit_unique_ids(&staged.root, &mut final_ids)
        || plans.iter().any(|plan| !final_ids.contains(&plan.node.id))
    {
        return ordinary.clone();
    }

    staged
}

fn preflight_separator_plans(
    ordinary: &GuiAutomationSnapshot,
    projections: &[SplitPaneSeparatorProjection],
) -> Option<Vec<SeparatorPlan>> {
    let mut ordinary_ids = HashSet::new();
    if !audit_unique_ids(&ordinary.root, &mut ordinary_ids) {
        return None;
    }

    let mut projection_ids = HashSet::with_capacity(projections.len());
    let mut generated_ids = HashSet::with_capacity(projections.len());
    let mut plans = Vec::with_capacity(projections.len());

    for projection in projections {
        let identity = projection.target;
        if identity.region_id != SPLIT_PANE_DIVIDER_REGION_ID
            || !projection.divider_bounds.has_finite_positive_area()
            || !projection.live_ratio.is_finite()
            || !(0.0..=1.0).contains(&projection.live_ratio)
            || !projection_ids.insert(identity)
        {
            return None;
        }

        let generated_id = separator_id(identity.container_id, identity.region_id.value());
        if !generated_ids.insert(generated_id.clone()) || ordinary_ids.contains(&generated_id) {
            return None;
        }

        let anchor_id = AutomationNodeId::new(identity.container_id.to_string());
        let anchor = unique_anchor(&ordinary.root, &anchor_id)?;
        if anchor.children.len() != 2 {
            return None;
        }

        let ratio = normalized_ratio_string(projection.live_ratio)?;
        let mut semantics =
            AutomationNodeSemantics::new(AutomationRole::Separator).with_value_text(ratio);
        semantics.checked = Some(false);
        semantics.metadata.insert(
            "orientation".to_owned(),
            orientation_name(projection.axis).to_owned(),
        );
        let node = AutomationNodeSnapshot::from_semantics(
            generated_id,
            AutomationBounds::from_rect(projection.divider_bounds),
            semantics,
        );
        if node.enabled
            && !node.semantics.disabled
            && !node.semantics.selected
            && node.semantics.checked == Some(false)
            && !node.semantics.read_only
            && !node.semantics.focusable
            && !node.semantics.focused
            && node.available_actions.is_empty()
        {
            plans.push(SeparatorPlan { anchor_id, node });
        } else {
            return None;
        }
    }

    Some(plans)
}

fn separator_id(container_id: NodeId, region_id: u64) -> AutomationNodeId {
    AutomationNodeId::new(format!(
        "radiant:layout-target:{container_id}:{region_id:016x}"
    ))
}

fn normalized_ratio_string(value: f32) -> Option<String> {
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        return None;
    }
    let normalized = value.clamp(0.0, 1.0);
    Some(if normalized == 0.0 {
        "0".to_owned()
    } else {
        normalized.to_string()
    })
}

fn orientation_name(axis: SplitPaneAxis) -> &'static str {
    match axis {
        SplitPaneAxis::Horizontal => "horizontal",
        SplitPaneAxis::Vertical => "vertical",
    }
}

fn unique_anchor<'a>(
    root: &'a AutomationNodeSnapshot,
    anchor_id: &AutomationNodeId,
) -> Option<&'a AutomationNodeSnapshot> {
    let mut count = 0;
    let mut anchor = None;
    collect_anchors(root, anchor_id, &mut count, &mut anchor);
    (count == 1)
        .then_some(anchor?)
        .filter(|node| node.children.len() == 2)
}

fn collect_anchors<'a>(
    node: &'a AutomationNodeSnapshot,
    anchor_id: &AutomationNodeId,
    count: &mut usize,
    anchor: &mut Option<&'a AutomationNodeSnapshot>,
) {
    if &node.id == anchor_id {
        *count = count.saturating_add(1);
        *anchor = Some(node);
    }
    for child in &node.children {
        collect_anchors(child, anchor_id, count, anchor);
    }
}

fn insert_separator(node: &mut AutomationNodeSnapshot, plan: &SeparatorPlan) -> usize {
    if node.id == plan.anchor_id {
        if node.children.len() != 2 {
            return 0;
        }
        node.children.insert(1, plan.node.clone());
        return 1;
    }

    node.children
        .iter_mut()
        .map(|child| insert_separator(child, plan))
        .sum()
}

fn audit_unique_ids(node: &AutomationNodeSnapshot, ids: &mut HashSet<AutomationNodeId>) -> bool {
    if !ids.insert(node.id.clone()) {
        return false;
    }
    node.children
        .iter()
        .all(|child| audit_unique_ids(child, ids))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        gui::{
            automation::AutomationNodeSnapshot,
            layout_core::{LayoutTargetIdentity, MountedContainerStateId},
        },
        layout::{ContainerStateId, LayoutHitRegionId},
    };
    use std::num::NonZeroU64;

    const REGION_ID: LayoutHitRegionId = SPLIT_PANE_DIVIDER_REGION_ID;

    fn group(id: &str, children: Vec<AutomationNodeSnapshot>) -> AutomationNodeSnapshot {
        AutomationNodeSnapshot::from_semantics(
            AutomationNodeId::new(id),
            AutomationBounds {
                x: 0.0,
                y: 0.0,
                width: 200.0,
                height: 120.0,
            },
            AutomationNodeSemantics::new(AutomationRole::Group),
        )
        .with_children(children)
    }

    fn leaf(id: &str) -> AutomationNodeSnapshot {
        AutomationNodeSnapshot::from_semantics(
            AutomationNodeId::new(id),
            AutomationBounds {
                x: 0.0,
                y: 0.0,
                width: 80.0,
                height: 40.0,
            },
            AutomationNodeSemantics::new(AutomationRole::Text),
        )
    }

    fn ordinary_snapshot(children: Vec<AutomationNodeSnapshot>) -> GuiAutomationSnapshot {
        GuiAutomationSnapshot {
            schema_version: SPLIT_PANE_SEMANTIC_SNAPSHOT_SCHEMA,
            viewport_width: 200,
            viewport_height: 120,
            root: group("root", children),
        }
    }

    fn projection(container_id: NodeId, axis: SplitPaneAxis) -> SplitPaneSeparatorProjection {
        SplitPaneSeparatorProjection {
            target: LayoutTargetIdentity::new(container_id, REGION_ID),
            mounted_state_id: MountedContainerStateId::new(
                ContainerStateId::new::<u32>(container_id, 1),
                NonZeroU64::new(1).expect("non-zero test generation"),
            ),
            axis,
            divider_bounds: crate::gui::types::Rect::from_xy_size(96.0, 0.0, 8.0, 120.0),
            live_ratio: 0.5,
        }
    }

    fn child_ids(node: &AutomationNodeSnapshot) -> Vec<&str> {
        node.children
            .iter()
            .map(|child| child.id.0.as_str())
            .collect()
    }

    fn find<'a>(
        node: &'a AutomationNodeSnapshot,
        id: &AutomationNodeId,
    ) -> Option<&'a AutomationNodeSnapshot> {
        if &node.id == id {
            return Some(node);
        }
        node.children.iter().find_map(|child| find(child, id))
    }

    #[test]
    fn publishes_separator_semantics_between_two_content_children() {
        let ordinary = ordinary_snapshot(vec![group("1", vec![leaf("2"), leaf("3")])]);
        let snapshot = compose_split_pane_automation_snapshot(
            &ordinary,
            &[projection(1, SplitPaneAxis::Horizontal)],
        );
        let container = find(&snapshot.root, &AutomationNodeId::new("1")).expect("container");
        assert_eq!(
            child_ids(container),
            vec!["2", "radiant:layout-target:1:53504c49545f4449", "3"]
        );
        let separator = &container.children[1];
        assert_eq!(separator.role, AutomationRole::Separator);
        assert_eq!(separator.bounds.x, 96.0);
        assert_eq!(separator.bounds.width, 8.0);
        assert_eq!(separator.value.as_deref(), Some("0.5"));
        assert_eq!(separator.semantics.value_text.as_deref(), Some("0.5"));
        assert_eq!(
            separator
                .semantics
                .metadata
                .get("orientation")
                .map(String::as_str),
            Some("horizontal")
        );
        assert!(separator.enabled);
        assert!(!separator.semantics.selected);
        assert_eq!(separator.semantics.checked, Some(false));
        assert!(!separator.semantics.read_only);
        assert!(!separator.semantics.focusable);
        assert!(!separator.semantics.focused);
        assert!(separator.available_actions.is_empty());
        assert!(separator.children.is_empty());
        assert_eq!(snapshot.schema_version, 3);
        assert_eq!(snapshot.target_snapshot().schema_version, 2);
    }

    #[test]
    fn nested_splits_insert_in_local_order() {
        let inner = group("4", vec![leaf("5"), leaf("6")]);
        let ordinary = ordinary_snapshot(vec![group("1", vec![inner, leaf("3")])]);
        let snapshot = compose_split_pane_automation_snapshot(
            &ordinary,
            &[
                projection(1, SplitPaneAxis::Horizontal),
                projection(4, SplitPaneAxis::Vertical),
            ],
        );
        let outer = find(&snapshot.root, &AutomationNodeId::new("1")).expect("outer");
        assert_eq!(
            child_ids(outer),
            vec!["4", "radiant:layout-target:1:53504c49545f4449", "3"]
        );
        let inner = find(&snapshot.root, &AutomationNodeId::new("4")).expect("inner");
        assert_eq!(
            child_ids(inner),
            vec!["5", "radiant:layout-target:4:53504c49545f4449", "6"]
        );
        assert_eq!(
            inner.children[1]
                .semantics
                .metadata
                .get("orientation")
                .map(String::as_str),
            Some("vertical")
        );
    }

    #[test]
    fn malformed_projection_or_anchor_returns_unchanged_ordinary_snapshot() {
        let ordinary = ordinary_snapshot(vec![group("1", vec![leaf("2"), leaf("3")])]);
        let mut malformed = projection(1, SplitPaneAxis::Horizontal);
        malformed.divider_bounds = crate::gui::types::Rect::from_xy_size(0.0, 0.0, 0.0, 120.0);
        assert_eq!(
            compose_split_pane_automation_snapshot(&ordinary, &[malformed]),
            ordinary
        );

        let wrong_cardinality = ordinary_snapshot(vec![group("1", vec![leaf("2")])]);
        assert_eq!(
            compose_split_pane_automation_snapshot(
                &wrong_cardinality,
                &[projection(1, SplitPaneAxis::Horizontal)]
            ),
            wrong_cardinality
        );

        let mut wrong_region = projection(1, SplitPaneAxis::Horizontal);
        wrong_region.target.region_id = LayoutHitRegionId::new(7);
        assert_eq!(
            compose_split_pane_automation_snapshot(&ordinary, &[wrong_region]),
            ordinary
        );

        let mut malformed_ratio = projection(1, SplitPaneAxis::Horizontal);
        malformed_ratio.live_ratio = f32::NAN;
        assert_eq!(
            compose_split_pane_automation_snapshot(&ordinary, &[malformed_ratio]),
            ordinary
        );

        assert_eq!(
            compose_split_pane_automation_snapshot(
                &ordinary,
                &[
                    projection(1, SplitPaneAxis::Horizontal),
                    projection(1, SplitPaneAxis::Horizontal)
                ]
            ),
            ordinary
        );

        let missing_anchor = ordinary_snapshot(vec![group("2", vec![leaf("3"), leaf("4")])]);
        assert_eq!(
            compose_split_pane_automation_snapshot(
                &missing_anchor,
                &[projection(1, SplitPaneAxis::Horizontal)]
            ),
            missing_anchor
        );

        let collision = ordinary_snapshot(vec![group(
            "1",
            vec![leaf("radiant:layout-target:1:53504c49545f4449"), leaf("3")],
        )]);
        assert_eq!(
            compose_split_pane_automation_snapshot(
                &collision,
                &[projection(1, SplitPaneAxis::Horizontal)]
            ),
            collision
        );

        let duplicate_anchor = ordinary_snapshot(vec![
            group("1", vec![leaf("2"), leaf("3")]),
            group("1", vec![leaf("4"), leaf("5")]),
        ]);
        assert_eq!(
            compose_split_pane_automation_snapshot(
                &duplicate_anchor,
                &[projection(1, SplitPaneAxis::Horizontal)]
            ),
            duplicate_anchor
        );

        let complete_set = ordinary_snapshot(vec![
            group("1", vec![leaf("2"), leaf("3")]),
            group("4", vec![leaf("5"), leaf("6")]),
        ]);
        let mut invalid_second = projection(4, SplitPaneAxis::Vertical);
        invalid_second.divider_bounds =
            crate::gui::types::Rect::from_xy_size(0.0, 0.0, f32::INFINITY, 120.0);
        assert_eq!(
            compose_split_pane_automation_snapshot(
                &complete_set,
                &[projection(1, SplitPaneAxis::Horizontal), invalid_second,],
            ),
            complete_set,
            "a later invalid projection must not leave an earlier separator behind"
        );
    }

    #[test]
    fn separator_is_not_emitted_under_an_older_snapshot_schema() {
        let mut ordinary = ordinary_snapshot(vec![group("1", vec![leaf("2"), leaf("3")])]);
        ordinary.schema_version = 2;
        assert_eq!(
            compose_split_pane_automation_snapshot(
                &ordinary,
                &[projection(1, SplitPaneAxis::Horizontal)]
            ),
            ordinary
        );
    }
}
