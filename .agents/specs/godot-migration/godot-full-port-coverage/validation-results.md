# Validation results

## Errors

No validation errors remain.

- The feature-spec validator passed the root pack, all 27 pre-existing child packs, and `godot-full-port-coverage`: **29/29 packs**, **0 errors**.
- `validate_audit.py` passed: **198 capabilities**, **21 domains**, **7 classifications**, **13,979 Godot source paths**, unique IDs, the expanded native-owner/dependency/path/boundary schema, exact evidence paths, exact domain and native-gate trace links, reconciled counts, frozen Zed/Godot source fingerprints, prohibited-delegation scans, and no checked tasks.
- The migration tree contains **344 unchecked tasks** and **0 checked tasks**.

## Warnings

The feature-spec validator emitted **56 repeated-write sequencing warnings**. No warning is an error.

- Twenty warnings come from the root and legacy Comfy packs: seven root shared-write warnings and thirteen Comfy coverage/runtime warnings. Several are truncated-path artifacts such as `crates/world_model` being reported as `crates/world`; real shared targets retain explicit dependency order and require implementation-time serialization.
- Twenty-two warnings come from the updated native owner specs. Their new proof task intentionally depends on the implementation task and reads/writes the same existing owner to add no-Godot validation. These are ordered, but the validator correctly flags the shared target for review.
- Fourteen warnings come from the generated audit pack. Capability tasks are ordered within each domain and also depend on the native gate task; the validator groups common existing-owner paths (and truncates some underscore-containing paths), so implementation-time regrouping and serialization remain necessary.

Before implementation, write ownership should be re-evaluated after DEC-GODOT-001 through DEC-GODOT-010 are resolved. That review may regroup capability tasks, but it must preserve capability IDs, existing-owner reuse, no-Godot proof metadata, and requirement/design/task traceability.

## Commands

```sh
python3 .agents/specs/godot-migration/godot-full-port-coverage/validate_audit.py

for directory in .agents/specs/godot-migration .agents/specs/godot-migration/*; do
  if test -f "$directory/requirements.md"; then
    python3 .agents/skills/feature-spec/scripts/validate_spec.py "$directory"
  fi
done
```
