//! Version checking for CLI/daemon compatibility.

use agent_tui_common::ValueExt;

use crate::client::DaemonClient;

/// Version mismatch information.
pub struct VersionMismatch {
    /// CLI version.
    pub cli_version: String,
    /// Daemon version.
    pub daemon_version: String,
}

/// Check for version mismatch between CLI and daemon.
///
/// Returns `Some(VersionMismatch)` if versions differ, `None` if they match
/// or if the daemon is not running.
pub fn check_version<C: DaemonClient>(
    client: &mut C,
    cli_version: &str,
) -> Option<VersionMismatch> {
    let health = client.call("health", None).ok()?;
    let daemon_version = health.str_or("version", "unknown");

    if cli_version != daemon_version {
        Some(VersionMismatch {
            cli_version: cli_version.to_string(),
            daemon_version: daemon_version.to_string(),
        })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock_client::MockClient;
    use serde_json::json;

    #[test]
    fn test_version_match_returns_none() {
        let mut client = MockClient::new();
        client.set_response(
            "health",
            json!({
                "status": "healthy",
                "version": "1.0.0"
            }),
        );

        let result = check_version(&mut client, "1.0.0");
        assert!(result.is_none());
    }

    #[test]
    fn test_version_mismatch_returns_some() {
        let mut client = MockClient::new();
        client.set_response(
            "health",
            json!({
                "status": "healthy",
                "version": "2.0.0"
            }),
        );

        let result = check_version(&mut client, "1.0.0");
        assert!(result.is_some());
        let mismatch = result.unwrap();
        assert_eq!(mismatch.cli_version, "1.0.0");
        assert_eq!(mismatch.daemon_version, "2.0.0");
    }

    #[test]
    fn test_daemon_not_running_returns_none() {
        let mut client = MockClient::new_strict();
        // new_strict() returns error for unconfigured methods

        let result = check_version(&mut client, "1.0.0");
        assert!(result.is_none());
    }

    #[test]
    fn test_unknown_daemon_version_reports_mismatch() {
        let mut client = MockClient::new();
        client.set_response(
            "health",
            json!({
                "status": "healthy"
                // No version field
            }),
        );

        let result = check_version(&mut client, "1.0.0");
        assert!(result.is_some());
        let mismatch = result.unwrap();
        assert_eq!(mismatch.daemon_version, "unknown");
    }
}
