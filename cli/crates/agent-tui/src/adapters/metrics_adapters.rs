//! Metrics adapter helpers.

use serde_json::Value;

use crate::domain::MetricsOutput;

pub(crate) fn metrics_to_json(metrics: &MetricsOutput) -> Value {
    serde_json::json!({
        "requests_total": metrics.requests_total,
        "errors_total": metrics.errors_total,
        "lock_timeouts": metrics.lock_timeouts,
        "poison_recoveries": metrics.poison_recoveries,
        "uptime_ms": metrics.uptime_ms,
        "active_connections": metrics.active_connections,
        "session_count": metrics.session_count
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_to_json() {
        let metrics = MetricsOutput {
            requests_total: 1,
            errors_total: 0,
            lock_timeouts: 1,
            poison_recoveries: 0,
            uptime_ms: 123,
            active_connections: 2,
            session_count: 3,
        };
        let json = metrics_to_json(&metrics);
        assert_eq!(json["requests_total"], 1);
        assert_eq!(json["lock_timeouts"], 1);
        assert_eq!(json["session_count"], 3);
    }
}
