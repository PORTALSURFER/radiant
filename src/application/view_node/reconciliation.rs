//! Application-owned immutable lowering receipts.

use crate::runtime::{
    ApplicationNodeKind, ApplicationNodeReceipt, ExactChangedRoot, FrozenSourceMetadata,
    SourceMetadata,
};
use std::rc::Rc;

#[allow(dead_code)]
#[derive(Clone)]
pub(crate) struct ApplicationProjectionReceipt {
    pub(crate) nodes: Box<[ApplicationNodeReceipt]>,
    pub(crate) components: Vec<(usize, Rc<()>)>,
    pub(crate) supported: bool,
    pub(crate) emitted_records: usize,
    pub(crate) comparison_count: usize,
}

#[allow(dead_code)]
#[derive(Clone)]
struct NodeDraft {
    path: Box<[usize]>,
    incoming_slot: Option<crate::layout::SlotParams>,
    id: crate::layout::NodeId,
    source: Option<Rc<SourceMetadata>>,
    kind: ApplicationNodeKind,
}

#[allow(dead_code)]
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ReceiptComparison {
    Exact(Vec<ExactChangedRoot>),
    Full,
}

#[allow(dead_code)]
pub(crate) struct ApplicationProjectionRecorder<'r> {
    previous: Option<&'r ApplicationProjectionReceipt>,
    drafts: Vec<NodeDraft>,
    unsupported: bool,
    components: Vec<(usize, Rc<()>)>,
}

impl<'r> ApplicationProjectionRecorder<'r> {
    #[allow(dead_code)]
    pub(crate) fn new(previous: Option<&'r ApplicationProjectionReceipt>) -> Self {
        Self {
            previous,
            drafts: Vec::new(),
            unsupported: false,
            components: Vec::new(),
        }
    }

    pub(crate) fn record<Message>(
        &mut self,
        path: &[usize],
        incoming_slot: Option<crate::layout::SlotParams>,
        node: &crate::runtime::SurfaceNode<Message>,
    ) {
        let source = node.source_metadata_handle();
        let kind = crate::runtime::application_node_kind(node);
        self.unsupported |= source.is_none()
            || matches!(kind, ApplicationNodeKind::Unsupported)
            || matches!(node, crate::runtime::SurfaceNode::Scene(_));
        self.drafts.push(NodeDraft {
            path: path.into(),
            incoming_slot,
            id: node.id(),
            source,
            kind,
        });
    }

    pub(crate) fn record_component(&mut self, snapshot: Rc<()>) {
        if let Some(index) = self.drafts.len().checked_sub(1) {
            self.components.push((index, snapshot));
        } else {
            self.unsupported = true;
        }
    }

    pub(crate) fn mark_unsupported(&mut self) {
        self.unsupported = true;
    }

    pub(crate) fn finish(&mut self) -> (ApplicationProjectionReceipt, ReceiptComparison) {
        let nodes: Vec<_> = self
            .drafts
            .drain(..)
            .map(|draft| ApplicationNodeReceipt {
                path: draft.path,
                incoming_slot: draft.incoming_slot,
                id: draft.id,
                source: draft
                    .source
                    .as_deref()
                    .map(SourceMetadata::freeze)
                    .unwrap_or_else(FrozenSourceMetadata::empty),
                kind: draft.kind,
            })
            .collect();
        let mut receipt = ApplicationProjectionReceipt {
            nodes: nodes.into_boxed_slice(),
            supported: !self.unsupported,
            components: std::mem::take(&mut self.components),
            emitted_records: 0,
            comparison_count: 0,
        };
        receipt.emitted_records = receipt.nodes.len();
        let (comparison, comparison_count) = self
            .previous
            .map_or((ReceiptComparison::Full, 0), |previous| {
                compare_receipts(previous, &receipt)
            });
        receipt.comparison_count = comparison_count;
        (receipt, comparison)
    }
}

fn compare_receipts(
    previous: &ApplicationProjectionReceipt,
    current: &ApplicationProjectionReceipt,
) -> (ReceiptComparison, usize) {
    if !previous.supported || !current.supported || previous.nodes.len() != current.nodes.len() {
        return (ReceiptComparison::Full, 0);
    }
    // A token belongs to one immutable, Clone-qualified cache result. Keeping
    // both Rc owners alive prevents address reuse from accepting a replacement.
    // Changed inputs, eviction and remount always create a new token.
    if previous.components.len() != current.components.len() {
        return (ReceiptComparison::Full, 0);
    }
    let mut comparison_count = 0;
    for ((old_index, old), (new_index, new)) in previous.components.iter().zip(&current.components)
    {
        comparison_count += 1;
        if old_index != new_index || !Rc::ptr_eq(old, new) {
            return (ReceiptComparison::Full, comparison_count);
        }
    }
    let mut changed = Vec::new();
    let mut changed_path_components: usize = 0;
    for (old, new) in previous.nodes.iter().zip(&current.nodes) {
        comparison_count += 1;
        if old.path != new.path
            || old.incoming_slot != new.incoming_slot
            || old.id != new.id
            || old.source != new.source
        {
            return (ReceiptComparison::Full, comparison_count);
        }
        match (&old.kind, &new.kind) {
            (
                ApplicationNodeKind::Widget { evidence: old },
                ApplicationNodeKind::Widget { evidence: current },
            ) => match crate::runtime::classify_interaction_leaf_evidence(old, current) {
                crate::runtime::InteractionLeafRevision::Interaction => {
                    if changed.len() >= crate::runtime::MAX_EXACT_CHANGED_ROOTS
                        || changed_path_components.saturating_add(new.path.len())
                            > crate::runtime::MAX_EXACT_CHANGED_ROOT_PATH_COMPONENTS
                    {
                        return (ReceiptComparison::Full, comparison_count);
                    }
                    changed.push(ExactChangedRoot {
                        node_id: new.id,
                        child_path: new.path.to_vec(),
                    });
                    changed_path_components += new.path.len();
                }
                crate::runtime::InteractionLeafRevision::Unchanged => {}
                crate::runtime::InteractionLeafRevision::Reject => {
                    return (ReceiptComparison::Full, comparison_count);
                }
            },
            (ApplicationNodeKind::Container { .. }, ApplicationNodeKind::Container { .. })
                if crate::runtime::application_container_kind_matches(&old.kind, &new.kind) => {}
            _ => return (ReceiptComparison::Full, comparison_count),
        }
    }
    (ReceiptComparison::Exact(changed), comparison_count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::{
        ApplicationProjectionContext, DeclarativeIdentityOrigin, IntoView, WidgetView,
        WidgetViewContext,
    };
    use crate::layout::{ContainerPolicy, SlotParams, Vector2};
    use crate::runtime::{
        EventMapper, SourceCompatibility, SourceIdentity, SourceMetadata, SourceTopology,
        SurfaceChild, SurfaceNode, UiSurface, WidgetMessageMapper,
    };
    use crate::widgets::{
        ButtonMessage, ButtonWidget, ColorMarkerProps, ColorMarkerWidget, ColorMarkerWidgetParts,
        FocusBehavior, WidgetSizing,
    };
    use std::cell::Cell;
    use std::rc::Rc;

    #[derive(Clone, Copy, Debug)]
    enum ButtonVariant {
        Plain,
        Interaction,
        Slot,
        Geometry,
        Paint,
        Membership,
        Mapper,
        ConservativeMapper,
    }

    fn supported<Message>(node: SurfaceNode<Message>) -> SurfaceNode<Message> {
        let id = node.id();
        let metadata = SourceMetadata::new(
            SourceIdentity {
                resolved_id: id,
                structural_scope: id,
                origin: DeclarativeIdentityOrigin::UnreidentifiedDirectRuntimeRoot,
            },
            SourceCompatibility::from_surface_node(&node),
            SourceTopology::default(),
        );
        node.with_source_metadata(metadata)
    }

    fn button(id: u64, label: &str) -> SurfaceNode<()> {
        supported(SurfaceNode::widget(
            ButtonWidget::new(id, label, WidgetSizing::fixed(Vector2::new(80.0, 24.0))),
            WidgetMessageMapper::none(),
        ))
    }

    fn interaction_marker(id: u64, tooltip: Option<&str>) -> SurfaceNode<()> {
        let mut widget = ColorMarkerWidget::from_parts(ColorMarkerWidgetParts {
            id,
            sizing: WidgetSizing::fixed(Vector2::new(80.0, 24.0)),
            props: ColorMarkerProps::new(None),
        });
        widget.common.tooltip = tooltip.map(str::to_owned);
        supported(SurfaceNode::widget(widget, WidgetMessageMapper::none()))
    }

    fn nested_marker_view(tooltip: Option<&str>) -> super::super::ViewNode<()> {
        super::super::ViewNode::new(super::super::ViewNodeKind::Container {
            policy: ContainerPolicy::default(),
            children: vec![
                super::super::ViewNode::new(super::super::ViewNodeKind::Widget(Box::new({
                    let mut widget = ColorMarkerWidget::from_parts(ColorMarkerWidgetParts {
                        id: 7,
                        sizing: WidgetSizing::fixed(Vector2::new(80.0, 24.0)),
                        props: ColorMarkerProps::new(None),
                    });
                    widget.common.tooltip = tooltip.map(str::to_owned);
                    widget
                })))
                .id(7),
            ],
        })
        .id(1)
    }

    fn unidentified_marker_view() -> super::super::ViewNode<()> {
        super::super::ViewNode::new(super::super::ViewNodeKind::Widget(Box::new(
            ColorMarkerWidget::from_parts(ColorMarkerWidgetParts {
                id: 0,
                sizing: WidgetSizing::fixed(Vector2::new(80.0, 24.0)),
                props: ColorMarkerProps::new(None),
            }),
        )))
    }

    struct CountingButtonView {
        widget: ButtonWidget,
        projections: Rc<Cell<usize>>,
    }

    impl WidgetView<()> for CountingButtonView {
        fn default_sizing(&self) -> WidgetSizing {
            self.widget.common.sizing
        }

        fn into_surface_node(mut self: Box<Self>, context: WidgetViewContext) -> SurfaceNode<()> {
            self.projections
                .set(self.projections.get().saturating_add(1));
            context.apply_to(&mut self.widget);
            SurfaceNode::widget(self.widget, WidgetMessageMapper::none())
        }
    }

    fn counting_nested_view(projections: Rc<Cell<usize>>) -> super::super::ViewNode<()> {
        super::super::ViewNode::new(super::super::ViewNodeKind::Container {
            policy: ContainerPolicy::default(),
            children: vec![
                super::super::ViewNode::new(super::super::ViewNodeKind::Widget(Box::new(
                    CountingButtonView {
                        widget: ButtonWidget::new(
                            7,
                            "counted",
                            WidgetSizing::fixed(Vector2::new(80.0, 24.0)),
                        ),
                        projections,
                    },
                )))
                .id(7),
            ],
        })
        .id(1)
    }

    fn button_view(id: u64, variant: ButtonVariant) -> super::super::ViewNode<()> {
        let sizing = match variant {
            ButtonVariant::Geometry => WidgetSizing::fixed(Vector2::new(120.0, 24.0)),
            _ => WidgetSizing::fixed(Vector2::new(80.0, 24.0)),
        };
        let mut widget = ButtonWidget::new(id, "stable", sizing);
        if matches!(variant, ButtonVariant::Interaction) {
            widget = widget.with_secondary_click();
        }
        if matches!(variant, ButtonVariant::Paint) {
            widget = widget.with_hover_chrome_only();
        }
        if matches!(variant, ButtonVariant::Membership) {
            widget.common.focus = FocusBehavior::None;
        }
        let messages = if matches!(variant, ButtonVariant::Mapper) {
            WidgetMessageMapper::button_mapped(EventMapper::with_revision(
                2_u8,
                |_message: ButtonMessage| (),
            ))
        } else if matches!(variant, ButtonVariant::ConservativeMapper) {
            WidgetMessageMapper::button_mapped(EventMapper::new(|_message: ButtonMessage| ()))
        } else {
            WidgetMessageMapper::none()
        };
        let mut view = super::super::ViewNode::new(super::super::ViewNodeKind::Widget(Box::new(
            crate::application::MappedWidget::new(widget, messages),
        )))
        .id(id);
        if matches!(variant, ButtonVariant::Slot) {
            view = view.width(120.0);
        }
        view
    }

    fn button_row(variants: &[ButtonVariant]) -> super::super::ViewNode<()> {
        button_row_with_ids(
            &variants
                .iter()
                .enumerate()
                .map(|(index, variant)| (100 + index as u64, *variant))
                .collect::<Vec<_>>(),
        )
    }

    fn button_row_with_ids(entries: &[(u64, ButtonVariant)]) -> super::super::ViewNode<()> {
        super::super::ViewNode::new(super::super::ViewNodeKind::Container {
            policy: ContainerPolicy::default(),
            children: entries
                .iter()
                .map(|(id, variant)| button_view(*id, *variant))
                .collect(),
        })
        .id(1)
    }

    fn single_button_container(variant: ButtonVariant) -> super::super::ViewNode<()> {
        button_row_with_ids(&[(7, variant)])
    }

    fn keyed_parent_view_with_variant(variant: ButtonVariant) -> super::super::ViewNode<()> {
        let child = button_view(7, variant).key("child");
        super::super::ViewNode::new(super::super::ViewNodeKind::Container {
            policy: ContainerPolicy::default(),
            children: vec![child],
        })
        .key("parent")
    }

    fn keyed_parent_view() -> super::super::ViewNode<()> {
        keyed_parent_view_with_variant(ButtonVariant::Plain)
    }

    fn button_count_view(count: usize, interaction: Option<usize>) -> super::super::ViewNode<()> {
        let entries: Vec<_> = (0..count)
            .map(|index| {
                (
                    10_000 + index as u64,
                    if interaction.is_some_and(|changed| changed == index) {
                        ButtonVariant::Interaction
                    } else {
                        ButtonVariant::Plain
                    },
                )
            })
            .collect();
        button_row_with_ids(&entries)
    }

    fn button_count_all_view(count: usize, interaction: bool) -> super::super::ViewNode<()> {
        let entries: Vec<_> = (0..count)
            .map(|index| {
                (
                    10_000 + index as u64,
                    if interaction {
                        ButtonVariant::Interaction
                    } else {
                        ButtonVariant::Plain
                    },
                )
            })
            .collect();
        button_row_with_ids(&entries)
    }

    fn nested_button_depth(depth: usize, interaction: bool) -> super::super::ViewNode<()> {
        let mut view = button_view(
            7,
            if interaction {
                ButtonVariant::Interaction
            } else {
                ButtonVariant::Plain
            },
        );
        for level in 0..depth {
            view = super::super::ViewNode::new(super::super::ViewNodeKind::Container {
                policy: ContainerPolicy::default(),
                children: vec![view],
            })
            .id(20_000 + level as u64);
        }
        view
    }

    struct EmptyVirtualPolicy;

    impl crate::layout::VirtualLayoutPolicy for EmptyVirtualPolicy {
        fn query(
            &self,
            _input: &crate::layout::VirtualLayoutQueryInput,
            _sink: &mut crate::layout::VirtualLayoutQuerySink,
        ) -> crate::layout::VirtualLayoutPolicyDecision {
            crate::layout::VirtualLayoutPolicyDecision::Ready
        }
    }

    fn virtual_view() -> super::super::ViewNode<()> {
        crate::application::virtual_layout_from_parts(crate::application::VirtualLayoutParts::new(
            Rc::new(EmptyVirtualPolicy),
            crate::layout::VirtualLayoutPolicyIdentity::new("receipt-test"),
            crate::layout::VirtualLayoutOverscan::new(0.0, 0.0).unwrap(),
            crate::layout::VirtualLayoutBudget::new(1),
            crate::runtime::VirtualLayoutRevisions::new(0, 0, 0, 0),
            Rc::new(|| nested_marker_view(None)),
            Rc::new(|_| nested_marker_view(None)),
            Rc::new(|_| crate::layout::VirtualLayoutPolicyIdentity::new("item")),
        ))
    }

    fn direct_runtime_view() -> super::super::ViewNode<()> {
        super::super::ViewNode::from(SurfaceNode::widget(
            ButtonWidget::new(7, "direct", WidgetSizing::fixed(Vector2::new(80.0, 24.0))),
            WidgetMessageMapper::none(),
        ))
    }

    fn custom_projection_view() -> crate::application::ViewProjection<()> {
        crate::application::ViewProjection::from_surface(UiSurface::new(SurfaceNode::widget(
            ButtonWidget::new(7, "custom", WidgetSizing::fixed(Vector2::new(80.0, 24.0))),
            WidgetMessageMapper::none(),
        )))
    }

    struct CountingCustomView {
        projections: Rc<Cell<usize>>,
    }

    impl IntoView<()> for CountingCustomView {
        fn into_projection(self) -> crate::application::ViewProjection<()> {
            self.projections
                .set(self.projections.get().saturating_add(1));
            custom_projection_view()
        }
    }

    fn offset_settled_row(enabled: bool) -> super::super::ViewNode<()> {
        let row = button_row(&[ButtonVariant::Plain]);
        if enabled {
            row.on_offset_settled(|_| ())
        } else {
            row
        }
    }

    fn split_callback_view(enabled: bool) -> super::super::ViewNode<()> {
        let builder = crate::application::split_pane(
            button_view(10, ButtonVariant::Plain),
            button_view(11, ButtonVariant::Plain),
        );
        if enabled {
            builder.on_ratio_settled(|_| ()).into_view()
        } else {
            builder.into_view()
        }
    }

    fn scene_view(layered: bool) -> super::super::ViewNode<()> {
        let scene = crate::application::scene(unidentified_marker_view());
        if layered {
            scene
                .layer(crate::application::Layer::tooltip(
                    unidentified_marker_view(),
                ))
                .into_view()
        } else {
            scene.into_view()
        }
    }

    fn scene_with_public_lifecycle_view() -> super::super::ViewNode<()> {
        crate::application::scene(unidentified_marker_view())
            .frame_clock(crate::application::FrameClock::<(), ()>::message(()))
            .shortcuts(crate::gui::shortcuts::ShortcutCatalog::new())
            .into_view()
    }

    fn base_owned_overlay_view() -> super::super::ViewNode<()> {
        crate::application::scene(
            unidentified_marker_view()
                .overlays(crate::application::overlays().floating(unidentified_marker_view())),
        )
        .into_view()
    }

    fn lower_view(
        view: super::super::ViewNode<()>,
        previous: Option<&ApplicationProjectionReceipt>,
    ) -> (ApplicationProjectionReceipt, ReceiptComparison) {
        let mut recorder = ApplicationProjectionRecorder::new(previous);
        let mut context = ApplicationProjectionContext::new(&mut recorder);
        let _projection = view.into_application_projection(&mut context);
        context.finish()
    }

    #[test]
    fn identical_receipts_are_exact_without_roots() {
        let node = button(7, "same");
        let mut first = ApplicationProjectionRecorder::new(None);
        first.record(&[], None, &node);
        let (previous, _) = first.finish();

        let mut second = ApplicationProjectionRecorder::new(Some(&previous));
        second.record(&[], None, &node);
        let (_, comparison) = second.finish();
        assert_eq!(comparison, ReceiptComparison::Exact(Vec::new()));
    }

    #[test]
    fn synthetic_receipt_path_comparison_accepts_explicit_path() {
        let old_child = interaction_marker(7, None);
        let new_child = interaction_marker(7, Some("changed"));
        let mut first = ApplicationProjectionRecorder::new(None);
        first.record(&[1], Some(SlotParams::fill()), &old_child);
        let old_parent = supported(SurfaceNode::container(
            1,
            ContainerPolicy::default(),
            vec![SurfaceChild::fill(old_child)],
        ));
        first.record(&[], None, &old_parent);
        let (previous, _) = first.finish();

        let mut second = ApplicationProjectionRecorder::new(Some(&previous));
        second.record(&[1], Some(SlotParams::fill()), &new_child);
        let new_parent = supported(SurfaceNode::container(
            1,
            ContainerPolicy::default(),
            vec![SurfaceChild::fill(new_child)],
        ));
        second.record(&[], None, &new_parent);
        let (_, comparison) = second.finish();
        assert_eq!(
            comparison,
            ReceiptComparison::Exact(vec![ExactChangedRoot {
                node_id: 7,
                child_path: vec![1],
            }])
        );
    }

    #[test]
    fn unsupported_previous_receipt_cannot_become_exact() {
        let node = button(7, "same");
        let mut first = ApplicationProjectionRecorder::new(None);
        first.mark_unsupported();
        first.record(&[], None, &node);
        let (previous, _) = first.finish();

        let mut second = ApplicationProjectionRecorder::new(Some(&previous));
        second.record(&[], None, &node);
        let (_, comparison) = second.finish();
        assert_eq!(comparison, ReceiptComparison::Full);
    }

    #[test]
    fn synthetic_changed_root_bound_is_full_instead_of_truncated() {
        let old_nodes: Vec<_> = (0..65).map(|id| button(id, "old")).collect();
        let new_nodes: Vec<_> = (0..65).map(|id| button(id, "new")).collect();
        let mut first = ApplicationProjectionRecorder::new(None);
        for (index, node) in old_nodes.iter().enumerate() {
            first.record(&[index], Some(SlotParams::fill()), node);
        }
        let (previous, _) = first.finish();
        let mut second = ApplicationProjectionRecorder::new(Some(&previous));
        for (index, node) in new_nodes.iter().enumerate() {
            second.record(&[index], Some(SlotParams::fill()), node);
        }
        let (_, comparison) = second.finish();
        assert_eq!(comparison, ReceiptComparison::Full);
    }

    #[test]
    fn lowering_records_postorder_paths_and_one_widget_projection() {
        let first_projections = Rc::new(Cell::new(0));
        let (previous, _) = lower_view(counting_nested_view(Rc::clone(&first_projections)), None);
        assert_eq!(first_projections.get(), 1);
        assert_eq!(previous.nodes.len(), 2);
        assert_eq!(&*previous.nodes[0].path, &[0]);
        assert!(previous.nodes[1].path.is_empty());

        let second_projections = Rc::new(Cell::new(0));
        let (_, comparison) = lower_view(
            counting_nested_view(Rc::clone(&second_projections)),
            Some(&previous),
        );
        assert_eq!(second_projections.get(), 1);
        assert_eq!(comparison, ReceiptComparison::Exact(Vec::new()));
    }

    #[test]
    fn actual_nested_button_change_reports_one_root_and_unchanged_siblings() {
        let (previous, _) = lower_view(button_row(&[ButtonVariant::Plain; 3]), None);
        let (_, comparison) = lower_view(
            button_row(&[
                ButtonVariant::Plain,
                ButtonVariant::Interaction,
                ButtonVariant::Plain,
            ]),
            Some(&previous),
        );
        assert_eq!(
            comparison,
            ReceiptComparison::Exact(vec![ExactChangedRoot {
                node_id: 101,
                child_path: vec![1],
            }])
        );
    }

    #[test]
    fn actual_identical_view_has_no_exact_roots() {
        let (previous, _) = lower_view(button_row(&[ButtonVariant::Plain; 3]), None);
        let (_, comparison) = lower_view(button_row(&[ButtonVariant::Plain; 3]), Some(&previous));
        assert_eq!(comparison, ReceiptComparison::Exact(Vec::new()));
    }

    #[test]
    fn actual_slot_change_falls_back_full() {
        let (previous, _) = lower_view(button_row(&[ButtonVariant::Plain]), None);
        let (_, comparison) = lower_view(button_row(&[ButtonVariant::Slot]), Some(&previous));
        assert_eq!(comparison, ReceiptComparison::Full);
    }

    #[test]
    fn actual_reorder_falls_back_full() {
        let (previous, _) = lower_view(
            button_row_with_ids(&[(100, ButtonVariant::Plain), (101, ButtonVariant::Plain)]),
            None,
        );
        let (_, reordered) = lower_view(
            button_row_with_ids(&[(101, ButtonVariant::Plain), (100, ButtonVariant::Plain)]),
            Some(&previous),
        );
        assert_eq!(reordered, ReceiptComparison::Full);
    }

    #[test]
    fn actual_child_add_remove_falls_back_full() {
        let (previous, _) = lower_view(
            button_row_with_ids(&[(100, ButtonVariant::Plain), (101, ButtonVariant::Plain)]),
            None,
        );
        let (_, added) = lower_view(
            button_row_with_ids(&[
                (100, ButtonVariant::Plain),
                (101, ButtonVariant::Plain),
                (102, ButtonVariant::Plain),
            ]),
            Some(&previous),
        );
        assert_eq!(added, ReceiptComparison::Full);

        let (_, removed) = lower_view(
            button_row_with_ids(&[(100, ButtonVariant::Plain)]),
            Some(&previous),
        );
        assert_eq!(removed, ReceiptComparison::Full);
    }

    fn assert_actual_variant_falls_back_full(variant: ButtonVariant) {
        let (previous, _) = lower_view(single_button_container(ButtonVariant::Plain), None);
        let (_, comparison) = lower_view(single_button_container(variant), Some(&previous));
        assert_eq!(comparison, ReceiptComparison::Full, "variant {variant:?}");
    }

    #[test]
    fn actual_source_compatibility_change_falls_back_full() {
        let (previous, _) = lower_view(single_button_container(ButtonVariant::Plain), None);
        let (_, source_changed) = lower_view(nested_marker_view(None), Some(&previous));
        assert_eq!(source_changed, ReceiptComparison::Full);
    }

    #[test]
    fn actual_geometry_change_falls_back_full() {
        assert_actual_variant_falls_back_full(ButtonVariant::Geometry);
    }

    #[test]
    fn actual_paint_change_falls_back_full() {
        assert_actual_variant_falls_back_full(ButtonVariant::Paint);
    }

    #[test]
    fn actual_exact_mapper_change_falls_back_full() {
        assert_actual_variant_falls_back_full(ButtonVariant::Mapper);
    }

    #[test]
    fn actual_conservative_mapper_change_falls_back_full() {
        assert_actual_variant_falls_back_full(ButtonVariant::ConservativeMapper);
    }

    #[test]
    fn actual_membership_change_falls_back_full() {
        assert_actual_variant_falls_back_full(ButtonVariant::Membership);
    }

    #[test]
    fn actual_offset_settled_callback_change_falls_back_full() {
        let (previous, _) = lower_view(offset_settled_row(true), None);
        let (_, comparison) = lower_view(offset_settled_row(false), Some(&previous));
        assert_eq!(comparison, ReceiptComparison::Full);

        let (previous, _) = lower_view(offset_settled_row(true), None);
        let (_, comparison) = lower_view(offset_settled_row(true), Some(&previous));
        assert_eq!(comparison, ReceiptComparison::Full);
    }

    #[test]
    fn actual_split_callback_and_layout_capability_change_falls_back_full() {
        let (previous, _) = lower_view(split_callback_view(true), None);
        let (_, comparison) = lower_view(split_callback_view(false), Some(&previous));
        assert_eq!(comparison, ReceiptComparison::Full);

        let (previous, _) = lower_view(split_callback_view(true), None);
        let (_, comparison) = lower_view(split_callback_view(true), Some(&previous));
        assert_eq!(comparison, ReceiptComparison::Full);
    }

    #[test]
    fn actual_keyed_parent_and_child_freeze_final_compatibility() {
        let (receipt, _) = lower_view(keyed_parent_view(), None);
        assert!(receipt.supported);
        let keyed = receipt
            .nodes
            .iter()
            .filter(|node| {
                matches!(
                    node.source.identity.origin,
                    DeclarativeIdentityOrigin::ExplicitContinuityKey
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(keyed.len(), 2, "parent and child keyed receipts");
        for node in keyed {
            assert!(node.source.compatibility.is_known());
            assert!(
                node.source
                    .keyed_nodes
                    .iter()
                    .any(|(identity, compatibility, _)| {
                        identity.origin == DeclarativeIdentityOrigin::ExplicitContinuityKey
                            && compatibility.is_known()
                    })
            );
        }
    }

    #[test]
    fn actual_keyed_child_path_and_parent_compatibility_drive_one_exact_root() {
        let (previous, _) = lower_view(keyed_parent_view(), None);
        let parent = previous
            .nodes
            .iter()
            .find(|node| {
                node.path.is_empty() && matches!(node.kind, ApplicationNodeKind::Container { .. })
            })
            .expect("keyed parent container receipt");
        let child = previous
            .nodes
            .iter()
            .find(|node| {
                node.path.as_ref() == [0] && matches!(node.kind, ApplicationNodeKind::Widget { .. })
            })
            .expect("keyed child widget receipt");
        assert!(parent.source.compatibility.is_known());
        assert!(child.source.compatibility.is_known());
        assert!(
            child
                .source
                .keyed_nodes
                .iter()
                .any(|(identity, compatibility, _)| {
                    identity.origin == DeclarativeIdentityOrigin::ExplicitContinuityKey
                        && compatibility.is_known()
                })
        );

        let (_, comparison) = lower_view(
            keyed_parent_view_with_variant(ButtonVariant::Interaction),
            Some(&previous),
        );
        assert_eq!(
            comparison,
            ReceiptComparison::Exact(vec![ExactChangedRoot {
                node_id: child.id,
                child_path: vec![0],
            }])
        );
    }

    #[test]
    fn actual_changed_root_bound_admits_64_and_rejects_65() {
        let (previous_64, _) = lower_view(button_count_all_view(64, false), None);
        let (current_64, exact_64) =
            lower_view(button_count_all_view(64, true), Some(&previous_64));
        let ReceiptComparison::Exact(changed_64) = exact_64 else {
            panic!("64 changed roots should remain exact");
        };
        assert_eq!(changed_64.len(), 64);
        assert_eq!(current_64.emitted_records, 65);
        assert_eq!(current_64.comparison_count, 65);

        let (previous_65, _) = lower_view(button_count_all_view(65, false), None);
        let (current_65, full_65) = lower_view(button_count_all_view(65, true), Some(&previous_65));
        assert_eq!(full_65, ReceiptComparison::Full);
        assert_eq!(current_65.emitted_records, 66);
        assert_eq!(current_65.comparison_count, 65);
    }

    #[test]
    fn actual_changed_path_bound_admits_256_and_rejects_257_components() {
        std::thread::Builder::new()
            .name("receipt-depth-boundary".to_owned())
            .stack_size(16 * 1024 * 1024)
            .spawn(|| {
                let (previous_256, _) = lower_view(nested_button_depth(256, false), None);
                let (current_256, exact_256) =
                    lower_view(nested_button_depth(256, true), Some(&previous_256));
                let ReceiptComparison::Exact(changed_256) = exact_256 else {
                    panic!("256 path components should remain exact");
                };
                assert_eq!(changed_256.len(), 1);
                assert_eq!(changed_256[0].child_path.len(), 256);
                assert_eq!(current_256.comparison_count, 257);

                let (previous_257, _) = lower_view(nested_button_depth(257, false), None);
                let (current_257, full_257) =
                    lower_view(nested_button_depth(257, true), Some(&previous_257));
                assert_eq!(full_257, ReceiptComparison::Full);
                assert_eq!(current_257.comparison_count, 1);
            })
            .expect("depth-boundary thread should spawn")
            .join()
            .expect("depth-boundary lowering should complete");
    }

    #[test]
    fn actual_3000_button_tree_reports_one_changed_root() {
        let (previous, _) = lower_view(button_count_view(3000, None), None);
        assert_eq!(previous.nodes.len(), 3001);
        let (current, comparison) =
            lower_view(button_count_view(3000, Some(1777)), Some(&previous));
        assert_eq!(
            comparison,
            ReceiptComparison::Exact(vec![ExactChangedRoot {
                node_id: 10_000 + 1777,
                child_path: vec![1777],
            }])
        );
        assert_eq!(current.emitted_records, 3001);
        assert_eq!(current.comparison_count, 3001);
    }

    #[test]
    fn actual_3000_changed_buttons_stop_at_bounded_root_comparison() {
        let (previous, _) = lower_view(button_count_all_view(3000, false), None);
        let (current, comparison) = lower_view(button_count_all_view(3000, true), Some(&previous));
        assert_eq!(comparison, ReceiptComparison::Full);
        assert_eq!(current.emitted_records, 3001);
        assert_eq!(current.comparison_count, 65);
    }

    #[test]
    fn actual_direct_runtime_view_falls_back_full() {
        let (previous, _) = lower_view(button_row(&[ButtonVariant::Plain]), None);
        let (_, direct) = lower_view(direct_runtime_view(), Some(&previous));
        assert_eq!(direct, ReceiptComparison::Full);
    }

    #[test]
    fn actual_custom_projection_falls_back_full() {
        let projections = Rc::new(Cell::new(0));
        let (custom_receipt, custom_initial) = {
            let mut recorder = ApplicationProjectionRecorder::new(None);
            let mut context = ApplicationProjectionContext::new(&mut recorder);
            let _projection = (CountingCustomView {
                projections: Rc::clone(&projections),
            })
            .into_application_projection(&mut context);
            context.finish()
        };
        assert_eq!(custom_initial, ReceiptComparison::Full);
        assert!(!custom_receipt.supported);
        assert_eq!(projections.get(), 1);
    }

    #[test]
    fn actual_virtual_layout_falls_back_full() {
        let (virtual_receipt, virtual_initial) = lower_view(virtual_view(), None);
        assert_eq!(virtual_initial, ReceiptComparison::Full);
        assert!(!virtual_receipt.supported);
    }

    #[test]
    fn actual_scene_lifecycle_and_layer_views_fall_back_full() {
        let (unsupported_previous, _) = lower_view(scene_view(false), None);
        for scene in [
            scene_view(false),
            scene_view(true),
            base_owned_overlay_view(),
        ] {
            let (scene_receipt, scene_initial) = lower_view(scene, None);
            assert_eq!(scene_initial, ReceiptComparison::Full);
            assert!(!scene_receipt.supported);
        }

        let (lifecycle_receipt, lifecycle_initial) =
            lower_view(scene_with_public_lifecycle_view(), None);
        assert_eq!(lifecycle_initial, ReceiptComparison::Full);
        assert!(!lifecycle_receipt.supported);

        let (_, unsupported_to_supported) = lower_view(
            button_row(&[ButtonVariant::Plain]),
            Some(&unsupported_previous),
        );
        assert_eq!(unsupported_to_supported, ReceiptComparison::Full);
    }
}
