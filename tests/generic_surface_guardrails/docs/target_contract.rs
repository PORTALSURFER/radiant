use super::*;

const CONTRACT_DOCS: &[&str] = &[
    "docs/DESIGN_DIRECTION.md",
    "docs/TARGET.md",
    "docs/API.md",
    "docs/ARCHITECTURE.md",
    "docs/TARGET_ALIGNMENT_STATUS.md",
];

fn normalized(source: &str) -> String {
    source.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn read_contract_doc(manifest_dir: &Path, relative: &str) -> String {
    fs::read_to_string(manifest_dir.join(relative))
        .unwrap_or_else(|error| panic!("{relative} should be readable: {error}"))
}

#[test]
fn normative_docs_keep_current_boundaries_distinct_from_future_work() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let documents = CONTRACT_DOCS
        .iter()
        .map(|relative| {
            (
                *relative,
                normalized(&read_contract_doc(&manifest_dir, relative)),
            )
        })
        .collect::<Vec<_>>();
    let normative = documents
        .iter()
        .filter(|(relative, _)| *relative != "docs/TARGET_ALIGNMENT_STATUS.md")
        .map(|(_, source)| source.as_str())
        .collect::<Vec<_>>()
        .join(" ");

    // Historical arithmetic and stale claims can silently turn acceptance or
    // branch evidence into a false current implementation boundary.
    for forbidden in [
        "900 / 11",
        "901 / 11",
        "903 / 11",
        "shipped without runtime registration",
    ] {
        assert!(
            !normative.contains(forbidden),
            "normative docs must not retain `{forbidden}`"
        );
    }

    // These lexical anchors keep the six issue-owned current/future seams
    // explicit while allowing surrounding prose and section order to evolve.
    for required in [
        "Only canonical merged source counts for shipped status",
        "X11 and product-specific behavior remain explicit non-goals",
        "private `EffectOrigin`",
        "`ResourceTasks` remains application-owned",
        "`runtime/effects` is not complete",
        "OPT-1387",
        "OPT-1390",
        "OPT-1370",
        "prepared refresh constructs the Projection, layout, and paint-plan candidate synchronously",
        "later no-yield publication gate",
        "Independently schedulable Reconciliation, Layout, and Paint stages remain future work under OPT-1389",
        "`LayoutPolicy` is limited to",
        "separate from the built-in `ContainerPolicy`",
        "OPT-1272 is Done",
        "does not reopen that issue",
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
        "Locale and writing-direction services remain future work under OPT-1386",
        "bidi and complex shaping remain future renderer/text-layout work under OPT-1402",
        "RenderCanvas",
        "compatibility",
        "GpuSurface",
        "PaintPrimitive::GpuSurface",
        "CanvasProgram",
        "CanvasGraph",
        "OPT-1407",
        "OPT-1408",
    ] {
        assert!(
            normative.contains(required),
            "normative docs must retain the current/future contract anchor `{required}`"
        );
    }

    for (relative, source) in documents
        .iter()
        .filter(|(relative, _)| *relative != "docs/TARGET_ALIGNMENT_STATUS.md")
    {
        assert!(
            source.contains("Only canonical merged source counts for shipped status"),
            "{relative} must identify canonical merged source as the shipped-status authority"
        );
        assert!(
            source.contains("X11") && source.contains("product-specific"),
            "{relative} must preserve the X11 and product-specific non-goals"
        );
    }
}

#[test]
fn target_alignment_scorecard_has_exact_rows_and_equal_weight_broad_mean() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let scorecard = read_contract_doc(&manifest_dir, "docs/TARGET_ALIGNMENT_STATUS.md");
    let rows = scorecard
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    let expected_rows = vec![
        "# Radiant Target Alignment",
        "| Overall measure | Estimate |",
        "| --- | ---: |",
        "| Generic architecture-sequence completion | ~100% |",
        "| Broad end-to-end target coverage | ~87.5% |",
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

    // The compact scorecard is intentionally the complete file: prose/history
    // appended around the rows would make its arithmetic and authority unclear.
    assert_eq!(
        rows, expected_rows,
        "TARGET_ALIGNMENT_STATUS.md must contain only the established scorecard rows"
    );

    fn estimate(row: &str) -> f64 {
        let cells = row
            .split('|')
            .map(str::trim)
            .filter(|cell| !cell.is_empty())
            .collect::<Vec<_>>();
        cells[1]
            .trim_start_matches('~')
            .trim_end_matches('%')
            .parse::<f64>()
            .unwrap_or_else(|error| panic!("invalid scorecard estimate in {row:?}: {error}"))
    }

    let broad = estimate(rows[4]);
    let category_estimates = rows[7..]
        .iter()
        .map(|row| estimate(row))
        .collect::<Vec<_>>();
    assert_eq!(
        category_estimates.len(),
        11,
        "the scorecard mean must include exactly eleven categories"
    );
    let mean = category_estimates.iter().sum::<f64>() / category_estimates.len() as f64;
    let rounded_mean = (mean * 10.0).round() / 10.0;
    assert!(
        (broad - rounded_mean).abs() < f64::EPSILON,
        "broad coverage {broad:.1}% must equal the equal-weight eleven-category mean {rounded_mean:.1}%"
    );
}
