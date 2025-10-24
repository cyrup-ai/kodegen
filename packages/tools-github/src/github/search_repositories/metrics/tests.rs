//! Test metrics collection

use super::check_file_size;
use crate::github::search_repositories::config::SearchConfig;
use crate::github::search_repositories::helpers::{is_git_dir, is_hidden, is_vendor_dir};
use crate::github::search_repositories::types::TestMetrics;
use log::warn;
use std::path::Path;
use walkdir::WalkDir;

/// Collects test metrics
pub(crate) async fn collect_test_metrics(
    repo_path: &Path,
    code_lines: u32,
    config: &SearchConfig,
) -> Option<TestMetrics> {
    let mut test_files_count = 0u32;
    let mut test_lines = 0u32;
    let mut test_frameworks = Vec::new();
    let mut has_unit_tests = false;
    let mut has_integration_tests = false;
    let mut has_e2e_tests = false;
    let mut has_benchmark_tests = false;
    let mut assertion_count = 0u32;

    // Check for test directories
    let test_dirs = ["test", "tests", "__tests__", "spec", "e2e"];
    let mut has_test_dir = false;
    for dir in &test_dirs {
        if repo_path.join(dir).exists() {
            has_test_dir = true;
            if *dir == "e2e" {
                has_e2e_tests = true;
            }
            break;
        }
    }

    // Scan for test files
    for entry in WalkDir::new(repo_path)
        .into_iter()
        .filter_entry(|e| !is_hidden(e) && !is_git_dir(e) && !is_vendor_dir(e))
        .filter_map(std::result::Result::ok)
    {
        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();
        let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");

        // Check if it's a test file
        let is_test_file = file_name.contains("_test.")
            || file_name.contains("_spec.")
            || file_name.starts_with("test_")
            || file_name.contains(".test.")
            || file_name.contains(".spec.")
            || path.components().any(|c| {
                let s = c.as_os_str().to_str().unwrap_or("");
                test_dirs.contains(&s)
            });

        if is_test_file {
            test_files_count += 1;

            // Check file size before reading
            if let Err(e) = check_file_size(path, config.max_file_size) {
                warn!("Test file skipped: {e}");
                continue;
            }

            if let Ok(content) = std::fs::read_to_string(path) {
                test_lines += content.lines().count() as u32;

                // Count assertions
                assertion_count += content.matches("assert").count() as u32;
                assertion_count += content.matches("expect").count() as u32;
                assertion_count += content.matches("should").count() as u32;
                assertion_count += content.matches("assertEqual").count() as u32;
                assertion_count += content.matches("assertTrue").count() as u32;

                // Detect frameworks
                if (content.contains("pytest") || content.contains("unittest"))
                    && !test_frameworks.contains(&"pytest".to_string())
                {
                    test_frameworks.push("pytest".to_string());
                }
                if (content.contains("jest") || content.contains("describe("))
                    && !test_frameworks.contains(&"jest".to_string())
                {
                    test_frameworks.push("jest".to_string());
                }
                if (content.contains("#[test]") || content.contains("#[cfg(test)]"))
                    && !test_frameworks.contains(&"cargo test".to_string())
                {
                    test_frameworks.push("cargo test".to_string());
                }
                if content.contains("@Test") && !test_frameworks.contains(&"JUnit".to_string()) {
                    test_frameworks.push("JUnit".to_string());
                }

                // Detect test types
                if file_name.contains("unit") {
                    has_unit_tests = true;
                }
                if file_name.contains("integration") || file_name.contains("integ") {
                    has_integration_tests = true;
                }
                if file_name.contains("e2e") || file_name.contains("end-to-end") {
                    has_e2e_tests = true;
                }
                if file_name.contains("bench") {
                    has_benchmark_tests = true;
                }
            }
        }
    }

    let has_tests = test_files_count > 0 || has_test_dir;
    let test_to_code_ratio = if code_lines > 0 {
        test_lines as f32 / code_lines as f32
    } else {
        0.0
    };

    let test_coverage_estimate = if has_tests {
        // Base estimate from file count
        let mut estimate: f32 = if test_files_count > 10 {
            0.7
        } else if test_files_count > 5 {
            0.5
        } else {
            0.3
        };

        // Adjust based on test-to-code ratio
        if test_to_code_ratio > 0.5 {
            estimate += 0.1;
        } else if test_to_code_ratio < 0.1 {
            estimate -= 0.1;
        }

        // Adjust based on assertion density
        let assertions_per_100_lines = if test_lines > 0 {
            (assertion_count as f32 / test_lines as f32) * 100.0
        } else {
            0.0
        };

        if assertions_per_100_lines > 5.0 {
            estimate += 0.1;
        } else if assertions_per_100_lines < 1.0 {
            estimate -= 0.1;
        }

        estimate.clamp(0.0, 1.0)
    } else {
        0.0
    };

    Some(TestMetrics {
        has_tests,
        test_files_count,
        test_lines,
        test_coverage_estimate,
        test_frameworks,
        integration_tests: has_integration_tests,
        unit_tests: has_unit_tests || has_tests,
        e2e_tests: has_e2e_tests,
        benchmark_tests: has_benchmark_tests,
        test_to_code_ratio,
    })
}
