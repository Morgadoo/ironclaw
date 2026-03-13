//! Gate 6: Test gate — runs `cargo test`.

use std::path::Path;
use std::time::Duration;

use project_e_core::types::GateName;
use tokio::process::Command;

use super::{GateError, GateResult};

/// Run `cargo test` in the given directory.
pub async fn check_tests(workdir: &Path, timeout: Duration) -> GateResult {
    let result = tokio::time::timeout(
        timeout,
        Command::new("cargo")
            .args(["test"])
            .current_dir(workdir)
            .output(),
    )
    .await;

    match result {
        Ok(Ok(output)) => {
            if output.status.success() {
                GateResult::Pass
            } else {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                GateResult::Fail(
                    GateError::new(GateName::Test, "Tests failed")
                        .with_details(format!("{stdout}\n{stderr}")),
                )
            }
        }
        Ok(Err(e)) => GateResult::Fail(
            GateError::new(GateName::Test, "Failed to run cargo test").with_details(e.to_string()),
        ),
        Err(_) => GateResult::Fail(GateError::new(
            GateName::Test,
            format!("Tests timed out after {}s", timeout.as_secs()),
        )),
    }
}
