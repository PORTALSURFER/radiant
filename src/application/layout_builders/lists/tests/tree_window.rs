use super::*;
use crate::{
    application::{ViewNode, button},
    gui::{
        list::{TreeGuideRow, TreeGuideStyle, VirtualListWindow},
        types::Rgba8,
    },
    layout::NodeId,
};
use std::{cell::RefCell, rc::Rc};

#[test]
fn virtual_tree_list_window_projects_rows_and_guide_overlay_together() {
    let window = VirtualListWindow {
        total_items: 10_000,
        viewport_start: 120,
        viewport_end: 128,
        window_start: 116,
        window_end: 132,
    };
    let guides = (0..10_000)
        .map(|index| TreeGuideRow::new(index % 3, index % 4 == 0))
        .collect::<Vec<_>>();
    let style = TreeGuideStyle::new(12.0, 24.0, Rgba8::new(90, 120, 160, 255));
    let mut projected = Vec::new();

    let view: ViewNode<()> = virtual_tree_list_window(
        window,
        24.0,
        &guides,
        style,
        |index| {
            projected.push(index);
            list_row_id(
                50_000 + index as NodeId,
                [button(format!("Folder {index:05}"))
                    .message(())
                    .id(60_000 + index as NodeId)],
            )
        },
        48.0,
    );

    assert_eq!(projected, (116..132).collect::<Vec<_>>());
    let layout = view.into_surface().layout_node();
    assert!(
        count_layout_nodes(&layout) < 72,
        "virtual tree projection should stay bounded to materialized rows plus overlay"
    );
}

#[test]
fn virtual_tree_list_windowed_projects_rows_and_accepts_window_messages() {
    let window = VirtualListWindow {
        total_items: 10_000,
        viewport_start: 120,
        viewport_end: 128,
        window_start: 116,
        window_end: 132,
    };
    let guides = (0..10_000)
        .map(|index| TreeGuideRow::new(index % 3, index % 4 == 0))
        .collect::<Vec<_>>();
    let style = TreeGuideStyle::new(12.0, 24.0, Rgba8::new(90, 120, 160, 255));
    let mut projected = Vec::new();

    let view: ViewNode<()> = virtual_tree_list_windowed(window, 24.0, &guides, style, |index| {
        projected.push(index);
        list_row_id(
            50_000 + index as NodeId,
            [button(format!("Folder {index:05}"))
                .message(())
                .id(60_000 + index as NodeId)],
        )
    })
    .overscan_px(48.0)
    .on_window_changed(|_| ())
    .view();

    assert_eq!(projected, (116..132).collect::<Vec<_>>());
    let layout = view.into_surface().layout_node();
    assert!(
        count_layout_nodes(&layout) < 72,
        "windowed virtual tree projection should stay bounded to materialized rows plus overlay"
    );
}

#[test]
fn virtual_tree_list_window_mapper_accepts_ui_local_capture_on_scroll() {
    let calls = Rc::new(RefCell::new(0usize));
    let calls_for_projector = Rc::clone(&calls);
    let bridge = crate::runtime::DeclarativeOwnedRuntimeBridge::new(
        Vec::<()>::new(),
        move |_| {
            let calls_for_mapper = Rc::clone(&calls_for_projector);
            let guides = (0..256)
                .map(|index| TreeGuideRow::new(index % 2, index % 3 == 0))
                .collect::<Vec<_>>();
            virtual_tree_list_windowed(
                VirtualListWindow {
                    total_items: 256,
                    viewport_start: 0,
                    viewport_end: 6,
                    window_start: 0,
                    window_end: 8,
                },
                24.0,
                &guides,
                TreeGuideStyle::new(12.0, 24.0, Rgba8::new(90, 120, 160, 255)),
                |index| {
                    list_row_id(
                        50_000 + index as NodeId,
                        [button(format!("Folder {index:05}"))
                            .message(())
                            .id(60_000 + index as NodeId)],
                    )
                },
            )
            .on_window_changed(move |_| {
                *calls_for_mapper.borrow_mut() += 1;
            })
            .view()
            .into_surface()
        },
        |state, message| state.push(message),
    );
    let mut runtime =
        crate::runtime::SurfaceRuntime::new(bridge, crate::layout::Vector2::new(320.0, 144.0));

    assert!(runtime.scroll_at(
        crate::gui::types::Point::new(20.0, 20.0),
        crate::layout::Vector2::new(0.0, 144.0),
    ));
    assert_eq!(*calls.borrow(), 1);
    assert_eq!(runtime.bridge().state().len(), 1);
}
