//! Type definitions for GitHub repository search

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use thiserror::Error;

/// Comprehensive error handling for search operations
#[derive(Error, Debug)]
pub enum SearchError {
    #[error("GitHub API error: {0}")]
    ApiError(String),

    #[error("Local analysis error: {0}")]
    LocalAnalysisError(String),

    #[error("Rate limit exceeded. Remaining: {remaining}, Reset time: {reset_time}")]
    RateLimitExceeded {
        remaining: u32,
        reset_time: DateTime<Utc>,
    },

    #[error("Invalid query or missing repository info: {details}")]
    InvalidQuery { details: String },

    #[error("Resource limit exceeded. Resource: {resource}, Limit: {limit}")]
    ResourceLimitExceeded { resource: String, limit: String },

    #[error("No search results found for query: {query}")]
    NoResults { query: String },

    #[error("Operation timed out: {operation} after {duration:?}")]
    TimeoutError {
        operation: String,
        duration: Duration,
    },

    #[error("Authentication failed")]
    AuthenticationError,

    #[error("Access denied to repository: {repo}")]
    AccessDenied { repo: String },

    #[error("Configuration error: {0}")]
    ConfigError(String),
}

pub type SearchResult<T> = Result<T, SearchError>;

/// Input query parameters following the protocol specification
#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct SearchQuery {
    pub terms: Vec<String>,
    pub language: Option<String>,
    pub min_stars: u32,
    pub license: Option<String>,
    pub created_after: Option<DateTime<Utc>>,
    pub pushed_after: Option<DateTime<Utc>>,
    pub topic: Option<String>,
    pub user: Option<String>,
    pub org: Option<String>,
    pub exclude_forks: bool,
    pub exclude_archived: bool,
}

/// Composite output of the search
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Output {
    pub status: String,
    pub results: Vec<RepositoryResult>,
    pub metadata: MetadataInfo,
    pub errors: Vec<String>,
}

/// Search metadata and statistics
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct MetadataInfo {
    pub total_results: u32,
    pub cache_hit_rate: f32,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub processing_time_ms: u128,
    pub api_rate_limit_remaining: u32,
    pub partial_results: bool,
}

/// Comprehensive repository analysis result
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct RepositoryResult {
    pub name: String,
    pub full_name: String,
    pub url: String,
    pub clone_url: String,
    pub description: Option<String>,
    pub stars: u32,
    pub forks: u32,
    pub watchers: u32,
    pub language: Option<String>,
    pub topics: Vec<String>,
    pub license: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub pushed_at: DateTime<Utc>,
    pub size_kb: u32,
    pub quality_metrics: QualityMetrics,
    pub activity_metrics: Option<ActivityMetrics>,
    pub local_metrics: Option<LocalMetrics>,
    pub errors: Vec<String>,
}

/// Quality scoring metrics
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct QualityMetrics {
    pub overall_score: f32,
    pub api_score: f32,
    pub local_score: f32,
    pub popularity_score: f32,
    pub maintenance_score: f32,
    pub documentation_score: f32,
    pub security_score: f32,
}

/// Repository activity and engagement metrics
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ActivityMetrics {
    pub commits_last_month: u32,
    pub commits_last_6_months: u32,
    pub commits_last_year: u32,
    pub last_commit: String,
    pub last_commit_date: DateTime<Utc>,
    pub contributors_count: u32,
    pub active_contributors_last_3_months: u32,
    pub pull_requests_merged_last_month: u32,
    pub issues_closed_last_month: u32,
    pub release_frequency: String,
    pub latest_release: Option<String>,
}

/// Local code analysis metrics
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct LocalMetrics {
    pub readme_quality: ReadmeMetrics,
    pub code_quality: CodeQualityMetrics,
    pub test_metrics: TestMetrics,
    pub ci_cd_metrics: CiCdMetrics,
    pub documentation_metrics: DocumentationMetrics,
    pub security_metrics: SecurityMetrics,
    pub dependency_metrics: DependencyMetrics,
    pub structure_metrics: StructureMetrics,
}

/// README file quality analysis
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ReadmeMetrics {
    pub exists: bool,
    pub length: u32,
    pub sections_count: u32,
    pub code_blocks_count: u32,
    pub links_count: u32,
    pub images_count: u32,
    pub badges_count: u32,
    pub table_of_contents: bool,
    pub installation_instructions: bool,
    pub usage_examples: bool,
    pub api_documentation: bool,
    pub license_mentioned: bool,
    pub contributing_guidelines: bool,
    pub quality_score: f32,
}

/// Code quality and complexity metrics
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct CodeQualityMetrics {
    pub total_lines: u32,
    pub code_lines: u32,
    pub comment_lines: u32,
    pub blank_lines: u32,
    pub comment_ratio: f32,
    pub average_function_length: f32,
    pub cyclomatic_complexity: f32,
    pub duplicate_code_ratio: f32,
    pub files_count: u32,
    pub languages: HashMap<String, u32>,
}

/// Testing coverage and framework metrics
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct TestMetrics {
    pub has_tests: bool,
    pub test_files_count: u32,
    pub test_lines: u32,
    pub test_coverage_estimate: f32,
    pub test_frameworks: Vec<String>,
    pub integration_tests: bool,
    pub unit_tests: bool,
    pub e2e_tests: bool,
    pub benchmark_tests: bool,
    pub test_to_code_ratio: f32,
}

/// CI/CD pipeline and automation metrics
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct CiCdMetrics {
    pub has_ci: bool,
    pub ci_providers: Vec<String>,
    pub workflow_files: u32,
    pub build_status: String,
    pub test_automation: bool,
    pub deployment_automation: bool,
    pub code_quality_checks: bool,
    pub security_scanning: bool,
    pub dependency_updates: bool,
    pub release_automation: bool,
}

/// Documentation completeness metrics
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct DocumentationMetrics {
    pub has_docs_folder: bool,
    pub docs_files_count: u32,
    pub api_docs_generated: bool,
    pub changelog_exists: bool,
    pub contributing_guide: bool,
    pub code_of_conduct: bool,
    pub issue_templates: bool,
    pub pr_templates: bool,
    pub wiki_pages: u32,
}

/// Security practices and vulnerability metrics
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct SecurityMetrics {
    pub security_policy: bool,
    pub vulnerability_disclosure: bool,
    pub dependency_scanning: bool,
    pub secrets_scanning: bool,
    pub signed_commits_ratio: f32,
    pub security_advisories: u32,
    pub cve_references: u32,
    pub license_compatibility: bool,
}

/// Dependency management and freshness metrics
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct DependencyMetrics {
    pub total_dependencies: u32,
    pub direct_dependencies: u32,
    pub dev_dependencies: u32,
    pub outdated_dependencies: u32,
    pub vulnerable_dependencies: u32,
    pub dependency_freshness_score: f32,
    pub package_managers: Vec<String>,
    pub lock_files_present: bool,
}

/// Project structure and organization metrics
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct StructureMetrics {
    pub root_files: Vec<String>,
    pub directory_structure_score: f32,
    pub follows_conventions: bool,
    pub modular_structure: bool,
    pub separation_of_concerns: bool,
    pub configuration_externalized: bool,
}

/// Internal helper struct for local analysis scores
pub(crate) struct LocalScores {
    pub overall_local: f32,
    pub readme_score: f32,
    pub coverage_score: f32,
    pub metrics: Option<LocalMetrics>,
}

/// Wiki information for cloning and analysis
#[derive(Clone, Debug)]
pub(crate) struct WikiInfo {
    pub has_wiki: bool,
    pub clone_url: String,
}

/// Cache entry with expiration tracking
pub(crate) struct RepoCacheEntry {
    pub result: RepositoryResult,
    pub commit_hash: String,
    pub cached_at: DateTime<Utc>,
}

impl RepoCacheEntry {
    pub fn is_expired(&self, ttl: Duration) -> bool {
        Utc::now() - self.cached_at > chrono::Duration::from_std(ttl).unwrap_or_default()
    }
}
