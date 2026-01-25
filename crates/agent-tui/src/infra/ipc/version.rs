use crate::common::ValueExt;

use crate::infra::ipc::client::DaemonClient;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionMismatch {
    pub cli_version: String,
    pub daemon_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersionCheckResult {
    Match,
    Mismatch(VersionMismatch),
    CheckFailed(String),
}

pub fn check_version<C: DaemonClient>(client: &mut C, cli_version: &str) -> VersionCheckResult {
    match client.call("health", None) {
        Err(e) => VersionCheckResult::CheckFailed(e.to_string()),
        Ok(health) => {
            let daemon_version = health.str_or("version", "unknown");
            if cli_version != daemon_version {
                VersionCheckResult::Mismatch(VersionMismatch {
                    cli_version: cli_version.to_string(),
                    daemon_version: daemon_version.to_string(),
                })
            } else {
                VersionCheckResult::Match
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::ipc::mock_client::MockClient;
    use serde_json::json;

    #[test]
    fn test_version_match_returns_match() {
        let mut client = MockClient::new();
        client.set_response(
            "health",
            json!({
                "status": "healthy",
                "version": "1.0.0"
            }),
        );

        let result = check_version(&mut client, "1.0.0");
        assert_eq!(result, VersionCheckResult::Match);
    }

    #[test]
    fn test_version_mismatch_returns_mismatch() {
        let mut client = MockClient::new();
        client.set_response(
            "health",
            json!({
                "status": "healthy",
                "version": "2.0.0"
            }),
        );

        let result = check_version(&mut client, "1.0.0");
        match result {
            VersionCheckResult::Mismatch(mismatch) => {
                assert_eq!(mismatch.cli_version, "1.0.0");
                assert_eq!(mismatch.daemon_version, "2.0.0");
            }
            _ => panic!("Expected Mismatch, got {:?}", result),
        }
    }

    #[test]
    fn test_daemon_not_running_returns_check_failed() {
        let mut client = MockClient::new_strict();

        let result = check_version(&mut client, "1.0.0");
        match result {
            VersionCheckResult::CheckFailed(msg) => {
                assert!(!msg.is_empty(), "Error message should not be empty");
            }
            _ => panic!("Expected CheckFailed, got {:?}", result),
        }
    }

    #[test]
    fn test_unknown_daemon_version_reports_mismatch() {
        let mut client = MockClient::new();
        client.set_response(
            "health",
            json!({
                "status": "healthy"

            }),
        );

        let result = check_version(&mut client, "1.0.0");
        match result {
            VersionCheckResult::Mismatch(mismatch) => {
                assert_eq!(mismatch.daemon_version, "unknown");
            }
            _ => panic!("Expected Mismatch, got {:?}", result),
        }
    }
}
