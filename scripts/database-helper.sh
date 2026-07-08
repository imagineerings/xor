#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

usage() {
  cat >&2 <<'USAGE'
usage: scripts/database-helper.sh <command> [--yes]

commands:
  list-test        List sim-test-* PostgreSQL databases.
  drop-test        Drop sim-test-* PostgreSQL databases. Requires --yes.
  reset-dev        Drop and recreate local development databases. Requires --yes.
USAGE
}

require_confirmation() {
  if [[ "${1:-}" != "--yes" ]]; then
    echo "Refusing destructive database action without --yes." >&2
    exit 64
  fi
}

command="${1:-}"
case "$command" in
  list-test)
    psql --tuples-only --command "
      SELECT datname
      FROM pg_database
      WHERE datistemplate = false
        AND datname LIKE 'sim-test-%'
      ORDER BY datname
    "
    ;;
  drop-test)
    require_confirmation "${2:-}"
    "$repo_root/script/drop-test-dbs"
    ;;
  reset-dev)
    require_confirmation "${2:-}"
    "$repo_root/script/reset_db"
    ;;
  -h|--help|help|"")
    usage
    ;;
  *)
    echo "unknown database helper command: $command" >&2
    usage
    exit 64
    ;;
esac
