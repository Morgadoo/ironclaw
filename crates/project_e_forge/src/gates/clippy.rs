//! Gate 4: Clippy lint gate — runs `cargo clippy -- -D warnings`.

use std::path::Path;
use std::time::Duration;

use project_e_core::types::GateName;
use tokio::process::Command;

use super::{GateError, GateResult};

/// Run `cargo clippy -- -D warnings` in the given directory.
pub async fn check_clippy(workdir: &Path, timeout: Duration) -> GateResult {
    let result = tokio::time::timeout(
        timeout,
        Command::new("cargo")
            .args(["clippy", "--", "-D", "warnings"])
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
                GateResult::Fail(
                    GateError::new(GateName::Clippy, "Clippy warnings found")
                        .with_details(stderr.to_string()),
                )
            }
        }
        Ok(Err(e)) => GateResult::Fail(
            GateError::new(GateName::Clippy, "Failed to run cargo clippy")
                .with_details(e.to_string()),
        ),
        Err(_) => GateResult::Fail(GateError::new(
            GateName::Clippy,
            format!("Clippy timed out after {}s", timeout.as_secs()),
        )),
    }
}
