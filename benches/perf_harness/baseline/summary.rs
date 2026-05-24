use std::collections::BTreeMap;

use super::{MetricComparison, format::json_escape};
use crate::runner::OutputFormat;

#[derive(Default)]
pub(in crate::runner) struct BaselineSummary {
    categories: BTreeMap<String, BaselineSummaryCounts>,
    total: BaselineSummaryCounts,
}

#[derive(Clone, Copy, Default)]
struct BaselineSummaryCounts {
    matched: usize,
    missing: usize,
    faster: usize,
    similar: usize,
    slower: usize,
}

impl BaselineSummary {
    pub(in crate::runner) fn record(
        &mut self,
        category: &str,
        comparison: Option<MetricComparison>,
    ) {
        self.total.record(comparison);
        self.categories
            .entry(category.to_owned())
            .or_default()
            .record(comparison);
    }

    pub(in crate::runner) fn print(&self, scenarios: usize, output_format: OutputFormat) {
        self.total.print_total(scenarios, output_format);
        for (category, counts) in &self.categories {
            counts.print_category(category, output_format);
        }
    }

    pub(in crate::runner) fn has_regression(&self) -> bool {
        self.total.slower > 0
    }

    pub(in crate::runner) fn has_missing_baseline(&self) -> bool {
        self.total.missing > 0
    }

    pub(in crate::runner) fn slower(&self) -> usize {
        self.total.slower
    }

    pub(in crate::runner) fn missing(&self) -> usize {
        self.total.missing
    }
}

impl BaselineSummaryCounts {
    fn scenarios(&self) -> usize {
        self.matched + self.missing
    }

    fn record(&mut self, comparison: Option<MetricComparison>) {
        match comparison {
            Some(MetricComparison::Matched { status, .. }) => {
                self.matched += 1;
                match status {
                    "faster" => self.faster += 1,
                    "similar" => self.similar += 1,
                    "slower" => self.slower += 1,
                    _ => {}
                }
            }
            Some(MetricComparison::Missing) => {
                self.missing += 1;
            }
            None => {}
        }
    }

    fn print_total(&self, scenarios: usize, output_format: OutputFormat) {
        match output_format {
            OutputFormat::Text => {
                println!(
                    "radiant_perf_summary scenarios={scenarios} baseline_matched={} baseline_missing={} baseline_faster={} baseline_similar={} baseline_slower={}",
                    self.matched, self.missing, self.faster, self.similar, self.slower
                );
            }
            OutputFormat::JsonLines => {
                println!(
                    "{{\"type\":\"radiant_perf_summary\",\"scenarios\":{scenarios},\"baseline_matched\":{},\"baseline_missing\":{},\"baseline_faster\":{},\"baseline_similar\":{},\"baseline_slower\":{}}}",
                    self.matched, self.missing, self.faster, self.similar, self.slower
                );
            }
        }
    }

    fn print_category(&self, category: &str, output_format: OutputFormat) {
        let scenarios = self.scenarios();
        match output_format {
            OutputFormat::Text => {
                println!(
                    "radiant_perf_category_summary category={category} scenarios={scenarios} baseline_matched={} baseline_missing={} baseline_faster={} baseline_similar={} baseline_slower={}",
                    self.matched, self.missing, self.faster, self.similar, self.slower
                );
            }
            OutputFormat::JsonLines => {
                println!(
                    "{{\"type\":\"radiant_perf_category_summary\",\"category\":\"{}\",\"scenarios\":{scenarios},\"baseline_matched\":{},\"baseline_missing\":{},\"baseline_faster\":{},\"baseline_similar\":{},\"baseline_slower\":{}}}",
                    json_escape(category),
                    self.matched,
                    self.missing,
                    self.faster,
                    self.similar,
                    self.slower
                );
            }
        }
    }
}
