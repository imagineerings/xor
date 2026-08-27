# Implementation Plan: Godot Full Port Coverage

## Approach

Keep the audit baseline reproducible, enforce native Zed ownership, resolve blocking product decisions, then execute one independently reviewable capability task at the existing Zed owner. Task order is catalog order only; implementation may be regrouped into dependency waves after the decisions in `decisions.md` are approved and write conflicts are reviewed. Every task is intentionally unchecked.

## Tasks

- [ ] 1. Maintain the frozen audit baseline and master reconciliation
  - Recompute source revision/content evidence, catalog schema, classifications, native ownership/dependency paths, traceability, counts, overclaims, duplicates, contradictions, and decision registers without promoting plans, wrappers, delegation, or documentation to implementation evidence.
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: none_
  - _Reads: projects/godot/version.py, projects/godot/SConstruct, projects/godot/modules/*/config.py, projects/godot/platform/*/detect.py, Cargo.toml, .agents/specs/godot-migration/**_
  - _Writes: .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv, .agents/specs/godot-migration/godot-full-port-coverage/coverage-summary.md, .agents/specs/godot-migration/godot-full-port-coverage/baseline.md_
  - _Validation: python3 .agents/specs/godot-migration/godot-full-port-coverage/validate_audit.py_

- [ ] 2. Close or verify GODOT-PROJ-001: create a project with name, path, renderer, version-control metadata, and default files
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 1, 200_
  - _Reads: projects/godot/editor/project_manager/project_manager.cpp, crates/project/src/project.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/project/src/project.rs#GODOT-PROJ-001, crates/workspace/src/workspace.rs#GODOT-PROJ-001, crates/recent_projects/src/recent_projects.rs#GODOT-PROJ-001_
  - _Validation: cargo test -p project -p workspace -p recent_projects godot; run the PROJ scenario for create a project with name, path, renderer, version-control metadata, and default files in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 3. Close or verify GODOT-PROJ-002: import an existing project.godot and reject invalid or duplicate roots without losing user data
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 2, 200_
  - _Reads: projects/godot/editor/project_manager/project_manager.cpp, crates/project/src/project.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/project/src/project.rs#GODOT-PROJ-002, crates/workspace/src/workspace.rs#GODOT-PROJ-002, crates/recent_projects/src/recent_projects.rs#GODOT-PROJ-002_
  - _Validation: cargo test -p project -p workspace -p recent_projects godot; run the PROJ scenario for import an existing project.godot and reject invalid or duplicate roots without losing user data in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 4. Close or verify GODOT-PROJ-003: scan, sort, filter, favorite, rename, remove, and reopen projects in the project manager
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 3, 200_
  - _Reads: projects/godot/editor/project_manager/project_manager.cpp, crates/project/src/project.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/project/src/project.rs#GODOT-PROJ-003, crates/workspace/src/workspace.rs#GODOT-PROJ-003, crates/recent_projects/src/recent_projects.rs#GODOT-PROJ-003_
  - _Validation: cargo test -p project -p workspace -p recent_projects godot; run the PROJ scenario for scan, sort, filter, favorite, rename, remove, and reopen projects in the project manager in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 5. Close or verify GODOT-PROJ-004: persist recent projects, favorites, tags, sort mode, and missing-project state
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 4, 200_
  - _Reads: projects/godot/editor/project_manager/project_manager.cpp, crates/project/src/project.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/project/src/project.rs#GODOT-PROJ-004, crates/workspace/src/workspace.rs#GODOT-PROJ-004, crates/recent_projects/src/recent_projects.rs#GODOT-PROJ-004_
  - _Validation: cargo test -p project -p workspace -p recent_projects godot; run the PROJ scenario for persist recent projects, favorites, tags, sort mode, and missing-project state in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 6. Close or verify GODOT-PROJ-005: parse project features, application metadata, main scene, autoloads, input map, and rendering settings
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 5, 200_
  - _Reads: projects/godot/editor/project_manager/project_manager.cpp, crates/project/src/project.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/project/src/project.rs#GODOT-PROJ-005, crates/workspace/src/workspace.rs#GODOT-PROJ-005, crates/recent_projects/src/recent_projects.rs#GODOT-PROJ-005_
  - _Validation: cargo test -p project -p workspace -p recent_projects godot; run the PROJ scenario for parse project features, application metadata, main scene, autoloads, input map, and rendering settings in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 7. Close or verify GODOT-PROJ-006: start the editor, project manager, or game based on project discovery and command-line mode
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 6, 200_
  - _Reads: projects/godot/editor/project_manager/project_manager.cpp, crates/project/src/project.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/project/src/project.rs#GODOT-PROJ-006, crates/workspace/src/workspace.rs#GODOT-PROJ-006, crates/recent_projects/src/recent_projects.rs#GODOT-PROJ-006_
  - _Validation: cargo test -p project -p workspace -p recent_projects godot; run the PROJ scenario for start the editor, project manager, or game based on project discovery and command-line mode in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 8. Close or verify GODOT-PROJ-007: detect incompatible engine versions and offer project conversion or manager-assisted upgrade
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 7, 200_
  - _Reads: projects/godot/editor/project_manager/project_manager.cpp, crates/project/src/project.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/project/src/project.rs#GODOT-PROJ-007, crates/workspace/src/workspace.rs#GODOT-PROJ-007, crates/recent_projects/src/recent_projects.rs#GODOT-PROJ-007_
  - _Validation: cargo test -p project -p workspace -p recent_projects godot; run the PROJ scenario for detect incompatible engine versions and offer project conversion or manager-assisted upgrade in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 9. Close or verify GODOT-PROJ-008: open in safe mode after editor/plugin failure and recover unsaved scene state
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 8, 200_
  - _Reads: projects/godot/editor/project_manager/project_manager.cpp, crates/project/src/project.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/project/src/project.rs#GODOT-PROJ-008, crates/workspace/src/workspace.rs#GODOT-PROJ-008, crates/recent_projects/src/recent_projects.rs#GODOT-PROJ-008_
  - _Validation: cargo test -p project -p workspace -p recent_projects godot; run the PROJ scenario for open in safe mode after editor/plugin failure and recover unsaved scene state in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 10. Close or verify GODOT-PROJ-009: install and instantiate project templates while surfacing download and extraction failures
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 9, 200_
  - _Reads: projects/godot/editor/project_manager/project_manager.cpp, crates/project/src/project.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/project/src/project.rs#GODOT-PROJ-009, crates/workspace/src/workspace.rs#GODOT-PROJ-009, crates/recent_projects/src/recent_projects.rs#GODOT-PROJ-009_
  - _Validation: cargo test -p project -p workspace -p recent_projects godot; run the PROJ scenario for install and instantiate project templates while surfacing download and extraction failures in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 11. Close or verify GODOT-PROJ-010: use per-project .godot data and cache roots without treating generated metadata as source
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 10, 200_
  - _Reads: projects/godot/editor/project_manager/project_manager.cpp, crates/project/src/project.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/project/src/project.rs#GODOT-PROJ-010, crates/workspace/src/workspace.rs#GODOT-PROJ-010, crates/recent_projects/src/recent_projects.rs#GODOT-PROJ-010_
  - _Validation: cargo test -p project -p workspace -p recent_projects godot; run the PROJ scenario for use per-project .godot data and cache roots without treating generated metadata as source in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 12. Close or verify GODOT-PROJ-011: apply project settings overrides and feature-tag-specific overrides with deterministic precedence
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 11, 200_
  - _Reads: projects/godot/editor/project_manager/project_manager.cpp, crates/project/src/project.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/project/src/project.rs#GODOT-PROJ-011, crates/workspace/src/workspace.rs#GODOT-PROJ-011, crates/recent_projects/src/recent_projects.rs#GODOT-PROJ-011_
  - _Validation: cargo test -p project -p workspace -p recent_projects godot; run the PROJ scenario for apply project settings overrides and feature-tag-specific overrides with deterministic precedence in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 13. Close or verify GODOT-SCENE-001: create, parent, reorder, name, own, group, and free nodes while preserving scene-tree invariants
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 1, 200_
  - _Reads: projects/godot/scene/main/node.cpp, crates/project/src/project.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/project/src/project.rs#GODOT-SCENE-001, crates/worktree/src/worktree.rs#GODOT-SCENE-001, crates/language/src/language_registry.rs#GODOT-SCENE-001_
  - _Validation: cargo test -p project -p worktree -p language godot_scene; run the SCENE scenario for create, parent, reorder, name, own, group, and free nodes while preserving scene-tree invariants in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 14. Close or verify GODOT-SCENE-002: deliver enter-tree, ready, process, physics-process, pause, exit-tree, and deletion lifecycle notifications
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 13, 200_
  - _Reads: projects/godot/scene/main/node.cpp, crates/project/src/project.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/project/src/project.rs#GODOT-SCENE-002, crates/worktree/src/worktree.rs#GODOT-SCENE-002, crates/language/src/language_registry.rs#GODOT-SCENE-002_
  - _Validation: cargo test -p project -p worktree -p language godot_scene; run the SCENE scenario for deliver enter-tree, ready, process, physics-process, pause, exit-tree, and deletion lifecycle notifications in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 15. Close or verify GODOT-SCENE-003: connect, persist, emit, disconnect, and inspect typed and deferred signals
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 14, 200_
  - _Reads: projects/godot/scene/main/node.cpp, crates/project/src/project.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/project/src/project.rs#GODOT-SCENE-003, crates/worktree/src/worktree.rs#GODOT-SCENE-003, crates/language/src/language_registry.rs#GODOT-SCENE-003_
  - _Validation: cargo test -p project -p worktree -p language godot_scene; run the SCENE scenario for connect, persist, emit, disconnect, and inspect typed and deferred signals in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 16. Close or verify GODOT-SCENE-004: pack, instantiate, inherit, edit, save, reload, and revert scenes with editable children and ownership
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 15, 200_
  - _Reads: projects/godot/scene/main/node.cpp, crates/project/src/project.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/project/src/project.rs#GODOT-SCENE-004, crates/worktree/src/worktree.rs#GODOT-SCENE-004, crates/language/src/language_registry.rs#GODOT-SCENE-004_
  - _Validation: cargo test -p project -p worktree -p language godot_scene; run the SCENE scenario for pack, instantiate, inherit, edit, save, reload, and revert scenes with editable children and ownership in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 17. Close or verify GODOT-SCENE-005: load, preload, cache, duplicate, localize, reference-count, and release resources
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 16, 200_
  - _Reads: projects/godot/scene/main/node.cpp, crates/project/src/project.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/project/src/project.rs#GODOT-SCENE-005, crates/worktree/src/worktree.rs#GODOT-SCENE-005, crates/language/src/language_registry.rs#GODOT-SCENE-005_
  - _Validation: cargo test -p project -p worktree -p language godot_scene; run the SCENE scenario for load, preload, cache, duplicate, localize, reference-count, and release resources in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 18. Close or verify GODOT-SCENE-006: round-trip .tscn and .tres values, subresources, ext_resources, scripts, and connection records
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 17, 200_
  - _Reads: projects/godot/scene/main/node.cpp, crates/project/src/project.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/project/src/project.rs#GODOT-SCENE-006, crates/worktree/src/worktree.rs#GODOT-SCENE-006, crates/language/src/language_registry.rs#GODOT-SCENE-006_
  - _Validation: cargo test -p project -p worktree -p language godot_scene; run the SCENE scenario for round-trip .tscn and .tres values, subresources, ext_resources, scripts, and connection records in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 19. Close or verify GODOT-SCENE-007: round-trip binary .scn and .res resources with version and endianness compatibility
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 18, 200_
  - _Reads: projects/godot/scene/main/node.cpp, crates/project/src/project.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/project/src/project.rs#GODOT-SCENE-007, crates/worktree/src/worktree.rs#GODOT-SCENE-007, crates/language/src/language_registry.rs#GODOT-SCENE-007_
  - _Validation: cargo test -p project -p worktree -p language godot_scene; run the SCENE scenario for round-trip binary .scn and .res resources with version and endianness compatibility in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 20. Close or verify GODOT-SCENE-008: assign stable resource UIDs and repair moved dependency paths without corrupting references
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 19, 200_
  - _Reads: projects/godot/scene/main/node.cpp, crates/project/src/project.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/project/src/project.rs#GODOT-SCENE-008, crates/worktree/src/worktree.rs#GODOT-SCENE-008, crates/language/src/language_registry.rs#GODOT-SCENE-008_
  - _Validation: cargo test -p project -p worktree -p language godot_scene; run the SCENE scenario for assign stable resource UIDs and repair moved dependency paths without corrupting references in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 21. Close or verify GODOT-SCENE-009: enumerate dependencies and surface missing, cyclic, corrupt, or type-mismatched resources
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 20, 200_
  - _Reads: projects/godot/scene/main/node.cpp, crates/project/src/project.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/project/src/project.rs#GODOT-SCENE-009, crates/worktree/src/worktree.rs#GODOT-SCENE-009, crates/language/src/language_registry.rs#GODOT-SCENE-009_
  - _Validation: cargo test -p project -p worktree -p language godot_scene; run the SCENE scenario for enumerate dependencies and surface missing, cyclic, corrupt, or type-mismatched resources in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 22. Close or verify GODOT-SCENE-010: serialize Variant values, exported properties, dictionaries, arrays, typed containers, and object references
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 21, 200_
  - _Reads: projects/godot/scene/main/node.cpp, crates/project/src/project.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/project/src/project.rs#GODOT-SCENE-010, crates/worktree/src/worktree.rs#GODOT-SCENE-010, crates/language/src/language_registry.rs#GODOT-SCENE-010_
  - _Validation: cargo test -p project -p worktree -p language godot_scene; run the SCENE scenario for serialize Variant values, exported properties, dictionaries, arrays, typed containers, and object references in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 23. Close or verify GODOT-SCENE-011: apply autoload singletons and change, reload, and quit the active scene predictably
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 22, 200_
  - _Reads: projects/godot/scene/main/node.cpp, crates/project/src/project.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/project/src/project.rs#GODOT-SCENE-011, crates/worktree/src/worktree.rs#GODOT-SCENE-011, crates/language/src/language_registry.rs#GODOT-SCENE-011_
  - _Validation: cargo test -p project -p worktree -p language godot_scene; run the SCENE scenario for apply autoload singletons and change, reload, and quit the active scene predictably in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 24. Close or verify GODOT-SCENE-012: preserve unknown or newer-format data sufficiently for non-destructive migration
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 23, 200_
  - _Reads: projects/godot/scene/main/node.cpp, crates/project/src/project.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/project/src/project.rs#GODOT-SCENE-012, crates/worktree/src/worktree.rs#GODOT-SCENE-012, crates/language/src/language_registry.rs#GODOT-SCENE-012_
  - _Validation: cargo test -p project -p worktree -p language godot_scene; run the SCENE scenario for preserve unknown or newer-format data sufficiently for non-destructive migration in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 25. Close or verify GODOT-EDITOR-001: restore open scenes, selected objects, bottom panels, docks, and workspace layout per project
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 1, 200_
  - _Reads: projects/godot/editor/editor_node.cpp, crates/workspace/src/workspace.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/workspace/src/workspace.rs#GODOT-EDITOR-001, crates/project_panel/src/project_panel.rs#GODOT-EDITOR-001, crates/inspector_ui/src/inspector_ui.rs#GODOT-EDITOR-001_
  - _Validation: cargo test -p workspace -p project_panel -p inspector_ui godot; run the EDITOR scenario for restore open scenes, selected objects, bottom panels, docks, and workspace layout per project in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 26. Close or verify GODOT-EDITOR-002: browse and manipulate the scene tree with create, rename, reparent, group, visibility, and ownership operations
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 25, 200_
  - _Reads: projects/godot/editor/editor_node.cpp, crates/workspace/src/workspace.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/workspace/src/workspace.rs#GODOT-EDITOR-002, crates/project_panel/src/project_panel.rs#GODOT-EDITOR-002, crates/inspector_ui/src/inspector_ui.rs#GODOT-EDITOR-002_
  - _Validation: cargo test -p workspace -p project_panel -p inspector_ui godot; run the EDITOR scenario for browse and manipulate the scene tree with create, rename, reparent, group, visibility, and ownership operations in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 27. Close or verify GODOT-EDITOR-003: browse project files with type filters, favorites, move/rename dependency repair, and reimport state
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 26, 200_
  - _Reads: projects/godot/editor/editor_node.cpp, crates/workspace/src/workspace.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/workspace/src/workspace.rs#GODOT-EDITOR-003, crates/project_panel/src/project_panel.rs#GODOT-EDITOR-003, crates/inspector_ui/src/inspector_ui.rs#GODOT-EDITOR-003_
  - _Validation: cargo test -p workspace -p project_panel -p inspector_ui godot; run the EDITOR scenario for browse project files with type filters, favorites, move/rename dependency repair, and reimport state in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 28. Close or verify GODOT-EDITOR-004: inspect and edit grouped, typed, ranged, resource, node-path, and script-exposed properties
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 27, 200_
  - _Reads: projects/godot/editor/editor_node.cpp, crates/workspace/src/workspace.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/workspace/src/workspace.rs#GODOT-EDITOR-004, crates/project_panel/src/project_panel.rs#GODOT-EDITOR-004, crates/inspector_ui/src/inspector_ui.rs#GODOT-EDITOR-004_
  - _Validation: cargo test -p workspace -p project_panel -p inspector_ui godot; run the EDITOR scenario for inspect and edit grouped, typed, ranged, resource, node-path, and script-exposed properties in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 29. Close or verify GODOT-EDITOR-005: edit scenes through dedicated 2D, 3D, script, asset-library, and game workspaces
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 28, 200_
  - _Reads: projects/godot/editor/editor_node.cpp, crates/workspace/src/workspace.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/workspace/src/workspace.rs#GODOT-EDITOR-005, crates/project_panel/src/project_panel.rs#GODOT-EDITOR-005, crates/inspector_ui/src/inspector_ui.rs#GODOT-EDITOR-005_
  - _Validation: cargo test -p workspace -p project_panel -p inspector_ui godot; run the EDITOR scenario for edit scenes through dedicated 2D, 3D, script, asset-library, and game workspaces in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 30. Close or verify GODOT-EDITOR-006: provide searchable menus and command palette actions with context-sensitive enablement
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 29, 200_
  - _Reads: projects/godot/editor/editor_node.cpp, crates/workspace/src/workspace.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/workspace/src/workspace.rs#GODOT-EDITOR-006, crates/project_panel/src/project_panel.rs#GODOT-EDITOR-006, crates/inspector_ui/src/inspector_ui.rs#GODOT-EDITOR-006_
  - _Validation: cargo test -p workspace -p project_panel -p inspector_ui godot; run the EDITOR scenario for provide searchable menus and command palette actions with context-sensitive enablement in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 31. Close or verify GODOT-EDITOR-007: configure and resolve user, project, feature-tag, and platform-specific editor settings
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 30, 200_
  - _Reads: projects/godot/editor/editor_node.cpp, crates/workspace/src/workspace.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/workspace/src/workspace.rs#GODOT-EDITOR-007, crates/project_panel/src/project_panel.rs#GODOT-EDITOR-007, crates/inspector_ui/src/inspector_ui.rs#GODOT-EDITOR-007_
  - _Validation: cargo test -p workspace -p project_panel -p inspector_ui godot; run the EDITOR scenario for configure and resolve user, project, feature-tag, and platform-specific editor settings in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 32. Close or verify GODOT-EDITOR-008: edit shortcuts, chords, physical keys, and platform variants with conflict diagnostics
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 31, 200_
  - _Reads: projects/godot/editor/editor_node.cpp, crates/workspace/src/workspace.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/workspace/src/workspace.rs#GODOT-EDITOR-008, crates/project_panel/src/project_panel.rs#GODOT-EDITOR-008, crates/inspector_ui/src/inspector_ui.rs#GODOT-EDITOR-008_
  - _Validation: cargo test -p workspace -p project_panel -p inspector_ui godot; run the EDITOR scenario for edit shortcuts, chords, physical keys, and platform variants with conflict diagnostics in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 33. Close or verify GODOT-EDITOR-009: perform undo, redo, history navigation, inspector pinning, and multi-object edits without reentrant updates
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 32, 200_
  - _Reads: projects/godot/editor/editor_node.cpp, crates/workspace/src/workspace.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/workspace/src/workspace.rs#GODOT-EDITOR-009, crates/project_panel/src/project_panel.rs#GODOT-EDITOR-009, crates/inspector_ui/src/inspector_ui.rs#GODOT-EDITOR-009_
  - _Validation: cargo test -p workspace -p project_panel -p inspector_ui godot; run the EDITOR scenario for perform undo, redo, history navigation, inspector pinning, and multi-object edits without reentrant updates in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 34. Close or verify GODOT-EDITOR-010: save, save-as, save-all, autosave, recover, and warn before closing unsaved resources
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 33, 200_
  - _Reads: projects/godot/editor/editor_node.cpp, crates/workspace/src/workspace.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/workspace/src/workspace.rs#GODOT-EDITOR-010, crates/project_panel/src/project_panel.rs#GODOT-EDITOR-010, crates/inspector_ui/src/inspector_ui.rs#GODOT-EDITOR-010_
  - _Validation: cargo test -p workspace -p project_panel -p inspector_ui godot; run the EDITOR scenario for save, save-as, save-all, autosave, recover, and warn before closing unsaved resources in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 35. Close or verify GODOT-EDITOR-011: run and stop the main scene, current scene, selected scene, and custom runnable with embedded game controls
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 34, 200_
  - _Reads: projects/godot/editor/editor_node.cpp, crates/workspace/src/workspace.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/workspace/src/workspace.rs#GODOT-EDITOR-011, crates/project_panel/src/project_panel.rs#GODOT-EDITOR-011, crates/inspector_ui/src/inspector_ui.rs#GODOT-EDITOR-011_
  - _Validation: cargo test -p workspace -p project_panel -p inspector_ui godot; run the EDITOR scenario for run and stop the main scene, current scene, selected scene, and custom runnable with embedded game controls in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 36. Close or verify GODOT-EDITOR-012: expose output, debugger, profiler, audio, animation, shader, navigation, and import bottom panels
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 35, 200_
  - _Reads: projects/godot/editor/editor_node.cpp, crates/workspace/src/workspace.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/workspace/src/workspace.rs#GODOT-EDITOR-012, crates/project_panel/src/project_panel.rs#GODOT-EDITOR-012, crates/inspector_ui/src/inspector_ui.rs#GODOT-EDITOR-012_
  - _Validation: cargo test -p workspace -p project_panel -p inspector_ui godot; run the EDITOR scenario for expose output, debugger, profiler, audio, animation, shader, navigation, and import bottom panels in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 37. Close or verify GODOT-EDITOR-013: support distraction-free, multi-window, presentation, and embedded-play layout modes
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 36, 200_
  - _Reads: projects/godot/editor/editor_node.cpp, crates/workspace/src/workspace.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/workspace/src/workspace.rs#GODOT-EDITOR-013, crates/project_panel/src/project_panel.rs#GODOT-EDITOR-013, crates/inspector_ui/src/inspector_ui.rs#GODOT-EDITOR-013_
  - _Validation: cargo test -p workspace -p project_panel -p inspector_ui godot; run the EDITOR scenario for support distraction-free, multi-window, presentation, and embedded-play layout modes in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 38. Close or verify GODOT-EDITOR-014: search help and class reference by class, method, property, signal, constant, and theme item
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 37, 200_
  - _Reads: projects/godot/editor/editor_node.cpp, crates/workspace/src/workspace.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/workspace/src/workspace.rs#GODOT-EDITOR-014, crates/project_panel/src/project_panel.rs#GODOT-EDITOR-014, crates/inspector_ui/src/inspector_ui.rs#GODOT-EDITOR-014_
  - _Validation: cargo test -p workspace -p project_panel -p inspector_ui godot; run the EDITOR scenario for search help and class reference by class, method, property, signal, constant, and theme item in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 39. Close or verify GODOT-R2D-001: compose CanvasItem and Node2D transforms, visibility, modulation, clipping, z-order, y-sort, and draw commands
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 5.1, 5.2, 5.3, 5.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 1, 200_
  - _Reads: projects/godot/scene/main/canvas_item.cpp, crates/gpui/src/element.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/gpui/src/element.rs#GODOT-R2D-001, crates/gpui_wgpu/src/wgpu_renderer.rs#GODOT-R2D-001, crates/image_viewer/src/image_viewer.rs#GODOT-R2D-001_
  - _Validation: cargo test -p gpui -p gpui_wgpu -p image_viewer godot_canvas; run the R2D scenario for compose CanvasItem and Node2D transforms, visibility, modulation, clipping, z-order, y-sort, and draw commands in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 40. Close or verify GODOT-R2D-002: render sprites, regions, nine-patches, polygons, lines, text, and texture rectangles with filtering and repeat modes
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 5.1, 5.2, 5.3, 5.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 39, 200_
  - _Reads: projects/godot/scene/main/canvas_item.cpp, crates/gpui/src/element.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/gpui/src/element.rs#GODOT-R2D-002, crates/gpui_wgpu/src/wgpu_renderer.rs#GODOT-R2D-002, crates/image_viewer/src/image_viewer.rs#GODOT-R2D-002_
  - _Validation: cargo test -p gpui -p gpui_wgpu -p image_viewer godot_canvas; run the R2D scenario for render sprites, regions, nine-patches, polygons, lines, text, and texture rectangles with filtering and repeat modes in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 41. Close or verify GODOT-R2D-003: author and render tile sets and tile-map layers with terrains, alternatives, patterns, quadrants, and navigation/physics metadata
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 5.1, 5.2, 5.3, 5.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 40, 200_
  - _Reads: projects/godot/scene/main/canvas_item.cpp, crates/gpui/src/element.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/gpui/src/element.rs#GODOT-R2D-003, crates/gpui_wgpu/src/wgpu_renderer.rs#GODOT-R2D-003, crates/image_viewer/src/image_viewer.rs#GODOT-R2D-003_
  - _Validation: cargo test -p gpui -p gpui_wgpu -p image_viewer godot_canvas; run the R2D scenario for author and render tile sets and tile-map layers with terrains, alternatives, patterns, quadrants, and navigation/physics metadata in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 42. Close or verify GODOT-R2D-004: render 2D lights, normal/specular maps, occluders, shadow atlases, masks, and blend modes
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 5.1, 5.2, 5.3, 5.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 41, 200_
  - _Reads: projects/godot/scene/main/canvas_item.cpp, crates/gpui/src/element.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/gpui/src/element.rs#GODOT-R2D-004, crates/gpui_wgpu/src/wgpu_renderer.rs#GODOT-R2D-004, crates/image_viewer/src/image_viewer.rs#GODOT-R2D-004_
  - _Validation: cargo test -p gpui -p gpui_wgpu -p image_viewer godot_canvas; run the R2D scenario for render 2D lights, normal/specular maps, occluders, shadow atlases, masks, and blend modes in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 43. Close or verify GODOT-R2D-005: execute canvas shaders and materials with uniforms, screen textures, time, and instance parameters
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 5.1, 5.2, 5.3, 5.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 42, 200_
  - _Reads: projects/godot/scene/main/canvas_item.cpp, crates/gpui/src/element.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/gpui/src/element.rs#GODOT-R2D-005, crates/gpui_wgpu/src/wgpu_renderer.rs#GODOT-R2D-005, crates/image_viewer/src/image_viewer.rs#GODOT-R2D-005_
  - _Validation: cargo test -p gpui -p gpui_wgpu -p image_viewer godot_canvas; run the R2D scenario for execute canvas shaders and materials with uniforms, screen textures, time, and instance parameters in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 44. Close or verify GODOT-R2D-006: deform 2D skeletons, bones, polygons, and particles and preview the result in the editor
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 5.1, 5.2, 5.3, 5.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 43, 200_
  - _Reads: projects/godot/scene/main/canvas_item.cpp, crates/gpui/src/element.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/gpui/src/element.rs#GODOT-R2D-006, crates/gpui_wgpu/src/wgpu_renderer.rs#GODOT-R2D-006, crates/image_viewer/src/image_viewer.rs#GODOT-R2D-006_
  - _Validation: cargo test -p gpui -p gpui_wgpu -p image_viewer godot_canvas; run the R2D scenario for deform 2D skeletons, bones, polygons, and particles and preview the result in the editor in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 45. Close or verify GODOT-R2D-007: cull and batch canvas items while preserving draw order and viewport isolation
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 5.1, 5.2, 5.3, 5.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 44, 200_
  - _Reads: projects/godot/scene/main/canvas_item.cpp, crates/gpui/src/element.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/gpui/src/element.rs#GODOT-R2D-007, crates/gpui_wgpu/src/wgpu_renderer.rs#GODOT-R2D-007, crates/image_viewer/src/image_viewer.rs#GODOT-R2D-007_
  - _Validation: cargo test -p gpui -p gpui_wgpu -p image_viewer godot_canvas; run the R2D scenario for cull and batch canvas items while preserving draw order and viewport isolation in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 46. Close or verify GODOT-R2D-008: render SubViewport output to textures and embed or capture the result
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 5.1, 5.2, 5.3, 5.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 45, 200_
  - _Reads: projects/godot/scene/main/canvas_item.cpp, crates/gpui/src/element.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/gpui/src/element.rs#GODOT-R2D-008, crates/gpui_wgpu/src/wgpu_renderer.rs#GODOT-R2D-008, crates/image_viewer/src/image_viewer.rs#GODOT-R2D-008_
  - _Validation: cargo test -p gpui -p gpui_wgpu -p image_viewer godot_canvas; run the R2D scenario for render SubViewport output to textures and embed or capture the result in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 47. Close or verify GODOT-R2D-009: preview common Godot image and texture assets without executing the Godot renderer
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 5.1, 5.2, 5.3, 5.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 46, 200_
  - _Reads: projects/godot/scene/main/canvas_item.cpp, crates/gpui/src/element.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/gpui/src/element.rs#GODOT-R2D-009, crates/gpui_wgpu/src/wgpu_renderer.rs#GODOT-R2D-009, crates/image_viewer/src/image_viewer.rs#GODOT-R2D-009_
  - _Validation: cargo test -p gpui -p gpui_wgpu -p image_viewer godot_canvas; run the R2D scenario for preview common Godot image and texture assets without executing the Godot renderer in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 48. Close or verify GODOT-R3D-001: compose Node3D transforms, visibility, layers, top-level state, and camera projections
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 6.1, 6.2, 6.3, 6.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 1, 200_
  - _Reads: projects/godot/scene/3d/node_3d.cpp, crates/gpui_wgpu/src/wgpu_renderer.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/gpui_wgpu/src/wgpu_renderer.rs#GODOT-R3D-001, crates/component_preview/src/component_preview.rs#GODOT-R3D-001, crates/image_viewer/src/image_viewer.rs#GODOT-R3D-001_
  - _Validation: cargo test -p gpui_wgpu -p component_preview godot_3d; run the R3D scenario for compose Node3D transforms, visibility, layers, top-level state, and camera projections in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 49. Close or verify GODOT-R3D-002: render meshes, surfaces, blend shapes, skeleton skinning, MultiMesh instances, and material overrides
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 6.1, 6.2, 6.3, 6.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 48, 200_
  - _Reads: projects/godot/scene/3d/node_3d.cpp, crates/gpui_wgpu/src/wgpu_renderer.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/gpui_wgpu/src/wgpu_renderer.rs#GODOT-R3D-002, crates/component_preview/src/component_preview.rs#GODOT-R3D-002, crates/image_viewer/src/image_viewer.rs#GODOT-R3D-002_
  - _Validation: cargo test -p gpui_wgpu -p component_preview godot_3d; run the R3D scenario for render meshes, surfaces, blend shapes, skeleton skinning, MultiMesh instances, and material overrides in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 50. Close or verify GODOT-R3D-003: render standard, ORM, shader, particle, fog, sky, and post-process materials with platform fallbacks
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 6.1, 6.2, 6.3, 6.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 49, 200_
  - _Reads: projects/godot/scene/3d/node_3d.cpp, crates/gpui_wgpu/src/wgpu_renderer.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/gpui_wgpu/src/wgpu_renderer.rs#GODOT-R3D-003, crates/component_preview/src/component_preview.rs#GODOT-R3D-003, crates/image_viewer/src/image_viewer.rs#GODOT-R3D-003_
  - _Validation: cargo test -p gpui_wgpu -p component_preview godot_3d; run the R3D scenario for render standard, ORM, shader, particle, fog, sky, and post-process materials with platform fallbacks in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 51. Close or verify GODOT-R3D-004: render directional, omni, and spot lights with shadows, cookies, distance fade, and culling masks
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 6.1, 6.2, 6.3, 6.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 50, 200_
  - _Reads: projects/godot/scene/3d/node_3d.cpp, crates/gpui_wgpu/src/wgpu_renderer.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/gpui_wgpu/src/wgpu_renderer.rs#GODOT-R3D-004, crates/component_preview/src/component_preview.rs#GODOT-R3D-004, crates/image_viewer/src/image_viewer.rs#GODOT-R3D-004_
  - _Validation: cargo test -p gpui_wgpu -p component_preview godot_3d; run the R3D scenario for render directional, omni, and spot lights with shadows, cookies, distance fade, and culling masks in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 52. Close or verify GODOT-R3D-005: apply environments, sky, fog, exposure, tone mapping, glow, SSAO, SSIL, SSR, DOF, and color adjustment
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 6.1, 6.2, 6.3, 6.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 51, 200_
  - _Reads: projects/godot/scene/3d/node_3d.cpp, crates/gpui_wgpu/src/wgpu_renderer.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/gpui_wgpu/src/wgpu_renderer.rs#GODOT-R3D-005, crates/component_preview/src/component_preview.rs#GODOT-R3D-005, crates/image_viewer/src/image_viewer.rs#GODOT-R3D-005_
  - _Validation: cargo test -p gpui_wgpu -p component_preview godot_3d; run the R3D scenario for apply environments, sky, fog, exposure, tone mapping, glow, SSAO, SSIL, SSR, DOF, and color adjustment in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 53. Close or verify GODOT-R3D-006: select Forward+, Mobile, or Compatibility rendering and report unavailable driver/feature combinations
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 6.1, 6.2, 6.3, 6.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 52, 200_
  - _Reads: projects/godot/scene/3d/node_3d.cpp, crates/gpui_wgpu/src/wgpu_renderer.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/gpui_wgpu/src/wgpu_renderer.rs#GODOT-R3D-006, crates/component_preview/src/component_preview.rs#GODOT-R3D-006, crates/image_viewer/src/image_viewer.rs#GODOT-R3D-006_
  - _Validation: cargo test -p gpui_wgpu -p component_preview godot_3d; run the R3D scenario for select Forward+, Mobile, or Compatibility rendering and report unavailable driver/feature combinations in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 54. Close or verify GODOT-R3D-007: compile and execute spatial, sky, fog, and compute shaders with include and uniform dependency tracking
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 6.1, 6.2, 6.3, 6.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 53, 200_
  - _Reads: projects/godot/scene/3d/node_3d.cpp, crates/gpui_wgpu/src/wgpu_renderer.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/gpui_wgpu/src/wgpu_renderer.rs#GODOT-R3D-007, crates/component_preview/src/component_preview.rs#GODOT-R3D-007, crates/image_viewer/src/image_viewer.rs#GODOT-R3D-007_
  - _Validation: cargo test -p gpui_wgpu -p component_preview godot_3d; run the R3D scenario for compile and execute spatial, sky, fog, and compute shaders with include and uniform dependency tracking in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 55. Close or verify GODOT-R3D-008: perform visibility range, frustum, occlusion, LOD, portal-like room, and instance culling
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 6.1, 6.2, 6.3, 6.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 54, 200_
  - _Reads: projects/godot/scene/3d/node_3d.cpp, crates/gpui_wgpu/src/wgpu_renderer.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/gpui_wgpu/src/wgpu_renderer.rs#GODOT-R3D-008, crates/component_preview/src/component_preview.rs#GODOT-R3D-008, crates/image_viewer/src/image_viewer.rs#GODOT-R3D-008_
  - _Validation: cargo test -p gpui_wgpu -p component_preview godot_3d; run the R3D scenario for perform visibility range, frustum, occlusion, LOD, portal-like room, and instance culling in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 56. Close or verify GODOT-R3D-009: bake and consume lightmaps, probes, voxel GI, SDFGI, reflection probes, and environment captures
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 6.1, 6.2, 6.3, 6.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 55, 200_
  - _Reads: projects/godot/scene/3d/node_3d.cpp, crates/gpui_wgpu/src/wgpu_renderer.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/gpui_wgpu/src/wgpu_renderer.rs#GODOT-R3D-009, crates/component_preview/src/component_preview.rs#GODOT-R3D-009, crates/image_viewer/src/image_viewer.rs#GODOT-R3D-009_
  - _Validation: cargo test -p gpui_wgpu -p component_preview godot_3d; run the R3D scenario for bake and consume lightmaps, probes, voxel GI, SDFGI, reflection probes, and environment captures in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 57. Close or verify GODOT-R3D-010: render nested viewports, camera feeds, render targets, scaling, MSAA, TAA, FSR, and screen capture
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 6.1, 6.2, 6.3, 6.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 56, 200_
  - _Reads: projects/godot/scene/3d/node_3d.cpp, crates/gpui_wgpu/src/wgpu_renderer.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/gpui_wgpu/src/wgpu_renderer.rs#GODOT-R3D-010, crates/component_preview/src/component_preview.rs#GODOT-R3D-010, crates/image_viewer/src/image_viewer.rs#GODOT-R3D-010_
  - _Validation: cargo test -p gpui_wgpu -p component_preview godot_3d; run the R3D scenario for render nested viewports, camera feeds, render targets, scaling, MSAA, TAA, FSR, and screen capture in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 58. Close or verify GODOT-R3D-011: preview imported meshes and materials with orbit, lighting, animation, and failure diagnostics
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 6.1, 6.2, 6.3, 6.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 57, 200_
  - _Reads: projects/godot/scene/3d/node_3d.cpp, crates/gpui_wgpu/src/wgpu_renderer.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/gpui_wgpu/src/wgpu_renderer.rs#GODOT-R3D-011, crates/component_preview/src/component_preview.rs#GODOT-R3D-011, crates/image_viewer/src/image_viewer.rs#GODOT-R3D-011_
  - _Validation: cargo test -p gpui_wgpu -p component_preview godot_3d; run the R3D scenario for preview imported meshes and materials with orbit, lighting, animation, and failure diagnostics in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 59. Close or verify GODOT-UI-001: lay out Controls using anchors, offsets, grow directions, minimum sizes, containers, aspect ratios, and RTL mirroring
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 7.1, 7.2, 7.3, 7.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 1, 200_
  - _Reads: projects/godot/scene/gui/control.cpp, crates/gpui/src/elements/div.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/ui/src/ui.rs#GODOT-UI-001, crates/theme/src/theme.rs#GODOT-UI-001, crates/ui_input/src/ui_input.rs#GODOT-UI-001_
  - _Validation: cargo test -p ui -p theme -p ui_input godot_control; run the UI scenario for lay out Controls using anchors, offsets, grow directions, minimum sizes, containers, aspect ratios, and RTL mirroring in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 60. Close or verify GODOT-UI-002: route mouse, touch, keyboard, controller, shortcut, focus, tooltip, and drag/drop events through Control hierarchy
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 7.1, 7.2, 7.3, 7.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 59, 200_
  - _Reads: projects/godot/scene/gui/control.cpp, crates/gpui/src/elements/div.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/ui/src/ui.rs#GODOT-UI-002, crates/theme/src/theme.rs#GODOT-UI-002, crates/ui_input/src/ui_input.rs#GODOT-UI-002_
  - _Validation: cargo test -p ui -p theme -p ui_input godot_control; run the UI scenario for route mouse, touch, keyboard, controller, shortcut, focus, tooltip, and drag/drop events through Control hierarchy in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 61. Close or verify GODOT-UI-003: provide buttons, ranges, lists, trees, tabs, menus, dialogs, color/file pickers, splitters, and scroll containers
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 7.1, 7.2, 7.3, 7.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 60, 200_
  - _Reads: projects/godot/scene/gui/control.cpp, crates/gpui/src/elements/div.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/ui/src/ui.rs#GODOT-UI-003, crates/theme/src/theme.rs#GODOT-UI-003, crates/ui_input/src/ui_input.rs#GODOT-UI-003_
  - _Validation: cargo test -p ui -p theme -p ui_input godot_control; run the UI scenario for provide buttons, ranges, lists, trees, tabs, menus, dialogs, color/file pickers, splitters, and scroll containers in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 62. Close or verify GODOT-UI-004: edit plain and rich text with selection, undo, syntax, bidi, shaping, images, tables, meta links, and IME
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 7.1, 7.2, 7.3, 7.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 61, 200_
  - _Reads: projects/godot/scene/gui/control.cpp, crates/gpui/src/elements/div.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/ui/src/ui.rs#GODOT-UI-004, crates/theme/src/theme.rs#GODOT-UI-004, crates/ui_input/src/ui_input.rs#GODOT-UI-004_
  - _Validation: cargo test -p ui -p theme -p ui_input godot_control; run the UI scenario for edit plain and rich text with selection, undo, syntax, bidi, shaping, images, tables, meta links, and IME in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 63. Close or verify GODOT-UI-005: resolve theme inheritance, type variations, icons, fonts, sizes, colors, style boxes, and live overrides
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 7.1, 7.2, 7.3, 7.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 62, 200_
  - _Reads: projects/godot/scene/gui/control.cpp, crates/gpui/src/elements/div.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/ui/src/ui.rs#GODOT-UI-005, crates/theme/src/theme.rs#GODOT-UI-005, crates/ui_input/src/ui_input.rs#GODOT-UI-005_
  - _Validation: cargo test -p ui -p theme -p ui_input godot_control; run the UI scenario for resolve theme inheritance, type variations, icons, fonts, sizes, colors, style boxes, and live overrides in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 64. Close or verify GODOT-UI-006: manage popups, modal dialogs, embedded windows, exclusive state, and safe cancellation
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 7.1, 7.2, 7.3, 7.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 63, 200_
  - _Reads: projects/godot/scene/gui/control.cpp, crates/gpui/src/elements/div.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/ui/src/ui.rs#GODOT-UI-006, crates/theme/src/theme.rs#GODOT-UI-006, crates/ui_input/src/ui_input.rs#GODOT-UI-006_
  - _Validation: cargo test -p ui -p theme -p ui_input godot_control; run the UI scenario for manage popups, modal dialogs, embedded windows, exclusive state, and safe cancellation in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 65. Close or verify GODOT-UI-007: expose accessible roles, names, values, actions, focus, and tree updates to platform assistive technology
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 7.1, 7.2, 7.3, 7.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 64, 200_
  - _Reads: projects/godot/scene/gui/control.cpp, crates/gpui/src/elements/div.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/ui/src/ui.rs#GODOT-UI-007, crates/theme/src/theme.rs#GODOT-UI-007, crates/ui_input/src/ui_input.rs#GODOT-UI-007_
  - _Validation: cargo test -p ui -p theme -p ui_input godot_control; run the UI scenario for expose accessible roles, names, values, actions, focus, and tree updates to platform assistive technology in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 66. Close or verify GODOT-UI-008: preview and migrate Godot UI scenes without claiming GPUI is runtime-compatible by default
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 7.1, 7.2, 7.3, 7.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 65, 200_
  - _Reads: projects/godot/scene/gui/control.cpp, crates/gpui/src/elements/div.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/ui/src/ui.rs#GODOT-UI-008, crates/theme/src/theme.rs#GODOT-UI-008, crates/ui_input/src/ui_input.rs#GODOT-UI-008_
  - _Validation: cargo test -p ui -p theme -p ui_input godot_control; run the UI scenario for preview and migrate Godot UI scenes without claiming GPUI is runtime-compatible by default in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 67. Close or verify GODOT-INPUT-001: define InputMap actions, deadzones, physical/logical keys, device filters, and multiple event bindings
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 8.1, 8.2, 8.3, 8.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 1, 200_
  - _Reads: projects/godot/core/input/input.cpp, crates/gpui/src/platform.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/gpui/src/platform.rs#GODOT-INPUT-001, crates/gpui_platform/src/gpui_platform.rs#GODOT-INPUT-001, crates/settings/src/settings.rs#GODOT-INPUT-001_
  - _Validation: cargo test -p gpui -p gpui_platform -p keymap_editor godot_input; run the INPUT scenario for define InputMap actions, deadzones, physical/logical keys, device filters, and multiple event bindings in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 68. Close or verify GODOT-INPUT-002: report pressed, just-pressed, just-released, strength, vector, mouse velocity, and accumulated input deterministically
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 8.1, 8.2, 8.3, 8.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 67, 200_
  - _Reads: projects/godot/core/input/input.cpp, crates/gpui/src/platform.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/gpui/src/platform.rs#GODOT-INPUT-002, crates/gpui_platform/src/gpui_platform.rs#GODOT-INPUT-002, crates/settings/src/settings.rs#GODOT-INPUT-002_
  - _Validation: cargo test -p gpui -p gpui_platform -p keymap_editor godot_input; run the INPUT scenario for report pressed, just-pressed, just-released, strength, vector, mouse velocity, and accumulated input deterministically in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 69. Close or verify GODOT-INPUT-003: handle keyboard, mouse, pen, touch, gestures, gamepads, hotplug, mappings, vibration, sensors, and emulation
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 8.1, 8.2, 8.3, 8.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 68, 200_
  - _Reads: projects/godot/core/input/input.cpp, crates/gpui/src/platform.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/gpui/src/platform.rs#GODOT-INPUT-003, crates/gpui_platform/src/gpui_platform.rs#GODOT-INPUT-003, crates/settings/src/settings.rs#GODOT-INPUT-003_
  - _Validation: cargo test -p gpui -p gpui_platform -p keymap_editor godot_input; run the INPUT scenario for handle keyboard, mouse, pen, touch, gestures, gamepads, hotplug, mappings, vibration, sensors, and emulation in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 70. Close or verify GODOT-INPUT-004: create and manage multiple windows, screens, modes, flags, focus, DPI, scale, vsync, orientation, and safe areas
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 8.1, 8.2, 8.3, 8.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 69, 200_
  - _Reads: projects/godot/core/input/input.cpp, crates/gpui/src/platform.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/gpui/src/platform.rs#GODOT-INPUT-004, crates/gpui_platform/src/gpui_platform.rs#GODOT-INPUT-004, crates/settings/src/settings.rs#GODOT-INPUT-004_
  - _Validation: cargo test -p gpui -p gpui_platform -p keymap_editor godot_input; run the INPUT scenario for create and manage multiple windows, screens, modes, flags, focus, DPI, scale, vsync, orientation, and safe areas in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 71. Close or verify GODOT-INPUT-005: support clipboard, cursor, mouse modes, virtual keyboard, IME composition, and text input
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 8.1, 8.2, 8.3, 8.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 70, 200_
  - _Reads: projects/godot/core/input/input.cpp, crates/gpui/src/platform.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/gpui/src/platform.rs#GODOT-INPUT-005, crates/gpui_platform/src/gpui_platform.rs#GODOT-INPUT-005, crates/settings/src/settings.rs#GODOT-INPUT-005_
  - _Validation: cargo test -p gpui -p gpui_platform -p keymap_editor godot_input; run the INPUT scenario for support clipboard, cursor, mouse modes, virtual keyboard, IME composition, and text input in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 72. Close or verify GODOT-INPUT-006: expose accessibility activation, semantic trees, actions, bounds, focus, announcements, and deactivation
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 8.1, 8.2, 8.3, 8.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 71, 200_
  - _Reads: projects/godot/core/input/input.cpp, crates/gpui/src/platform.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/gpui/src/platform.rs#GODOT-INPUT-006, crates/gpui_platform/src/gpui_platform.rs#GODOT-INPUT-006, crates/settings/src/settings.rs#GODOT-INPUT-006_
  - _Validation: cargo test -p gpui -p gpui_platform -p keymap_editor godot_input; run the INPUT scenario for expose accessibility activation, semantic trees, actions, bounds, focus, announcements, and deactivation in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 73. Close or verify GODOT-INPUT-007: load translations, select locale and fallbacks, pluralize, remap resources, shape bidi text, and mirror layout
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 8.1, 8.2, 8.3, 8.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 72, 200_
  - _Reads: projects/godot/core/input/input.cpp, crates/gpui/src/platform.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/gpui/src/platform.rs#GODOT-INPUT-007, crates/gpui_platform/src/gpui_platform.rs#GODOT-INPUT-007, crates/settings/src/settings.rs#GODOT-INPUT-007_
  - _Validation: cargo test -p gpui -p gpui_platform -p keymap_editor godot_input; run the INPUT scenario for load translations, select locale and fallbacks, pluralize, remap resources, shape bidi text, and mirror layout in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 74. Close or verify GODOT-INPUT-008: handle suspend, resume, low-memory, quit, focus, file-drop, and platform notification events
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 8.1, 8.2, 8.3, 8.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 73, 200_
  - _Reads: projects/godot/core/input/input.cpp, crates/gpui/src/platform.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/gpui/src/platform.rs#GODOT-INPUT-008, crates/gpui_platform/src/gpui_platform.rs#GODOT-INPUT-008, crates/settings/src/settings.rs#GODOT-INPUT-008_
  - _Validation: cargo test -p gpui -p gpui_platform -p keymap_editor godot_input; run the INPUT scenario for handle suspend, resume, low-memory, quit, focus, file-drop, and platform notification events in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 75. Close or verify GODOT-INPUT-009: provide dummy/headless display, audio, input, and text drivers with explicit unsupported behavior
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 8.1, 8.2, 8.3, 8.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 74, 200_
  - _Reads: projects/godot/core/input/input.cpp, crates/gpui/src/platform.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/gpui/src/platform.rs#GODOT-INPUT-009, crates/gpui_platform/src/gpui_platform.rs#GODOT-INPUT-009, crates/settings/src/settings.rs#GODOT-INPUT-009_
  - _Validation: cargo test -p gpui -p gpui_platform -p keymap_editor godot_input; run the INPUT scenario for provide dummy/headless display, audio, input, and text drivers with explicit unsupported behavior in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 76. Close or verify GODOT-ZED-001: simulate 2D rigid, static, character, animatable, and soft bodies with areas, shapes, joints, layers, masks, sleeping, and callbacks
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 9.1, 9.2, 9.3, 9.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 1, 200_
  - _Reads: projects/godot/servers/physics_2d/physics_server_2d.cpp, Cargo.toml, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/project/src/project.rs#GODOT-ZED-001, crates/task/src/task.rs#GODOT-ZED-001, crates/audio/src/audio.rs#GODOT-ZED-001, crates/media/src/media.rs#GODOT-ZED-001_
  - _Validation: cargo test -p project -p task -p audio godot_simulation; run the ZED scenario for simulate 2D rigid, static, character, animatable, and soft bodies with areas, shapes, joints, layers, masks, sleeping, and callbacks in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 77. Close or verify GODOT-ZED-002: simulate 3D rigid, static, character, animatable, and soft bodies with areas, shapes, joints, layers, masks, sleeping, and callbacks
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 9.1, 9.2, 9.3, 9.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 76, 200_
  - _Reads: projects/godot/servers/physics_2d/physics_server_2d.cpp, Cargo.toml, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/project/src/project.rs#GODOT-ZED-002, crates/task/src/task.rs#GODOT-ZED-002, crates/audio/src/audio.rs#GODOT-ZED-002, crates/media/src/media.rs#GODOT-ZED-002_
  - _Validation: cargo test -p project -p task -p audio godot_simulation; run the ZED scenario for simulate 3D rigid, static, character, animatable, and soft bodies with areas, shapes, joints, layers, masks, sleeping, and callbacks in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 78. Close or verify GODOT-ZED-003: select Godot Physics or Jolt 3D and report backend-specific settings and unsupported behavior
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 9.1, 9.2, 9.3, 9.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 77, 200_
  - _Reads: projects/godot/servers/physics_2d/physics_server_2d.cpp, Cargo.toml, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/project/src/project.rs#GODOT-ZED-003, crates/task/src/task.rs#GODOT-ZED-003, crates/audio/src/audio.rs#GODOT-ZED-003, crates/media/src/media.rs#GODOT-ZED-003_
  - _Validation: cargo test -p project -p task -p audio godot_simulation; run the ZED scenario for select Godot Physics or Jolt 3D and report backend-specific settings and unsupported behavior in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 79. Close or verify GODOT-ZED-004: perform direct-space point, ray, shape, motion, contact, and rest-info queries with exclusions and limits
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 9.1, 9.2, 9.3, 9.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 78, 200_
  - _Reads: projects/godot/servers/physics_2d/physics_server_2d.cpp, Cargo.toml, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/project/src/project.rs#GODOT-ZED-004, crates/task/src/task.rs#GODOT-ZED-004, crates/audio/src/audio.rs#GODOT-ZED-004, crates/media/src/media.rs#GODOT-ZED-004_
  - _Validation: cargo test -p project -p task -p audio godot_simulation; run the ZED scenario for perform direct-space point, ray, shape, motion, contact, and rest-info queries with exclusions and limits in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 80. Close or verify GODOT-ZED-005: build navigation maps from regions, meshes, links, obstacles, costs, layers, and avoidance agents in 2D and 3D
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 9.1, 9.2, 9.3, 9.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 79, 200_
  - _Reads: projects/godot/servers/physics_2d/physics_server_2d.cpp, Cargo.toml, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/project/src/project.rs#GODOT-ZED-005, crates/task/src/task.rs#GODOT-ZED-005, crates/audio/src/audio.rs#GODOT-ZED-005, crates/media/src/media.rs#GODOT-ZED-005_
  - _Validation: cargo test -p project -p task -p audio godot_simulation; run the ZED scenario for build navigation maps from regions, meshes, links, obstacles, costs, layers, and avoidance agents in 2D and 3D in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 81. Close or verify GODOT-ZED-006: bake, parse, cache, update, and debug navigation meshes and source geometry asynchronously with cancellation
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 9.1, 9.2, 9.3, 9.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 80, 200_
  - _Reads: projects/godot/servers/physics_2d/physics_server_2d.cpp, Cargo.toml, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/project/src/project.rs#GODOT-ZED-006, crates/task/src/task.rs#GODOT-ZED-006, crates/audio/src/audio.rs#GODOT-ZED-006, crates/media/src/media.rs#GODOT-ZED-006_
  - _Validation: cargo test -p project -p task -p audio godot_simulation; run the ZED scenario for bake, parse, cache, update, and debug navigation meshes and source geometry asynchronously with cancellation in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 82. Close or verify GODOT-ZED-007: author and play Animation, AnimationPlayer, AnimationTree, Tween, tracks, blends, state machines, method/audio tracks, and root motion
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 9.1, 9.2, 9.3, 9.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 81, 200_
  - _Reads: projects/godot/servers/physics_2d/physics_server_2d.cpp, Cargo.toml, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/project/src/project.rs#GODOT-ZED-007, crates/task/src/task.rs#GODOT-ZED-007, crates/audio/src/audio.rs#GODOT-ZED-007, crates/media/src/media.rs#GODOT-ZED-007_
  - _Validation: cargo test -p project -p task -p audio godot_simulation; run the ZED scenario for author and play Animation, AnimationPlayer, AnimationTree, Tween, tracks, blends, state machines, method/audio tracks, and root motion in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 83. Close or verify GODOT-ZED-008: route sample playback through buses, sends, effects, capture, device switching, spatial emitters, polyphony, and interactive music
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 9.1, 9.2, 9.3, 9.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 82, 200_
  - _Reads: projects/godot/servers/physics_2d/physics_server_2d.cpp, Cargo.toml, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/project/src/project.rs#GODOT-ZED-008, crates/task/src/task.rs#GODOT-ZED-008, crates/audio/src/audio.rs#GODOT-ZED-008, crates/media/src/media.rs#GODOT-ZED-008_
  - _Validation: cargo test -p project -p task -p audio godot_simulation; run the ZED scenario for route sample playback through buses, sends, effects, capture, device switching, spatial emitters, polyphony, and interactive music in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 84. Close or verify GODOT-ZED-009: simulate CPU and GPU particles, trails, collisions, attractors, process materials, subemitters, fixed FPS, and restart state
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 9.1, 9.2, 9.3, 9.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 83, 200_
  - _Reads: projects/godot/servers/physics_2d/physics_server_2d.cpp, Cargo.toml, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/project/src/project.rs#GODOT-ZED-009, crates/task/src/task.rs#GODOT-ZED-009, crates/audio/src/audio.rs#GODOT-ZED-009, crates/media/src/media.rs#GODOT-ZED-009_
  - _Validation: cargo test -p project -p task -p audio godot_simulation; run the ZED scenario for simulate CPU and GPU particles, trails, collisions, attractors, process materials, subemitters, fixed FPS, and restart state in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 85. Close or verify GODOT-SCRIPT-001: register script languages and create, load, reload, instance, attach, detach, and free scripts with object lifetime
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 10.1, 10.2, 10.3, 10.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 1, 200_
  - _Reads: projects/godot/core/object/script_language.cpp, crates/language/src/language_registry.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/languages/src/lib.rs#GODOT-SCRIPT-001, crates/language/src/language_registry.rs#GODOT-SCRIPT-001, crates/lsp/src/lsp.rs#GODOT-SCRIPT-001, crates/dap/src/dap.rs#GODOT-SCRIPT-001_
  - _Validation: cargo test -p language -p languages -p lsp -p dap godot; run the SCRIPT scenario for register script languages and create, load, reload, instance, attach, detach, and free scripts with object lifetime in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 86. Close or verify GODOT-SCRIPT-002: parse and compile GDScript including typed syntax, annotations, lambdas, pattern matching, classes, inheritance, and warnings
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 10.1, 10.2, 10.3, 10.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 85, 200_
  - _Reads: projects/godot/core/object/script_language.cpp, crates/language/src/language_registry.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/languages/src/lib.rs#GODOT-SCRIPT-002, crates/language/src/language_registry.rs#GODOT-SCRIPT-002, crates/lsp/src/lsp.rs#GODOT-SCRIPT-002, crates/dap/src/dap.rs#GODOT-SCRIPT-002_
  - _Validation: cargo test -p language -p languages -p lsp -p dap godot; run the SCRIPT scenario for parse and compile GDScript including typed syntax, annotations, lambdas, pattern matching, classes, inheritance, and warnings in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 87. Close or verify GODOT-SCRIPT-003: execute GDScript bytecode, calls, properties, signals, coroutines, awaits, errors, stack traces, and deterministic tests
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 10.1, 10.2, 10.3, 10.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 86, 200_
  - _Reads: projects/godot/core/object/script_language.cpp, crates/language/src/language_registry.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/languages/src/lib.rs#GODOT-SCRIPT-003, crates/language/src/language_registry.rs#GODOT-SCRIPT-003, crates/lsp/src/lsp.rs#GODOT-SCRIPT-003, crates/dap/src/dap.rs#GODOT-SCRIPT-003_
  - _Validation: cargo test -p language -p languages -p lsp -p dap godot; run the SCRIPT scenario for execute GDScript bytecode, calls, properties, signals, coroutines, awaits, errors, stack traces, and deterministic tests in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 88. Close or verify GODOT-SCRIPT-004: run @tool scripts in the editor with explicit trust, reload, inspector, undo, and failure isolation
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 10.1, 10.2, 10.3, 10.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 87, 200_
  - _Reads: projects/godot/core/object/script_language.cpp, crates/language/src/language_registry.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/languages/src/lib.rs#GODOT-SCRIPT-004, crates/language/src/language_registry.rs#GODOT-SCRIPT-004, crates/lsp/src/lsp.rs#GODOT-SCRIPT-004, crates/dap/src/dap.rs#GODOT-SCRIPT-004_
  - _Validation: cargo test -p language -p languages -p lsp -p dap godot; run the SCRIPT scenario for run @tool scripts in the editor with explicit trust, reload, inspector, undo, and failure isolation in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 89. Close or verify GODOT-SCRIPT-005: build, load, run, debug, hot-reload, export, and diagnose C# projects and assemblies when Mono is enabled
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 10.1, 10.2, 10.3, 10.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 88, 200_
  - _Reads: projects/godot/core/object/script_language.cpp, crates/language/src/language_registry.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/languages/src/lib.rs#GODOT-SCRIPT-005, crates/language/src/language_registry.rs#GODOT-SCRIPT-005, crates/lsp/src/lsp.rs#GODOT-SCRIPT-005, crates/dap/src/dap.rs#GODOT-SCRIPT-005_
  - _Validation: cargo test -p language -p languages -p lsp -p dap godot; run the SCRIPT scenario for build, load, run, debug, hot-reload, export, and diagnose C# projects and assemblies when Mono is enabled in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 90. Close or verify GODOT-SCRIPT-006: serve GDScript completion, hover, symbols, rename, references, formatting, diagnostics, semantic tokens, and DAP debugging
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 10.1, 10.2, 10.3, 10.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 89, 200_
  - _Reads: projects/godot/core/object/script_language.cpp, crates/language/src/language_registry.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/languages/src/lib.rs#GODOT-SCRIPT-006, crates/language/src/language_registry.rs#GODOT-SCRIPT-006, crates/lsp/src/lsp.rs#GODOT-SCRIPT-006, crates/dap/src/dap.rs#GODOT-SCRIPT-006_
  - _Validation: cargo test -p language -p languages -p lsp -p dap godot; run the SCRIPT scenario for serve GDScript completion, hover, symbols, rename, references, formatting, diagnostics, semantic tokens, and DAP debugging in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 91. Close or verify GODOT-SCRIPT-007: evaluate Expression resources with input names, base instances, parse errors, and execute failures
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 10.1, 10.2, 10.3, 10.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 90, 200_
  - _Reads: projects/godot/core/object/script_language.cpp, crates/language/src/language_registry.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/languages/src/lib.rs#GODOT-SCRIPT-007, crates/language/src/language_registry.rs#GODOT-SCRIPT-007, crates/lsp/src/lsp.rs#GODOT-SCRIPT-007, crates/dap/src/dap.rs#GODOT-SCRIPT-007_
  - _Validation: cargo test -p language -p languages -p lsp -p dap godot; run the SCRIPT scenario for evaluate Expression resources with input names, base instances, parse errors, and execute failures in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 92. Close or verify GODOT-SCRIPT-008: preserve exported script properties and placeholder instances when a script is missing or invalid
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 10.1, 10.2, 10.3, 10.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 91, 200_
  - _Reads: projects/godot/core/object/script_language.cpp, crates/language/src/language_registry.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/languages/src/lib.rs#GODOT-SCRIPT-008, crates/language/src/language_registry.rs#GODOT-SCRIPT-008, crates/lsp/src/lsp.rs#GODOT-SCRIPT-008, crates/dap/src/dap.rs#GODOT-SCRIPT-008_
  - _Validation: cargo test -p language -p languages -p lsp -p dap godot; run the SCRIPT scenario for preserve exported script properties and placeholder instances when a script is missing or invalid in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 93. Close or verify GODOT-SCRIPT-009: recognize SimScript and generate inspectable diffs from natural-language authoring intent
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 10.1, 10.2, 10.3, 10.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 92, 200_
  - _Reads: projects/godot/core/object/script_language.cpp, crates/language/src/language_registry.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/languages/src/lib.rs#GODOT-SCRIPT-009, crates/language/src/language_registry.rs#GODOT-SCRIPT-009, crates/lsp/src/lsp.rs#GODOT-SCRIPT-009, crates/dap/src/dap.rs#GODOT-SCRIPT-009_
  - _Validation: cargo test -p language -p languages -p lsp -p dap godot; run the SCRIPT scenario for recognize SimScript and generate inspectable diffs from natural-language authoring intent in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 94. Close or verify GODOT-EXT-001: parse .gdextension manifests and select libraries by OS, architecture, build, and feature tags
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 11.1, 11.2, 11.3, 11.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 1, 200_
  - _Reads: projects/godot/core/extension/gdextension.cpp, crates/extension/src/extension.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/extension_host/src/extension_host.rs#GODOT-EXT-001, crates/extension_api/src/extension_api.rs#GODOT-EXT-001, crates/extensions_ui/src/extensions_ui.rs#GODOT-EXT-001_
  - _Validation: cargo test -p extension -p extension_host -p extensions_ui godot; run the EXT scenario for parse .gdextension manifests and select libraries by OS, architecture, build, and feature tags in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 95. Close or verify GODOT-EXT-002: validate GDExtension minimum version, entry symbol, ABI, interface functions, and initialization levels
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 11.1, 11.2, 11.3, 11.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 94, 200_
  - _Reads: projects/godot/core/extension/gdextension.cpp, crates/extension/src/extension.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/extension_host/src/extension_host.rs#GODOT-EXT-002, crates/extension_api/src/extension_api.rs#GODOT-EXT-002, crates/extensions_ui/src/extensions_ui.rs#GODOT-EXT-002_
  - _Validation: cargo test -p extension -p extension_host -p extensions_ui godot; run the EXT scenario for validate GDExtension minimum version, entry symbol, ABI, interface functions, and initialization levels in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 96. Close or verify GODOT-EXT-003: load and unload extension libraries while registering classes, methods, properties, signals, constants, virtuals, and singletons
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 11.1, 11.2, 11.3, 11.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 95, 200_
  - _Reads: projects/godot/core/extension/gdextension.cpp, crates/extension/src/extension.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/extension_host/src/extension_host.rs#GODOT-EXT-003, crates/extension_api/src/extension_api.rs#GODOT-EXT-003, crates/extensions_ui/src/extensions_ui.rs#GODOT-EXT-003_
  - _Validation: cargo test -p extension -p extension_host -p extensions_ui godot; run the EXT scenario for load and unload extension libraries while registering classes, methods, properties, signals, constants, virtuals, and singletons in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 97. Close or verify GODOT-EXT-004: marshal Variants, native structures, pointers, call errors, object bindings, memory, strings, arrays, and dictionaries across the ABI
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 11.1, 11.2, 11.3, 11.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 96, 200_
  - _Reads: projects/godot/core/extension/gdextension.cpp, crates/extension/src/extension.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/extension_host/src/extension_host.rs#GODOT-EXT-004, crates/extension_api/src/extension_api.rs#GODOT-EXT-004, crates/extensions_ui/src/extensions_ui.rs#GODOT-EXT-004_
  - _Validation: cargo test -p extension -p extension_host -p extensions_ui godot; run the EXT scenario for marshal Variants, native structures, pointers, call errors, object bindings, memory, strings, arrays, and dictionaries across the ABI in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 98. Close or verify GODOT-EXT-005: generate and preserve extension_api.json and gdextension_interface.h compatibility contracts
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 11.1, 11.2, 11.3, 11.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 97, 200_
  - _Reads: projects/godot/core/extension/gdextension.cpp, crates/extension/src/extension.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/extension_host/src/extension_host.rs#GODOT-EXT-005, crates/extension_api/src/extension_api.rs#GODOT-EXT-005, crates/extensions_ui/src/extensions_ui.rs#GODOT-EXT-005_
  - _Validation: cargo test -p extension -p extension_host -p extensions_ui godot; run the EXT scenario for generate and preserve extension_api.json and gdextension_interface.h compatibility contracts in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 99. Close or verify GODOT-EXT-006: discover plugin.cfg addons and enable, disable, persist, reload, and diagnose EditorPlugin instances
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 11.1, 11.2, 11.3, 11.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 98, 200_
  - _Reads: projects/godot/core/extension/gdextension.cpp, crates/extension/src/extension.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/extension_host/src/extension_host.rs#GODOT-EXT-006, crates/extension_api/src/extension_api.rs#GODOT-EXT-006, crates/extensions_ui/src/extensions_ui.rs#GODOT-EXT-006_
  - _Validation: cargo test -p extension -p extension_host -p extensions_ui godot; run the EXT scenario for discover plugin.cfg addons and enable, disable, persist, reload, and diagnose EditorPlugin instances in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 100. Close or verify GODOT-EXT-007: allow editor plugins to add docks, inspectors, importers, exporters, gizmos, debuggers, settings, shortcuts, and autoloads with cleanup
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 11.1, 11.2, 11.3, 11.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 99, 200_
  - _Reads: projects/godot/core/extension/gdextension.cpp, crates/extension/src/extension.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/extension_host/src/extension_host.rs#GODOT-EXT-007, crates/extension_api/src/extension_api.rs#GODOT-EXT-007, crates/extensions_ui/src/extensions_ui.rs#GODOT-EXT-007_
  - _Validation: cargo test -p extension -p extension_host -p extensions_ui godot; run the EXT scenario for allow editor plugins to add docks, inspectors, importers, exporters, gizmos, debuggers, settings, shortcuts, and autoloads with cleanup in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 101. Close or verify GODOT-EXT-008: reuse Zed extension trust, capability, installation, and UI boundaries instead of creating a second plugin manager
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 11.1, 11.2, 11.3, 11.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 100, 200_
  - _Reads: projects/godot/core/extension/gdextension.cpp, crates/extension/src/extension.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/extension_host/src/extension_host.rs#GODOT-EXT-008, crates/extension_api/src/extension_api.rs#GODOT-EXT-008, crates/extensions_ui/src/extensions_ui.rs#GODOT-EXT-008_
  - _Validation: cargo test -p extension -p extension_host -p extensions_ui godot; run the EXT scenario for reuse Zed extension trust, capability, installation, and UI boundaries instead of creating a second plugin manager in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 102. Close or verify GODOT-IMPORT-001: scan the project filesystem incrementally with ignore rules, UIDs, type detection, moves, removals, and watcher reconciliation
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 12.1, 12.2, 12.3, 12.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 1, 200_
  - _Reads: projects/godot/editor/file_system/editor_file_system.cpp, crates/worktree/src/worktree.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/worktree/src/worktree.rs#GODOT-IMPORT-001, crates/project/src/project.rs#GODOT-IMPORT-001, crates/image_viewer/src/image_viewer.rs#GODOT-IMPORT-001_
  - _Validation: cargo test -p worktree -p project -p image_viewer godot_import; run the IMPORT scenario for scan the project filesystem incrementally with ignore rules, UIDs, type detection, moves, removals, and watcher reconciliation in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 103. Close or verify GODOT-IMPORT-002: select importers by extension and priority and persist importer, options, source, destination, remap, generator, and validity metadata
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 12.1, 12.2, 12.3, 12.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 102, 200_
  - _Reads: projects/godot/editor/file_system/editor_file_system.cpp, crates/worktree/src/worktree.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/worktree/src/worktree.rs#GODOT-IMPORT-002, crates/project/src/project.rs#GODOT-IMPORT-002, crates/image_viewer/src/image_viewer.rs#GODOT-IMPORT-002_
  - _Validation: cargo test -p worktree -p project -p image_viewer godot_import; run the IMPORT scenario for select importers by extension and priority and persist importer, options, source, destination, remap, generator, and validity metadata in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 104. Close or verify GODOT-IMPORT-003: queue threaded imports and reimports with progress, cancellation, restart, dependency ordering, and failure isolation
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 12.1, 12.2, 12.3, 12.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 103, 200_
  - _Reads: projects/godot/editor/file_system/editor_file_system.cpp, crates/worktree/src/worktree.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/worktree/src/worktree.rs#GODOT-IMPORT-003, crates/project/src/project.rs#GODOT-IMPORT-003, crates/image_viewer/src/image_viewer.rs#GODOT-IMPORT-003_
  - _Validation: cargo test -p worktree -p project -p image_viewer godot_import; run the IMPORT scenario for queue threaded imports and reimports with progress, cancellation, restart, dependency ordering, and failure isolation in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 105. Close or verify GODOT-IMPORT-004: invalidate imported caches from source hashes, importer versions, settings, dependencies, feature tags, and generated files
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 12.1, 12.2, 12.3, 12.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 104, 200_
  - _Reads: projects/godot/editor/file_system/editor_file_system.cpp, crates/worktree/src/worktree.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/worktree/src/worktree.rs#GODOT-IMPORT-004, crates/project/src/project.rs#GODOT-IMPORT-004, crates/image_viewer/src/image_viewer.rs#GODOT-IMPORT-004_
  - _Validation: cargo test -p worktree -p project -p image_viewer godot_import; run the IMPORT scenario for invalidate imported caches from source hashes, importer versions, settings, dependencies, feature tags, and generated files in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 106. Close or verify GODOT-IMPORT-005: import images and SVGs into textures with compression, mipmaps, color-space, normal-map, atlas, and platform variants
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 12.1, 12.2, 12.3, 12.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 105, 200_
  - _Reads: projects/godot/editor/file_system/editor_file_system.cpp, crates/worktree/src/worktree.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/worktree/src/worktree.rs#GODOT-IMPORT-005, crates/project/src/project.rs#GODOT-IMPORT-005, crates/image_viewer/src/image_viewer.rs#GODOT-IMPORT-005_
  - _Validation: cargo test -p worktree -p project -p image_viewer godot_import; run the IMPORT scenario for import images and SVGs into textures with compression, mipmaps, color-space, normal-map, atlas, and platform variants in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 107. Close or verify GODOT-IMPORT-006: import audio into streams/samples with compression, looping, normalization, trimming, and channel modes
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 12.1, 12.2, 12.3, 12.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 106, 200_
  - _Reads: projects/godot/editor/file_system/editor_file_system.cpp, crates/worktree/src/worktree.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/worktree/src/worktree.rs#GODOT-IMPORT-006, crates/project/src/project.rs#GODOT-IMPORT-006, crates/image_viewer/src/image_viewer.rs#GODOT-IMPORT-006_
  - _Validation: cargo test -p worktree -p project -p image_viewer godot_import; run the IMPORT scenario for import audio into streams/samples with compression, looping, normalization, trimming, and channel modes in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 108. Close or verify GODOT-IMPORT-007: import 3D scenes and animations with node/path filters, materials, meshes, skins, LOD, lightmaps, physics, and post-import scripts
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 12.1, 12.2, 12.3, 12.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 107, 200_
  - _Reads: projects/godot/editor/file_system/editor_file_system.cpp, crates/worktree/src/worktree.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/worktree/src/worktree.rs#GODOT-IMPORT-007, crates/project/src/project.rs#GODOT-IMPORT-007, crates/image_viewer/src/image_viewer.rs#GODOT-IMPORT-007_
  - _Validation: cargo test -p worktree -p project -p image_viewer godot_import; run the IMPORT scenario for import 3D scenes and animations with node/path filters, materials, meshes, skins, LOD, lightmaps, physics, and post-import scripts in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 109. Close or verify GODOT-IMPORT-008: import glTF, FBX, OBJ, Blender, DAE, and other enabled formats with dependency and unsupported-feature diagnostics
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 12.1, 12.2, 12.3, 12.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 108, 200_
  - _Reads: projects/godot/editor/file_system/editor_file_system.cpp, crates/worktree/src/worktree.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/worktree/src/worktree.rs#GODOT-IMPORT-008, crates/project/src/project.rs#GODOT-IMPORT-008, crates/image_viewer/src/image_viewer.rs#GODOT-IMPORT-008_
  - _Validation: cargo test -p worktree -p project -p image_viewer godot_import; run the IMPORT scenario for import glTF, FBX, OBJ, Blender, DAE, and other enabled formats with dependency and unsupported-feature diagnostics in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 110. Close or verify GODOT-IMPORT-009: import fonts, translations, CSV, bitmaps, textures, shaders, and custom plugin formats
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 12.1, 12.2, 12.3, 12.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 109, 200_
  - _Reads: projects/godot/editor/file_system/editor_file_system.cpp, crates/worktree/src/worktree.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/worktree/src/worktree.rs#GODOT-IMPORT-009, crates/project/src/project.rs#GODOT-IMPORT-009, crates/image_viewer/src/image_viewer.rs#GODOT-IMPORT-009_
  - _Validation: cargo test -p worktree -p project -p image_viewer godot_import; run the IMPORT scenario for import fonts, translations, CSV, bitmaps, textures, shaders, and custom plugin formats in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 111. Close or verify GODOT-IMPORT-010: link source assets, imported outputs, generated files, resource UIDs, dependencies, owners, and reimport actions in the project panel
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 12.1, 12.2, 12.3, 12.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 110, 200_
  - _Reads: projects/godot/editor/file_system/editor_file_system.cpp, crates/worktree/src/worktree.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/worktree/src/worktree.rs#GODOT-IMPORT-010, crates/project/src/project.rs#GODOT-IMPORT-010, crates/image_viewer/src/image_viewer.rs#GODOT-IMPORT-010_
  - _Validation: cargo test -p worktree -p project -p image_viewer godot_import; run the IMPORT scenario for link source assets, imported outputs, generated files, resource UIDs, dependencies, owners, and reimport actions in the project panel in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 112. Close or verify GODOT-EXPORT-001: parse, edit, duplicate, reorder, persist, and validate export presets, filters, features, patches, and custom options
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 13.1, 13.2, 13.3, 13.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 1, 200_
  - _Reads: projects/godot/editor/export/editor_export.cpp, crates/task/src/task_template.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/task/src/task.rs#GODOT-EXPORT-001, crates/project/src/project.rs#GODOT-EXPORT-001, crates/settings/src/settings.rs#GODOT-EXPORT-001_
  - _Validation: cargo test -p task -p project -p settings godot_export; run the EXPORT scenario for parse, edit, duplicate, reorder, persist, and validate export presets, filters, features, patches, and custom options in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 113. Close or verify GODOT-EXPORT-002: discover, install, uninstall, mirror, and validate matching debug/release export templates without silent downloads
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 13.1, 13.2, 13.3, 13.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 112, 200_
  - _Reads: projects/godot/editor/export/editor_export.cpp, crates/task/src/task_template.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/task/src/task.rs#GODOT-EXPORT-002, crates/project/src/project.rs#GODOT-EXPORT-002, crates/settings/src/settings.rs#GODOT-EXPORT-002_
  - _Validation: cargo test -p task -p project -p settings godot_export; run the EXPORT scenario for discover, install, uninstall, mirror, and validate matching debug/release export templates without silent downloads in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 114. Close or verify GODOT-EXPORT-003: export project data as PCK/ZIP or embedded pack with include/exclude filters, remaps, conversion, and deterministic manifests
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 13.1, 13.2, 13.3, 13.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 113, 200_
  - _Reads: projects/godot/editor/export/editor_export.cpp, crates/task/src/task_template.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/task/src/task.rs#GODOT-EXPORT-003, crates/project/src/project.rs#GODOT-EXPORT-003, crates/settings/src/settings.rs#GODOT-EXPORT-003_
  - _Validation: cargo test -p task -p project -p settings godot_export; run the EXPORT scenario for export project data as PCK/ZIP or embedded pack with include/exclude filters, remaps, conversion, and deterministic manifests in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 115. Close or verify GODOT-EXPORT-004: export debug, release, and dedicated-server builds from editor or CLI and propagate progress, cancellation, warnings, and errors
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 13.1, 13.2, 13.3, 13.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 114, 200_
  - _Reads: projects/godot/editor/export/editor_export.cpp, crates/task/src/task_template.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/task/src/task.rs#GODOT-EXPORT-004, crates/project/src/project.rs#GODOT-EXPORT-004, crates/settings/src/settings.rs#GODOT-EXPORT-004_
  - _Validation: cargo test -p task -p project -p settings godot_export; run the EXPORT scenario for export debug, release, and dedicated-server builds from editor or CLI and propagate progress, cancellation, warnings, and errors in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 116. Close or verify GODOT-EXPORT-005: export and deploy Android APK/AAB/Gradle builds with SDK/JDK/keystore/permissions/architectures and remote run
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 13.1, 13.2, 13.3, 13.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 115, 200_
  - _Reads: projects/godot/editor/export/editor_export.cpp, crates/task/src/task_template.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/task/src/task.rs#GODOT-EXPORT-005, crates/project/src/project.rs#GODOT-EXPORT-005, crates/settings/src/settings.rs#GODOT-EXPORT-005_
  - _Validation: cargo test -p task -p project -p settings godot_export; run the EXPORT scenario for export and deploy Android APK/AAB/Gradle builds with SDK/JDK/keystore/permissions/architectures and remote run in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 117. Close or verify GODOT-EXPORT-006: export iOS, macOS, and visionOS bundles/projects with entitlements, privacy manifests, provisioning, codesign, notarization, and architectures
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 13.1, 13.2, 13.3, 13.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 116, 200_
  - _Reads: projects/godot/editor/export/editor_export.cpp, crates/task/src/task_template.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/task/src/task.rs#GODOT-EXPORT-006, crates/project/src/project.rs#GODOT-EXPORT-006, crates/settings/src/settings.rs#GODOT-EXPORT-006_
  - _Validation: cargo test -p task -p project -p settings godot_export; run the EXPORT scenario for export iOS, macOS, and visionOS bundles/projects with entitlements, privacy manifests, provisioning, codesign, notarization, and architectures in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 118. Close or verify GODOT-EXPORT-007: export Linux/BSD and Windows executables with architectures, icons, metadata, signing, console mode, and embedded data
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 13.1, 13.2, 13.3, 13.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 117, 200_
  - _Reads: projects/godot/editor/export/editor_export.cpp, crates/task/src/task_template.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/task/src/task.rs#GODOT-EXPORT-007, crates/project/src/project.rs#GODOT-EXPORT-007, crates/settings/src/settings.rs#GODOT-EXPORT-007_
  - _Validation: cargo test -p task -p project -p settings godot_export; run the EXPORT scenario for export Linux/BSD and Windows executables with architectures, icons, metadata, signing, console mode, and embedded data in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 119. Close or verify GODOT-EXPORT-008: export Web builds with WASM, threads, service worker/PWA, extensions, HTML shell, compression, and browser feature validation
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 13.1, 13.2, 13.3, 13.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 118, 200_
  - _Reads: projects/godot/editor/export/editor_export.cpp, crates/task/src/task_template.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/task/src/task.rs#GODOT-EXPORT-008, crates/project/src/project.rs#GODOT-EXPORT-008, crates/settings/src/settings.rs#GODOT-EXPORT-008_
  - _Validation: cargo test -p task -p project -p settings godot_export; run the EXPORT scenario for export Web builds with WASM, threads, service worker/PWA, extensions, HTML shell, compression, and browser feature validation in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 120. Close or verify GODOT-EXPORT-009: encrypt packs or scripts and protect credentials/signing material without persisting secrets in project files
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 13.1, 13.2, 13.3, 13.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 119, 200_
  - _Reads: projects/godot/editor/export/editor_export.cpp, crates/task/src/task_template.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/task/src/task.rs#GODOT-EXPORT-009, crates/project/src/project.rs#GODOT-EXPORT-009, crates/settings/src/settings.rs#GODOT-EXPORT-009_
  - _Validation: cargo test -p task -p project -p settings godot_export; run the EXPORT scenario for encrypt packs or scripts and protect credentials/signing material without persisting secrets in project files in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 121. Close or verify GODOT-EXPORT-010: launch, stop, remote-deploy, and collect logs from an exported or editor-run project through existing Zed tasks
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 13.1, 13.2, 13.3, 13.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 120, 200_
  - _Reads: projects/godot/editor/export/editor_export.cpp, crates/task/src/task_template.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/task/src/task.rs#GODOT-EXPORT-010, crates/project/src/project.rs#GODOT-EXPORT-010, crates/settings/src/settings.rs#GODOT-EXPORT-010_
  - _Validation: cargo test -p task -p project -p settings godot_export; run the EXPORT scenario for launch, stop, remote-deploy, and collect logs from an exported or editor-run project through existing Zed tasks in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 122. Close or verify GODOT-NET-001: read, write, seek, resize, flush, compress, encrypt, hash, map, and atomically replace files through res:// and user://
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 14.1, 14.2, 14.3, 14.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 1, 200_
  - _Reads: projects/godot/core/io/file_access.cpp, crates/fs/src/fs.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/fs/src/fs.rs#GODOT-NET-001, crates/net/src/net.rs#GODOT-NET-001, crates/http_client/src/http_client.rs#GODOT-NET-001, crates/rpc/src/rpc.rs#GODOT-NET-001, crates/collab/src/lib.rs#GODOT-NET-001_
  - _Validation: cargo test -p fs -p net -p http_client -p rpc godot; run the NET scenario for read, write, seek, resize, flush, compress, encrypt, hash, map, and atomically replace files through res:// and user:// in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 123. Close or verify GODOT-NET-002: list, create, rename, copy, remove, and watch directories while confining paths and preserving platform semantics
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 14.1, 14.2, 14.3, 14.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 122, 200_
  - _Reads: projects/godot/core/io/file_access.cpp, crates/fs/src/fs.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/fs/src/fs.rs#GODOT-NET-002, crates/net/src/net.rs#GODOT-NET-002, crates/http_client/src/http_client.rs#GODOT-NET-002, crates/rpc/src/rpc.rs#GODOT-NET-002, crates/collab/src/lib.rs#GODOT-NET-002_
  - _Validation: cargo test -p fs -p net -p http_client -p rpc godot; run the NET scenario for list, create, rename, copy, remove, and watch directories while confining paths and preserving platform semantics in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 124. Close or verify GODOT-NET-003: resolve DNS and use TCP, UDP, Unix sockets, PacketPeer, StreamPeer, multicast, broadcast, IPv4, and IPv6 with nonblocking errors
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 14.1, 14.2, 14.3, 14.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 123, 200_
  - _Reads: projects/godot/core/io/file_access.cpp, crates/fs/src/fs.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/fs/src/fs.rs#GODOT-NET-003, crates/net/src/net.rs#GODOT-NET-003, crates/http_client/src/http_client.rs#GODOT-NET-003, crates/rpc/src/rpc.rs#GODOT-NET-003, crates/collab/src/lib.rs#GODOT-NET-003_
  - _Validation: cargo test -p fs -p net -p http_client -p rpc godot; run the NET scenario for resolve DNS and use TCP, UDP, Unix sockets, PacketPeer, StreamPeer, multicast, broadcast, IPv4, and IPv6 with nonblocking errors in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 125. Close or verify GODOT-NET-004: perform HTTP requests, redirects, proxies, cookies/headers, body streaming, downloads, timeouts, cancellation, TLS, and size limits
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 14.1, 14.2, 14.3, 14.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 124, 200_
  - _Reads: projects/godot/core/io/file_access.cpp, crates/fs/src/fs.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/fs/src/fs.rs#GODOT-NET-004, crates/net/src/net.rs#GODOT-NET-004, crates/http_client/src/http_client.rs#GODOT-NET-004, crates/rpc/src/rpc.rs#GODOT-NET-004, crates/collab/src/lib.rs#GODOT-NET-004_
  - _Validation: cargo test -p fs -p net -p http_client -p rpc godot; run the NET scenario for perform HTTP requests, redirects, proxies, cookies/headers, body streaming, downloads, timeouts, cancellation, TLS, and size limits in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 126. Close or verify GODOT-NET-005: connect WebSocket peers and multiplayer peers with protocols, channels, packet modes, close codes, heartbeats, and browser constraints
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 14.1, 14.2, 14.3, 14.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 125, 200_
  - _Reads: projects/godot/core/io/file_access.cpp, crates/fs/src/fs.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/fs/src/fs.rs#GODOT-NET-005, crates/net/src/net.rs#GODOT-NET-005, crates/http_client/src/http_client.rs#GODOT-NET-005, crates/rpc/src/rpc.rs#GODOT-NET-005, crates/collab/src/lib.rs#GODOT-NET-005_
  - _Validation: cargo test -p fs -p net -p http_client -p rpc godot; run the NET scenario for connect WebSocket peers and multiplayer peers with protocols, channels, packet modes, close codes, heartbeats, and browser constraints in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 127. Close or verify GODOT-NET-006: connect WebRTC peers and data channels with SDP, ICE, polling, ordered/reliable modes, and platform plugin availability
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 14.1, 14.2, 14.3, 14.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 126, 200_
  - _Reads: projects/godot/core/io/file_access.cpp, crates/fs/src/fs.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/fs/src/fs.rs#GODOT-NET-006, crates/net/src/net.rs#GODOT-NET-006, crates/http_client/src/http_client.rs#GODOT-NET-006, crates/rpc/src/rpc.rs#GODOT-NET-006, crates/collab/src/lib.rs#GODOT-NET-006_
  - _Validation: cargo test -p fs -p net -p http_client -p rpc godot; run the NET scenario for connect WebRTC peers and data channels with SDP, ICE, polling, ordered/reliable modes, and platform plugin availability in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 128. Close or verify GODOT-NET-007: connect ENet peers with server/client/mesh topology, compression, bandwidth, channels, disconnects, and statistics
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 14.1, 14.2, 14.3, 14.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 127, 200_
  - _Reads: projects/godot/core/io/file_access.cpp, crates/fs/src/fs.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/fs/src/fs.rs#GODOT-NET-007, crates/net/src/net.rs#GODOT-NET-007, crates/http_client/src/http_client.rs#GODOT-NET-007, crates/rpc/src/rpc.rs#GODOT-NET-007, crates/collab/src/lib.rs#GODOT-NET-007_
  - _Validation: cargo test -p fs -p net -p http_client -p rpc godot; run the NET scenario for connect ENet peers with server/client/mesh topology, compression, bandwidth, channels, disconnects, and statistics in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 129. Close or verify GODOT-NET-008: perform high-level multiplayer RPC authority, transfer modes, object configuration, peer authentication, and refusal
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 14.1, 14.2, 14.3, 14.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 128, 200_
  - _Reads: projects/godot/core/io/file_access.cpp, crates/fs/src/fs.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/fs/src/fs.rs#GODOT-NET-008, crates/net/src/net.rs#GODOT-NET-008, crates/http_client/src/http_client.rs#GODOT-NET-008, crates/rpc/src/rpc.rs#GODOT-NET-008, crates/collab/src/lib.rs#GODOT-NET-008_
  - _Validation: cargo test -p fs -p net -p http_client -p rpc godot; run the NET scenario for perform high-level multiplayer RPC authority, transfer modes, object configuration, peer authentication, and refusal in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 130. Close or verify GODOT-NET-009: replicate and spawn scene state with MultiplayerSynchronizer/Spawner, visibility filters, authority changes, and late joins
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 14.1, 14.2, 14.3, 14.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 129, 200_
  - _Reads: projects/godot/core/io/file_access.cpp, crates/fs/src/fs.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/fs/src/fs.rs#GODOT-NET-009, crates/net/src/net.rs#GODOT-NET-009, crates/http_client/src/http_client.rs#GODOT-NET-009, crates/rpc/src/rpc.rs#GODOT-NET-009, crates/collab/src/lib.rs#GODOT-NET-009_
  - _Validation: cargo test -p fs -p net -p http_client -p rpc godot; run the NET scenario for replicate and spawn scene state with MultiplayerSynchronizer/Spawner, visibility filters, authority changes, and late joins in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 131. Close or verify GODOT-NET-010: discover and manage UPnP mappings with timeout, gateway, conflict, and cleanup behavior
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 14.1, 14.2, 14.3, 14.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 130, 200_
  - _Reads: projects/godot/core/io/file_access.cpp, crates/fs/src/fs.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/fs/src/fs.rs#GODOT-NET-010, crates/net/src/net.rs#GODOT-NET-010, crates/http_client/src/http_client.rs#GODOT-NET-010, crates/rpc/src/rpc.rs#GODOT-NET-010, crates/collab/src/lib.rs#GODOT-NET-010_
  - _Validation: cargo test -p fs -p net -p http_client -p rpc godot; run the NET scenario for discover and manage UPnP mappings with timeout, gateway, conflict, and cleanup behavior in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 132. Close or verify GODOT-NET-011: bridge browser JavaScript, downloads, clipboard, virtual keyboard, service workers, and cross-origin restrictions in Web exports
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 14.1, 14.2, 14.3, 14.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 131, 200_
  - _Reads: projects/godot/core/io/file_access.cpp, crates/fs/src/fs.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/fs/src/fs.rs#GODOT-NET-011, crates/net/src/net.rs#GODOT-NET-011, crates/http_client/src/http_client.rs#GODOT-NET-011, crates/rpc/src/rpc.rs#GODOT-NET-011, crates/collab/src/lib.rs#GODOT-NET-011_
  - _Validation: cargo test -p fs -p net -p http_client -p rpc godot; run the NET scenario for bridge browser JavaScript, downloads, clipboard, virtual keyboard, service workers, and cross-origin restrictions in Web exports in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 133. Close or verify GODOT-DEBUG-001: format, route, filter, timestamp, persist, and flush stdout/stderr, print, warning, error, and structured engine log messages
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 15.1, 15.2, 15.3, 15.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 1, 200_
  - _Reads: projects/godot/core/debugger/engine_debugger.cpp, crates/dap/src/dap.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/dap/src/dap.rs#GODOT-DEBUG-001, crates/debugger_ui/src/debugger_ui.rs#GODOT-DEBUG-001, crates/diagnostics/src/diagnostics.rs#GODOT-DEBUG-001, crates/crashes/src/crashes.rs#GODOT-DEBUG-001_
  - _Validation: cargo test -p dap -p debugger_ui -p diagnostics -p crashes godot; run the DEBUG scenario for format, route, filter, timestamp, persist, and flush stdout/stderr, print, warning, error, and structured engine log messages in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 134. Close or verify GODOT-DEBUG-002: connect and authenticate editor/runtime debugger sessions with protocol negotiation, timeouts, reconnect, and multiple instances
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 15.1, 15.2, 15.3, 15.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 133, 200_
  - _Reads: projects/godot/core/debugger/engine_debugger.cpp, crates/dap/src/dap.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/dap/src/dap.rs#GODOT-DEBUG-002, crates/debugger_ui/src/debugger_ui.rs#GODOT-DEBUG-002, crates/diagnostics/src/diagnostics.rs#GODOT-DEBUG-002, crates/crashes/src/crashes.rs#GODOT-DEBUG-002_
  - _Validation: cargo test -p dap -p debugger_ui -p diagnostics -p crashes godot; run the DEBUG scenario for connect and authenticate editor/runtime debugger sessions with protocol negotiation, timeouts, reconnect, and multiple instances in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 135. Close or verify GODOT-DEBUG-003: set breakpoints and exception breaks and inspect stacks, locals, members, globals, expressions, errors, and live script reload
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 15.1, 15.2, 15.3, 15.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 134, 200_
  - _Reads: projects/godot/core/debugger/engine_debugger.cpp, crates/dap/src/dap.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/dap/src/dap.rs#GODOT-DEBUG-003, crates/debugger_ui/src/debugger_ui.rs#GODOT-DEBUG-003, crates/diagnostics/src/diagnostics.rs#GODOT-DEBUG-003, crates/crashes/src/crashes.rs#GODOT-DEBUG-003_
  - _Validation: cargo test -p dap -p debugger_ui -p diagnostics -p crashes godot; run the DEBUG scenario for set breakpoints and exception breaks and inspect stacks, locals, members, globals, expressions, errors, and live script reload in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 136. Close or verify GODOT-DEBUG-004: inspect and edit the remote scene tree, nodes, resources, properties, camera overrides, selection, and live edits safely
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 15.1, 15.2, 15.3, 15.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 135, 200_
  - _Reads: projects/godot/core/debugger/engine_debugger.cpp, crates/dap/src/dap.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/dap/src/dap.rs#GODOT-DEBUG-004, crates/debugger_ui/src/debugger_ui.rs#GODOT-DEBUG-004, crates/diagnostics/src/diagnostics.rs#GODOT-DEBUG-004, crates/crashes/src/crashes.rs#GODOT-DEBUG-004_
  - _Validation: cargo test -p dap -p debugger_ui -p diagnostics -p crashes godot; run the DEBUG scenario for inspect and edit the remote scene tree, nodes, resources, properties, camera overrides, selection, and live edits safely in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 137. Close or verify GODOT-DEBUG-005: profile script/native time, calls, frame stages, GPU, servers, memory, resources, and custom monitors with bounded sampling
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 15.1, 15.2, 15.3, 15.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 136, 200_
  - _Reads: projects/godot/core/debugger/engine_debugger.cpp, crates/dap/src/dap.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/dap/src/dap.rs#GODOT-DEBUG-005, crates/debugger_ui/src/debugger_ui.rs#GODOT-DEBUG-005, crates/diagnostics/src/diagnostics.rs#GODOT-DEBUG-005, crates/crashes/src/crashes.rs#GODOT-DEBUG-005_
  - _Validation: cargo test -p dap -p debugger_ui -p diagnostics -p crashes godot; run the DEBUG scenario for profile script/native time, calls, frame stages, GPU, servers, memory, resources, and custom monitors with bounded sampling in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 138. Close or verify GODOT-DEBUG-006: profile multiplayer RPC/bandwidth and visualize collisions, paths, navigation, canvas redraw, and rendering diagnostics
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 15.1, 15.2, 15.3, 15.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 137, 200_
  - _Reads: projects/godot/core/debugger/engine_debugger.cpp, crates/dap/src/dap.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/dap/src/dap.rs#GODOT-DEBUG-006, crates/debugger_ui/src/debugger_ui.rs#GODOT-DEBUG-006, crates/diagnostics/src/diagnostics.rs#GODOT-DEBUG-006, crates/crashes/src/crashes.rs#GODOT-DEBUG-006_
  - _Validation: cargo test -p dap -p debugger_ui -p diagnostics -p crashes godot; run the DEBUG scenario for profile multiplayer RPC/bandwidth and visualize collisions, paths, navigation, canvas redraw, and rendering diagnostics in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 139. Close or verify GODOT-DEBUG-007: capture errors and crashes with backtraces, symbols, platform handlers, suppression rules, and safe shutdown/reporting
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 15.1, 15.2, 15.3, 15.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 138, 200_
  - _Reads: projects/godot/core/debugger/engine_debugger.cpp, crates/dap/src/dap.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/dap/src/dap.rs#GODOT-DEBUG-007, crates/debugger_ui/src/debugger_ui.rs#GODOT-DEBUG-007, crates/diagnostics/src/diagnostics.rs#GODOT-DEBUG-007, crates/crashes/src/crashes.rs#GODOT-DEBUG-007_
  - _Validation: cargo test -p dap -p debugger_ui -p diagnostics -p crashes godot; run the DEBUG scenario for capture errors and crashes with backtraces, symbols, platform handlers, suppression rules, and safe shutdown/reporting in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 140. Close or verify GODOT-DEBUG-008: recover editor state after a crashed game/editor/plugin and preserve actionable logs without claiming success
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 15.1, 15.2, 15.3, 15.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 139, 200_
  - _Reads: projects/godot/core/debugger/engine_debugger.cpp, crates/dap/src/dap.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/dap/src/dap.rs#GODOT-DEBUG-008, crates/debugger_ui/src/debugger_ui.rs#GODOT-DEBUG-008, crates/diagnostics/src/diagnostics.rs#GODOT-DEBUG-008, crates/crashes/src/crashes.rs#GODOT-DEBUG-008_
  - _Validation: cargo test -p dap -p debugger_ui -p diagnostics -p crashes godot; run the DEBUG scenario for recover editor state after a crashed game/editor/plugin and preserve actionable logs without claiming success in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 141. Close or verify GODOT-CLI-001: resolve project path, main pack, scene, editor, project-manager, and runtime mode with conflict diagnostics
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 16.1, 16.2, 16.3, 16.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 1, 200_
  - _Reads: projects/godot/main/main.cpp, crates/cli/src/cli.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/cli/src/cli.rs#GODOT-CLI-001, crates/task/src/task.rs#GODOT-CLI-001, crates/remote_server/src/main.rs#GODOT-CLI-001_
  - _Validation: cargo test -p cli -p task -p remote_server godot; run the CLI scenario for resolve project path, main pack, scene, editor, project-manager, and runtime mode with conflict diagnostics in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 142. Close or verify GODOT-CLI-002: run headless or with dummy display/audio/text/input drivers and report unsupported visual operations
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 16.1, 16.2, 16.3, 16.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 141, 200_
  - _Reads: projects/godot/main/main.cpp, crates/cli/src/cli.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/cli/src/cli.rs#GODOT-CLI-002, crates/task/src/task.rs#GODOT-CLI-002, crates/remote_server/src/main.rs#GODOT-CLI-002_
  - _Validation: cargo test -p cli -p task -p remote_server godot; run the CLI scenario for run headless or with dummy display/audio/text/input drivers and report unsupported visual operations in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 143. Close or verify GODOT-CLI-003: scan/import resources and quit after import or after a requested frame/time boundary with useful exit status
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 16.1, 16.2, 16.3, 16.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 142, 200_
  - _Reads: projects/godot/main/main.cpp, crates/cli/src/cli.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/cli/src/cli.rs#GODOT-CLI-003, crates/task/src/task.rs#GODOT-CLI-003, crates/remote_server/src/main.rs#GODOT-CLI-003_
  - _Validation: cargo test -p cli -p task -p remote_server godot; run the CLI scenario for scan/import resources and quit after import or after a requested frame/time boundary with useful exit status in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 144. Close or verify GODOT-CLI-004: export or pack named presets from CLI and propagate template, toolchain, signing, progress, cancellation, and failure status
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 16.1, 16.2, 16.3, 16.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 143, 200_
  - _Reads: projects/godot/main/main.cpp, crates/cli/src/cli.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/cli/src/cli.rs#GODOT-CLI-004, crates/task/src/task.rs#GODOT-CLI-004, crates/remote_server/src/main.rs#GODOT-CLI-004_
  - _Validation: cargo test -p cli -p task -p remote_server godot; run the CLI scenario for export or pack named presets from CLI and propagate template, toolchain, signing, progress, cancellation, and failure status in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 145. Close or verify GODOT-CLI-005: run a script or main loop, pass user arguments, select language, evaluate doctool/test modes, and exit deterministically
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 16.1, 16.2, 16.3, 16.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 144, 200_
  - _Reads: projects/godot/main/main.cpp, crates/cli/src/cli.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/cli/src/cli.rs#GODOT-CLI-005, crates/task/src/task.rs#GODOT-CLI-005, crates/remote_server/src/main.rs#GODOT-CLI-005_
  - _Validation: cargo test -p cli -p task -p remote_server godot; run the CLI scenario for run a script or main loop, pass user arguments, select language, evaluate doctool/test modes, and exit deterministically in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 146. Close or verify GODOT-CLI-006: enable remote debug, editor PID, breakpoints, profiler, GPU validation, crash handler, logging, and protocol ports
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 16.1, 16.2, 16.3, 16.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 145, 200_
  - _Reads: projects/godot/main/main.cpp, crates/cli/src/cli.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/cli/src/cli.rs#GODOT-CLI-006, crates/task/src/task.rs#GODOT-CLI-006, crates/remote_server/src/main.rs#GODOT-CLI-006_
  - _Validation: cargo test -p cli -p task -p remote_server godot; run the CLI scenario for enable remote debug, editor PID, breakpoints, profiler, GPU validation, crash handler, logging, and protocol ports in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 147. Close or verify GODOT-CLI-007: select rendering/audio/display drivers, GPU, screen, window mode, resolution, locale, time scale, and frame pacing
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 16.1, 16.2, 16.3, 16.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 146, 200_
  - _Reads: projects/godot/main/main.cpp, crates/cli/src/cli.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/cli/src/cli.rs#GODOT-CLI-007, crates/task/src/task.rs#GODOT-CLI-007, crates/remote_server/src/main.rs#GODOT-CLI-007_
  - _Validation: cargo test -p cli -p task -p remote_server godot; run the CLI scenario for select rendering/audio/display drivers, GPU, screen, window mode, resolution, locale, time scale, and frame pacing in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 148. Close or verify GODOT-CLI-008: print stable help, version, path, verbose, benchmark, and build-feature diagnostics without starting a project
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 16.1, 16.2, 16.3, 16.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 147, 200_
  - _Reads: projects/godot/main/main.cpp, crates/cli/src/cli.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/cli/src/cli.rs#GODOT-CLI-008, crates/task/src/task.rs#GODOT-CLI-008, crates/remote_server/src/main.rs#GODOT-CLI-008_
  - _Validation: cargo test -p cli -p task -p remote_server godot; run the CLI scenario for print stable help, version, path, verbose, benchmark, and build-feature diagnostics without starting a project in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 149. Close or verify GODOT-CLI-009: run dedicated-server exports and automation without editor-only services or interactive prompts
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 16.1, 16.2, 16.3, 16.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 148, 200_
  - _Reads: projects/godot/main/main.cpp, crates/cli/src/cli.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/cli/src/cli.rs#GODOT-CLI-009, crates/task/src/task.rs#GODOT-CLI-009, crates/remote_server/src/main.rs#GODOT-CLI-009_
  - _Validation: cargo test -p cli -p task -p remote_server godot; run the CLI scenario for run dedicated-server exports and automation without editor-only services or interactive prompts in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 150. Close or verify GODOT-SEC-001: confine res://, user://, temp, pack, import, extension, and export paths against traversal, symlink, and archive attacks
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 17.1, 17.2, 17.3, 17.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 1, 200_
  - _Reads: projects/godot/core/io/file_access.cpp, crates/sandbox/src/sandbox.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/sandbox/src/sandbox.rs#GODOT-SEC-001, crates/credentials_provider/src/credentials_provider.rs#GODOT-SEC-001, crates/extension_host/src/extension_host.rs#GODOT-SEC-001_
  - _Validation: cargo test -p sandbox -p credentials_provider -p extension_host godot_security; run the SEC scenario for confine res://, user://, temp, pack, import, extension, and export paths against traversal, symlink, and archive attacks in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 151. Close or verify GODOT-SEC-002: establish TLS trust from system/bundled/custom certificates and expose hostname, chain, expiry, and protocol failures
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 17.1, 17.2, 17.3, 17.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 150, 200_
  - _Reads: projects/godot/core/io/file_access.cpp, crates/sandbox/src/sandbox.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/sandbox/src/sandbox.rs#GODOT-SEC-002, crates/credentials_provider/src/credentials_provider.rs#GODOT-SEC-002, crates/extension_host/src/extension_host.rs#GODOT-SEC-002_
  - _Validation: cargo test -p sandbox -p credentials_provider -p extension_host godot_security; run the SEC scenario for establish TLS trust from system/bundled/custom certificates and expose hostname, chain, expiry, and protocol failures in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 152. Close or verify GODOT-SEC-003: request, explain, persist, revoke, and diagnose mobile camera, microphone, storage, network, notification, and XR permissions
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 17.1, 17.2, 17.3, 17.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 151, 200_
  - _Reads: projects/godot/core/io/file_access.cpp, crates/sandbox/src/sandbox.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/sandbox/src/sandbox.rs#GODOT-SEC-003, crates/credentials_provider/src/credentials_provider.rs#GODOT-SEC-003, crates/extension_host/src/extension_host.rs#GODOT-SEC-003_
  - _Validation: cargo test -p sandbox -p credentials_provider -p extension_host godot_security; run the SEC scenario for request, explain, persist, revoke, and diagnose mobile camera, microphone, storage, network, notification, and XR permissions in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 153. Close or verify GODOT-SEC-004: store export signing keys, passwords, tokens, and remote credentials through Zed secret facilities with redaction
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 17.1, 17.2, 17.3, 17.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 152, 200_
  - _Reads: projects/godot/core/io/file_access.cpp, crates/sandbox/src/sandbox.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/sandbox/src/sandbox.rs#GODOT-SEC-004, crates/credentials_provider/src/credentials_provider.rs#GODOT-SEC-004, crates/extension_host/src/extension_host.rs#GODOT-SEC-004_
  - _Validation: cargo test -p sandbox -p credentials_provider -p extension_host godot_security; run the SEC scenario for store export signing keys, passwords, tokens, and remote credentials through Zed secret facilities with redaction in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 154. Close or verify GODOT-SEC-005: gate @tool scripts, post-import scripts, GDExtension libraries, and EditorPlugins by explicit project trust and isolation policy
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 17.1, 17.2, 17.3, 17.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 153, 200_
  - _Reads: projects/godot/core/io/file_access.cpp, crates/sandbox/src/sandbox.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/sandbox/src/sandbox.rs#GODOT-SEC-005, crates/credentials_provider/src/credentials_provider.rs#GODOT-SEC-005, crates/extension_host/src/extension_host.rs#GODOT-SEC-005_
  - _Validation: cargo test -p sandbox -p credentials_provider -p extension_host godot_security; run the SEC scenario for gate @tool scripts, post-import scripts, GDExtension libraries, and EditorPlugins by explicit project trust and isolation policy in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 155. Close or verify GODOT-SEC-006: enforce browser sandbox, secure-context, cross-origin, CSP-like embedding, storage, clipboard, fullscreen, and thread prerequisites
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 17.1, 17.2, 17.3, 17.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 154, 200_
  - _Reads: projects/godot/core/io/file_access.cpp, crates/sandbox/src/sandbox.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/sandbox/src/sandbox.rs#GODOT-SEC-006, crates/credentials_provider/src/credentials_provider.rs#GODOT-SEC-006, crates/extension_host/src/extension_host.rs#GODOT-SEC-006_
  - _Validation: cargo test -p sandbox -p credentials_provider -p extension_host godot_security; run the SEC scenario for enforce browser sandbox, secure-context, cross-origin, CSP-like embedding, storage, clipboard, fullscreen, and thread prerequisites in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 156. Close or verify GODOT-SEC-007: bound resource parsing, decompression, image dimensions, archive entries, recursion, network bodies, queues, and worker memory/time
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 17.1, 17.2, 17.3, 17.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 155, 200_
  - _Reads: projects/godot/core/io/file_access.cpp, crates/sandbox/src/sandbox.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/sandbox/src/sandbox.rs#GODOT-SEC-007, crates/credentials_provider/src/credentials_provider.rs#GODOT-SEC-007, crates/extension_host/src/extension_host.rs#GODOT-SEC-007_
  - _Validation: cargo test -p sandbox -p credentials_provider -p extension_host godot_security; run the SEC scenario for bound resource parsing, decompression, image dimensions, archive entries, recursion, network bodies, queues, and worker memory/time in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 157. Close or verify GODOT-SEC-008: encrypt project data/scripts where configured and document integrity, key-management, and threat-model limitations
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 17.1, 17.2, 17.3, 17.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 156, 200_
  - _Reads: projects/godot/core/io/file_access.cpp, crates/sandbox/src/sandbox.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/sandbox/src/sandbox.rs#GODOT-SEC-008, crates/credentials_provider/src/credentials_provider.rs#GODOT-SEC-008, crates/extension_host/src/extension_host.rs#GODOT-SEC-008_
  - _Validation: cargo test -p sandbox -p credentials_provider -p extension_host godot_security; run the SEC scenario for encrypt project data/scripts where configured and document integrity, key-management, and threat-model limitations in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 158. Close or verify GODOT-PERSIST-001: round-trip project.godot and override.cfg sections, values, feature overrides, ordering/comments policy, and unknown settings
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 18.1, 18.2, 18.3, 18.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 1, 200_
  - _Reads: projects/godot/core/io/config_file.cpp, crates/settings/src/settings.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/settings/src/settings.rs#GODOT-PERSIST-001, crates/session/src/session.rs#GODOT-PERSIST-001, crates/workspace/src/persistence/model.rs#GODOT-PERSIST-001, crates/migrator/src/migrator.rs#GODOT-PERSIST-001_
  - _Validation: cargo test -p settings -p session -p workspace -p migrator godot_persistence; run the PERSIST scenario for round-trip project.godot and override.cfg sections, values, feature overrides, ordering/comments policy, and unknown settings in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 159. Close or verify GODOT-PERSIST-002: round-trip text and binary scene/resource formats with version, UID, dependency, unknown-field, and compatibility guarantees
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 18.1, 18.2, 18.3, 18.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 158, 200_
  - _Reads: projects/godot/core/io/config_file.cpp, crates/settings/src/settings.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/settings/src/settings.rs#GODOT-PERSIST-002, crates/session/src/session.rs#GODOT-PERSIST-002, crates/workspace/src/persistence/model.rs#GODOT-PERSIST-002, crates/migrator/src/migrator.rs#GODOT-PERSIST-002_
  - _Validation: cargo test -p settings -p session -p workspace -p migrator godot_persistence; run the PERSIST scenario for round-trip text and binary scene/resource formats with version, UID, dependency, unknown-field, and compatibility guarantees in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 160. Close or verify GODOT-PERSIST-003: persist import metadata, file cache, UID cache, editor filesystem state, and generated artifacts without treating them as source
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 18.1, 18.2, 18.3, 18.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 159, 200_
  - _Reads: projects/godot/core/io/config_file.cpp, crates/settings/src/settings.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/settings/src/settings.rs#GODOT-PERSIST-003, crates/session/src/session.rs#GODOT-PERSIST-003, crates/workspace/src/persistence/model.rs#GODOT-PERSIST-003, crates/migrator/src/migrator.rs#GODOT-PERSIST-003_
  - _Validation: cargo test -p settings -p session -p workspace -p migrator godot_persistence; run the PERSIST scenario for persist import metadata, file cache, UID cache, editor filesystem state, and generated artifacts without treating them as source in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 161. Close or verify GODOT-PERSIST-004: persist global editor settings, shortcuts, favorites, templates, asset-library state, and per-version migrations
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 18.1, 18.2, 18.3, 18.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 160, 200_
  - _Reads: projects/godot/core/io/config_file.cpp, crates/settings/src/settings.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/settings/src/settings.rs#GODOT-PERSIST-004, crates/session/src/session.rs#GODOT-PERSIST-004, crates/workspace/src/persistence/model.rs#GODOT-PERSIST-004, crates/migrator/src/migrator.rs#GODOT-PERSIST-004_
  - _Validation: cargo test -p settings -p session -p workspace -p migrator godot_persistence; run the PERSIST scenario for persist global editor settings, shortcuts, favorites, templates, asset-library state, and per-version migrations in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 162. Close or verify GODOT-PERSIST-005: persist per-project editor metadata, layouts, open scenes, folding, script breakpoints, run instances, and debugger state
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 18.1, 18.2, 18.3, 18.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 161, 200_
  - _Reads: projects/godot/core/io/config_file.cpp, crates/settings/src/settings.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/settings/src/settings.rs#GODOT-PERSIST-005, crates/session/src/session.rs#GODOT-PERSIST-005, crates/workspace/src/persistence/model.rs#GODOT-PERSIST-005, crates/migrator/src/migrator.rs#GODOT-PERSIST-005_
  - _Validation: cargo test -p settings -p session -p workspace -p migrator godot_persistence; run the PERSIST scenario for persist per-project editor metadata, layouts, open scenes, folding, script breakpoints, run instances, and debugger state in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 163. Close or verify GODOT-PERSIST-006: provide user:// ConfigFile, FileAccess, resource save, and save-game behavior across desktop/mobile/web storage
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 18.1, 18.2, 18.3, 18.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 162, 200_
  - _Reads: projects/godot/core/io/config_file.cpp, crates/settings/src/settings.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/settings/src/settings.rs#GODOT-PERSIST-006, crates/session/src/session.rs#GODOT-PERSIST-006, crates/workspace/src/persistence/model.rs#GODOT-PERSIST-006, crates/migrator/src/migrator.rs#GODOT-PERSIST-006_
  - _Validation: cargo test -p settings -p session -p workspace -p migrator godot_persistence; run the PERSIST scenario for provide user:// ConfigFile, FileAccess, resource save, and save-game behavior across desktop/mobile/web storage in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 164. Close or verify GODOT-PERSIST-007: perform atomic saves, backups, conflict detection, autosave, crash recovery, permission handling, and disk-full reporting
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 18.1, 18.2, 18.3, 18.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 163, 200_
  - _Reads: projects/godot/core/io/config_file.cpp, crates/settings/src/settings.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/settings/src/settings.rs#GODOT-PERSIST-007, crates/session/src/session.rs#GODOT-PERSIST-007, crates/workspace/src/persistence/model.rs#GODOT-PERSIST-007, crates/migrator/src/migrator.rs#GODOT-PERSIST-007_
  - _Validation: cargo test -p settings -p session -p workspace -p migrator godot_persistence; run the PERSIST scenario for perform atomic saves, backups, conflict detection, autosave, crash recovery, permission handling, and disk-full reporting in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 165. Close or verify GODOT-PERSIST-008: convert supported legacy projects/resources/settings with dry-run diagnostics, backups, idempotence, and explicit unsupported cases
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 18.1, 18.2, 18.3, 18.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 164, 200_
  - _Reads: projects/godot/core/io/config_file.cpp, crates/settings/src/settings.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/settings/src/settings.rs#GODOT-PERSIST-008, crates/session/src/session.rs#GODOT-PERSIST-008, crates/workspace/src/persistence/model.rs#GODOT-PERSIST-008, crates/migrator/src/migrator.rs#GODOT-PERSIST-008_
  - _Validation: cargo test -p settings -p session -p workspace -p migrator godot_persistence; run the PERSIST scenario for convert supported legacy projects/resources/settings with dry-run diagnostics, backups, idempotence, and explicit unsupported cases in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 166. Close or verify GODOT-PERSIST-009: publish and test a stable compatibility matrix for imported, edited, externally-run, and exported Godot versions
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 18.1, 18.2, 18.3, 18.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 165, 200_
  - _Reads: projects/godot/core/io/config_file.cpp, crates/settings/src/settings.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/settings/src/settings.rs#GODOT-PERSIST-009, crates/session/src/session.rs#GODOT-PERSIST-009, crates/workspace/src/persistence/model.rs#GODOT-PERSIST-009, crates/migrator/src/migrator.rs#GODOT-PERSIST-009_
  - _Validation: cargo test -p settings -p session -p workspace -p migrator godot_persistence; run the PERSIST scenario for publish and test a stable compatibility matrix for imported, edited, externally-run, and exported Godot versions in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 167. Close or verify GODOT-PLAT-001: run and export on Windows with native windows, input, IME, accessibility, gamepads, audio/MIDI, filesystem, registry, crash handling, signing, and D3D12/Vulkan/GLES
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 19.1, 19.2, 19.3, 19.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 1, 200_
  - _Reads: projects/godot/platform/windows/os_windows.cpp, crates/gpui_windows/src/gpui_windows.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/gpui_platform/src/gpui_platform.rs#GODOT-PLAT-001, crates/task/src/task.rs#GODOT-PLAT-001_
  - _Validation: cargo test -p gpui_platform -p task godot_platform; run the PLAT scenario for run and export on Windows with native windows, input, IME, accessibility, gamepads, audio/MIDI, filesystem, registry, crash handling, signing, and D3D12/Vulkan/GLES in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 168. Close or verify GODOT-PLAT-002: run and export on macOS with Cocoa windows, input/IME, accessibility, Metal/Vulkan/GLES, audio/MIDI, filesystem, menus, bundles, sandbox, signing, and notarization
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 19.1, 19.2, 19.3, 19.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 167, 200_
  - _Reads: projects/godot/platform/windows/os_windows.cpp, crates/gpui_windows/src/gpui_windows.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/gpui_platform/src/gpui_platform.rs#GODOT-PLAT-002, crates/task/src/task.rs#GODOT-PLAT-002_
  - _Validation: cargo test -p gpui_platform -p task godot_platform; run the PLAT scenario for run and export on macOS with Cocoa windows, input/IME, accessibility, Metal/Vulkan/GLES, audio/MIDI, filesystem, menus, bundles, sandbox, signing, and notarization in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 169. Close or verify GODOT-PLAT-003: run and export on Linux/BSD with X11 and Wayland variants, portals/DBus, input, accessibility/TTS, audio/MIDI, Vulkan/GLES, headless, packaging, and dynamic libraries
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 19.1, 19.2, 19.3, 19.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 168, 200_
  - _Reads: projects/godot/platform/windows/os_windows.cpp, crates/gpui_windows/src/gpui_windows.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/gpui_platform/src/gpui_platform.rs#GODOT-PLAT-003, crates/task/src/task.rs#GODOT-PLAT-003_
  - _Validation: cargo test -p gpui_platform -p task godot_platform; run the PLAT scenario for run and export on Linux/BSD with X11 and Wayland variants, portals/DBus, input, accessibility/TTS, audio/MIDI, Vulkan/GLES, headless, packaging, and dynamic libraries in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 170. Close or verify GODOT-PLAT-004: run and export on Android with editor/runtime variants, lifecycle, permissions, input/sensors, accessibility, audio, Vulkan/GLES, plugins, Gradle, APK/AAB, and remote deploy
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 19.1, 19.2, 19.3, 19.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 169, 200_
  - _Reads: projects/godot/platform/windows/os_windows.cpp, crates/gpui_windows/src/gpui_windows.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/gpui_platform/src/gpui_platform.rs#GODOT-PLAT-004, crates/task/src/task.rs#GODOT-PLAT-004_
  - _Validation: cargo test -p gpui_platform -p task godot_platform; run the PLAT scenario for run and export on Android with editor/runtime variants, lifecycle, permissions, input/sensors, accessibility, audio, Vulkan/GLES, plugins, Gradle, APK/AAB, and remote deploy in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 171. Close or verify GODOT-PLAT-005: run and export on iOS with lifecycle, permissions, touch/sensors, accessibility, audio, Metal, plugins, Xcode project, simulator/device, signing, and privacy manifests
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 19.1, 19.2, 19.3, 19.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 170, 200_
  - _Reads: projects/godot/platform/windows/os_windows.cpp, crates/gpui_windows/src/gpui_windows.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/gpui_platform/src/gpui_platform.rs#GODOT-PLAT-005, crates/task/src/task.rs#GODOT-PLAT-005_
  - _Validation: cargo test -p gpui_platform -p task godot_platform; run the PLAT scenario for run and export on iOS with lifecycle, permissions, touch/sensors, accessibility, audio, Metal, plugins, Xcode project, simulator/device, signing, and privacy manifests in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 172. Close or verify GODOT-PLAT-006: run and export on visionOS with spatial lifecycle, simulator/device, permissions, Metal, Xcode, signing, and OpenXR/spatial integration
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 19.1, 19.2, 19.3, 19.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 171, 200_
  - _Reads: projects/godot/platform/windows/os_windows.cpp, crates/gpui_windows/src/gpui_windows.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/gpui_platform/src/gpui_platform.rs#GODOT-PLAT-006, crates/task/src/task.rs#GODOT-PLAT-006_
  - _Validation: cargo test -p gpui_platform -p task godot_platform; run the PLAT scenario for run and export on visionOS with spatial lifecycle, simulator/device, permissions, Metal, Xcode, signing, and OpenXR/spatial integration in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 173. Close or verify GODOT-PLAT-007: run and export on Web with WASM, single-thread/pthread variants, browser input/display/audio, storage, networking, JavaScript, WebXR, PWA, and secure-context limits
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 19.1, 19.2, 19.3, 19.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 172, 200_
  - _Reads: projects/godot/platform/windows/os_windows.cpp, crates/gpui_windows/src/gpui_windows.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/gpui_platform/src/gpui_platform.rs#GODOT-PLAT-007, crates/task/src/task.rs#GODOT-PLAT-007_
  - _Validation: cargo test -p gpui_platform -p task godot_platform; run the PLAT scenario for run and export on Web with WASM, single-thread/pthread variants, browser input/display/audio, storage, networking, JavaScript, WebXR, PWA, and secure-context limits in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 174. Close or verify GODOT-PLAT-008: run headless and dedicated-server builds without window/audio dependencies and with deterministic exit, signals, and resource limits
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 19.1, 19.2, 19.3, 19.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 173, 200_
  - _Reads: projects/godot/platform/windows/os_windows.cpp, crates/gpui_windows/src/gpui_windows.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/gpui_platform/src/gpui_platform.rs#GODOT-PLAT-008, crates/task/src/task.rs#GODOT-PLAT-008_
  - _Validation: cargo test -p gpui_platform -p task godot_platform; run the PLAT scenario for run headless and dedicated-server builds without window/audio dependencies and with deterministic exit, signals, and resource limits in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 175. Close or verify GODOT-PLAT-009: run OpenXR, WebXR, and mobile VR interfaces with sessions, action maps, tracking, composition layers, spatial entities, permissions, and teardown
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 19.1, 19.2, 19.3, 19.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 174, 200_
  - _Reads: projects/godot/platform/windows/os_windows.cpp, crates/gpui_windows/src/gpui_windows.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/gpui_platform/src/gpui_platform.rs#GODOT-PLAT-009, crates/task/src/task.rs#GODOT-PLAT-009_
  - _Validation: cargo test -p gpui_platform -p task godot_platform; run the PLAT scenario for run OpenXR, WebXR, and mobile VR interfaces with sessions, action maps, tracking, composition layers, spatial entities, permissions, and teardown in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 176. Close or verify GODOT-PLAT-010: report unsupported consoles and out-of-tree platform ports as non-baseline rather than implying coverage
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 19.1, 19.2, 19.3, 19.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 175, 200_
  - _Reads: projects/godot/platform/windows/os_windows.cpp, crates/gpui_windows/src/gpui_windows.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/gpui_platform/src/gpui_platform.rs#GODOT-PLAT-010, crates/task/src/task.rs#GODOT-PLAT-010_
  - _Validation: cargo test -p gpui_platform -p task godot_platform; run the PLAT scenario for report unsupported consoles and out-of-tree platform ports as non-baseline rather than implying coverage in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 177. Close or verify GODOT-QA-001: run core, scene, server, module, editor, and platform unit tests with filters, tags, repeats, seeds, timing, and machine-readable exit status
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 20.1, 20.2, 20.3, 20.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 1, 200_
  - _Reads: projects/godot/tests/test_main.cpp, crates/project/tests/integration/project_tests.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/project/tests/integration/project_tests.rs#GODOT-QA-001, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv#GODOT-QA-001, .github/workflows/run_tests.yml#GODOT-QA-001_
  - _Validation: cargo test -p project godot_compat && ./script/clippy; run the QA scenario for run core, scene, server, module, editor, and platform unit tests with filters, tags, repeats, seeds, timing, and machine-readable exit status in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 178. Close or verify GODOT-QA-002: run resource/API compatibility tests against declared previous versions and detect removed/changed classes, methods, properties, signals, enums, and hashes
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 20.1, 20.2, 20.3, 20.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 177, 200_
  - _Reads: projects/godot/tests/test_main.cpp, crates/project/tests/integration/project_tests.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/project/tests/integration/project_tests.rs#GODOT-QA-002, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv#GODOT-QA-002, .github/workflows/run_tests.yml#GODOT-QA-002_
  - _Validation: cargo test -p project godot_compat && ./script/clippy; run the QA scenario for run resource/API compatibility tests against declared previous versions and detect removed/changed classes, methods, properties, signals, enums, and hashes in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 179. Close or verify GODOT-QA-003: exercise editor workflows, import/export fixtures, headless modes, crashes, recovery, and platform-specific behavior in integration tests
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 20.1, 20.2, 20.3, 20.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 178, 200_
  - _Reads: projects/godot/tests/test_main.cpp, crates/project/tests/integration/project_tests.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/project/tests/integration/project_tests.rs#GODOT-QA-003, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv#GODOT-QA-003, .github/workflows/run_tests.yml#GODOT-QA-003_
  - _Validation: cargo test -p project godot_compat && ./script/clippy; run the QA scenario for exercise editor workflows, import/export fixtures, headless modes, crashes, recovery, and platform-specific behavior in integration tests in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 180. Close or verify GODOT-QA-004: generate and validate class-reference documentation from bound APIs, examples, links, inheritance, and translations
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 20.1, 20.2, 20.3, 20.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 179, 200_
  - _Reads: projects/godot/tests/test_main.cpp, crates/project/tests/integration/project_tests.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/project/tests/integration/project_tests.rs#GODOT-QA-004, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv#GODOT-QA-004, .github/workflows/run_tests.yml#GODOT-QA-004_
  - _Validation: cargo test -p project godot_compat && ./script/clippy; run the QA scenario for generate and validate class-reference documentation from bound APIs, examples, links, inheritance, and translations in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 181. Close or verify GODOT-QA-005: provide source-backed user/developer docs for supported, divergent, decision-blocked, and excluded migration behavior
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 20.1, 20.2, 20.3, 20.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 180, 200_
  - _Reads: projects/godot/tests/test_main.cpp, crates/project/tests/integration/project_tests.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/project/tests/integration/project_tests.rs#GODOT-QA-005, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv#GODOT-QA-005, .github/workflows/run_tests.yml#GODOT-QA-005_
  - _Validation: cargo test -p project godot_compat && ./script/clippy; run the QA scenario for provide source-backed user/developer docs for supported, divergent, decision-blocked, and excluded migration behavior in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 182. Close or verify GODOT-QA-006: preserve fixture, icon, font, sample, test-data, and converted-output attribution and license metadata
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 20.1, 20.2, 20.3, 20.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 181, 200_
  - _Reads: projects/godot/tests/test_main.cpp, crates/project/tests/integration/project_tests.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/project/tests/integration/project_tests.rs#GODOT-QA-006, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv#GODOT-QA-006, .github/workflows/run_tests.yml#GODOT-QA-006_
  - _Validation: cargo test -p project godot_compat && ./script/clippy; run the QA scenario for preserve fixture, icon, font, sample, test-data, and converted-output attribution and license metadata in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 183. Close or verify GODOT-QA-007: build Android, iOS, Linux, macOS, Web, and Windows matrices plus static checks with explicit options and optional modules
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 20.1, 20.2, 20.3, 20.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 182, 200_
  - _Reads: projects/godot/tests/test_main.cpp, crates/project/tests/integration/project_tests.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/project/tests/integration/project_tests.rs#GODOT-QA-007, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv#GODOT-QA-007, .github/workflows/run_tests.yml#GODOT-QA-007_
  - _Validation: cargo test -p project godot_compat && ./script/clippy; run the QA scenario for build Android, iOS, Linux, macOS, Web, and Windows matrices plus static checks with explicit options and optional modules in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 184. Close or verify GODOT-QA-008: run formatting, header, documentation, API, shader, generated-file, sanitizers, warnings, licenses, and dependency checks
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 20.1, 20.2, 20.3, 20.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 183, 200_
  - _Reads: projects/godot/tests/test_main.cpp, crates/project/tests/integration/project_tests.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/project/tests/integration/project_tests.rs#GODOT-QA-008, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv#GODOT-QA-008, .github/workflows/run_tests.yml#GODOT-QA-008_
  - _Validation: cargo test -p project godot_compat && ./script/clippy; run the QA scenario for run formatting, header, documentation, API, shader, generated-file, sanitizers, warnings, licenses, and dependency checks in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 185. Close or verify GODOT-QA-009: distinguish external demo projects and tutorials from engine source capabilities and port only examples needed to verify supported behavior
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 20.1, 20.2, 20.3, 20.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 184, 200_
  - _Reads: projects/godot/tests/test_main.cpp, crates/project/tests/integration/project_tests.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/project/tests/integration/project_tests.rs#GODOT-QA-009, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv#GODOT-QA-009, .github/workflows/run_tests.yml#GODOT-QA-009_
  - _Validation: cargo test -p project godot_compat && ./script/clippy; run the QA scenario for distinguish external demo projects and tutorials from engine source capabilities and port only examples needed to verify supported behavior in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 186. Close or verify GODOT-MOD-001: resolve all 55 built-in modules and custom modules by default, explicit module flags, dependencies, can_build, platform, architecture, and build profile
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 21.1, 21.2, 21.3, 21.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 1, 200_
  - _Reads: projects/godot/SConstruct, Cargo.toml, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/system_specs/src/system_specs.rs#GODOT-MOD-001, crates/task/src/task.rs#GODOT-MOD-001, Cargo.toml#GODOT-MOD-001_
  - _Validation: cargo test -p system_specs -p task godot_features; run the MOD scenario for resolve all 55 built-in modules and custom modules by default, explicit module flags, dependencies, can_build, platform, architecture, and build profile in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 187. Close or verify GODOT-MOD-002: enable GDScript and common codec/text/network/import modules by default only when their dependencies and product profile permit
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 21.1, 21.2, 21.3, 21.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 186, 200_
  - _Reads: projects/godot/SConstruct, Cargo.toml, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/system_specs/src/system_specs.rs#GODOT-MOD-002, crates/task/src/task.rs#GODOT-MOD-002, Cargo.toml#GODOT-MOD-002_
  - _Validation: cargo test -p system_specs -p task godot_features; run the MOD scenario for enable GDScript and common codec/text/network/import modules by default only when their dependencies and product profile permit in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 188. Close or verify GODOT-MOD-003: keep Mono/C# and fallback text server opt-in and expose build/runtime/tooling prerequisites
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 21.1, 21.2, 21.3, 21.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 187, 200_
  - _Reads: projects/godot/SConstruct, Cargo.toml, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/system_specs/src/system_specs.rs#GODOT-MOD-003, crates/task/src/task.rs#GODOT-MOD-003, Cargo.toml#GODOT-MOD-003_
  - _Validation: cargo test -p system_specs -p task godot_features; run the MOD scenario for keep Mono/C# and fallback text server opt-in and expose build/runtime/tooling prerequisites in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 189. Close or verify GODOT-MOD-004: select Godot Physics 2D/3D, Jolt, navigation, OpenXR, WebXR, mobile VR, raycast, and lightmapper modules by subsystem/platform flags
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 21.1, 21.2, 21.3, 21.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 188, 200_
  - _Reads: projects/godot/SConstruct, Cargo.toml, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/system_specs/src/system_specs.rs#GODOT-MOD-004, crates/task/src/task.rs#GODOT-MOD-004, Cargo.toml#GODOT-MOD-004_
  - _Validation: cargo test -p system_specs -p task godot_features; run the MOD scenario for select Godot Physics 2D/3D, Jolt, navigation, OpenXR, WebXR, mobile VR, raycast, and lightmapper modules by subsystem/platform flags in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 190. Close or verify GODOT-MOD-005: select image/audio/video/texture/mesh/import codecs and builtin-versus-system third-party implementations with license and feature effects
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 21.1, 21.2, 21.3, 21.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 189, 200_
  - _Reads: projects/godot/SConstruct, Cargo.toml, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/system_specs/src/system_specs.rs#GODOT-MOD-005, crates/task/src/task.rs#GODOT-MOD-005, Cargo.toml#GODOT-MOD-005_
  - _Validation: cargo test -p system_specs -p task godot_features; run the MOD scenario for select image/audio/video/texture/mesh/import codecs and builtin-versus-system third-party implementations with license and feature effects in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 191. Close or verify GODOT-MOD-006: select Vulkan, GLES3, D3D12, Metal, ANGLE, AccessKit, SDL, audio, MIDI, display, and profiler drivers by build and platform
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 21.1, 21.2, 21.3, 21.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 190, 200_
  - _Reads: projects/godot/SConstruct, Cargo.toml, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/system_specs/src/system_specs.rs#GODOT-MOD-006, crates/task/src/task.rs#GODOT-MOD-006, Cargo.toml#GODOT-MOD-006_
  - _Validation: cargo test -p system_specs -p task godot_features; run the MOD scenario for select Vulkan, GLES3, D3D12, Metal, ANGLE, AccessKit, SDL, audio, MIDI, display, and profiler drivers by build and platform in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 192. Close or verify GODOT-MOD-007: apply disable_3d, advanced GUI, physics, navigation, XR, overrides, path overrides, threads, precision, deprecated, and production options consistently
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 21.1, 21.2, 21.3, 21.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 191, 200_
  - _Reads: projects/godot/SConstruct, Cargo.toml, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/system_specs/src/system_specs.rs#GODOT-MOD-007, crates/task/src/task.rs#GODOT-MOD-007, Cargo.toml#GODOT-MOD-007_
  - _Validation: cargo test -p system_specs -p task godot_features; run the MOD scenario for apply disable_3d, advanced GUI, physics, navigation, XR, overrides, path overrides, threads, precision, deprecated, and production options consistently in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 193. Close or verify GODOT-MOD-008: generate module registration, enabled defines, extension API, docs, tests, and build outputs from the same resolved feature set
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 21.1, 21.2, 21.3, 21.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 192, 200_
  - _Reads: projects/godot/SConstruct, Cargo.toml, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: crates/system_specs/src/system_specs.rs#GODOT-MOD-008, crates/task/src/task.rs#GODOT-MOD-008, Cargo.toml#GODOT-MOD-008_
  - _Validation: cargo test -p system_specs -p task godot_features; run the MOD scenario for generate module registration, enabled defines, extension API, docs, tests, and build outputs from the same resolved feature set in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 194. Close or verify GODOT-UPSTREAM-001: track vendored libraries, versions, patches, licenses, notices, security updates, and builtin/system selection that affect shipped behavior
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 22.1, 22.2, 22.3, 22.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 1, 200_
  - _Reads: projects/godot/thirdparty/README.md, script/check-licenses, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: script/check-licenses#GODOT-UPSTREAM-001, script/generate-licenses#GODOT-UPSTREAM-001, tooling/compliance/src/lib.rs#GODOT-UPSTREAM-001_
  - _Validation: ./script/check-licenses && cargo test -p compliance godot; run the UPSTREAM scenario for track vendored libraries, versions, patches, licenses, notices, security updates, and builtin/system selection that affect shipped behavior in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 195. Close or verify GODOT-UPSTREAM-002: generate bindings, extension APIs, docs, shaders, fonts, icons, translations, platform templates, and registration sources reproducibly
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 22.1, 22.2, 22.3, 22.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 194, 200_
  - _Reads: projects/godot/thirdparty/README.md, script/check-licenses, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: script/check-licenses#GODOT-UPSTREAM-002, script/generate-licenses#GODOT-UPSTREAM-002, tooling/compliance/src/lib.rs#GODOT-UPSTREAM-002_
  - _Validation: ./script/check-licenses && cargo test -p compliance godot; run the UPSTREAM scenario for generate bindings, extension APIs, docs, shaders, fonts, icons, translations, platform templates, and registration sources reproducibly in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 196. Close or verify GODOT-UPSTREAM-003: maintain SCons helpers, compiler/linker probes, caches, SCU/Ninja/compile-db support, and platform toolchain integration
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 22.1, 22.2, 22.3, 22.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 195, 200_
  - _Reads: projects/godot/thirdparty/README.md, script/check-licenses, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: script/check-licenses#GODOT-UPSTREAM-003, script/generate-licenses#GODOT-UPSTREAM-003, tooling/compliance/src/lib.rs#GODOT-UPSTREAM-003_
  - _Validation: ./script/check-licenses && cargo test -p compliance godot; run the UPSTREAM scenario for maintain SCons helpers, compiler/linker probes, caches, SCU/Ninja/compile-db support, and platform toolchain integration in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 197. Close or verify GODOT-UPSTREAM-004: maintain upstream CI, packaging, signing, release, update-check, and artifact-publishing workflows separately from product parity
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 22.1, 22.2, 22.3, 22.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 196, 200_
  - _Reads: projects/godot/thirdparty/README.md, script/check-licenses, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: script/check-licenses#GODOT-UPSTREAM-004, script/generate-licenses#GODOT-UPSTREAM-004, tooling/compliance/src/lib.rs#GODOT-UPSTREAM-004_
  - _Validation: ./script/check-licenses && cargo test -p compliance godot; run the UPSTREAM scenario for maintain upstream CI, packaging, signing, release, update-check, and artifact-publishing workflows separately from product parity in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 198. Close or verify GODOT-UPSTREAM-005: treat imported documentation, examples, test fixtures, and generated files as evidence until a connected Zed behavior consumes them
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 22.1, 22.2, 22.3, 22.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 197, 200_
  - _Reads: projects/godot/thirdparty/README.md, script/check-licenses, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: script/check-licenses#GODOT-UPSTREAM-005, script/generate-licenses#GODOT-UPSTREAM-005, tooling/compliance/src/lib.rs#GODOT-UPSTREAM-005_
  - _Validation: ./script/check-licenses && cargo test -p compliance godot; run the UPSTREAM scenario for treat imported documentation, examples, test fixtures, and generated files as evidence until a connected Zed behavior consumes them in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 199. Close or verify GODOT-UPSTREAM-006: reuse Zed license, dependency, compliance, CI, documentation, and release infrastructure instead of porting Godot's equivalents
  - Apply the cataloged native Zed owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Zed behavior and hermetic no-Godot evidence.
  - _Requirements: 22.1, 22.2, 22.3, 22.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 198, 200_
  - _Reads: projects/godot/thirdparty/README.md, script/check-licenses, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Writes: script/check-licenses#GODOT-UPSTREAM-006, script/generate-licenses#GODOT-UPSTREAM-006, tooling/compliance/src/lib.rs#GODOT-UPSTREAM-006_
  - _Validation: ./script/check-licenses && cargo test -p compliance godot; run the UPSTREAM scenario for reuse Zed license, dependency, compliance, CI, documentation, and release infrastructure instead of porting Godot's equivalents in a hermetic environment with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked._

- [ ] 200. Enforce the native Zed implementation gate across the Godot migration
  - Audit every migration requirement, design, task, dependency proposal, and catalog row for embedding, bundling, invocation, linkage, wrappers, hidden instances, external delegation, source copying, duplicate Godot-specific owners, placeholder-only support, and missing no-Godot validation. Keep material product, compatibility, licensing, and architecture choices in `decisions.md`.
  - _Requirements: 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_
  - _Depends on: 1_
  - _Reads: .agents/specs/godot-migration/**, Cargo.toml, Cargo.lock, deny.toml, projects/godot/COPYRIGHT.txt, projects/godot/thirdparty/README.md_
  - _Writes: .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv, .agents/specs/godot-migration/godot-full-port-coverage/findings.md, .agents/specs/godot-migration/godot-full-port-coverage/decisions.md, .agents/specs/godot-migration/godot-full-port-coverage/validation-results.md, .agents/specs/godot-migration/**/requirements.md, .agents/specs/godot-migration/**/design.md, .agents/specs/godot-migration/**/tasks.md_
  - _Validation: python3 .agents/specs/godot-migration/godot-full-port-coverage/validate_audit.py; validate all feature-spec packs; assert zero checked tasks; assert supported/fully specified rows include hermetic execution and package/link/process inspection with Godot absent_
