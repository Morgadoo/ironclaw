//! Forge pipeline orchestration — runs all six gates in sequence
//! with retry logic.

use std::path::Path;
use std::time::Duration;

use project_e_core::config::ForgeConfig;
use project_e_core::types::GateName;

use crate::gates::{self, GateResult};
use crate::retry::RetryState;

/// Result of running the full forge pipeline on a piece of code.
#[derive(Debug)]
pub enum ForgeOutcome {
    /// All gates passed. Contains the final code.
    Success { code: String },
    /// Failed after all retry attempts. Contains the best candidate.
    Failure {
        best_code: Option<String>,
        retry_state: RetryState,
    },
}

/// Run the six-gate pipeline on the given source code.
///
/// Returns the gate where failure occurred (if any).
pub async fn run_gates(source: &str, workdir: &Path, config: &ForgeConfig) -> GateResult {
    // Gate 1: Unsafe check (static analysis, no I/O)
    let result = gates::unsafe_check::check_no_unsafe(source);
    if !result.is_pass() {
        return result;
    }

    // Gate 2: Forbidden patterns (static analysis, no I/O)
    let result = gates::forbidden::check_forbidden_patterns(source);
    if !result.is_pass() {
        return result;
    }

    // Gate 3: Compilation
    let result =
        gates::compile::check_compile(workdir, Duration::from_secs(config.compile_timeout_secs))
            .await;
    if !result.is_pass() {
        return result;
    }

    // Gate 4: Clippy
    let result =
        gates::clippy::check_clippy(workdir, Duration::from_secs(config.clippy_timeout_secs)).await;
    if !result.is_pass() {
        return result;
    }

    // Gate 5: Security audit
    let result =
        gates::audit::check_audit(workdir, Duration::from_secs(config.audit_timeout_secs)).await;
    if !result.is_pass() {
        return result;
    }

    // Gate 6: Tests
    gates::test::check_tests(workdir, Duration::from_secs(config.test_timeout_secs)).await
}

/// Determine which gate failed from a GateResult.
pub fn failed_gate(result: &GateResult) -> Option<GateName> {
    match result {
        GateResult::Pass => None,
        GateResult::Fail(e) => Some(e.gate),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gate_sequence() {
        // Verify gate ordering is correct
        assert!(GateName::UnsafeCheck.ordinal() < GateName::ForbiddenPatterns.ordinal());
        assert!(GateName::ForbiddenPatterns.ordinal() < GateName::Compile.ordinal());
        assert!(GateName::Compile.ordinal() < GateName::Clippy.ordinal());
        assert!(GateName::Clippy.ordinal() < GateName::Audit.ordinal());
        assert!(GateName::Audit.ordinal() < GateName::Test.ordinal());
    }
}
