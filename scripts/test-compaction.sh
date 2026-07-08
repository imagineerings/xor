#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

cargo test -p agent_settings auto_compact --lib
cargo test -p acp_thread compact --lib
