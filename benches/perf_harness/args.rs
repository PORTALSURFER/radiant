use std::path::PathBuf;

use super::{
    OutputFormat,
    baseline::{BaselineOutput, BaselineSet},
};

pub(crate) const RUN_ALL_IN_DEBUG_ENV: &str = "RADIANT_PERF_RUN_ALL_IN_DEBUG";
const BASELINE_JSONL_ARG: &str = "--baseline-jsonl";
const WRITE_BASELINE_JSONL_ARG: &str = "--write-baseline-jsonl";
const CATEGORY_ARG: &str = "--category";
const GROUP_ARG: &str = "--group";
const FAIL_ON_BASELINE_REGRESSION_ARG: &str = "--fail-on-baseline-regression";
const FAIL_ON_MISSING_BASELINE_ARG: &str = "--fail-on-missing-baseline";
const JSONL_ARG: &str = "--jsonl";
const LIST_ARG: &str = "--list";

pub(crate) fn scenario_filters_from_args(args: &[String]) -> Vec<String> {
    let mut filters = Vec::new();
    let mut skip_next = false;
    for arg in args.iter().skip(1) {
        if skip_next {
            skip_next = false;
            continue;
        }
        if consumes_next_value(arg) {
            skip_next = true;
            continue;
        }
        if has_inline_value(arg) {
            continue;
        }
        if !arg.starts_with('-') && !arg.is_empty() {
            filters.push(arg.clone());
        }
    }
    filters
}

pub(crate) fn category_filters_from_args(args: &[String]) -> Vec<String> {
    values_after_arg(args, CATEGORY_ARG)
}

pub(crate) fn group_filters_from_args(args: &[String]) -> Vec<String> {
    values_after_arg(args, GROUP_ARG)
}

pub(crate) fn scenario_list_requested(args: &[String]) -> bool {
    args.iter().skip(1).any(|arg| arg == LIST_ARG)
}

pub(crate) fn output_format_from_args(args: &[String]) -> OutputFormat {
    if args.iter().skip(1).any(|arg| arg == JSONL_ARG) {
        OutputFormat::JsonLines
    } else {
        OutputFormat::Text
    }
}

pub(crate) fn baseline_from_args(args: &[String]) -> Option<BaselineSet> {
    let path = match value_after_arg(args, BASELINE_JSONL_ARG) {
        Some(path) => path,
        None if has_flag(args, BASELINE_JSONL_ARG) => {
            eprintln!("radiant_perf baseline error: --baseline-jsonl requires a path");
            std::process::exit(2);
        }
        None => return None,
    };
    match BaselineSet::from_jsonl_file(PathBuf::from(path)) {
        Ok(baseline) => Some(baseline),
        Err(err) => {
            eprintln!("radiant_perf baseline error: {err}");
            std::process::exit(2);
        }
    }
}

pub(crate) fn baseline_output_from_args(args: &[String]) -> Option<BaselineOutput> {
    let path = match value_after_arg(args, WRITE_BASELINE_JSONL_ARG) {
        Some(path) => path,
        None if has_flag(args, WRITE_BASELINE_JSONL_ARG) => {
            eprintln!("radiant_perf baseline error: --write-baseline-jsonl requires a path");
            std::process::exit(2);
        }
        None => return None,
    };
    Some(BaselineOutput::new(PathBuf::from(path)))
}

pub(crate) fn fail_on_baseline_regression_from_args(args: &[String]) -> bool {
    let fail_on_regression = has_flag(args, FAIL_ON_BASELINE_REGRESSION_ARG);
    if fail_on_regression && value_after_arg(args, BASELINE_JSONL_ARG).is_none() {
        eprintln!(
            "radiant_perf baseline error: {FAIL_ON_BASELINE_REGRESSION_ARG} requires --baseline-jsonl"
        );
        std::process::exit(2);
    }
    fail_on_regression
}

pub(crate) fn fail_on_missing_baseline_from_args(args: &[String]) -> bool {
    let fail_on_missing = has_flag(args, FAIL_ON_MISSING_BASELINE_ARG);
    if fail_on_missing && value_after_arg(args, BASELINE_JSONL_ARG).is_none() {
        eprintln!(
            "radiant_perf baseline error: {FAIL_ON_MISSING_BASELINE_ARG} requires --baseline-jsonl"
        );
        std::process::exit(2);
    }
    fail_on_missing
}

fn consumes_next_value(arg: &str) -> bool {
    [
        BASELINE_JSONL_ARG,
        WRITE_BASELINE_JSONL_ARG,
        CATEGORY_ARG,
        GROUP_ARG,
    ]
    .contains(&arg)
}

fn has_inline_value(arg: &str) -> bool {
    [
        BASELINE_JSONL_ARG,
        WRITE_BASELINE_JSONL_ARG,
        CATEGORY_ARG,
        GROUP_ARG,
    ]
    .iter()
    .any(|name| arg.starts_with(&format!("{name}=")))
}

fn has_flag(args: &[String], name: &str) -> bool {
    args.iter().skip(1).any(|arg| arg == name)
}

fn value_after_arg(args: &[String], name: &str) -> Option<String> {
    let mut iter = args.iter().skip(1);
    while let Some(arg) = iter.next() {
        if arg == name {
            return iter.next().cloned();
        }
        if let Some(value) = arg.strip_prefix(&format!("{name}=")) {
            return Some(value.to_owned());
        }
    }
    None
}

fn values_after_arg(args: &[String], name: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut iter = args.iter().skip(1);
    while let Some(arg) = iter.next() {
        if arg == name {
            if let Some(value) = iter.next() {
                values.push(value.clone());
            }
            continue;
        }
        if let Some(value) = arg.strip_prefix(&format!("{name}=")) {
            values.push(value.to_owned());
        }
    }
    values
}
