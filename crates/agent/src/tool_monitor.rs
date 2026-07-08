use collections::HashMap;
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolInvocation {
    pub tool_name: String,
    pub started_at_ms: u128,
    pub duration_ms: u128,
    pub success: bool,
    pub error_kind: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ToolStats {
    pub invocation_count: u64,
    pub success_count: u64,
    pub failure_count: u64,
    pub total_duration_ms: u128,
    pub min_duration_ms: Option<u128>,
    pub max_duration_ms: Option<u128>,
    pub average_duration_ms: Option<f64>,
    pub last_invoked_at_ms: Option<u128>,
    pub last_error_kind: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ToolMonitor {
    stats: HashMap<String, ToolStats>,
    invocations: Vec<ToolInvocation>,
}

impl ToolMonitor {
    pub fn record_invocation(&mut self, tool: &str, duration: Duration, success: bool) {
        self.record_invocation_at(tool, now_ms(), duration, success, None);
    }

    pub fn record_failure(
        &mut self,
        tool: &str,
        duration: Duration,
        error_kind: impl Into<String>,
    ) {
        self.record_invocation_at(tool, now_ms(), duration, false, Some(error_kind.into()));
    }

    pub fn record_invocation_at(
        &mut self,
        tool: &str,
        started_at_ms: u128,
        duration: Duration,
        success: bool,
        error_kind: Option<String>,
    ) {
        let duration_ms = duration.as_millis();
        self.invocations.push(ToolInvocation {
            tool_name: tool.to_string(),
            started_at_ms,
            duration_ms,
            success,
            error_kind: error_kind.clone(),
        });

        let stats = self.stats.entry(tool.to_string()).or_default();
        stats.invocation_count += 1;
        if success {
            stats.success_count += 1;
        } else {
            stats.failure_count += 1;
            stats.last_error_kind = error_kind;
        }
        stats.total_duration_ms += duration_ms;
        stats.min_duration_ms = Some(
            stats
                .min_duration_ms
                .map_or(duration_ms, |current| current.min(duration_ms)),
        );
        stats.max_duration_ms = Some(
            stats
                .max_duration_ms
                .map_or(duration_ms, |current| current.max(duration_ms)),
        );
        stats.average_duration_ms =
            Some(stats.total_duration_ms as f64 / stats.invocation_count as f64);
        stats.last_invoked_at_ms = Some(started_at_ms);
    }

    pub fn get_stats(&self, tool: &str) -> Option<&ToolStats> {
        self.stats.get(tool)
    }

    pub fn get_all_stats(&self) -> HashMap<String, ToolStats> {
        self.stats.clone()
    }

    pub fn invocations(&self) -> &[ToolInvocation] {
        &self.invocations
    }

    pub fn reset(&mut self) {
        self.stats.clear();
        self.invocations.clear();
    }
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_tool_invocation_stats() {
        let mut monitor = ToolMonitor::default();

        monitor.record_invocation_at("read_file", 10, Duration::from_millis(20), true, None);
        monitor.record_invocation_at(
            "read_file",
            40,
            Duration::from_millis(80),
            false,
            Some("schema_error".to_string()),
        );

        let stats = monitor
            .get_stats("read_file")
            .expect("stats should exist for recorded tool");
        assert_eq!(stats.invocation_count, 2);
        assert_eq!(stats.success_count, 1);
        assert_eq!(stats.failure_count, 1);
        assert_eq!(stats.total_duration_ms, 100);
        assert_eq!(stats.min_duration_ms, Some(20));
        assert_eq!(stats.max_duration_ms, Some(80));
        assert_eq!(stats.average_duration_ms, Some(50.0));
        assert_eq!(stats.last_invoked_at_ms, Some(40));
        assert_eq!(stats.last_error_kind.as_deref(), Some("schema_error"));
    }

    #[test]
    fn keeps_invocation_history_and_resets() {
        let mut monitor = ToolMonitor::default();
        monitor.record_invocation_at("terminal", 100, Duration::from_millis(5), true, None);

        assert_eq!(monitor.invocations().len(), 1);
        assert_eq!(monitor.invocations()[0].tool_name, "terminal");

        monitor.reset();

        assert!(monitor.invocations().is_empty());
        assert!(monitor.get_all_stats().is_empty());
    }
}
