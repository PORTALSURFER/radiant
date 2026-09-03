use super::*;

const CONTRACT_DOCS: &[&str] = &[
    "docs/DESIGN_DIRECTION.md",
    "docs/TARGET.md",
    "docs/API.md",
    "docs/ARCHITECTURE.md",
    "docs/PLATFORM_ACCEPTANCE.md",
    "docs/TARGET_ALIGNMENT_STATUS.md",
    "docs/VIRTUAL_LAYOUT_DESIGN.md",
];

fn normalized(source: &str) -> String {
    source.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn read_doc(manifest_dir: &Path, relative: &str) -> String {
    fs::read_to_string(manifest_dir.join(relative))
        .unwrap_or_else(|error| panic!("{relative} should be readable: {error}"))
}

fn section_body(source: &str, heading: &str) -> String {
    let lines = source.lines().collect::<Vec<_>>();
    let start = lines
        .iter()
        .position(|line| *line == heading)
        .unwrap_or_else(|| panic!("missing heading `{heading}`"));
    let level = heading
        .chars()
        .take_while(|character| *character == '#')
        .count();
    let prefix = format!("{} ", "#".repeat(level));
    let end = lines
        .iter()
        .enumerate()
        .skip(start + 1)
        .find(|(_, line)| line.starts_with(&prefix))
        .map_or(lines.len(), |(index, _)| index);
    lines[start..end].join("\n")
}

fn markdown_table_rows(section: &str) -> Vec<Vec<String>> {
    section
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if !line.starts_with('|') {
                return None;
            }

            let cells = line
                .trim_matches('|')
                .split('|')
                .map(|cell| cell.trim().to_owned())
                .collect::<Vec<_>>();
            if cells.iter().all(|cell| {
                !cell.is_empty() && cell.chars().all(|character| matches!(character, '-' | ':'))
            }) {
                None
            } else {
                Some(cells)
            }
        })
        .collect()
}

fn completion_criteria(target: &str) -> Vec<String> {
    let section = section_body(target, "## Completion Criteria");
    let mut criteria = Vec::new();
    let mut current: Option<String> = None;

    for line in section.lines().skip(1) {
        if let Some(criterion) = line.strip_prefix("- ") {
            if let Some(previous) = current.take() {
                criteria.push(normalized(&previous));
            }
            current = Some(criterion.trim().to_owned());
        } else if line.starts_with("  ") {
            let Some(current) = current.as_mut() else {
                continue;
            };
            current.push_str(&format!(" {}", line.trim()));
        } else if !line.trim().is_empty() && current.is_some() {
            break;
        }
    }

    if let Some(last) = current {
        criteria.push(normalized(&last));
    }
    criteria
}

#[test]
fn normative_docs_reject_stale_metrics_and_unmerged_credit() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source = CONTRACT_DOCS
        .iter()
        .map(|path| normalized(&read_doc(&manifest_dir, path)))
        .collect::<Vec<_>>()
        .join(" ");

    for forbidden in [
        "900 / 11",
        "901 / 11",
        "903 / 11",
        "shipped without runtime registration",
        "runtime/effects is complete",
        "100% complete",
        "estimate credit was awarded",
        "estimate credit is awarded",
        "branch evidence counts",
        "draft evidence counts",
        "acceptance-only evidence counts",
        "unverified evidence counts",
    ] {
        assert!(
            !source.contains(forbidden),
            "stale contract text: `{forbidden}`"
        );
    }

    for path in &CONTRACT_DOCS[..4] {
        let doc = read_doc(&manifest_dir, path);
        assert!(
            doc.contains("Only canonical merged source counts for shipped status"),
            "{path} must identify the merged-source status authority"
        );
        assert!(
            doc.contains("X11") && doc.contains("product-specific"),
            "{path} must retain the X11 and product-specific non-goals"
        );
    }

    let all = CONTRACT_DOCS
        .iter()
        .map(|path| normalized(&read_doc(&manifest_dir, path)))
        .collect::<Vec<_>>()
        .join(" ");
    for required in [
        "private `EffectOrigin`",
        "`ResourceTasks` remains application-owned",
        "`runtime/effects` is not complete",
        "OPT-1387",
        "OPT-1390",
        "OPT-1370",
        "prepared refresh constructs the Projection, layout, and paint-plan candidate synchronously",
        "later no-yield publication gate",
        "future work under OPT-1389",
        "`LayoutPolicy` is limited to",
        "separate from the built-in `ContainerPolicy`",
        "OPT-1272 is Done",
        "public declarative attachment",
        "mounted runtime registration",
        "first-class production consumer/collection family remains future work",
        "OPT-1362",
        "OPT-1400",
        "OPT-1398",
        "OPT-1397",
        "OPT-1399",
        "OPT-1401",
        "environment exposes only display scale, color scheme, contrast, and reduced-motion preference",
        "Unicode-scalar editing is shipped",
        "OPT-1386",
        "OPT-1402",
        "RenderCanvas",
        "GpuSurface",
        "PaintPrimitive::GpuSurface",
        "CanvasProgram",
        "CanvasGraph",
        "OPT-1407",
        "OPT-1408",
    ] {
        assert!(
            all.contains(required),
            "missing current/future anchor: `{required}`"
        );
    }
}

#[test]
fn target_alignment_scorecard_has_current_estimates() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let status = read_doc(&manifest_dir, "docs/TARGET_ALIGNMENT_STATUS.md");
    let expected_lines = [
        "# Radiant Target Alignment",
        "",
        "| Overall measure | Estimate |",
        "| --- | ---: |",
        "| Generic architecture-sequence completion | ~100% |",
        "| Broad end-to-end target coverage | ~87.5% |",
        "",
        "| Category | Estimate |",
        "| --- | ---: |",
        "| Public API and module boundaries | 99.5% |",
        "| Declarative model, identity, reconciliation | 84% |",
        "| Input, provenance, and edit lifecycle | 98.5% |",
        "| Layout, composition, virtualization | 75% |",
        "| Text, focus, and selection | ~72% |",
        "| Numeric controls | 90% |",
        "| Runtime, effects, and scheduling | 96% |",
        "| Rendering, invalidation, retained GPU surfaces | ~89% |",
        "| Platform, windowing, and host boundaries | 67% |",
        "| Diagnostics, profiling, and performance validation | ~94% |",
        "| Examples, documentation, and CI guardrails | 97% |",
    ];
    assert_eq!(status.lines().collect::<Vec<_>>(), expected_lines);

    let estimate = |line: &str| {
        line.split('|')
            .nth(2)
            .expect("scorecard row estimate")
            .trim()
            .trim_start_matches('~')
            .trim_end_matches('%')
            .parse::<f64>()
            .expect("numeric scorecard estimate")
    };
    let lines = status.lines().collect::<Vec<_>>();
    let category_values = lines[9..20].iter().map(|line| estimate(line));
    let mean = category_values.sum::<f64>() / 11.0;
    let broad = estimate(lines[5]);
    assert_eq!(broad, (mean * 10.0).round() / 10.0);
}

#[test]
fn platform_acceptance_policy_has_structured_lanes_outcomes_and_target_map() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let policy = read_doc(&manifest_dir, "docs/PLATFORM_ACCEPTANCE.md");
    let target = read_doc(&manifest_dir, "docs/TARGET.md");

    for heading in [
        "# Radiant Platform Acceptance and Evidence Policy",
        "## Authority and claim rules",
        "## Evidence lanes",
        "## Outcome states",
        "## Platform baselines and session requirements",
        "## Capability acceptance matrix",
        "## Target completion criteria map",
        "## Release gates and cadence",
        "## Artifact schema and retention",
        "## Current evidence inventory",
        "## Downstream ticket map",
    ] {
        assert!(
            policy.lines().any(|line| line == heading),
            "policy must retain the structural anchor `{heading}`"
        );
    }

    let lane_rows = markdown_table_rows(&section_body(&policy, "## Evidence lanes"));
    assert_eq!(
        lane_rows.first(),
        Some(&vec![
            "Lane".to_owned(),
            "Name".to_owned(),
            "Proves".to_owned(),
            "Does not prove".to_owned(),
        ])
    );
    let expected_lanes = [
        ("C", "Static/build/cross-target/compile"),
        ("A", "Automated deterministic/core"),
        (
            "H",
            "Headless native host with a real event loop/compositor",
        ),
        ("N", "Logged-in live native desktop automation"),
        ("M", "Manual native/hardware"),
    ];
    assert_eq!(lane_rows.len(), expected_lanes.len() + 1);
    let mut seen_lanes = BTreeSet::new();
    for (lane, name) in expected_lanes {
        let row = lane_rows
            .iter()
            .find(|row| row.first().map(String::as_str) == Some(lane))
            .unwrap_or_else(|| panic!("missing evidence lane `{lane}`"));
        assert!(row.len() >= 4, "evidence lane `{lane}` needs all columns");
        assert!(
            row[1].starts_with(name),
            "evidence lane `{lane}` has wrong name"
        );
        assert!(row[2..].iter().all(|cell| !cell.is_empty()));
        assert!(seen_lanes.insert(lane));
    }
    assert_eq!(seen_lanes.len(), expected_lanes.len());

    let outcome_rows = markdown_table_rows(&section_body(&policy, "## Outcome states"));
    assert_eq!(
        outcome_rows.first(),
        Some(&vec![
            "Outcome".to_owned(),
            "Meaning".to_owned(),
            "Gate treatment".to_owned(),
        ])
    );
    let expected_outcomes = [
        "PASS",
        "FAIL",
        "UNSUPPORTED",
        "UNAVAILABLE",
        "NOT_RUN",
        "NOT_APPLICABLE",
    ];
    assert_eq!(outcome_rows.len(), expected_outcomes.len() + 1);
    let mut seen_outcomes = BTreeSet::new();
    for outcome in expected_outcomes {
        let row = outcome_rows
            .iter()
            .find(|row| row.first().map(String::as_str) == Some(outcome))
            .unwrap_or_else(|| panic!("missing outcome `{outcome}`"));
        assert!(
            row.len() >= 3,
            "outcome `{outcome}` needs definition and gate columns"
        );
        assert!(row[1..].iter().all(|cell| !cell.is_empty()));
        assert!(seen_outcomes.insert(outcome));
    }
    assert_eq!(seen_outcomes.len(), expected_outcomes.len());

    let target_criteria = completion_criteria(&target);
    assert_eq!(
        target_criteria.len(),
        30,
        "TARGET completion criteria changed"
    );
    let target_unique = target_criteria.iter().collect::<BTreeSet<_>>();
    assert_eq!(target_unique.len(), target_criteria.len());

    let criteria_table =
        markdown_table_rows(&section_body(&policy, "## Target completion criteria map"));
    assert_eq!(
        criteria_table.first(),
        Some(&vec![
            "ID".to_owned(),
            "Completion criterion from `docs/TARGET.md`".to_owned(),
            "Required lanes".to_owned(),
            "Applicable platforms".to_owned(),
            "Gate status".to_owned(),
            "Owner".to_owned(),
            "Allowed unavailable/capability-conditional outcome".to_owned(),
        ])
    );
    let criteria_rows = criteria_table.into_iter().skip(1).collect::<Vec<_>>();
    assert_eq!(
        criteria_rows.len(),
        30,
        "policy must have exactly 30 TC rows"
    );

    let expected_ids = (1..=30)
        .map(|number| format!("TC-{number:02}"))
        .collect::<Vec<_>>();
    let mut seen_ids = BTreeSet::new();
    let policy_criteria = criteria_rows
        .iter()
        .enumerate()
        .map(|(index, row)| {
            assert_eq!(row.len(), 7, "{} must have seven required columns", row[0]);
            assert_eq!(
                row[0], expected_ids[index],
                "target criterion IDs must be ordered"
            );
            assert!(
                seen_ids.insert(row[0].clone()),
                "duplicate target criterion ID"
            );
            assert!(
                row[1..].iter().all(|cell| !cell.is_empty()),
                "{} must have non-empty required columns",
                row[0]
            );
            normalized(&row[1])
        })
        .collect::<Vec<_>>();

    assert_eq!(seen_ids.len(), 30);
    assert_eq!(policy_criteria, target_criteria);
    for criterion in &target_criteria {
        assert_eq!(
            policy_criteria
                .iter()
                .filter(|candidate| *candidate == criterion)
                .count(),
            1,
            "target criterion must map exactly once: {criterion}"
        );
    }

    let downstream = markdown_table_rows(&section_body(&policy, "## Downstream ticket map"));
    let downstream_ids = [
        "OPT-1371", "OPT-1372", "OPT-1373", "OPT-1377", "OPT-1376", "OPT-1378", "OPT-1375",
        "OPT-1418", "OPT-1381", "OPT-1417",
    ];
    for ticket in downstream_ids {
        assert!(
            downstream
                .iter()
                .any(|row| row.first().map(String::as_str) == Some(ticket)),
            "downstream ticket map must include {ticket}"
        );
    }

    let inventory = section_body(&policy, "## Current evidence inventory");
    let numeric_row = markdown_table_rows(&inventory)
        .into_iter()
        .find(|row| {
            row.first()
                .is_some_and(|cell| cell.contains("macos_numeric_accessibility_acceptance"))
        })
        .expect("numeric accessibility inventory row");
    assert_eq!(
        numeric_row[1],
        "`N/NOT_RUN` for current policy-compliant evidence"
    );
    assert!(numeric_row[2].contains("historical bounded AppKit/Computer Use"));
    assert!(numeric_row[3].contains("not VoiceOver or release evidence"));
    assert!(numeric_row[3].contains("no complete policy manifest is recorded"));

    let api = read_doc(&manifest_dir, "docs/API.md");
    let numeric_api = section_body(&api, "### macOS numeric accessibility acceptance");
    let normalized_numeric_api = normalized(&numeric_api);
    assert!(normalized_numeric_api.contains("current policy-compliant evidence is `N/NOT_RUN`"));
    assert!(normalized_numeric_api.contains("no complete policy manifest is recorded"));
    assert!(normalized_numeric_api.contains("historical bounded AppKit/Computer Use result"));
    assert!(normalized_numeric_api.contains("not VoiceOver or release evidence"));
}

#[test]
fn platform_acceptance_policy_is_linked_from_normative_sections() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let read = |relative| read_doc(&manifest_dir, relative);
    let linked = |document: &str, heading: &str| {
        let section = section_body(document, heading);
        assert!(
            section.contains("PLATFORM_ACCEPTANCE.md"),
            "{heading} must link the platform acceptance policy"
        );
        assert!(
            section.contains("Platform Acceptance and Evidence Policy"),
            "{heading} must name the platform acceptance policy"
        );
    };

    let target = read("docs/TARGET.md");
    for heading in [
        "## Platform Target",
        "## Documentation Goals",
        "## Feature Definition of Done",
        "## Validation and CI Expectations",
        "## Completion Criteria",
    ] {
        linked(&target, heading);
    }

    let design = read("docs/DESIGN_DIRECTION.md");
    for heading in [
        "## Deterministic Testing and Replay",
        "### Cross-window CPU frame fairness",
        "### Next scheduler policy contract",
        "## Native Platform Services",
        "## Performance and Verification Requirements",
    ] {
        linked(&design, heading);
    }

    let architecture = read("docs/ARCHITECTURE.md");
    for heading in ["## Platform Boundary", "## Validation Map"] {
        linked(&architecture, heading);
    }

    let api = read("docs/API.md");
    let api_introduction = api
        .lines()
        .take_while(|line| !line.starts_with("## "))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(api_introduction.contains("PLATFORM_ACCEPTANCE.md"));
    assert!(api_introduction.contains("Platform Acceptance and Evidence Policy"));

    for heading in [
        "### macOS live frame-profile acceptance",
        "### macOS live devtools acceptance",
        "### macOS live external-drag acceptance",
        "### macOS numeric accessibility acceptance",
        "### macOS live Japanese IME acceptance",
    ] {
        linked(&api, heading);
        let section = section_body(&api, heading);
        assert!(section.contains("Policy classification:"));
        assert!(
            section.contains("/NOT_RUN") || section.contains("/PASS"),
            "{heading} must expose a policy lane/outcome ID"
        );
    }

    for heading in ["## Performance Harness", "## Quality Gate"] {
        linked(&api, heading);
    }

    let readme = read("README.md");
    linked(&readme, "## Documentation Map");
    linked(&readme, "## Validation");
}

#[test]
fn platform_acceptance_policy_preserves_current_evidence_and_non_goals() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let policy = read_doc(&manifest_dir, "docs/PLATFORM_ACCEPTANCE.md");
    let inventory =
        normalized(&section_body(&policy, "## Current evidence inventory")).to_lowercase();
    let normalized_policy = normalized(&policy);

    for required in [
        ".github/workflows/ci.yml",
        "macos-15-intel",
        "macos quality",
        "cross-target",
        "performance",
        "windows-compile",
        "windows-2025",
        "windows compile-only",
        "native host",
        "ime",
        "accessibility",
        "gpu",
        "screen-reader",
        "not a second",
    ] {
        assert!(
            inventory.contains(required),
            "current evidence boundary missing `{required}`"
        );
    }

    for required in [
        "X11",
        "product-specific",
        "VST/plugin SDK",
        "scorecard estimates",
        "does not add a new timing threshold",
        "Ubuntu 26.04 LTS Desktop",
        "Windows 11 25H2",
        "current supported macOS",
    ] {
        assert!(
            normalized_policy.contains(required),
            "policy boundary missing `{required}`"
        );
    }
}
