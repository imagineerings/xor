use serde_json::{Value, json};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AgentBenchmarkKind {
    ResponseLatency,
    ToolExecution,
    ContextCompaction,
    ConcurrentSessions,
}

impl AgentBenchmarkKind {
    pub fn name(self) -> &'static str {
        match self {
            Self::ResponseLatency => "response_latency",
            Self::ToolExecution => "tool_execution",
            Self::ContextCompaction => "context_compaction",
            Self::ConcurrentSessions => "concurrent_sessions",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentBenchmarkResult {
    pub name: &'static str,
    pub iterations: usize,
    pub total_duration: Duration,
    pub token_count: usize,
}

impl AgentBenchmarkResult {
    pub fn average_latency_millis(&self) -> f64 {
        if self.iterations == 0 {
            0.0
        } else {
            self.total_duration.as_secs_f64() * 1000.0 / self.iterations as f64
        }
    }

    pub fn to_json(&self) -> Value {
        json!({
            "name": self.name,
            "iterations": self.iterations,
            "total_duration_millis": self.total_duration.as_millis(),
            "average_latency_millis": self.average_latency_millis(),
            "token_count": self.token_count,
        })
    }
}

pub fn run_agent_benchmark(
    kind: AgentBenchmarkKind,
    iterations: usize,
    mut operation: impl FnMut(usize) -> usize,
) -> AgentBenchmarkResult {
    let started_at = Instant::now();
    let mut token_count = 0;
    for iteration in 0..iterations {
        token_count += operation(iteration);
    }
    AgentBenchmarkResult {
        name: kind.name(),
        iterations,
        total_duration: started_at.elapsed(),
        token_count,
    }
}

pub fn benchmark_report(results: &[AgentBenchmarkResult]) -> Value {
    json!({
        "schema_version": 1,
        "benchmarks": results.iter().map(AgentBenchmarkResult::to_json).collect::<Vec<_>>(),
    })
}

pub fn synthetic_agent_benchmark_report(iterations: usize) -> Value {
    let categories = [
        AgentBenchmarkKind::ResponseLatency,
        AgentBenchmarkKind::ToolExecution,
        AgentBenchmarkKind::ContextCompaction,
        AgentBenchmarkKind::ConcurrentSessions,
    ];
    let results = categories
        .into_iter()
        .map(|kind| run_agent_benchmark(kind, iterations, |_| synthetic_token_count(kind)))
        .collect::<Vec<_>>();
    benchmark_report(&results)
}

fn synthetic_token_count(kind: AgentBenchmarkKind) -> usize {
    match kind {
        AgentBenchmarkKind::ResponseLatency => 128,
        AgentBenchmarkKind::ToolExecution => 64,
        AgentBenchmarkKind::ContextCompaction => 512,
        AgentBenchmarkKind::ConcurrentSessions => 256,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_benchmark_report_is_structured_json() {
        let report = synthetic_agent_benchmark_report(2);
        let benchmarks = report
            .get("benchmarks")
            .and_then(Value::as_array)
            .expect("benchmarks array");

        assert_eq!(report["schema_version"], 1);
        assert_eq!(benchmarks.len(), 4);
        assert!(benchmarks.iter().all(|benchmark| {
            benchmark.get("name").and_then(Value::as_str).is_some()
                && benchmark
                    .get("token_count")
                    .and_then(Value::as_u64)
                    .is_some()
                && benchmark
                    .get("average_latency_millis")
                    .and_then(Value::as_f64)
                    .is_some()
        }));
    }
}
