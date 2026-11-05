//! Every pipeline file shipped in `assets/pipelines` must parse and compile against the built-in
//! registry, so a broken file fails here instead of on the projector.

use common::pipeline::{compile, PipelineRegistry, PipelineSpec};
use std::path::{Path, PathBuf};

fn pipeline_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            pipeline_files(&path, out);
        } else if path.to_string_lossy().ends_with(".pipeline.yaml") {
            out.push(path);
        }
    }
}

#[test]
fn all_shipped_pipelines_compile() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets/pipelines");
    let mut files = Vec::new();
    pipeline_files(&root, &mut files);
    assert!(!files.is_empty(), "no pipeline files under {}", root.display());

    let registry = PipelineRegistry::with_builtins();
    let mut problems = Vec::new();
    for file in &files {
        let relative = file.strip_prefix(&root).unwrap().to_string_lossy().replace('\\', "/");
        let text = std::fs::read_to_string(file).unwrap();
        let spec = match PipelineSpec::from_yaml(&text) {
            Ok(spec) => spec,
            Err(e) => {
                problems.push(format!("{relative}: {e}"));
                continue;
            }
        };
        // The name is the path under assets/pipelines, which is how UsePipeline finds the file
        let expected = relative.trim_end_matches(".pipeline.yaml");
        if spec.name != expected {
            problems.push(format!("{relative}: name `{}` should be `{expected}`", spec.name));
        }
        if let Err(errors) = compile(&spec, &registry) {
            problems.extend(errors.iter().map(|e| format!("{relative}: {e}")));
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}
