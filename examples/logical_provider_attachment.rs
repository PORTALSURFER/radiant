//! Portable declarative logical virtual-layout provider attachment.

use radiant::prelude as ui;
use radiant::{
    application::virtual_layout::VirtualLayoutSemanticCardinality,
    application::{VirtualLayoutParts, virtual_layout_from_parts},
    gui::automation::{AutomationNodeId, AutomationNodeSemantics, AutomationRole},
    layout::{
        VirtualLayoutBoundsConfidence, VirtualLayoutBudget, VirtualLayoutExtentCandidate,
        VirtualLayoutItemCandidate, VirtualLayoutItemKey, VirtualLayoutOverscan,
        VirtualLayoutPolicy, VirtualLayoutPolicyDecision, VirtualLayoutPolicyIdentity,
        VirtualLayoutQueryInput, VirtualLayoutQuerySink, VirtualLayoutVisibility,
    },
    runtime::{
        VirtualLayoutRevisions, VirtualLayoutSemanticEntry, VirtualLayoutSemanticProviderOutcome,
        VirtualLayoutSemanticRangeProvider, VirtualLayoutSemanticRangeRequest,
    },
};
use std::rc::Rc;

struct PlaylistPolicy;

impl VirtualLayoutPolicy for PlaylistPolicy {
    fn query(
        &self,
        _input: &VirtualLayoutQueryInput,
        sink: &mut VirtualLayoutQuerySink,
    ) -> VirtualLayoutPolicyDecision {
        sink.visit(VirtualLayoutItemCandidate::new(
            VirtualLayoutItemKey::new(1_u32),
            0,
            radiant::layout::Rect::from_xy_size(0.0, 0.0, 280.0, 28.0),
            VirtualLayoutVisibility::Visible,
            VirtualLayoutBoundsConfidence::Exact,
        ))
        .expect("the example budget admits one item");
        sink.set_extent(VirtualLayoutExtentCandidate::exact(
            radiant::layout::Vector2::new(280.0, 28.0),
        ))
        .expect("the example supplies one extent");
        VirtualLayoutPolicyDecision::Ready
    }
}

fn provider_entry() -> VirtualLayoutSemanticEntry {
    VirtualLayoutSemanticEntry::new(
        VirtualLayoutItemKey::new(1_u32),
        0,
        radiant::layout::Rect::from_xy_size(0.0, 0.0, 280.0, 28.0),
        AutomationNodeSemantics::new(AutomationRole::Row).with_label("Playlist item"),
        AutomationNodeId::new("playlist-item-1"),
    )
}

fn view() -> ui::View<()> {
    let range_provider: Rc<dyn VirtualLayoutSemanticRangeProvider> =
        Rc::new(|_request: &VirtualLayoutSemanticRangeRequest| {
            VirtualLayoutSemanticProviderOutcome::Found(vec![provider_entry()])
        });
    let parts = VirtualLayoutParts::new(
        Rc::new(PlaylistPolicy),
        VirtualLayoutPolicyIdentity::new("playlist"),
        VirtualLayoutOverscan::new(0.0, 0.0).expect("finite overscan"),
        VirtualLayoutBudget::new(8),
        VirtualLayoutRevisions::new(1, 1, 1, 1),
        Rc::new(|| ui::scroll(ui::spacer::<()>().size(280.0, 28.0))),
        Rc::new(|_item| ui::text::<()>("Playlist item")),
        Rc::new(|_item| VirtualLayoutPolicyIdentity::new("playlist-item")),
    )
    .with_semantic_range_provider(range_provider)
    .with_semantic_cardinality(VirtualLayoutSemanticCardinality::new(1, 1));

    ui::column([
        ui::text("Logical provider attachment"),
        virtual_layout_from_parts(parts).fill_height(),
    ])
    .padding(16.0)
}

fn main() -> radiant::Result {
    radiant::window("Logical Provider Attachment")
        .size(360, 180)
        .min_size(280, 140)
        .run(view())
}
