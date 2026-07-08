#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

cargo test \
  -p mcp_memory \
  -p mcp_autovisualiser \
  -p mcp_peekaboo \
  -p mcp_tutorial
