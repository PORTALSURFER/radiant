use super::*;

pub(super) fn registered_example_names(manifest: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut in_example = false;

    for line in manifest.lines().map(str::trim) {
        match line {
            "[[example]]" => in_example = true,
            line if line.starts_with("[[") || line.starts_with('[') => in_example = false,
            line if in_example && line.starts_with("name = ") => {
                let Some(name) = quoted_toml_value(line) else {
                    continue;
                };
                names.push(name.to_owned());
            }
            _ => {}
        }
    }

    names
}

fn quoted_toml_value(line: &str) -> Option<&str> {
    line.split_once('"')
        .and_then(|(_, tail)| tail.split_once('"'))
        .map(|(value, _)| value)
}

pub(super) fn example_source(manifest_dir: &Path, name: &str, path: &str) -> String {
    let root_path = manifest_dir.join(path);
    let mut source = fs::read_to_string(&root_path)
        .unwrap_or_else(|_| panic!("{name} example should be readable"));
    let module_dir = manifest_dir.join("examples").join(name);
    if module_dir.exists() {
        let mut modules = Vec::new();
        collect_example_modules(&module_dir, &mut modules);
        modules.sort();
        for module in modules {
            source.push('\n');
            source.push_str(&fs::read_to_string(&module).unwrap_or_else(|err| {
                panic!("failed to read example module {}: {err}", module.display())
            }));
        }
    }
    source
}

fn collect_example_modules(dir: &Path, modules: &mut Vec<PathBuf>) {
    for entry in
        fs::read_dir(dir).unwrap_or_else(|err| panic!("failed to read {}: {err}", dir.display()))
    {
        let path = entry
            .unwrap_or_else(|err| panic!("failed to read entry in {}: {err}", dir.display()))
            .path();
        if path.is_dir() {
            collect_example_modules(&path, modules);
        } else if path.extension().and_then(|extension| extension.to_str()) == Some("rs") {
            modules.push(path);
        }
    }
}
