#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

cargo test -p security --lib
cargo test -p permission --lib
cargo test -p agent security_integration --lib
cargo test -p agent permission_integration --lib
