#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

export CARGO_INCREMENTAL="${CARGO_INCREMENTAL:-0}"

cargo test -p language_model_core token_counter --lib
cargo test -p language_model_core rate_limiter --lib
cargo test -p telemetry langfuse --lib
cargo test -p telemetry otel --lib
cargo test -p telemetry observation --lib
cargo test -p posthog --lib
cargo test -p agent tool_monitor --lib
cargo test -p agent tool_inspector --lib
cargo test -p agent observability_integration --lib
