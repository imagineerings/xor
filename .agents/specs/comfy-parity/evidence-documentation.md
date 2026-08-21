# Documentation and embedded-docs evidence

## Baselines

The documentation repository declares no project version. Its package manifest
contains tooling dependencies and scripts only. The canonical fingerprint covers
5,800 files after excluding the discovered `.DS_Store` OS artifact:
`1f4c9c460b8f5b35e30eb4d2d64bc201a958f247ab21af6c68743cce28c33931`. The lockfile resolves Mintlify `mint` 4.2.585, `sharp`
0.33.5, and `@executeautomation/playwright-mcp-server` 1.0.12.

Embedded docs declares version 0.5.7 and has 10,298 source files
after excluding the discovered Python bytecode/cache artifact. Its fingerprint is
`5aebf925cf36fe7b8df3c89466ad96ffa42110542a392ec6156b88fc807ec956`. ComfyUI `requirements.txt` pins
`comfyui-embedded-docs==0.5.6`; therefore the added 0.5.7 tree is a separate,
version-skewed evidence source rather than the exact package pinned by the
ComfyUI snapshot.

## Evidence discipline

English MDX is the documentation source of truth. Translations, CMS staging,
README prose, and generated node help remain `documented-only` unless a catalog
row names executable source or a test. Route-shape or identifier matches do not
corroborate cloud semantics, defaults, errors, billing, retention, or lifecycle.
The production design must never execute Python or JavaScript from these sources.
Legacy extension claims map to versioned Rust/WASM descriptors, explicit host
ports, and legacy identifier/port migrations.

## Docs source coverage

The 5,800-row source ledger is reconciled as follows:

| Disposition | Files |
| --- | ---: |
| CI workflow | 9 |
| CMS staging content | 14 |
| English built-in-node documentation | 896 |
| English product documentation | 307 |
| English reusable snippet | 56 |
| configuration/schema/lock/registry | 20 |
| executable automation/tooling | 45 |
| governance/tool documentation | 16 |
| localized generated content | 3,723 |
| media asset | 708 |
| repository/site infrastructure | 6 |

The 1,273 non-primary-translation MDX records comprise 896 built-in-node
references, 307 English product pages, 56 English snippets, two English CMS
staging files, and twelve localized CMS staging files. The 307 product pages are
split by domain in `docs-reconciliation.json`; tutorials (139), custom nodes
(36), Registry (31), development (21), interface (19), and installation (16)
are the largest groups.

`docs.json` defines four languages and six tabs per language. It has 65 redirects.
English, Chinese, and Japanese each have 1,166 unique navigation references;
Korean has 1,120. Twelve English CLIP-related navigation paths differ from their
actual filenames only by case, which is a portability risk on case-sensitive
filesystems. English has 119 MDX files not directly listed in navigation: 56
snippets, 24 Registry API pages, 18 built-in-node pages, 14 CMS staging files,
and seven other pages.

Japanese and Chinese each have 1,202 page translations plus 56 snippet
translations and each lacks `tutorials/partner-nodes/ideogram/ideogram-v3`.
Korean has 1,151 pages plus 56 snippets, with 64 exact missing paths and 12
case-only extras. The full path lists are machine-readable.

## Embedded node documentation

All 855 node directories contain the same twelve locale files (`ar`, `en`,
`es`, `fa`, `fr`, `ja`, `ko`, `pt-BR`, `ru`, `tr`, `zh`, `zh-TW`), yielding
10,260 Markdown files, plus 23 visual media assets and one JSON ancillary asset.
Every English file contains the
AI-generated-content marker.

Registry reconciliation is:

| Match | Node documents |
| --- | ---: |
| frontend-consumer-without-snapshot-provider | 4 |
| frontend-native-virtual-node | 3 |
| legacy-node-replacement | 2 |
| provider-unverified | 38 |
| registered-class-name-exact | 10 |
| registered-node-id-case-or-punctuation | 41 |
| registered-node-id-exact | 756 |
| unregistered-executable-class | 1 |

The 848 embedded records represented in the docs site split into 710 matching
declared source fingerprints, one mismatch (`CreateBoundingBoxes`), and 137
records without comparable fingerprints. Seven embedded records are absent from
the docs site. The docs site additionally contains one overview and 47 nested
legacy/partner node pages.

The following 38 embedded node claims have no
registered backend identifier/class, frontend-native virtual-node registration,
explicit replacement, unregistered executable class, or conditional frontend
consumer corroboration in this baseline:

- `ByteDanceImageEditNode`
- `DeprecatedCheckpointLoader`
- `DeprecatedDiffusersLoader`
- `FluxProCannyNode`
- `FluxProDepthNode`
- `FluxProImageNode`
- `IdeogramV1`
- `IdeogramV2`
- `LoadImageSetFromFolderNode`
- `LoadImageSetNode`
- `LoadImageTextSetFromFolderNode`
- `MoonvalleyImg2VideoNode`
- `MoonvalleyTxt2VideoNode`
- `MoonvalleyVideo2VideoNode`
- `PikaImageToVideoNode2_2`
- `PikaScenesV2_2`
- `PikaStartEndFrameNode2_2`
- `PikaTextToVideoNode2_2`
- `Pikadditions`
- `Pikaffects`
- `Pikaswaps`
- `SaveLoRANode`
- `SeedVR2Conditioning`
- `SeedVR2PostProcessing`
- `SeedVR2Preprocess`
- `SeedVR2ProgressiveSampler`
- `SeedVR2TemporalChunk`
- `SeedVR2TemporalMerge`
- `StabilityAudioInpaint`
- `StabilityAudioToAudio`
- `StabilityStableImageSD_3_5Node`
- `StabilityStableImageUltraNode`
- `StabilityTextToAudio`
- `StabilityUpscaleConservativeNode`
- `StabilityUpscaleCreativeNode`
- `StabilityUpscaleFastNode`
- `TerminalLog`
- `TextOverlay`

They remain provider-unverified and `documented-only`; they must not be promoted
to active native nodes without an executable schema or a deliberate compatibility
decision.

## Cloud OpenAPI

`openapi-cloud.yaml` declares OpenAPI 3.0.3 and API info version 1.0.0. It has
34 paths, 42 operations, 56 schemas, one API-key security scheme, and eight tags.
Thirty-nine method/path shapes occur in the backend and/or frontend executable
catalogs. The uncorroborated shapes are `GET /api/assets/remote-metadata`,
`POST /api/assets/download`, and `PUT /api/assets/{id}`. All cloud behavior
remains experimental, cloud/paid, and documented-only even where a route shape
matches.

## Commands, configuration, formats, lifecycle, and extensions

The tooling catalog has 108 rows: 28 package scripts, 41 distinct static tool
flag literals, 30 tooling environment variables, and nine CI workflows. These
are developer/infrastructure behavior, not production Zed flags. The
configuration/format catalog has 20 source configuration/schema/lock/registry
files and 15 documented format contracts.

The extension catalog has 56 contracts: seven Python V1 legacy contracts,
seventeen Python V3 legacy contracts, all 27 executable frontend extension
interface members, and five embedded-documentation contracts. Each row names a
native Rust/WASM port and marks production legacy execution prohibited. The
lifecycle ledger has 20 separately testable documented transitions, including
load failure, replacement ordering, async jobs, interrupted tooling recovery,
version skew, token/asset expiry claims, redirects, and package publication.

## Observed validation

- `python3 .github/scripts/validate-links.py --check`: 4,988 documentation
  files checked, pass.
- `bun test ./.github/scripts/i18n/chunked-translate.test.ts
  ./.github/scripts/i18n/repair-fences.test.ts`: 8 pass, 0 fail, 17 assertions.
- `bun .github/scripts/i18n/check-translation-truncation.ts`: 51 issues
  observed. The checker-generated gitignored reports were removed, and the
  canonical fingerprint was reverified.
- `python3 .github/scripts/check_md_links.py` in embedded docs: all local
  resource links pass.

## Generated catalogs

- `catalogs/docs-pages.csv`
- `catalogs/docs-node-docs.csv`
- `catalogs/embedded-docs-nodes.csv`
- `catalogs/docs-openapi-cloud.csv`
- `catalogs/docs-redirects.csv`
- `catalogs/docs-tooling.csv`
- `catalogs/docs-config-formats.csv`
- `catalogs/docs-extension-contracts.csv`
- `catalogs/docs-lifecycle-contracts.csv`
- `catalogs/docs-source-coverage.csv`
- `catalogs/embedded-docs-source-coverage.csv`
- `catalogs/docs-reconciliation.json`
