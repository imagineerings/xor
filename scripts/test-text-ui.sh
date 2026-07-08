#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

cargo test -p cli interactive
cargo test -p cli configure
cargo test -p cli extension
cargo test -p cli onboarding
