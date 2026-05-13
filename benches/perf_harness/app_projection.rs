//! Application-builder projection performance scenarios.

use radiant::prelude::{
    IntoView, VirtualListWindowRequest, button, list_row_id, resolve_virtual_list_window,
    selectable, virtual_list, virtual_list_window,
};
use std::hint::black_box;

pub(super) fn virtual_list_projection_10k() -> impl FnMut() {
    bench_app_virtual_list_projection_10k
}

pub(super) fn virtual_list_projection_generated_child_ids_10k() -> impl FnMut() {
    bench_app_virtual_list_projection_generated_child_ids_10k
}

pub(super) fn virtual_selectable_list_projection_10k() -> impl FnMut() {
    bench_app_virtual_selectable_list_projection_10k
}

pub(super) fn virtual_list_window_projection_10k() -> impl FnMut() {
    bench_app_virtual_list_window_projection_10k
}

fn bench_app_virtual_list_projection_10k() {
    let surface = virtual_list(
        0..10_000_u64,
        |index| {
            list_row_id(
                index + 10_000,
                [button(format!("Row {index:05}"))
                    .message(())
                    .id(index + 20_000)
                    .fill_width()
                    .height(28.0)],
            )
            .height(32.0)
        },
        96.0,
    )
    .into_surface();
    let layout = surface.layout_node();
    assert_eq!(layout.id(), 1);
    black_box((surface, layout));
}

fn bench_app_virtual_list_projection_generated_child_ids_10k() {
    let surface = virtual_list(
        0..10_000_u64,
        |index| {
            list_row_id(
                index + 10_000,
                [button(format!("Row {index:05}"))
                    .message(())
                    .fill_width()
                    .height(28.0)],
            )
            .height(32.0)
        },
        96.0,
    )
    .into_surface();
    let layout = surface.layout_node();
    assert_eq!(layout.id(), 1);
    black_box((surface, layout));
}

fn bench_app_virtual_selectable_list_projection_10k() {
    let surface = virtual_list(
        0..10_000_u64,
        |index| {
            selectable(format!("Row {index:05}"), false)
                .message(move |_| ())
                .id(index + 10_000)
                .fill_width()
                .height(32.0)
        },
        96.0,
    )
    .into_surface();
    let layout = surface.layout_node();
    assert_eq!(layout.id(), 1);
    black_box((surface, layout));
}

fn bench_app_virtual_list_window_projection_10k() {
    let window = resolve_virtual_list_window(VirtualListWindowRequest {
        total_items: 10_000,
        viewport_len: 18,
        requested_start: 4_000,
        overscan: 4,
        ..VirtualListWindowRequest::default()
    });
    let surface = virtual_list_window(
        window,
        32.0,
        |index| {
            list_row_id(
                index as u64 + 10_000,
                [button(format!("Row {index:05}"))
                    .message(())
                    .id(index as u64 + 20_000)
                    .fill_width()
                    .height(28.0)],
            )
        },
        96.0,
    )
    .into_surface();
    let layout = surface.layout_node();
    assert_eq!(layout.id(), 1);
    black_box((surface, layout));
}
