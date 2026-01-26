#![cfg(feature = "slow-tests")]

#[path = "slow/e2e_workflow_tests.rs"]
mod e2e_workflow_tests;
#[path = "slow/integration_concurrent_tests.rs"]
mod integration_concurrent_tests;
#[path = "slow/integration_connection_failure_tests.rs"]
mod integration_connection_failure_tests;
#[path = "slow/integration_contracts_tests.rs"]
mod integration_contracts_tests;
#[path = "slow/integration_daemon_no_autostart_tests.rs"]
mod integration_daemon_no_autostart_tests;
#[path = "slow/integration_daemon_tests.rs"]
mod integration_daemon_tests;
#[path = "slow/integration_dbl_click_tests.rs"]
mod integration_dbl_click_tests;
#[path = "slow/integration_error_propagation_tests.rs"]
mod integration_error_propagation_tests;
#[path = "slow/integration_lock_timeout_tests.rs"]
mod integration_lock_timeout_tests;
#[path = "slow/integration_parameter_validation_tests.rs"]
mod integration_parameter_validation_tests;
#[path = "slow/integration_response_edge_cases_tests.rs"]
mod integration_response_edge_cases_tests;
#[path = "slow/integration_retry_mechanism_tests.rs"]
mod integration_retry_mechanism_tests;
#[path = "slow/integration_session_state_tests.rs"]
mod integration_session_state_tests;
