//! Runtime command-drain performance scenarios.

use crate::runner::ScenarioCounters;
use radiant::{
    layout::{ContainerPolicy, Vector2},
    runtime::{
        Command, RuntimeBridge, RuntimeHostCapabilities, RuntimeQueueHost, SurfaceNode,
        SurfaceRuntime, UiSurface,
    },
};
use std::{hint::black_box, sync::Arc};

pub(super) fn flat_command_drain() -> impl FnMut() -> ScenarioCounters {
    let mut command_drain = StatefulCommandDrainBench::new();
    move || command_drain.step()
}

pub(super) fn nested_command_drain() -> impl FnMut() -> ScenarioCounters {
    let mut nested_command_drain = StatefulNestedCommandDrainBench::new();
    move || nested_command_drain.step()
}

struct StatefulCommandDrainBench {
    runtime: SurfaceRuntime<QueuedCommandDrainBridge, usize>,
    next_message: usize,
}

impl StatefulCommandDrainBench {
    fn new() -> Self {
        Self {
            runtime: SurfaceRuntime::new(
                QueuedCommandDrainBridge::default(),
                Vector2::new(120.0, 40.0),
            ),
            next_message: 0,
        }
    }

    fn step(&mut self) -> ScenarioCounters {
        let start = self.next_message;
        let end = start + 1_024;
        self.runtime
            .bridge_mut()
            .commands
            .extend((start..end).map(Command::message));
        drain_until_idle(&mut self.runtime);
        self.next_message = end;
        assert_eq!(self.runtime.bridge().dispatched, end);
        black_box(self.runtime.layout());
        ScenarioCounters::default().with_allocation_sensitive_work_count(1_024)
    }
}

struct StatefulNestedCommandDrainBench {
    runtime: SurfaceRuntime<QueuedCommandDrainBridge, usize>,
    next_message: usize,
}

impl StatefulNestedCommandDrainBench {
    fn new() -> Self {
        Self {
            runtime: SurfaceRuntime::new(
                QueuedCommandDrainBridge::default(),
                Vector2::new(120.0, 40.0),
            ),
            next_message: 0,
        }
    }

    fn step(&mut self) -> ScenarioCounters {
        let start = self.next_message;
        let end = start + 1_024;
        self.runtime.bridge_mut().commands.extend([
            Command::batch((start..end).map(Command::message)),
            Command::message(end),
        ]);
        drain_until_idle(&mut self.runtime);
        self.next_message = end + 1;
        assert_eq!(self.runtime.bridge().dispatched, end + 1);
        black_box(self.runtime.layout());
        ScenarioCounters::default().with_allocation_sensitive_work_count(1_025)
    }
}

fn drain_until_idle(runtime: &mut SurfaceRuntime<QueuedCommandDrainBridge, usize>) {
    loop {
        let outcome = runtime.drain_runtime_messages();
        if !outcome.runtime_work_remaining {
            break;
        }
    }
}

#[derive(Default)]
struct QueuedCommandDrainBridge {
    commands: Vec<Command<usize>>,
    dispatched: usize,
}

impl RuntimeBridge<usize> for QueuedCommandDrainBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<usize>> {
        Arc::new(UiSurface::new(SurfaceNode::container(
            1,
            ContainerPolicy::default(),
            Vec::new(),
        )))
    }

    fn reduce_message(&mut self, message: usize) {
        self.dispatched = message + 1;
    }

    fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, usize> {
        RuntimeHostCapabilities::new().with_queues()
    }
}

impl RuntimeQueueHost<usize> for QueuedCommandDrainBridge {
    fn drain_runtime_commands_into(&mut self, commands: &mut Vec<Command<usize>>) {
        commands.append(&mut self.commands);
    }
}
