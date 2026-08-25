# Collaboration Helm chart

This chart owns the Kubernetes shape of the canonical Zed collaboration
service. It is disabled by default. A production release must supply an
immutable image digest, existing runtime and migration Secrets, HTTPS service
endpoints, an Ingress or Gateway API attachment, and explicit egress ranges.

The runtime Secret must contain the keys named under `runtimeSecret` in
`values.yaml`. Database, object-store, internal API, Git-hook, push and mesh
credentials are never accepted as chart values. The migration Secret is
separate because it may have DDL privileges that the runtime identity must not
have.

`values-production.yaml` is intentionally unrenderable on its own. The release
system supplies environment-owned values, then runs:

```sh
helm lint deploy/collaboration/charts/collaboration \
  -f deploy/collaboration/charts/collaboration/values-production.yaml
helm template collaboration deploy/collaboration/charts/collaboration \
  -f deploy/collaboration/charts/collaboration/values-production.yaml
```

The pre-install/pre-upgrade hook is the deployment contract for `collab
migrate`. Task 44.4 owns the ordered migration package, checksums,
compatibility ceiling and halt/resume behavior; this chart does not substitute
for those controls.

Rollback overlays `values-rollback.yaml`, selects only a previous immutable
digest, requires that binary to admit the deployed schema floor, and suppresses
the migration hook. It leaves the Git claim and every external authority
unchanged.

Run `tests/render.sh` from the repository root to lint and render the default,
production, Ingress, optional pairing/push/mesh and rollback contracts, plus
their failure cases.
