use benchmarks::agent::{AgentBenchmarkKind, benchmark_report, run_agent_benchmark};
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn agent_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("agent");

    for kind in [
        AgentBenchmarkKind::ResponseLatency,
        AgentBenchmarkKind::ToolExecution,
        AgentBenchmarkKind::ContextCompaction,
        AgentBenchmarkKind::ConcurrentSessions,
    ] {
        group.bench_function(kind.name(), |bench| {
            bench.iter(|| {
                run_agent_benchmark(kind, 10, |iteration| {
                    black_box((iteration + 1) * kind.name().len())
                })
            });
        });
    }

    group.finish();

    let report = benchmark_report(&[run_agent_benchmark(
        AgentBenchmarkKind::ResponseLatency,
        1,
        |_| 1,
    )]);
    black_box(report);
}

criterion_group!(benches, agent_benchmarks);
criterion_main!(benches);
