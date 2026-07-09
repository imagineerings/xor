# Comfy Compatibility Fixtures

These fixtures exercise Comfy-compatible behavior through native Sim records and
services. They are not approvals for ComfyUI pass-through handlers.

## Fixture Groups

- script examples: `basic_api_prompt.json` covers HTTP prompt submission,
  status reads, WebSocket negotiation, progress, and preview fallback.
- route snapshots: `api_routes.json` covers supported route status and native
  Sim handler ownership.
- node schema snapshots: `core_nodes.json` covers core object-info nodes and
  prompt graph execution against native Sim planning/execution surfaces.
- blueprint manifest: `blueprints_manifest.json` covers imported workflow
  blueprint records and dependency diagnostics.
- provider catalog: `provider_nodes.json` covers native Sim provider records,
  provider families, Comfy-compatible node ids, capabilities, and unsupported
  diagnostics without ComfyUI pass-through.
- asset API: covered by native Sim asset API tests and route snapshot entries
  for upload/download/preview compatibility.
- media capability groups: current coverage comes from model execution and
  blueprint fixtures; detailed node capability snapshots are owned by
  comfy-media-node-pipelines task 1.

Every fixture that represents implemented behavior must indicate native Sim
records and must not mark itself as ComfyUI pass-through.

Implemented fixtures also record `captured_at`, a `projects/comfy` source path
or root, and an implementation owner under `.agents/specs/godot-migration/`.
Fixtures that stand in for provider API keys, model downloads, media codecs, or
runtime execution must use mock or metadata-only safety records unless a
dependency review explicitly approves heavier behavior.
