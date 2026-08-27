#!/usr/bin/env bash
set -euo pipefail

repository=$(git rev-parse --show-toplevel)
image="zed-collaboration-migrations:task-44-4-${PPID}"
network="zed-collaboration-migrations-${PPID}"
postgres="${network}-postgres"
drift_directory=$(mktemp -d)

cleanup() {
  docker rm --force "$postgres" >/dev/null 2>&1 || true
  docker network rm "$network" >/dev/null 2>&1 || true
  docker image rm "$image" >/dev/null 2>&1 || true
  rm -rf -- "$drift_directory"
}
trap cleanup EXIT

docker network create "$network" >/dev/null
docker run --detach --name "$postgres" --network "$network" \
  --env POSTGRES_PASSWORD=migration-canary \
  --env POSTGRES_DB=collaboration \
  postgres:17-alpine >/dev/null

for attempt in $(seq 1 60); do
  if docker exec "$postgres" pg_isready --username postgres --dbname collaboration >/dev/null 2>&1; then
    break
  fi
  if [[ "$attempt" == "60" ]]; then
    echo "PostgreSQL did not become ready" >&2
    exit 1
  fi
  sleep 1
done

docker build --file deploy/collaboration/migrations/Dockerfile --tag "$image" "$repository" >/dev/null

run_migration() {
  local database="$1"
  shift
  docker run --rm --network "$network" \
    --env "DATABASE_URL=postgres://postgres:migration-canary@${postgres}:5432/${database}" \
    --env COLLABORATION_REQUIRED_SCHEMA_VERSION=20260825000100 \
    "$image" "$@"
}

if docker run --rm \
  --env COLLABORATION_REQUIRED_SCHEMA_VERSION=20260824000400 \
  "$image" validate >/dev/null 2>&1; then
  echo "migration artifact accepted a mismatched required schema version" >&2
  exit 1
fi

run_migration collaboration up --target-version 20260820000500
run_migration collaboration verify | grep -q 'current=20260820000500.*applied=6'
run_migration collaboration up
run_migration collaboration verify | grep -q 'current=20260825000100.*applied=21'
run_migration collaboration up | grep -q 'migration_up_applied=0'

run_migration collaboration down --target-version 20260824000500
table_after_down=$(docker exec "$postgres" psql --username postgres --dbname collaboration --tuples-only --no-align \
  --command "SELECT to_regclass('public.collaboration_workflow_ready_queue_index') IS NULL")
[[ "$table_after_down" == "t" ]]
run_migration collaboration up
run_migration collaboration seal --expected-version 20260825000100
if run_migration collaboration down --target-version 20260824000500 >/dev/null 2>&1; then
  echo "rollback crossed the sealed compatibility boundary" >&2
  exit 1
fi
run_migration collaboration status | grep -q 'rollback_floor=20260825000100'

docker exec "$postgres" createdb --username postgres drift
run_migration drift up --target-version 20260815000100
cp crates/collab/migrations/*.up.sql crates/collab/migrations/*.down.sql "$drift_directory"
printf '\n-- checksum drift canary\n' >>"$drift_directory/20260815000100_collaboration_identity_bindings.up.sql"
if docker run --rm --network "$network" \
  --env "DATABASE_URL=postgres://postgres:migration-canary@${postgres}:5432/drift" \
  --env COLLABORATION_MIGRATION_DIRECTORY=/drift \
  --volume "$drift_directory:/drift:ro" \
  "$image" verify >/dev/null 2>&1; then
  echo "checksum drift did not halt the migration" >&2
  exit 1
fi
run_migration drift status | grep -q 'migration_status=halted.*halt_reason=checksum_drift'
if run_migration drift up >/dev/null 2>&1; then
  echo "a halted database resumed without operator recovery" >&2
  exit 1
fi

echo "collaboration migration smoke passed"
