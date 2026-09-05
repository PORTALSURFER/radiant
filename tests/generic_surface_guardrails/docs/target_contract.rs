use super::*;

use radiant::{
    gui::types::{ImageRgba, Rect, Vector2},
    layout::Point,
    prelude::{IntoView, Rgba8},
    runtime::{PaintFillRect, PaintPrimitive, RenderCanvasContent, render_canvas},
    theme::ThemeTokens,
};
use std::sync::Arc;

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
        "`runtime::Effect<Message>`",
        "EffectOwner::Application",
        "EffectOwner::Declarative",
        "`TaskTicket`",
        "`CancellationToken`",
        "typed `TaskCompletion`",
        "`Command::effect(...)`",
        "`Effect::latest_stream(...)`",
        "separate timer and worker lanes",
        "`ResourceTasks` remains application-owned",
        "`runtime/effects` is not complete",
        "OPT-1387",
        "OPT-1390",
        "OPT-1370",
        "scheduler/thread design",
        "native hosts",
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FixtureCapability {
    FullscreenRender,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FixturePass {
    FullscreenRender,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FixtureOperation {
    DrawFullscreen,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FixtureGraph {
    TypedFullscreen {
        pass: FixturePass,
        operation: FixtureOperation,
    },
    Invalid,
}

impl FixtureGraph {
    fn valid() -> Self {
        Self::TypedFullscreen {
            pass: FixturePass::FullscreenRender,
            operation: FixtureOperation::DrawFullscreen,
        }
    }

    fn is_structurally_valid(self) -> bool {
        matches!(
            self,
            Self::TypedFullscreen {
                pass: FixturePass::FullscreenRender,
                operation: FixtureOperation::DrawFullscreen,
            }
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FixtureDiagnostic {
    InvalidGraph,
    UnsupportedContractVersion,
    MissingCapability,
}

#[derive(Clone, Debug, PartialEq)]
struct FixtureCanvasProgram {
    contract_version: u16,
    required_capability: FixtureCapability,
    graph: FixtureGraph,
    primitive_fallback: PaintPrimitive,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FixtureAdapter {
    max_contract_version: u16,
    capabilities: u8,
}

impl FixtureAdapter {
    fn supports(self, capability: FixtureCapability) -> bool {
        match capability {
            FixtureCapability::FullscreenRender => self.capabilities & 1 != 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
enum FixtureDecision {
    Graph,
    PrimitiveFallback {
        primitive: PaintPrimitive,
        diagnostic: FixtureDiagnostic,
    },
}

fn select_fixture_decision(
    program: &FixtureCanvasProgram,
    adapter: FixtureAdapter,
    adapter_handoff: &mut bool,
) -> FixtureDecision {
    *adapter_handoff = false;
    if !program.graph.is_structurally_valid() {
        return FixtureDecision::PrimitiveFallback {
            primitive: program.primitive_fallback.clone(),
            diagnostic: FixtureDiagnostic::InvalidGraph,
        };
    }
    if program.contract_version > adapter.max_contract_version {
        return FixtureDecision::PrimitiveFallback {
            primitive: program.primitive_fallback.clone(),
            diagnostic: FixtureDiagnostic::UnsupportedContractVersion,
        };
    }
    if !adapter.supports(program.required_capability) {
        return FixtureDecision::PrimitiveFallback {
            primitive: program.primitive_fallback.clone(),
            diagnostic: FixtureDiagnostic::MissingCapability,
        };
    }

    *adapter_handoff = true;
    FixtureDecision::Graph
}

fn current_render_canvas_primitive() -> PaintPrimitive {
    let content = RenderCanvasContent::RgbaAtlas {
        source_rect: Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(1.0, 1.0)),
        atlas: Arc::new(ImageRgba::new(1, 1, vec![255; 4]).expect("valid compatibility atlas")),
    };
    assert!(content.validate().is_ok());

    let view = render_canvas::<FixtureMessage>(17, 3, content);
    let surface = view.into_surface();
    let layout = radiant::layout::layout_tree(
        &surface.layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(20.0, 20.0)),
    );
    let plan = surface.paint_plan(&layout, &ThemeTokens::default());

    plan.primitives
        .into_iter()
        .find_map(|primitive| match primitive {
            primitive @ PaintPrimitive::GpuSurface(_) => Some(primitive),
            _ => None,
        })
        .expect("current render_canvas must lower to PaintPrimitive::GpuSurface")
}

fn primitive_fallback() -> PaintPrimitive {
    PaintPrimitive::FillRect(PaintFillRect {
        widget_id: 17,
        rect: Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(20.0, 20.0)),
        color: Rgba8 {
            r: 32,
            g: 40,
            b: 48,
            a: 255,
        },
    })
}

struct FixtureMessage;

#[test]
fn canvas_program_fixture_validates_before_handoff_and_selects_explicit_fallback() {
    let current_compatibility_primitive = current_render_canvas_primitive();
    assert!(matches!(
        current_compatibility_primitive,
        PaintPrimitive::GpuSurface(_)
    ));

    let fallback = primitive_fallback();
    assert!(matches!(fallback, PaintPrimitive::FillRect(_)));
    let base = FixtureCanvasProgram {
        contract_version: 1,
        required_capability: FixtureCapability::FullscreenRender,
        graph: FixtureGraph::valid(),
        primitive_fallback: fallback.clone(),
    };
    let adapter = FixtureAdapter {
        max_contract_version: 1,
        capabilities: 1,
    };

    let cases = [
        (
            "invalid graph",
            FixtureCanvasProgram {
                graph: FixtureGraph::Invalid,
                ..base.clone()
            },
            FixtureDiagnostic::InvalidGraph,
        ),
        (
            "unsupported contract version",
            FixtureCanvasProgram {
                contract_version: 2,
                ..base.clone()
            },
            FixtureDiagnostic::UnsupportedContractVersion,
        ),
        (
            "missing capability",
            base.clone(),
            FixtureDiagnostic::MissingCapability,
        ),
    ];

    for (name, program, diagnostic) in cases {
        let case_adapter = if diagnostic == FixtureDiagnostic::MissingCapability {
            FixtureAdapter {
                capabilities: 0,
                ..adapter
            }
        } else {
            adapter
        };
        let mut adapter_handoff = false;
        assert_eq!(
            select_fixture_decision(&program, case_adapter, &mut adapter_handoff),
            FixtureDecision::PrimitiveFallback {
                primitive: fallback.clone(),
                diagnostic,
            },
            "{name} must choose its typed diagnostic and explicit primitive fallback"
        );
        assert!(!adapter_handoff, "{name} must stop before adapter handoff");
    }

    let mut adapter_handoff = false;
    assert_eq!(
        select_fixture_decision(&base, adapter, &mut adapter_handoff),
        FixtureDecision::Graph
    );
    assert!(adapter_handoff);
}

#[test]
fn render_canvas_contract_guardrail_keeps_supported_and_target_surfaces_distinct() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let design = read_doc(&manifest_dir, "docs/DESIGN_DIRECTION.md");
    let target = read_doc(&manifest_dir, "docs/TARGET.md");
    let api = read_doc(&manifest_dir, "docs/API.md");
    let architecture = read_doc(&manifest_dir, "docs/ARCHITECTURE.md");
    let acceptance = read_doc(&manifest_dir, "docs/PLATFORM_ACCEPTANCE.md");
    let all = [
        design.as_str(),
        target.as_str(),
        api.as_str(),
        architecture.as_str(),
        acceptance.as_str(),
    ]
    .into_iter()
    .map(normalized)
    .collect::<Vec<_>>()
    .join(" ");

    for required in [
        "Current supported 0.1.x",
        "`render_canvas_program(canvas)`",
        "one-argument `render_canvas(canvas)`",
        "`PaintPrimitive::RenderCanvas`",
        "explicit 0.2 breaking boundary after migration evidence",
        "`CanvasGraph`",
        "immutable",
        "typed",
        "bounded",
        "graph-lifetime transient resources",
        "compute/fullscreen-render passes",
        "no shader source",
        "loops, pointers, native handles",
        "mutable application payloads",
        "Structural validation completes before adapter handoff",
        "CanvasDiagnostic::InvalidGraph",
        "UnsupportedContractVersion",
        "MissingCapability",
        "CompilationFailed",
        "RecoveryIdentityMismatch",
        "retained allocation identity",
        "adapter/target generations",
        "Hashes are lookup aids only",
        "`WgslCanvasProgram`",
        "`expert-wgsl`",
        "Render-canvas contract and fallback guardrail",
    ] {
        assert!(
            all.contains(required),
            "render-canvas contract missing `{required}`"
        );
    }

    for (path, heading) in [
        (
            "docs/DESIGN_DIRECTION.md",
            "#### Render-canvas compatibility contract (OPT-1407)",
        ),
        (
            "docs/TARGET.md",
            "### CanvasProgram and CanvasGraph compatibility contract (OPT-1407)",
        ),
    ] {
        assert!(
            read_doc(&manifest_dir, path).contains(heading),
            "{path} must retain the normative render-canvas contract heading"
        );
    }

    let builder = read_doc(&manifest_dir, "src/application/builders/leaf/gpu.rs");
    assert!(builder.contains("pub fn render_canvas<Message: 'static>"));
    assert!(builder.contains("content: crate::runtime::RenderCanvasContent"));

    let primitive = read_doc(&manifest_dir, "src/runtime/paint/primitives/plan.rs");
    assert!(primitive.contains("GpuSurface(PaintGpuSurface)"));
    assert!(!primitive.contains("RenderCanvas(Paint"));
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
        "| Broad end-to-end target coverage | ~91.0% |",
        "",
        "| Category | Estimate |",
        "| --- | ---: |",
        "| Public API and module boundaries | 99.5% |",
        "| Declarative model, identity, reconciliation | 84.8% |",
        "| Input, provenance, and edit lifecycle | 98.5% |",
        "| Layout, composition, virtualization | ~87% |",
        "| Text, focus, and selection | ~90.5% |",
        "| Numeric controls | 90% |",
        "| Runtime, effects, and scheduling | 97% |",
        "| Rendering, invalidation, retained GPU surfaces | ~89.2% |",
        "| Platform, windowing, and host boundaries | 72% |",
        "| Diagnostics, profiling, and performance validation | ~95.5% |",
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
