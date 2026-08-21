# Requirements: Rendering and Media

## Introduction

Zed should preview relevant Godot media and world-model outputs without porting Godot rendering, audio, video, or text stacks.

### Requirement 1: Do Not Port Render Backends

#### Acceptance Criteria

1. **1.1** IF a feature requires Vulkan, D3D12, Metal, GLES, render server, audio server, or text server migration THEN THE system SHALL classify it as excluded.

### Requirement 2: Media Preview Routing

#### Acceptance Criteria

1. **2.1** WHEN a supported texture, image, shader, generated frame, or video is selected THEN THE system SHALL route it to existing Zed preview infrastructure.
2. **2.2** IF preview is unsupported THEN THE system SHALL show an unsupported-preview reason.

### Requirement 3: Generated Media

#### Acceptance Criteria

1. **3.1** WHEN world-model media is imported THEN THE system SHALL attach provenance.

### Requirement 9: Native Zed Ownership

#### Acceptance Criteria

1. **9.1** Supported preview, rendering, media, UI, storage, cancellation, and lifecycle behavior SHALL be owned by existing `gpui`, `gpui_wgpu`, `image_viewer`, `component_preview`, `media`, or `audio` components as applicable.
2. **9.2** THE system SHALL NOT embed, launch, wrap, proxy, link, or communicate with Godot render, audio, video, text, or particle servers.
3. **9.3** Godot-compatible media, texture, shader, material, and scene data MAY cross import/export boundaries, but decoded resources and execution state SHALL be Zed-native.
4. **9.4** A file classifier, preview route, shader declaration, interface, or placeholder SHALL NOT count as runtime rendering/media support.
5. **9.5** Validation for every supported or fully specified capability SHALL run with Godot absent and inspect package, process, loader, and dependency state.
