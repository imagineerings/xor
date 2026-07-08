#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/../.."

if command -v typos >/dev/null 2>&1; then
  typos docs
else
  echo "typos is not installed; skipping docs spell check" >&2
fi
