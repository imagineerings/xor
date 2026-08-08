# Implementation Plan: Evaluation Framework

## Overview

Implement structured evaluation capabilities: Harbor eval framework (`crates/eval_harbor/`), Open Model Gym for model comparison, enhanced scenario tests in `crates/agent/`, and benchmark scripts.

## Tasks

- [ ] 1. Create Harbor eval framework crate
  - Define ScenarioDefinition, ScenarioStep, ExpectedOutcome types
  - Implement EvalRunner with scenario loading and execution
  - Implement assertion engine for validating outcomes
  - Implement report generator (JSON output)

  - _Requirements: 1.1, 1.2, 1.3, 1.4_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/evaluation/requirements.md, .agents/specs/goose-migration/evaluation/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/eval_harbor/src/lib.rs, crates/eval_harbor/src/runner.rs, crates/eval_harbor/src/assertion.rs_
  - _Writes: crates/eval_harbor/src/lib.rs, crates/eval_harbor/src/runner.rs, crates/eval_harbor/src/assertion.rs_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 2. Implement mock LLM provider for deterministic evals
  - Configurable canned responses
  - Record/replay capability
  - Deterministic behavior for reproducible tests

  - _Requirements: 1.1, 1.2, 1.3, 1.4, 3.1, 3.2, 3.3, 3.4_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/evaluation/requirements.md, .agents/specs/goose-migration/evaluation/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/eval_harbor/src/mock_provider.rs_
  - _Writes: crates/eval_harbor/src/mock_provider.rs_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 3. Implement Open Model Gym
  - Define eval tasks for model comparison
  - Run evaluations across multiple models/providers
  - Produce comparison reports (latency, cost, quality score)

  - _Requirements: 2.1, 2.2, 2.3_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/evaluation/requirements.md, .agents/specs/goose-migration/evaluation/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/eval_harbor/src/gym.rs_
  - _Writes: crates/eval_harbor/src/gym.rs_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 4. Implement scenario test infrastructure in agent crate
  - Recording: capture interactions for replay
  - Playback: replay recorded interactions deterministically
  - Assertion: validate agent behavior against recorded expectations

  - _Requirements: 3.1, 3.2, 3.3, 3.4_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/evaluation/requirements.md, .agents/specs/goose-migration/evaluation/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/agent/src/tests/scenario_runner.rs_
  - _Writes: crates/agent/src/tests/scenario_runner.rs_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 5. Implement agent benchmark scripts
  - Response latency benchmark
  - Tool execution benchmark
  - Context compaction benchmark
  - Concurrent sessions benchmark
  - Structured JSON output for comparison

  - _Requirements: 4.1, 4.2, 4.3, 4.4_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/evaluation/requirements.md, .agents/specs/goose-migration/evaluation/design.md, .agents/specs/goose-migration/coverage-catalog.md, scripts/bench-agent.sh, crates/benchmarks/src/agent.rs_
  - _Writes: scripts/bench-agent.sh, crates/benchmarks/src/agent.rs_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 6. Write tests
  - Harbor eval runner with sample scenarios
  - Mock provider determinism tests
  - Scenario recording/playback round-trip
  - Benchmark script validation

  - _Requirements: 1.1, 1.2, 1.3, 1.4, 2.1, 2.2, 2.3, 3.1, 3.2, 3.3, 3.4, 4.1, 4.2, 4.3, 4.4_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/evaluation/requirements.md, .agents/specs/goose-migration/evaluation/design.md, .agents/specs/goose-migration/coverage-catalog.md_
  - _Writes: none_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

## Notes

- Harbor evals are CLI-driven: `cargo run -- eval harbor <scenario-dir>`
- Model Gym requires multiple providers configured
- Scenario recordings can be committed to the repository for regression testing
