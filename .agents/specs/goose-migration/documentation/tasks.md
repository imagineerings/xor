# Tasks: Goose Documentation Coverage in Zed

- [ ] 1. Build a documentation claim matrix from approved migration capabilities
  - Map each user-facing capability to setup, configuration, use, failures, security/privacy, platform gates, compatibility, examples, implementation evidence, and verification
  - Reject or label planned behavior that has no implementation evidence
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 5.3_
  - _Depends on: implementation and verification of the referenced migration capability_
  - _Reads: requirements.md, design.md, ../coverage-catalog.md, every approved migration requirements/design/tasks pack, affected Zed implementation/tests_
  - _Writes: documentation claim matrix and selected `docs/src/` pages_
  - _Validation: Review every published behavior claim against an implementation symbol and passing verification result_

- [ ] 2. Add approved migration content to the existing mdBook hierarchy
  - Extend `docs/src` and `docs/SUMMARY.md`; preserve Zed terminology, voice, navigation, and release-channel conventions
  - Cover providers, extensions, recipes, authentication, scheduling, gateways, local models, and embedded apps only when approved and implemented
  - _Requirements: 1.1, 1.2, 1.3, 2.1, 2.2, 2.4_
  - _Depends on: 1_
  - _Reads: requirements.md, design.md, docs/.rules, docs/book.toml, docs/SUMMARY.md, docs/src/_
  - _Writes: docs/SUMMARY.md, selected docs/src/_
  - _Validation: Build mdBook and review navigation, terminology, release-channel visibility, platform gates, and unavailable-feature claims_

- [ ] 3. Add and validate runnable examples and approved tutorials
  - Use existing documentation patterns and test fixtures; declare prerequisites, output, failure behavior, and owner
  - Do not import Goose blog, branding, author/tag, community, or marketing assets without separate approval
  - _Requirements: 1.4, 4.1, 4.2, 4.3_
  - _Depends on: 1, 2_
  - _Reads: requirements.md, design.md, projects/goose/documentation/docs/, projects/goose/documentation/blog/, docs/src/, affected public APIs and tests_
  - _Writes: selected docs/src tutorial/example pages_
  - _Validation: Run every documented command/example against a fixture or integration test and review explicit content exclusions_

- [ ] 4. Decide and, if approved, generate machine-consumable documentation artifacts
  - Identify whether docs maps, `llms.txt`, server catalogs, or skills manifests provide required Zed behavior
  - Generate approved artifacts from canonical metadata with determinism, validation, atomic replacement, and private-data checks
  - _Requirements: 3.1, 3.2, 3.3_
  - _Depends on: 1_
  - _Reads: requirements.md, design.md, projects/goose/documentation/scripts/, projects/goose/documentation/static/, Zed canonical command/provider/skill/server metadata_
  - _Writes: approved generator and public artifact locations_
  - _Validation: Run deterministic regeneration, invalid-source, stale-output, secret/private-endpoint, unpublished-feature, and local-path tests_

- [ ] 5. Integrate documentation validation with existing automation
  - Reuse formatter, mdBook/preprocessor build, link checks, and generated deploy workflow
  - Do not add Docusaurus, a parallel site, or a parallel deployment workflow
  - _Requirements: 2.3, 5.1, 5.2, 5.3_
  - _Depends on: 2, 3, 4_
  - _Reads: requirements.md, design.md, script/prettier, script/check-links, crates/docs_preprocessor/, .github/workflows/deploy_docs.yml, .github/workflows/run_tests.yml_
  - _Writes: existing docs validation/generation integration points only where a confirmed gap exists_
  - _Validation: Run `script/prettier`, mdBook with preprocessors, applicable link checks, generated workflow checks, and the claim-evidence traceability audit_
