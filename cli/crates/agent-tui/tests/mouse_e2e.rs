mod common;

mod e2e_mouse {
    use crate::common::RealTestHarness;
    use predicates::prelude::*;
    use serde_json::Value;
    use std::sync::Mutex;
    use std::sync::MutexGuard;
    use std::sync::OnceLock;

    fn slow_e2e_lock() -> MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn spawn_session(harness: &RealTestHarness, command: &str) -> String {
        let output = harness
            .cli_command()
            .args(["--format", "json", "run", command])
            .output()
            .expect("failed to run session");
        assert!(
            output.status.success(),
            "run command failed: stdout={}, stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let value: Value = serde_json::from_slice(&output.stdout).expect("run output must be JSON");
        value["session_id"]
            .as_str()
            .expect("run output must include session_id")
            .to_string()
    }

    #[test]
    #[ignore = "slow e2e"]
    fn e2e_mouse_commands_send_rpc_and_succeed() {
        let _lock = slow_e2e_lock();
        let harness = RealTestHarness::new();
        let session_id = spawn_session(&harness, "cat");

        harness
            .run(&["--session", &session_id, "mouse", "click", "10", "20"])
            .success()
            .stdout(predicate::str::contains("Mouse clicked at 10x20 with left"));

        harness
            .run(&[
                "--session",
                &session_id,
                "mouse",
                "click",
                "5",
                "5",
                "--button",
                "right",
            ])
            .success()
            .stdout(predicate::str::contains("Mouse clicked at 5x5 with right"));

        harness
            .run(&["--session", &session_id, "mouse", "move", "0", "0"])
            .success()
            .stdout(predicate::str::contains("Mouse moved to 0x0"));

        harness
            .run(&[
                "--session",
                &session_id,
                "mouse",
                "down",
                "10",
                "10",
                "--button",
                "middle",
            ])
            .success()
            .stdout(predicate::str::contains(
                "Mouse button down at 10x10 with middle",
            ));

        harness
            .run(&[
                "--session",
                &session_id,
                "mouse",
                "up",
                "10",
                "10",
                "--button",
                "middle",
            ])
            .success()
            .stdout(predicate::str::contains(
                "Mouse button up at 10x10 with middle",
            ));

        harness
            .run(&["--session", &session_id, "kill", "--yes"])
            .success();
    }
}
