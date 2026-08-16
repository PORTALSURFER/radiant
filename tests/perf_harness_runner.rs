// The perf target uses `harness = false`; include its runner here so focused
// percentile, serialization, baseline, and compatibility tests execute under
// the normal Rust test harness.
#![allow(dead_code, missing_docs, unused_imports)]

#[path = "../benches/perf_harness/runner.rs"]
mod runner;

#[path = "../examples/arrangement_shell/mod.rs"]
mod arrangement_shell;

#[path = "../benches/perf_harness/runtime_scenarios/arrangement_shell.rs"]
mod arrangement_shell_scenarios;
