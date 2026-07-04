# Implementation Plan: Spectrum 2 UI Language

## Overview

This plan covers 7 implementation steps across settings, cursor behavior, icon theme, border radius, spacing, cursor blink, and documentation. Steps are ordered so that each builds on the previous one.

The work is divided into three workstreams:
- **Settings & Cursor** (Steps 1-2, 6) — Rust code in `theme_settings` and `editor`
- **Icon Theme Extension** (Step 3) — Extension files with SVG icons
- **UI Component Updates** (Steps 4-5) — Rust code in `ui` components
- **Docs** (Step 7) — Update appearance docs

## Tasks

- [x] 1. Add Spectrum 2 font defaults settings proxy
  - Add `spectrum2_defaults()` function to `crates/theme_settings/src/theme_settings.rs`
    - Detects if active theme name contains "Spectrum 2 Inspired"
    - Returns `ThemeSettingsContent` with Inter (UI), SF Mono/JetBrains Mono (buffer), 14px/15px sizes, 1.3 line height
    - Respects existing user overrides (only applies when user hasn't set the value)
  - Call `spectrum2_defaults()` in `configured_theme()` to merge defaults into resolved settings
  - Add font feature defaults (`calt` for UI font, `liga` for buffer font)
  - _Requirements: 1, 2_
  - _writes: crates/theme_settings/src/theme_settings.rs_

- [x] 2. Add cursor style setting and Spectrum 2 defaults
  - Add `CursorStyle` enum to `crates/settings_content/src/theme.rs` with variants `Bar`, `Block`, `Underline`
  - Add `#[schemars(default = "default_cursor_style")] pub cursor_style: Option<CursorStyle>` to `ThemeSettingsContent`
  - Add defaults: `Block` when Spectrum 2 theme active, `Bar` otherwise (in `spectrum2_defaults()`)
  - Read `cursor_style` in `crates/editor/src/editor.rs` and use it when rendering the cursor
  - Add `CursorBlink` enum with `Smooth` and `Phase` variants
  - _Requirements: 7.1, 7.5_
  - _writes: crates/settings_content/src/theme.rs, crates/theme_settings/src/theme_settings.rs, crates/editor/src/editor.rs_

- [x] 3. Create Spectrum 2 Inspired icon theme extension
  - [x] 3.1 Create extension directory structure
    - Create `extensions/spectrum-2-icons/`
    - Create `extensions/spectrum-2-icons/extension.toml` with proper metadata
    - Create `extensions/spectrum-2-icons/themes/spectrum-2-inspired-icons.json`
    - Create `extensions/spectrum-2-icons/icons/file_icons/` directory

  - [x] 3.2 Create icon theme JSON
    - Define `directory_icons` (folder.svg, folder_open.svg)
    - Define `chevron_icons` (chevron_right.svg, chevron_down.svg)
    - Define `file_suffixes` mapping for all ~30 file types
    - Define `file_icons` mapping with all SVG paths
    - Default fallback icon for unrecognized file types

  - [x] 3.3 Create SVG icons for core file types
    - Create SVGs with: viewBox `0 0 16 16`, stroke `1.5`, stroke-linecap `round`, stroke-linejoin `round`, fill `none`, color `currentColor`
    - Core types (15 icons): rust, typescript, javascript, python, go, json, yaml, markdown, html, css, docker, git, toml, file (generic), folder, folder_open
    - _Requirements: 3, 4_
    - _writes: extensions/spectrum-2-icons/extension.toml, extensions/spectrum-2-icons/themes/spectrum-2-inspired-icons.json, extensions/spectrum-2-icons/icons/file_icons/*.svg_

- [x] 4. Update border radius constants and add theme key
  - [x] 4.1 Add `BorderRadiusContent` to theme schema
    - Add struct `BorderRadiusContent` to `crates/theme/src/schema.rs`
    - Add optional `border_radius` field to theme style
    - Add fields: `button`, `input`, `panel`, `modal`, `tooltip`, `autocomplete`, `scrollbar_thumb`

  - [x] 4.2 Update Spectrum 2 theme JSON with border radius
    - Add `border_radius` block to both light and dark variants in `assets/themes/spectrum/spectrum-2-inspired.json`
    - Values: button=8, input=6, panel=12, modal=16, tooltip=6, autocomplete=8, scrollbar_thumb=3

  - [x] 4.3 Update UI components to read border radius from theme
    - Buttons: read `border_radius.button` in `crates/ui/src/components/button/button_like.rs`
    - Inputs: read `border_radius.input` in input component
    - Modals: read `border_radius.modal` in `crates/ui/src/components/modal.rs`
    - Add fallback to current defaults when `border_radius` key not present

  - [x] 4.4 Update onboarding preview tile radius
    - Change `ThemePreviewTile::ROOT_RADIUS` from `px(8.0)` to `px(12.0)` in `crates/onboarding/src/theme_preview.rs`
    - This makes the onboarding theme tiles feel more rounded and approachable
    - _Requirements: 6_
    - _writes: crates/theme/src/schema.rs, crates/settings_content/src/theme.rs, crates/theme/src/styles/colors.rs, crates/theme/src/theme.rs, crates/theme_settings/src/theme_settings.rs, crates/ui/src/components/button/button_like.rs, crates/ui/src/components/modal.rs, crates/onboarding/src/theme_preview.rs, assets/themes/spectrum/spectrum-2-inspired.json, crates/theme/src/fallback_themes.rs, crates/theme_importer/src/vscode/converter.rs_

- [ ] 5. Update spacing for Spectrum 2
  - Change `ListItem` default `indent_step_size` from `px(12.)` to `px(16.)` in `crates/ui/src/components/list/list_item.rs`
  - Adjust `ListItem` dense spacing variant to use 4px vertical padding instead of 0px
  - Change project panel list item horizontal padding from `DynamicSpacing::Base04` to `DynamicSpacing::Base06`
  - _Requirements: 5.6_
  - _writes: crates/ui/src/components/list/list_item.rs_

- [ ] 6. Add smooth cursor blink animation
  - Modify cursor blink in `crates/editor/src/editor.rs` to use smooth opacity transition
  - Animation: 530ms period, opacity oscillates between 0.3 and 1.0
  - Use GPUI's animation system (`Animation::new(Duration::from_millis(530))`)
  - Only applies when CursorBlink is set to `Smooth` (Spectrum 2 default)
  - When editor is unfocused, reduce cursor opacity to 0.3
  - _Requirements: 7.2, 7.4_
  - _writes: crates/editor/src/editor.rs_

- [ ] 7. Update documentation
  - Update `docs/src/appearance.md` with:
    - Spectrum 2 typography recommendations
    - Instructions for installing the icon theme extension
    - Cursor and editor behavior notes
  - Update the Spectrum 2 Inspired theme docs to reference the icon theme
  - _Requirements: 1, 3_
  - _writes: docs/src/appearance.md_

## Notes

- Steps 1 and 2 are prerequisites for most other steps — they establish the settings infrastructure
- Step 3 (icon theme) is independent — can be done in parallel with any other step
- Steps 4-5 (shapes, spacing) are independent of each other
- Step 6 depends on Step 2 (cursor style setting must exist first)
- All changes are **backward compatible** — when Spectrum 2 theme is not active, no behavior changes
- The existing `DynamicSpacing` enum values are unchanged; only usage patterns adjust
