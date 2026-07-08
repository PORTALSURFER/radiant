//! Application-builder projection performance scenarios.

use crate::runner::ScenarioCounters;
use radiant::prelude::{
    IntoView, VirtualListWindowRequest, button, list_row_id, resolve_virtual_list_window,
    selectable, virtual_list_window,
};
use std::hint::black_box;

pub(super) fn virtual_list_projection_10k() -> impl FnMut() -> ScenarioCounters {
    bench_app_virtual_list_projection_10k
}

pub(super) fn virtual_list_projection_generated_child_ids_10k() -> impl FnMut() -> ScenarioCounters
{
    bench_app_virtual_list_projection_generated_child_ids_10k
}

pub(super) fn virtual_selectable_list_projection_10k() -> impl FnMut() -> ScenarioCounters {
    bench_app_virtual_selectable_list_projection_10k
}

pub(super) fn virtual_list_window_projection_10k() -> impl FnMut() -> ScenarioCounters {
    bench_app_virtual_list_window_projection_10k
}

fn bench_app_virtual_list_projection_10k() -> ScenarioCounters {
    let surface = virtual_list_window(
        full_virtual_list_window(),
        32.0,
        |index| {
            let index = index as u64;
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
        0.0,
    )
    .into_surface();
    let layout = surface.layout_node();
    assert_eq!(layout.id(), 1);
    black_box((surface, layout));
    ScenarioCounters::default().with_allocation_sensitive_work_count(10_000)
}

fn bench_app_virtual_list_projection_generated_child_ids_10k() -> ScenarioCounters {
    let surface = virtual_list_window(
        full_virtual_list_window(),
        32.0,
        |index| {
            let index = index as u64;
            list_row_id(
                index + 10_000,
                [button(format!("Row {index:05}"))
                    .message(())
                    .fill_width()
                    .height(28.0)],
            )
            .height(32.0)
        },
        0.0,
    )
    .into_surface();
    let layout = surface.layout_node();
    assert_eq!(layout.id(), 1);
    black_box((surface, layout));
    ScenarioCounters::default().with_allocation_sensitive_work_count(10_000)
}

fn bench_app_virtual_selectable_list_projection_10k() -> ScenarioCounters {
    let surface = virtual_list_window(
        full_virtual_list_window(),
        32.0,
        |index| {
            let index = index as u64;
            selectable(format!("Row {index:05}"), false)
                .message(move |_| ())
                .id(index + 10_000)
                .fill_width()
                .height(32.0)
        },
        0.0,
    )
    .into_surface();
    let layout = surface.layout_node();
    assert_eq!(layout.id(), 1);
    black_box((surface, layout));
    ScenarioCounters::default().with_allocation_sensitive_work_count(10_000)
}

fn full_virtual_list_window() -> radiant::prelude::VirtualListWindow {
    resolve_virtual_list_window(VirtualListWindowRequest {
        total_items: 10_000,
        viewport_len: 10_000,
        requested_start: 0,
        overscan: 0,
        ..VirtualListWindowRequest::default()
    })
}

fn bench_app_virtual_list_window_projection_10k() -> ScenarioCounters {
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
    ScenarioCounters::default().with_allocation_sensitive_work_count(window.window_len() as u64)
}
