# Requirements: Evaluation Framework

## Introduction

Migrate goose's evaluation infrastructure: the Harbor eval framework, Open Model Gym evaluation suite, scenario tests, and benchmark scripts. These provide systematic evaluation of agent performance across different models and configurations.

## Glossary

- **Eval**: Evaluation — testing agent performance against defined criteria
- **Harbor**: Goose's evaluation framework for testing agent capabilities
- **Open Model Gym**: Evaluation suite for comparing model performance
- **Scenario Test**: Automated test that runs the agent through predefined scenarios and checks outcomes
- **Benchmark**: Performance measurement of agent operations (latency, token usage, etc.)

## Requirements

### Requirement 1: Harbor Evaluation Framework

**User Story:** As a sim developer, I want a structured eval framework (Harbor), so that I can systematically evaluate agent performance.

#### Acceptance Criteria

1. THE Harbor framework SHALL support defining eval scenarios with expected outcomes
2. THE Harbor framework SHALL run eval scenarios against the agent
3. THE Harbor framework SHALL produce evaluation reports with pass/fail and metrics
4. THE Harbor framework SHALL support running evals with different models/providers

### Requirement 2: Open Model Gym

**User Story:** As a sim developer, I want to compare model performance across different providers, so that I can choose the best model for each task.

#### Acceptance Criteria

1. THE Open Model Gym SHALL run standardized evaluations across multiple models
2. THE Open Model Gym SHALL produce comparative results (latency, quality, cost)
3. THE Open Model Gym SHALL support configurable eval tasks

### Requirement 3: Scenario Tests

**User Story:** As a sim developer, I want scenario-based tests that simulate real user interactions, so that I can validate agent behavior in realistic situations.

#### Acceptance Criteria

1. THE scenario test system SHALL define test scenarios with scripted interactions
2. THE scenario test system SHALL support mock LLM providers for deterministic testing
3. THE scenario test system SHALL validate agent responses and tool calls against expected patterns
4. THE scenario test system SHALL support recording and replaying interactions

### Requirement 4: Benchmark Scripts

**User Story:** As a sim developer, I want benchmark scripts for measuring performance, so that I can track and compare performance over time.

#### Acceptance Criteria

1. THE benchmark scripts SHALL measure end-to-end agent response latency
2. THE benchmark scripts SHALL measure tool execution time
3. THE benchmark scripts SHALL measure token usage per task
4. THE benchmark scripts SHALL produce structured output for comparison

## References

- Source: `projects/goose/evals/harbor/`
- Source: `projects/goose/evals/open-model-gym/`
- Source: `projects/goose/crates/goose-cli/src/scenario_tests/`
- Source: `projects/goose/scripts/run-benchmarks.sh`, `parse-benchmark-results.sh`, `bench-postprocess-scripts/`
