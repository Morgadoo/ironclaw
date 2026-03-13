//! Gate 3: Compilation gate — runs `cargo build` on generated module code.

use std::path::Path;
use std::time::Duration;

use project_e_core::types::GateName;
use tokio::process::Command;

use super::{GateError, GateResult};

/// Run `cargo build --release` in the given directory with a timeout.
pub async fn check_compile(workdir: &Path, timeout: Duration) -> GateResult {
    let result = tokio::time::timeout(
        timeout,
        Command::new("cargo")
            .args(["build", "--release"])
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
                    GateError::new(GateName::Compile, "Compilation failed")
                        .with_details(stderr.to_string()),
                )
            }
        }
        Ok(Err(e)) => GateResult::Fail(
            GateError::new(GateName::Compile, "Failed to run cargo build")
                .with_details(e.to_string()),
        ),
        Err(_) => GateResult::Fail(GateError::new(
            GateName::Compile,
            format!("Compilation timed out after {}s", timeout.as_secs()),
        )),
    }
}
