#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

cargo test -p session import --lib
cargo test -p nostr_sharing --lib
node scripts/verify-misc-services.js
