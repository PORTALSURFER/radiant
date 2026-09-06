use super::*;
use crate::{
    runtime::{Event, PaintPrimitive, ScrollEditBatch, SurfaceNode, UiSurface},
    widgets::{EditPhase, PointerModifiers, WheelDelta, WheelPhase, WheelSample, WidgetSizing},
};
use std::{cell::Cell, rc::Rc, sync::Arc};

struct Bridge {
    version: Rc<Cell<u8>>,
    removed: Rc<Cell<bool>>,
    source_replacement: bool,
    edits: Vec<(u8, ScrollEditBatch)>,
}
impl RuntimeBridge<(u8, ScrollEditBatch)> for Bridge {
    #[allow(clippy::arc_with_non_send_sync)]
    fn project_surface(&mut self) -> Arc<UiSurface<(u8, ScrollEditBatch)>> {
        if self.removed.get() {
            return Arc::new(UiSurface::new(SurfaceNode::text(
                90,
                "replacement",
                WidgetSizing::fixed(Vector2::new(100.0, 40.0)),
            )));
        }
        let version = self.version.get();
        let content = SurfaceNode::text(
            32,
            "content",
            WidgetSizing::fixed(Vector2::new(
                180.0,
                if version == 0 || self.source_replacement {
                    400.0
                } else {
                    800.0
                },
            )),
        );
        let node = SurfaceNode::scroll_area(31, content)
            .scroll_policy(
                crate::layout::ScrollPolicy::default()
                    .scrollbar_visibility(crate::layout::ScrollbarVisibility::Always),
            )
            .on_scroll_edit(move |batch| (version, batch));
        let node = if self.source_replacement {
            use crate::runtime::{
                SourceCompatibility, SourceIdentity, SourceMetadata, SourceTopology,
            };
            let metadata = SourceMetadata::new(SourceIdentity {
                resolved_id: 31, structural_scope: 1000 + u64::from(version),
                origin: crate::application::DeclarativeIdentityOrigin::UnreidentifiedDirectRuntimeRoot,
            }, SourceCompatibility::from_surface_node(&node), SourceTopology::default());
            node.with_source_metadata(metadata)
        } else {
            node
        };
        Arc::new(UiSurface::new(node))
    }
    fn reduce_message(&mut self, message: (u8, ScrollEditBatch)) {
        self.edits.push(message);
    }
}
fn sample(phase: WheelPhase) -> WheelSample {
    WheelSample::new(
        WheelDelta::Pixels(Vector2::new(0.0, 20.0)),
        Some(phase),
        PointerModifiers::default(),
    )
    .unwrap()
}
fn thumb(runtime: &SurfaceRuntime<Bridge, (u8, ScrollEditBatch)>) -> Point {
    runtime
        .paint_plan(&Default::default())
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            PaintPrimitive::FillRect(fill) if fill.widget_id == 31 => Some(fill.rect.center()),
            _ => None,
        })
        .unwrap()
}

#[test]
fn removed_or_replaced_scroll_edits_queue_old_mapper_terminal_after_surface_commit() {
    for wheel in [false, true] {
        for (removed, source_replacement) in [(false, false), (true, false), (false, true)] {
            let version = Rc::new(Cell::new(0));
            let absent = Rc::new(Cell::new(false));
            let mut runtime = SurfaceRuntime::new(
                Bridge {
                    version: Rc::clone(&version),
                    removed: Rc::clone(&absent),
                    source_replacement,
                    edits: Vec::new(),
                },
                Vector2::new(220.0, 96.0),
            );
            let start = thumb(&runtime);
            if wheel {
                assert!(runtime.wheel_or_scroll_at_with_sample(
                    Point::new(20.0, 20.0),
                    sample(WheelPhase::Started)
                ));
            } else {
                runtime.dispatch_event(Event::primary_press(start));
                runtime.dispatch_event(Event::pointer_move(Point::new(start.x, start.y + 20.0)));
            }
            let transaction = runtime.bridge().edits[0].1.transaction();
            let before = runtime.bridge().edits.len();
            version.set(1);
            absent.set(removed);
            let pending = runtime.refresh_with_scope_inner(RepaintScope::Surface);
            assert_eq!(
                runtime.bridge().edits.len(),
                before,
                "no reducer may run during the staged refresh"
            );
            assert_eq!(pending.len(), 1);
            assert_eq!(pending[0].0, 0, "terminal belongs to the outgoing mapper");
            assert_eq!(pending[0].1.transaction(), transaction);
            assert_eq!(pending[0].1.events()[0].phase, EditPhase::Cancel);
            assert!(pending[0].1.offset_update().is_none());
            if removed {
                assert!(runtime.surface.find_widget(90).is_some());
            } else {
                assert_eq!(
                    runtime.layout().rects[&32].height(),
                    if source_replacement { 400.0 } else { 800.0 }
                );
            }
            runtime.dispatch_deferred_surface_messages(pending);
            assert_eq!(runtime.bridge().edits.len(), before + 1);

            absent.set(false);
            version.set(2);
            runtime.refresh();
            if wheel {
                assert!(!runtime.wheel_or_scroll_at_with_sample(
                    Point::new(20.0, 20.0),
                    sample(WheelPhase::Changed)
                ));
                assert!(!runtime.wheel_or_scroll_at_with_sample(
                    Point::new(20.0, 20.0),
                    sample(WheelPhase::Ended)
                ));
                runtime.wheel_or_scroll_at_with_sample(
                    Point::new(20.0, 20.0),
                    sample(WheelPhase::Started),
                );
            } else {
                runtime.dispatch_event(Event::pointer_move(Point::new(start.x, start.y + 40.0)));
                runtime.dispatch_event(Event::primary_release(Point::new(start.x, start.y + 40.0)));
                assert_eq!(runtime.bridge().edits.len(), before + 1);
                let start = thumb(&runtime);
                runtime.dispatch_event(Event::primary_press(start));
            }
            let latest = runtime.bridge().edits.last().unwrap();
            assert_eq!(latest.0, 2);
            assert_eq!(latest.1.events()[0].phase, EditPhase::Begin);
            assert_ne!(latest.1.transaction(), transaction);
        }
    }
}
