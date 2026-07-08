#!/usr/bin/env bash
set -euo pipefail

mode="${1:-json}"

case "$mode" in
  json)
    iterations="${2:-10}"
    cargo run -p benchmarks --bin agent_benchmark -- "$iterations"
    ;;
  criterion)
    cargo bench -p benchmarks --bench agent
    ;;
  *)
    echo "usage: $0 [json|criterion]" >&2
    exit 64
    ;;
esac
