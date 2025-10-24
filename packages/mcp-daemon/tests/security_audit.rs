use kodegen_daemon::security::audit::{
    AuditResult, AuditThresholds, Vulnerability, VulnerabilityMetrics, VulnerabilitySeverity,
};

#[test]
fn test_vulnerability_creation() {
    let vuln = Vulnerability::new(
        "RUSTSEC-2023-0001",
        "test-package",
        VulnerabilitySeverity::High,
        "Test vulnerability",
        "1.0.0",
        Some("1.0.1"),
    );

    assert!(vuln.is_some());
    let vuln = vuln.unwrap();
    assert_eq!(vuln.id.as_str(), "RUSTSEC-2023-0001");
    assert_eq!(vuln.package.as_str(), "test-package");
    assert_eq!(vuln.severity, VulnerabilitySeverity::High);
}

#[test]
fn test_audit_result_thresholds() {
    let thresholds = AuditThresholds::new(0, 1, 5, 10);
    let mut result = AuditResult::new();

    let vuln = Vulnerability::new(
        "RUSTSEC-2023-0001",
        "test-package",
        VulnerabilitySeverity::High,
        "Test vulnerability",
        "1.0.0",
        None,
    )
    .unwrap();

    result.add_vulnerability(vuln).unwrap();

    assert!(result.passes_thresholds(&thresholds));

    let critical_vuln = Vulnerability::new(
        "RUSTSEC-2023-0002",
        "test-package-2",
        VulnerabilitySeverity::Critical,
        "Critical vulnerability",
        "1.0.0",
        None,
    )
    .unwrap();

    result.add_vulnerability(critical_vuln).unwrap();

    assert!(!result.passes_thresholds(&thresholds));
}

#[test]
fn test_simd_pattern_matching() {
    let vuln = Vulnerability::new(
        "RUSTSEC-2023-0001",
        "test-package",
        VulnerabilitySeverity::High,
        "Test vulnerability with pattern",
        "1.0.0",
        None,
    )
    .unwrap();

    assert!(vuln.matches_pattern(b"RUSTSEC"));
    assert!(vuln.matches_pattern(b"pattern"));
    assert!(!vuln.matches_pattern(b"nonexistent"));
}

#[test]
fn test_vulnerability_metrics() {
    let metrics = VulnerabilityMetrics {
        critical_count: 1,
        high_count: 2,
        medium_count: 3,
        low_count: 4,
        total_scans: 10,
        successful_scans: 8,
        cache_size: 100,
    };

    assert_eq!(metrics.total_vulnerabilities(), 10);
    assert_eq!(metrics.success_rate(), 80.0);
    assert!(metrics.has_critical());
}
