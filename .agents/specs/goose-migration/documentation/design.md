# Design: Goose Documentation Coverage in Zed

## Ownership and reuse

- **D-DOCS-ARCH:** mdBook under `docs/` remains the sole documentation site. Extend `SUMMARY.md`, content, preprocessors, and current deployment.
- **D-DOCS-CAPABILITY:** Documentation is written per approved capability and uses the coverage catalog as the claim inventory. Pages name configuration, platform gates, permissions, security, failures, compatibility, and verification.
- **D-DOCS-GENERATED:** Any required docs map, server catalog, skills manifest, or model-readable index is generated deterministically from canonical Zed metadata and checked for private data.
- **D-DOCS-CONTENT-BOUNDARY:** Docusaurus themes/plugins, blog/community marketing, Goose brand/legal assets, and deployment choices are not portable behavior. Tutorials are added only for approved public workflows.
- **D-DOCS-VALIDATION:** Existing formatter, mdBook/preprocessor build, link checks, and generated workflow own validation and release channels.

## Failure behavior

Broken internal/action links, invalid front matter/metadata, failed preprocessing, stale generated files, or a claim without implementation evidence fails validation. External-link network instability follows the existing CI policy. Generated artifacts are replaced atomically only after successful generation.

## Requirements traceability

| Requirement | Design element | Verification |
| --- | --- | --- |
| 1.1, 1.2, 1.3, 1.4 | D-DOCS-CAPABILITY | Claim-evidence review, feature-gate/platform/security checklist, and runnable example tests |
| 2.1, 2.2, 2.3, 2.4 | D-DOCS-ARCH | mdBook/preprocessor/navigation/search/deployment validation and no-Docusaurus check |
| 3.1, 3.2, 3.3 | D-DOCS-GENERATED | Determinism, invalid-source, stale-output, and private-data tests |
| 4.1, 4.2, 4.3 | D-DOCS-CONTENT-BOUNDARY | Tutorial fixture tests and explicit exclusion review |
| 5.1, 5.2, 5.3 | D-DOCS-VALIDATION | Existing formatter/build/link/workflow checks and traceability audit |

## Open decisions

1. Which Goose tutorials merit Zed-native equivalents after the corresponding product capabilities are approved.
2. Whether Zed needs public machine-consumable docs artifacts beyond its current site/search outputs.
3. Whether blog/community marketing content is owned in this repository; this migration does not assume it is.
