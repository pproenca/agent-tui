//! Adapters for metrics serialization.
//!
//! This module contains adapter functions that convert DaemonMetrics
//! to serialized formats like JSON, keeping serialization concerns
//! out of the metrics module itself.

use serde_json::Value;

use crate::metrics::DaemonMetrics;

/// Convert DaemonMetrics to a JSON value.
pub fn metrics_to_json(metrics: &DaemonMetrics) -> Value {
    serde_json::json!({
        "requests_total": metrics.requests(),
        "errors_total": metrics.errors(),
        "lock_timeouts": metrics.lock_timeouts(),
        "poison_recoveries": metrics.poison_recoveries(),
        "uptime_ms": metrics.uptime_ms()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_to_json() {
        let metrics = DaemonMetrics::new();
        metrics.record_request();
        metrics.record_lock_timeout();
        let json = metrics_to_json(&metrics);
        assert_eq!(json["requests_total"], 1);
        assert_eq!(json["lock_timeouts"], 1);
    }
}
