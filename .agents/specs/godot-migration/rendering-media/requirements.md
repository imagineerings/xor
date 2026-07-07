# Requirements: Rendering and Media

## Introduction

Sim should preview relevant Godot media and world-model outputs without porting Godot rendering, audio, video, or text stacks.

### Requirement 1: Do Not Port Render Backends

#### Acceptance Criteria

1.1 IF a feature requires Vulkan, D3D12, Metal, GLES, render server, audio server, or text server migration THEN THE system SHALL classify it as excluded.

### Requirement 2: Media Preview Routing

#### Acceptance Criteria

2.1 WHEN a supported texture, image, shader, generated frame, or video is selected THEN THE system SHALL route it to existing Sim preview infrastructure.
2.2 IF preview is unsupported THEN THE system SHALL show an unsupported-preview reason.

### Requirement 3: Generated Media

#### Acceptance Criteria

3.1 WHEN world-model media is imported THEN THE system SHALL attach provenance.
