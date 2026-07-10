//! Application-builder projection performance scenarios.

use crate::runner::ScenarioCounters;
use radiant::{
    layout::layout_tree,
    prelude::{
        IntoView, PaintPrimitive, Point, Rect, ThemeTokens, Vector2, VirtualListWindowRequest,
        action_row, badge, button, close_button, column, determinate_progress_bar, list_row_id,
        resolve_virtual_list_window, selectable, text, toggle, virtual_list_window,
    },
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

pub(super) fn constant_message_controls_projection_1k() -> impl FnMut() -> ScenarioCounters {
    bench_app_constant_message_controls_projection_1k
}

pub(super) fn static_text_controls_projection_1k() -> impl FnMut() -> ScenarioCounters {
    bench_app_static_text_controls_projection_1k
}

fn bench_app_static_text_controls_projection_1k() -> ScenarioCounters {
    let mut controls = Vec::with_capacity(1_000);
    for index in 0..200_u64 {
        controls.push(text("Ready").id(index * 5 + 10));
        controls.push(button("Play").message(()).id(index * 5 + 11));
        controls.push(badge("Stable").message(()).id(index * 5 + 12));
        controls.push(toggle("Enabled", true).message(|_| ()).id(index * 5 + 13));
        controls.push(
            selectable("Selected", false)
                .message(|_| ())
                .id(index * 5 + 14),
        );
    }
    let surface = column(controls).into_surface();
    let layout = layout_tree(
        &surface.layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(240.0, 100_000.0)),
    );
    let plan = surface.paint_plan(&layout, &ThemeTokens::default());
    let text_runs = plan
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            PaintPrimitive::Text(run) => Some(run),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(text_runs.len(), 1_000);
    let text_storage_allocation_count =
        text_runs.iter().filter(|run| !run.text.is_static()).count() as u64;
    black_box((&surface, &layout, &plan));
    ScenarioCounters::default()
        .with_text_storage_allocation_count(text_storage_allocation_count)
        .with_allocation_sensitive_work_count(text_runs.len() as u64)
}

fn bench_app_constant_message_controls_projection_1k() -> ScenarioCounters {
    let mut controls = Vec::with_capacity(1_000);
    for index in 0..200_u64 {
        controls.push(button("Run").message(()).id(index * 5 + 10));
        controls.push(close_button().message(()).id(index * 5 + 11));
        controls.push(badge("Ready").message(()).id(index * 5 + 12));
        controls.push(action_row("Open").message(()).id(index * 5 + 13));
        controls.push(
            determinate_progress_bar(0.5)
                .activatable()
                .message(())
                .id(index * 5 + 14),
        );
    }
    let surface = column(controls).into_surface();
    assert_eq!(surface.layout_node().id(), 1);
    let callback_allocation_count = surface.widget_callback_allocation_count() as u64;
    black_box(surface);
    ScenarioCounters::default()
        .with_widget_callback_allocation_count(callback_allocation_count)
        .with_allocation_sensitive_work_count(1_000)
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
