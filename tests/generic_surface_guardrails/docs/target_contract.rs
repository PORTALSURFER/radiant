use super::*;

const CONTRACT_DOCS: &[&str] = &[
    "docs/DESIGN_DIRECTION.md",
    "docs/TARGET.md",
    "docs/API.md",
    "docs/ARCHITECTURE.md",
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
