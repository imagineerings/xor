use benchmarks::agent::synthetic_agent_benchmark_report;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let iterations = std::env::args()
        .nth(1)
        .map(|value| value.parse::<usize>())
        .transpose()?
        .unwrap_or(10);
    println!(
        "{}",
        serde_json::to_string_pretty(&synthetic_agent_benchmark_report(iterations))?
    );
    Ok(())
}
