# Collaboration schema migrations

This directory packages the canonical Collab SQL migration pairs as one
release artifact. `manifest.json` fixes their order, forward and backward
SHA-256 values, current schema ceiling, and the service-activation rollback
boundary. The manifest must cover every `*.up.sql` and `*.down.sql` file under
`crates/collab/migrations`; `check.py` rejects omissions or changed bytes.

The runner uses `DATABASE_URL` without placing it in process arguments. It
accepts PostgreSQL URLs with explicitly allowlisted libpq connection options,
creates an operator-owned history and control record, revokes their public
privileges, takes a session advisory lock, and applies each SQL file and its
history row in one transaction. A terminated process can resume from the
committed prefix. A SQL error or source/history checksum mismatch records a
closed halt reason and blocks further migration until operators restore or
repair the database under an approved recovery procedure.

Forward application defaults to the manifest ceiling:

```sh
DATABASE_URL=postgres://... ./deploy/collaboration/migrations/migrate.py up
```

`seal --expected-version VERSION` advances the monotonic rollback floor after
service activation. Before that boundary, `down --target-version VERSION`
applies checked backward files in reverse order. It can never cross the stored
floor. Normal service-image rollback does not run this command; the Helm chart
selects a schema-compatible prior binary while preserving data.

The Kubernetes hook uses the separately built migration image and a DDL-only
Secret. The chart's required schema version must equal the packaged manifest
ceiling before any database connection. Build it from the repository root;
release automation must pin both its base image and published result by digest:

```sh
docker build -f deploy/collaboration/migrations/Dockerfile .
```

`tests/smoke.sh` builds the artifact and uses a disposable PostgreSQL instance
to prove staged apply/resume, idempotence, backward execution before the
boundary, refusal after sealing, and durable checksum-drift halt behavior.

Buzz's `1321_backfill_default_community.sql` is deliberately excluded. It is a
one-off source-normalization cutover with no down path; its required pre-run
snapshot remains its rollback authority. Canonical Collab schema application
starts only after the separately owned Buzz import path has verified that
source state.
