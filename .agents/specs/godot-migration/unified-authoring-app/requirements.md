# Requirements: Unified Authoring App

## Introduction

Sim should provide a unified cross-platform game authoring application for game project assets, diffusion graphs, world-model runtime previews, and generated artifacts.

### Requirement 1: Unified Workspace

#### Acceptance Criteria

1.1 WHEN a game project opens THEN THE system SHALL present project assets, graphs, world-model requests, generated artifacts, and run/export tasks in one workspace model.
1.2 WHEN an item is selected THEN THE system SHALL route it to the correct editor, preview, inspector, or task view.

### Requirement 2: Runtime Preview

#### Acceptance Criteria

2.1 WHEN world-model preview is requested THEN THE system SHALL use worker diagnostics and generated artifact provenance.
2.2 IF preview cannot run THEN THE system SHALL show actionable diagnostics.
