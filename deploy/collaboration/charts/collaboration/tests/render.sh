#!/usr/bin/env bash
set -euo pipefail

chart=deploy/collaboration/charts/collaboration
default_output=$(mktemp)
active_output=$(mktemp)
production_output=$(mktemp)
rollback_output=$(mktemp)
ingress_output=$(mktemp)
error_output=$(mktemp)
trap 'rm -f "$default_output" "$active_output" "$production_output" "$rollback_output" "$ingress_output" "$error_output"' EXIT

active_args=(
  --set deployment.enabled=true
  --set image.tag=test
  --set runtimeSecret.name=collaboration-runtime
  --set migration.image.tag=test
  --set migration.secretName=collaboration-migration
  --set publicUrl=https://collaboration.example.invalid
  --set objectStore.endpoint=https://objects.example.invalid
  --set 'networkPolicy.postgresEgressCidrs[0]=10.10.0.0/16'
  --set 'networkPolicy.redisEgressCidrs[0]=10.20.0.0/16'
  --set 'networkPolicy.objectStoreEgressCidrs[0]=10.30.0.0/16'
)

production_args=(
  -f "$chart/values-production.yaml"
  --set image.digest=sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
  --set runtimeSecret.name=collaboration-runtime
  --set migration.image.digest=sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc
  --set migration.secretName=collaboration-migration
  --set publicUrl=https://collaboration.example.invalid
  --set objectStore.endpoint=https://objects.example.invalid
  --set autoscaling.enabled=true
  --set autoscaling.minReplicas=3
  --set autoscaling.maxReplicas=10
  --set autoscaling.websocketMetric.enabled=true
  --set podMonitor.enabled=true
  --set networkPolicy.monitoring.enabled=true
  --set 'networkPolicy.monitoring.namespaceSelector.kubernetes\.io/metadata\.name=monitoring'
  --set 'networkPolicy.monitoring.podSelector.app\.kubernetes\.io/name=prometheus'
  --set 'httpRoute.parentRefs[0].name=production-gateway'
  --set 'httpRoute.parentRefs[0].namespace=gateway-system'
  --set 'httpRoute.hostnames[0]=collaboration.example.invalid'
  --set 'networkPolicy.postgresEgressCidrs[0]=10.10.0.0/16'
  --set 'networkPolicy.redisEgressCidrs[0]=10.20.0.0/16'
  --set 'networkPolicy.objectStoreEgressCidrs[0]=10.30.0.0/16'
)

helm lint "$chart" >/dev/null
helm template collaboration "$chart" >"$default_output"
if grep -q '^kind:' "$default_output"; then
  echo "expected the default-disabled chart to render no resources" >&2
  exit 1
fi

helm lint "$chart" "${active_args[@]}" >/dev/null
helm template collaboration "$chart" "${active_args[@]}" >"$active_output"

helm lint "$chart" "${production_args[@]}" >/dev/null
helm template collaboration "$chart" "${production_args[@]}" >"$production_output"

helm template collaboration "$chart" "${active_args[@]}" \
  --set ingress.enabled=true \
  --set ingress.className=nginx \
  --set ingress.hostname=collaboration.example.invalid \
  --set ingress.tlsSecretName=collaboration-tls \
  >"$ingress_output"

helm template collaboration "$chart" "${active_args[@]}" \
  --set push.enabled=true \
  --set push.url=https://push.example.invalid \
  --set pairing.enabled=true \
  --set pairing.url=wss://pair.example.invalid \
  --set relayMesh.enabled=true \
  --set 'relayMesh.peers[0]=wss://mesh-a.example.invalid' \
  --set 'relayMesh.peers[1]=wss://mesh-b.example.invalid' \
  --set 'networkPolicy.optionalHttpsEgressCidrs[0]=10.40.0.0/16' \
  >/dev/null

rollback_args=(
  "${production_args[@]}"
  -f "$chart/values-rollback.yaml"
  --set rollback.targetImageDigest=sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb
  --set-string rollback.maximumSchemaVersion=20260824000500
)
helm lint "$chart" "${rollback_args[@]}" >/dev/null
helm template collaboration "$chart" "${rollback_args[@]}" >"$rollback_output"

ruby - "$active_output" "$production_output" "$rollback_output" "$ingress_output" <<'RUBY'
require "yaml"

active = YAML.load_stream(File.read(ARGV[0])).compact
production = YAML.load_stream(File.read(ARGV[1])).compact
rollback = YAML.load_stream(File.read(ARGV[2])).compact
ingress = YAML.load_stream(File.read(ARGV[3])).compact

deployment = active.find { |resource| resource["kind"] == "Deployment" }
service = active.find { |resource| resource["kind"] == "Service" }
job = active.find { |resource| resource["kind"] == "Job" }
pvc = active.find { |resource| resource["kind"] == "PersistentVolumeClaim" }
policies = active.select { |resource| resource["kind"] == "NetworkPolicy" }
raise "missing active resources" unless deployment && service && job && pvc && policies.length == 2
raise "non-public port exposed" unless service.dig("spec", "ports").map { |port| port["targetPort"] } == ["public"]
raise "Git storage is not ReadWriteMany" unless pvc.dig("spec", "accessModes") == ["ReadWriteMany"]

container = deployment.dig("spec", "template", "spec", "containers", 0)
raise "wrong liveness contract" unless container.dig("livenessProbe", "periodSeconds") == 10 && container.dig("livenessProbe", "timeoutSeconds") == 3
raise "wrong readiness contract" unless container.dig("readinessProbe", "periodSeconds") == 5 && container.dig("readinessProbe", "failureThreshold") == 3
raise "wrong startup budget" unless container.dig("startupProbe", "periodSeconds") * container.dig("startupProbe", "failureThreshold") == 120
raise "runtime can escalate" unless container.dig("securityContext", "allowPrivilegeEscalation") == false && container.dig("securityContext", "capabilities", "drop") == ["ALL"]
raise "unbounded runtime resources" unless container.dig("resources", "limits", "memory") == "2Gi" && container.dig("resources", "limits", "ephemeral-storage") == "1Gi"

environment = container["env"].to_h { |entry| [entry["name"], entry] }
required_environment = %w[
  COLLABORATION_PUBLIC_URL COLLABORATION_DATABASE_URL COLLABORATION_REDIS_URL
  COLLABORATION_OBJECT_ENDPOINT COLLABORATION_OBJECT_REGION COLLABORATION_OBJECT_BUCKET
  COLLABORATION_OBJECT_ACCESS_KEY COLLABORATION_OBJECT_SECRET_KEY
  COLLABORATION_OBJECT_ADDRESSING_STYLE COLLABORATION_GIT_REPOSITORY_PATH
  COLLABORATION_GIT_HOOK_SECRET COLLABORATION_REPLICA_COUNT COLLABORATION_PUSH_ENABLED
  COLLABORATION_PAIRING_ENABLED COLLABORATION_RELAY_MESH_ENABLED
]
raise "canonical configuration is incomplete" unless (required_environment - environment.keys).empty?
raise "database alias drift" unless environment.dig("DATABASE_URL", "valueFrom", "secretKeyRef") == environment.dig("COLLABORATION_DATABASE_URL", "valueFrom", "secretKeyRef")
raise "object key alias drift" unless environment.dig("BLOB_STORE_ACCESS_KEY", "valueFrom", "secretKeyRef") == environment.dig("COLLABORATION_OBJECT_ACCESS_KEY", "valueFrom", "secretKeyRef")
raise "object endpoint alias drift" unless environment.dig("BLOB_STORE_URL", "value") == environment.dig("COLLABORATION_OBJECT_ENDPOINT", "value")
raise "replica admission drift" unless environment.dig("COLLABORATION_REPLICA_COUNT", "value") == "2"

raise "migration hook missing" unless job.dig("metadata", "annotations", "helm.sh/hook") == "pre-install,pre-upgrade"
raise "migration is not bounded" unless job.dig("spec", "activeDeadlineSeconds") == 300 && job.dig("spec", "backoffLimit") == 2
migration_container = job.dig("spec", "template", "spec", "containers", 0)
raise "migration command drift" unless migration_container["args"] == ["up"]
raise "migration image owner drift" unless migration_container["image"] == "ghcr.io/zed-industries/collaboration-migrations:test"
migration_secret = migration_container.dig("env", 1, "valueFrom", "secretKeyRef", "name")
raise "migration aliases runtime credentials" unless migration_secret == "collaboration-migration"

production_deployment = production.find { |resource| resource["kind"] == "Deployment" }
production_container = production_deployment.dig("spec", "template", "spec", "containers", 0)
raise "production image is mutable" unless production_container["image"].end_with?("@sha256:" + "a" * 64)
production_job = production.find { |resource| resource["kind"] == "Job" }
raise "production migration image is mutable" unless production_job.dig("spec", "template", "spec", "containers", 0, "image").end_with?("@sha256:" + "c" * 64)
raise "autoscaling replica contract drift" unless production_container["env"].find { |entry| entry["name"] == "COLLABORATION_REPLICA_COUNT" }["value"] == "10"
route = production.find { |resource| resource["kind"] == "HTTPRoute" }
raise "production route is unattached" unless route && route.dig("spec", "parentRefs", 0, "name") == "production-gateway"
route_paths = route.dig("spec", "rules", 0, "matches").map { |match| match.dig("path", "value") }
raise "private metrics path exposed" if route_paths.include?("/metrics") || route_paths.include?("/") && route.dig("spec", "rules", 0, "matches", 0, "path", "type") == "PathPrefix"
hpa = production.find { |resource| resource["kind"] == "HorizontalPodAutoscaler" }
raise "autoscaling bounds drift" unless hpa && hpa.dig("spec", "minReplicas") == 3 && hpa.dig("spec", "maxReplicas") == 10 && hpa.dig("spec", "metrics").length == 2
raise "disruption budget missing" unless production.any? { |resource| resource["kind"] == "PodDisruptionBudget" }
raise "monitoring contract missing" unless production.any? { |resource| resource["kind"] == "PodMonitor" }

ingress_resource = ingress.find { |resource| resource["kind"] == "Ingress" }
raise "TLS ingress missing" unless ingress_resource && ingress_resource.dig("spec", "tls", 0, "secretName") == "collaboration-tls"
ingress_paths = ingress_resource.dig("spec", "rules", 0, "http", "paths")
raise "Ingress exposed private metrics" if ingress_paths.any? { |path| path["path"] == "/metrics" || path["path"] == "/" && path["pathType"] == "Prefix" }

rollback_deployment = rollback.find { |resource| resource["kind"] == "Deployment" }
raise "rollback rendered a migration" if rollback.any? { |resource| resource["kind"] == "Job" || resource.dig("metadata", "name")&.end_with?("-migration") }
raise "rollback target not selected" unless rollback_deployment.dig("spec", "template", "spec", "containers", 0, "image").end_with?("@sha256:" + "b" * 64)
raise "rollback annotation missing" unless rollback_deployment.dig("spec", "template", "metadata", "annotations", "collaboration.zed.dev/rollback") == "true"
rollback_pvc = rollback.find { |resource| resource["kind"] == "PersistentVolumeClaim" }
raise "rollback changed Git storage" unless rollback_pvc.dig("spec") == production.find { |resource| resource["kind"] == "PersistentVolumeClaim" }.dig("spec")
RUBY

if helm template collaboration "$chart" "${active_args[@]}" --set runtimeSecret.name= >"$error_output" 2>&1; then
  echo "expected an empty runtime secret name to fail" >&2
  exit 1
fi
grep -q 'runtimeSecret.name is required' "$error_output"

if helm template collaboration "$chart" "${active_args[@]}" --set migration.secretName= >"$error_output" 2>&1; then
  echo "expected an empty migration secret name to fail" >&2
  exit 1
fi
grep -q 'migration.secretName is required' "$error_output"

if helm template collaboration "$chart" "${active_args[@]}" --set migration.secretName=collaboration-runtime >"$error_output" 2>&1; then
  echo "expected runtime and migration credentials to remain separate" >&2
  exit 1
fi
grep -q 'runtime and DDL-capable migration credentials must use distinct secrets' "$error_output"

if helm template collaboration "$chart" "${production_args[@]}" --set image.digest= >"$error_output" 2>&1; then
  echo "expected a production tag without a digest to fail" >&2
  exit 1
fi
grep -q 'production deployment requires image.digest' "$error_output"

if helm template collaboration "$chart" "${production_args[@]}" --set migration.image.digest= >"$error_output" 2>&1; then
  echo "expected a mutable production migration image to fail" >&2
  exit 1
fi
grep -q 'production migration requires migration.image.digest' "$error_output"

if helm template collaboration "$chart" "${production_args[@]}" --set httpRoute.enabled=false >"$error_output" 2>&1; then
  echo "expected a production release without ingress to fail" >&2
  exit 1
fi
grep -q 'production deployment requires ingress.enabled or httpRoute.enabled' "$error_output"

if helm template collaboration "$chart" "${active_args[@]}" --set redis.enabled=false >"$error_output" 2>&1; then
  echo "expected a multi-replica release without Redis to fail" >&2
  exit 1
fi
grep -q 'multiple replicas require redis.enabled=true' "$error_output"

if helm template collaboration "$chart" "${active_args[@]}" --set persistence.git.accessMode=ReadWriteOnce >"$error_output" 2>&1; then
  echo "expected multi-replica ReadWriteOnce Git storage to fail" >&2
  exit 1
fi
grep -q 'multiple replicas with persistent Git storage require ReadWriteMany' "$error_output"

if helm template collaboration "$chart" "${production_args[@]}" -f "$chart/values-rollback.yaml" --set-string rollback.maximumSchemaVersion=20260824000500 >"$error_output" 2>&1; then
  echo "expected a rollback without a target digest to fail" >&2
  exit 1
fi
grep -q 'rollback.targetImageDigest is required' "$error_output"

if helm template collaboration "$chart" "${production_args[@]}" -f "$chart/values-rollback.yaml" --set rollback.targetImageDigest=sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb --set-string rollback.maximumSchemaVersion=20260824000499 >"$error_output" 2>&1; then
  echo "expected an incompatible rollback schema ceiling to fail" >&2
  exit 1
fi
grep -q 'rollback target does not support the deployed schema version' "$error_output"

if helm template collaboration "$chart" "${active_args[@]}" --set unreviewedService.enabled=true >"$error_output" 2>&1; then
  echo "expected an unknown configuration key to fail schema validation" >&2
  exit 1
fi
grep -q "additional properties 'unreviewedService' not allowed" "$error_output"

echo "collaboration Helm contract checks passed"
