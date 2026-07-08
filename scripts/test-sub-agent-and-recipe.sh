#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

cargo test -p agent subagent --lib
cargo test -p recipe --lib
