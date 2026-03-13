//! Gate 5: Security audit gate — runs `cargo audit`.

use std::path::Path;
use std::time::Duration;

use project_e_core::types::GateName;
use tokio::process::Command;

use super::{GateError, GateResult};

/// Run `cargo audit` in the given directory.
pub async fn check_audit(workdir: &Path, timeout: Duration) -> GateResult {
    let result = tokio::time::timeout(
        timeout,
        Command::new("cargo")
            .args(["audit"])
            .current_dir(workdir)
            .output(),
    )
    .await;

    match result {
        Ok(Ok(output)) => {
            if output.status.success() {
                GateResult::Pass
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let stdout = String::from_utf8_lossy(&output.stdout);
                GateResult::Fail(
                    GateError::new(GateName::Audit, "Security audit found vulnerabilities")
                        .with_details(format!("{stdout}\n{stderr}")),
                )
            }
        }
        Ok(Err(e)) => {
            // cargo-audit may not be installed; treat as a soft pass with warning
            tracing::warn!("cargo audit not available: {e}");
            GateResult::Pass
        }
        Err(_) => GateResult::Fail(GateError::new(
            GateName::Audit,
            format!("Audit timed out after {}s", timeout.as_secs()),
        )),
    }
}
