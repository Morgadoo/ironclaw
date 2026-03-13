//! Gate 2: Forbidden pattern detection.
//!
//! Detects `unwrap()`, `panic!()`, `todo!()`, `unimplemented!()` in source.

use project_e_core::types::GateName;

use super::{GateError, GateResult};

/// Patterns that are forbidden in production module code.
const FORBIDDEN_PATTERNS: &[(&str, &str)] = &[
    (
        ".unwrap()",
        "Use `?` operator or error handling instead of unwrap()",
    ),
    (
        ".expect(",
        "Use `?` operator or error handling instead of expect()",
    ),
    ("panic!(", "Do not use panic!() in module code"),
    ("todo!(", "Do not leave todo!() in module code"),
    (
        "unimplemented!(",
        "Do not leave unimplemented!() in module code",
    ),
];

/// Check source for forbidden patterns.
pub fn check_forbidden_patterns(source: &str) -> GateResult {
    let mut violations = Vec::new();

    for (line_no, line) in source.lines().enumerate() {
        let trimmed = line.trim();

        // Skip comments
        if trimmed.starts_with("//") || trimmed.starts_with("///") {
            continue;
        }

        for (pattern, message) in FORBIDDEN_PATTERNS {
            if line.contains(pattern) {
                violations.push(format!("line {}: {} — {}", line_no + 1, pattern, message));
            }
        }
    }

    if violations.is_empty() {
        GateResult::Pass
    } else {
        GateResult::Fail(
            GateError::new(
                GateName::ForbiddenPatterns,
                format!("Found {} forbidden pattern(s)", violations.len()),
            )
            .with_details(violations.join("\n")),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_code_passes() {
        let code = r#"
            fn safe() -> Result<String, Error> {
                let value = some_fn()?;
                Ok(value)
            }
        "#;
        assert!(check_forbidden_patterns(code).is_pass());
    }

    #[test]
    fn unwrap_is_forbidden() {
        let code = r#"
            fn bad() -> String {
                some_option().unwrap()
            }
        "#;
        assert!(!check_forbidden_patterns(code).is_pass());
    }

    #[test]
    fn panic_is_forbidden() {
        let code = r#"
            fn bad() {
                panic!("oh no");
            }
        "#;
        assert!(!check_forbidden_patterns(code).is_pass());
    }

    #[test]
    fn todo_is_forbidden() {
        let code = r#"
            fn incomplete() {
                todo!()
            }
        "#;
        assert!(!check_forbidden_patterns(code).is_pass());
    }

    #[test]
    fn comments_with_patterns_are_ok() {
        let code = r#"
            // This used to use unwrap() but we fixed it
            /// Example: .unwrap()
            fn safe() -> Result<(), Error> {
                Ok(())
            }
        "#;
        assert!(check_forbidden_patterns(code).is_pass());
    }
}
