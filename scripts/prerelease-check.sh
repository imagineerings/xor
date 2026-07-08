#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

mode="${1:-quick}"

case "$mode" in
  quick)
    git diff --check
    node scripts/validate-openapi-schema.js --help >/dev/null
    node scripts/diagnostics-viewer.js --help >/dev/null
    scripts/test-misc-services.sh
    scripts/test-mcp-servers.sh
    scripts/test-observability-analytics.sh
    scripts/test-security-permissions.sh
    ;;
  full)
    scripts/prerelease-check.sh quick
    scripts/test-sub-agent-and-recipe.sh
    scripts/test-compaction.sh
    ./script/clippy
    cargo test --workspace
    ;;
  -h|--help|help)
    echo "usage: scripts/prerelease-check.sh [quick|full]"
    ;;
  *)
    echo "unknown prerelease mode: $mode" >&2
    exit 64
    ;;
esac
