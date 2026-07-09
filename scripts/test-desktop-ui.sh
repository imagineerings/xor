#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

cargo test -p agent_ui agent_connection_store --lib
cargo test -p agent_ui recipe_browser --lib
cargo test -p agent_ui diagnostics --lib
cargo test -p settings_ui scheduling_settings --lib
cargo test -p agent shared_session --lib
cargo test -p sim shared_session
cargo test -p i18n --lib
cargo test -p auto_update_ui status_details --lib
