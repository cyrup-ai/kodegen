//! Code quality and complexity metrics collection

use super::check_file_size;
use crate::github::search_repositories::config::SearchConfig;
use crate::github::search_repositories::helpers::{is_git_dir, is_hidden, is_vendor_dir};
use crate::github::search_repositories::types::CodeQualityMetrics;
use log::warn;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::Path;
use walkdir::WalkDir;

/// Collects code quality metrics
pub(crate) async fn collect_code_quality_metrics(
    repo_path: &Path,
    config: &SearchConfig,
) -> Option<CodeQualityMetrics> {
    let mut total_lines = 0u32;
    let mut code_lines = 0u32;
    let mut comment_lines = 0u32;
    let mut blank_lines = 0u32;
    let mut files_count = 0u32;
    let mut languages: HashMap<String, u32> = HashMap::new();
    let mut total_function_lines = 0u32;
    let mut function_count = 0u32;
    let mut total_complexity = 0u32;
    let mut line_hashes: HashMap<u64, u32> = HashMap::new();
    let mut duplicate_lines = 0u32;

    for entry in WalkDir::new(repo_path)
        .into_iter()
        .filter_entry(|e| !is_hidden(e) && !is_git_dir(e) && !is_vendor_dir(e))
        .filter_map(std::result::Result::ok)
    {
        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();
        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");

        // Detect language by extension
        let lang = match ext {
            "rs" => "Rust",
            "py" => "Python",
            "js" | "jsx" => "JavaScript",
            "ts" | "tsx" => "TypeScript",
            "go" => "Go",
            "java" => "Java",
            "c" | "h" => "C",
            "cpp" | "cc" | "cxx" | "hpp" => "C++",
            "rb" => "Ruby",
            "php" => "PHP",
            "swift" => "Swift",
            "kt" | "kts" => "Kotlin",
            "cs" => "C#",
            "sh" | "bash" => "Shell",
            _ => continue,
        };

        files_count += 1;
        *languages.entry(lang.to_string()).or_insert(0) += 1;

        // Check file size before reading
        if let Err(e) = check_file_size(path, config.max_file_size) {
            warn!("Code file skipped: {e}");
            continue;
        }

        if let Ok(content) = std::fs::read_to_string(path) {
            let is_comment = |line: &str, extension: &str| -> bool {
                match extension {
                    "rs" | "c" | "h" | "cpp" | "cc" | "cxx" | "hpp" | "java" | "js" | "jsx"
                    | "ts" | "tsx" | "go" | "swift" | "kt" | "kts" | "cs" | "php" => {
                        line.starts_with("//") || line.starts_with("/*") || line.starts_with('*')
                    }
                    "py" | "sh" | "bash" | "rb" => line.starts_with('#'),
                    _ => false,
                }
            };

            let is_function_start = |line: &str, extension: &str| -> bool {
                match extension {
                    "rs" => line.contains("fn ") && line.contains('{'),
                    "py" => line.starts_with("def ") && line.contains(':'),
                    "js" | "jsx" | "ts" | "tsx" => {
                        (line.contains("function ") || line.contains("=>"))
                            && (line.contains('{') || line.contains('('))
                    }
                    "go" => line.starts_with("func ") && line.contains('{'),
                    "java" | "cs" | "kt" | "kts" => {
                        (line.contains("public ")
                            || line.contains("private ")
                            || line.contains("protected "))
                            && line.contains('(')
                            && line.contains('{')
                    }
                    "c" | "h" | "cpp" | "cc" | "cxx" | "hpp" => {
                        line.contains('(')
                            && line.contains(')')
                            && line.contains('{')
                            && !line.starts_with("if")
                            && !line.starts_with("while")
                            && !line.starts_with("for")
                    }
                    "rb" => line.starts_with("def "),
                    "php" => line.contains("function ") && line.contains('{'),
                    "swift" => line.contains("func ") && line.contains('{'),
                    _ => false,
                }
            };

            let mut in_function = false;
            let mut brace_depth = 0;
            let mut current_function_lines = 0u32;

            for line in content.lines() {
                total_lines += 1;
                let trimmed = line.trim();

                if trimmed.is_empty() {
                    blank_lines += 1;
                } else if is_comment(trimmed, ext) {
                    comment_lines += 1;
                } else {
                    code_lines += 1;

                    // Count decision points for cyclomatic complexity
                    total_complexity += trimmed.matches("if ").count() as u32;
                    total_complexity += trimmed.matches("else if").count() as u32;
                    total_complexity += trimmed.matches("for ").count() as u32;
                    total_complexity += trimmed.matches("while ").count() as u32;
                    total_complexity += trimmed.matches("case ").count() as u32;
                    total_complexity += trimmed.matches("catch ").count() as u32;
                    total_complexity += trimmed.matches("&&").count() as u32;
                    total_complexity += trimmed.matches("||").count() as u32;
                    total_complexity += trimmed.matches('?').count() as u32;

                    // Track duplicate lines (ignore very short lines)
                    if trimmed.len() > 10 {
                        let mut hasher = std::collections::hash_map::DefaultHasher::new();
                        trimmed.hash(&mut hasher);
                        let hash = hasher.finish();

                        let count = line_hashes.entry(hash).or_insert(0);
                        *count += 1;
                        if *count > 1 {
                            duplicate_lines += 1;
                        }
                    }
                }

                // Track function boundaries
                if !in_function && is_function_start(trimmed, ext) {
                    in_function = true;
                    current_function_lines = 1;
                    brace_depth =
                        trimmed.matches('{').count() as i32 - trimmed.matches('}').count() as i32;
                } else if in_function {
                    current_function_lines += 1;
                    brace_depth += trimmed.matches('{').count() as i32;
                    brace_depth -= trimmed.matches('}').count() as i32;

                    if brace_depth <= 0
                        || (ext == "py"
                            && !trimmed.is_empty()
                            && !trimmed.starts_with(' ')
                            && !trimmed.starts_with('\t'))
                    {
                        function_count += 1;
                        total_function_lines += current_function_lines;
                        in_function = false;
                        brace_depth = 0;
                        current_function_lines = 0;
                    }
                }
            }
        }
    }

    let comment_ratio = if total_lines > 0 {
        comment_lines as f32 / total_lines as f32
    } else {
        0.0
    };

    let average_function_length = if function_count > 0 {
        total_function_lines as f32 / function_count as f32
    } else {
        0.0
    };

    let cyclomatic_complexity = if function_count > 0 {
        total_complexity as f32 / function_count as f32
    } else {
        0.0
    };

    let duplicate_code_ratio = if code_lines > 0 {
        duplicate_lines as f32 / code_lines as f32
    } else {
        0.0
    };

    Some(CodeQualityMetrics {
        total_lines,
        code_lines,
        comment_lines,
        blank_lines,
        comment_ratio,
        average_function_length,
        cyclomatic_complexity,
        duplicate_code_ratio,
        files_count,
        languages,
    })
}
