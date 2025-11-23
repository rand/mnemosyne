//! Test output parsing for Reviewer agent
//!
//! Extracts test results from standard output formats (Cargo, Pytest).

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

/// Result of a test execution
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TestExecution {
    /// Whether the test suite passed
    pub passed: bool,
    /// Number of tests passed
    pub passed_count: usize,
    /// Number of tests failed
    pub failed_count: usize,
    /// Number of tests ignored/skipped
    pub ignored_count: usize,
    /// Total number of tests
    pub total_count: usize,
    /// Names of failed tests (if extractable)
    pub failed_tests: Vec<String>,
}

/// Parse test output string into TestExecution result
pub fn parse_test_output(output: &str) -> Option<TestExecution> {
    // Try Cargo format first
    if let Some(result) = parse_cargo_output(output) {
        return Some(result);
    }

    // Try Pytest format
    if let Some(result) = parse_pytest_output(output) {
        return Some(result);
    }

    None
}

static CARGO_RESULT_RE: OnceLock<Regex> = OnceLock::new();
static CARGO_FAILED_RE: OnceLock<Regex> = OnceLock::new();

fn parse_cargo_output(output: &str) -> Option<TestExecution> {
    // Look for: "test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out"
    let result_re = CARGO_RESULT_RE.get_or_init(|| {
        Regex::new(r"test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored").unwrap()
    });

    // Look for failed tests: "test tests::test_name ... FAILED"
    let failed_re =
        CARGO_FAILED_RE.get_or_init(|| Regex::new(r"test ([a-zA-Z0-9_:]+) \.\.\. FAILED").unwrap());

    // Scan specifically for the final result line
    for line in output.lines().rev() {
        if let Some(caps) = result_re.captures(line) {
            let status = caps.get(1).map_or("", |m| m.as_str());
            let passed_count = caps
                .get(2)
                .and_then(|m| m.as_str().parse().ok())
                .unwrap_or(0);
            let failed_count = caps
                .get(3)
                .and_then(|m| m.as_str().parse().ok())
                .unwrap_or(0);
            let ignored_count = caps
                .get(4)
                .and_then(|m| m.as_str().parse().ok())
                .unwrap_or(0);

            let passed = status == "ok";

            // Extract failed test names if failed
            let mut failed_tests = Vec::new();
            if !passed {
                for line in output.lines() {
                    if let Some(caps) = failed_re.captures(line) {
                        if let Some(name) = caps.get(1) {
                            failed_tests.push(name.as_str().to_string());
                        }
                    }
                }
            }

            return Some(TestExecution {
                passed,
                passed_count,
                failed_count,
                ignored_count,
                total_count: passed_count + failed_count + ignored_count,
                failed_tests,
            });
        }
    }

    None
}

static PYTEST_SUMMARY_RE: OnceLock<Regex> = OnceLock::new();
static PYTEST_FAILED_RE: OnceLock<Regex> = OnceLock::new();

fn parse_pytest_output(output: &str) -> Option<TestExecution> {
    // Look for: "=== 1 failed, 2 passed, 1 skipped in 0.12s ==="
    // Or: "=== 2 passed in 0.12s ==="
    let summary_re = PYTEST_SUMMARY_RE.get_or_init(|| {
        Regex::new(r"=+ (?:(\d+) failed, )?(?:(\d+) passed)?(?:, )?(?:(\d+) skipped)?").unwrap()
    });

    let failed_re = PYTEST_FAILED_RE.get_or_init(|| Regex::new(r"FAILED .+::(.+) -").unwrap());

    for line in output.lines().rev() {
        if line.contains("===") {
            // Optimization: check for boundary markers
            if let Some(caps) = summary_re.captures(line) {
                let failed_count = caps
                    .get(1)
                    .and_then(|m| m.as_str().parse().ok())
                    .unwrap_or(0);
                let passed_count = caps
                    .get(2)
                    .and_then(|m| m.as_str().parse().ok())
                    .unwrap_or(0);
                let ignored_count = caps
                    .get(3)
                    .and_then(|m| m.as_str().parse().ok())
                    .unwrap_or(0);

                // If we matched *something* numeric, it's likely a summary line
                if failed_count + passed_count + ignored_count > 0 {
                    let passed = failed_count == 0;

                    let mut failed_tests = Vec::new();
                    if !passed {
                        for line in output.lines() {
                            if let Some(caps) = failed_re.captures(line) {
                                if let Some(name) = caps.get(1) {
                                    failed_tests.push(name.as_str().to_string());
                                }
                            }
                        }
                    }

                    return Some(TestExecution {
                        passed,
                        passed_count,
                        failed_count,
                        ignored_count,
                        total_count: passed_count + failed_count + ignored_count,
                        failed_tests,
                    });
                }
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cargo_success() {
        let output = r#"
running 2 tests
test tests::test_1 ... ok
test tests::test_2 ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
"#;
        let result = parse_test_output(output).unwrap();
        assert!(result.passed);
        assert_eq!(result.passed_count, 2);
        assert_eq!(result.failed_count, 0);
    }

    #[test]
    fn test_parse_cargo_failure() {
        let output = r#"
running 2 tests
test tests::test_1 ... ok
test tests::test_2 ... FAILED

failures:

---- tests::test_2 stdout ----
Error: something wrong

failures:
    tests::test_2

test result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
"#;
        let result = parse_test_output(output).unwrap();
        assert!(!result.passed);
        assert_eq!(result.passed_count, 1);
        assert_eq!(result.failed_count, 1);
        assert!(result.failed_tests.contains(&"tests::test_2".to_string()));
    }

    #[test]
    fn test_parse_pytest_success() {
        let output = "=== 5 passed in 0.45s ===";
        let result = parse_test_output(output).unwrap();
        assert!(result.passed);
        assert_eq!(result.passed_count, 5);
        assert_eq!(result.failed_count, 0);
    }

    #[test]
    fn test_parse_pytest_failure() {
        let output = "=== 1 failed, 4 passed in 0.45s ===";
        let result = parse_test_output(output).unwrap();
        assert!(!result.passed);
        assert_eq!(result.passed_count, 4);
        assert_eq!(result.failed_count, 1);
    }
}
