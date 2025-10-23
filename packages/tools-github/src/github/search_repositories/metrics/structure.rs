//! Project structure and organization metrics collection

use crate::github::search_repositories::helpers::calculate_structure_score;
use crate::github::search_repositories::types::StructureMetrics;
use std::path::Path;

/// Collects structure metrics
pub(crate) async fn collect_structure_metrics(repo_path: &Path) -> Option<StructureMetrics> {
    let mut root_files = Vec::new();

    if let Ok(entries) = std::fs::read_dir(repo_path) {
        for entry in entries.filter_map(std::result::Result::ok) {
            if entry.file_type().map(|t| t.is_file()).unwrap_or(false)
                && let Some(name) = entry.file_name().to_str()
            {
                root_files.push(name.to_string());
            }
        }
    }

    let has_src = repo_path.join("src").exists();
    let has_lib = repo_path.join("lib").exists();
    let has_tests = repo_path.join("tests").exists() || repo_path.join("test").exists();
    let has_docs = repo_path.join("docs").exists();
    let has_examples = repo_path.join("examples").exists();
    let has_bin = repo_path.join("bin").exists();

    let follows_conventions = has_src || has_lib;
    let modular_structure = (has_src || has_lib) && has_tests;
    let separation_of_concerns = has_src && has_tests && has_docs;
    let configuration_externalized = root_files.iter().any(|f| {
        f.ends_with(".toml")
            || f.ends_with(".yaml")
            || f.ends_with(".yml")
            || f.ends_with(".json")
            || f == ".env.example"
    });

    let directory_structure_score =
        calculate_structure_score(has_src, has_lib, has_tests, has_docs, has_examples, has_bin);

    Some(StructureMetrics {
        root_files,
        directory_structure_score,
        follows_conventions,
        modular_structure,
        separation_of_concerns,
        configuration_externalized,
    })
}
