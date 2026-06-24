# Implementation Plan: Spectrum 2 Inspired Theme

## Overview

This implementation plan covers a single deliverable: a `spectrum-2-inspired.json` theme file for Baymax. Since this is a data-only change (no Rust code modification), the tasks are structured as a single build-and-validate flow:

1. Create the theme JSON with all color values
2. Validate against the schema
3. Verify contrast ratios
4. Install locally and smoke-test

## Tasks

- [ ] 1. Build the theme JSON file
  - Create `~/.config/baymax/themes/spectrum-2-inspired.json` with the full structure
  - Include `$schema`, `name`, `author`, and two appearance variants
  - Populate all theme keys from the design document color maps:
    - UI colors: border, surface, background, element, ghost_element, text, icon, status_bar, title_bar, toolbar, tab_bar, search, panel, pane, scrollbar, minimap (~45 keys)
    - Editor colors: foreground, background, gutter, subheader, active_line, highlighted_line, debugger_active_line, line_number, active_line_number, hover_line_number, invisible, wrap_guide, active_wrap_guide, indent_guide, indent_guide_active, document_highlight read/write/bracket, diff_hunk added/deleted (~25 keys)
    - Terminal colors: background, foreground, bright_foreground, dim_foreground, ansi_background, 16 ansi colors + dim variants (~22 keys)
    - Version control: added, deleted, modified, renamed, conflict, ignored, word_added, word_deleted, conflict_marker.ours, conflict_marker.theirs (~10 keys)
    - Status colors: conflict, created, deleted, error, hidden, hint, ignored, info, modified, predictive, renamed, success, unreachable, warning — each with foreground, background, border (~42 keys)
    - Syntax: all ~44 syntax capture tokens with color, font_style, font_weight
    - Players: 8 color sets with cursor, background, selection
    - Accents array: 9 colors
  - _Requirements: 1, 2, 3, 4, 6_
  - _writes: ~/.config/baymax/themes/spectrum-2-inspired.json_

- [ ] 2. Validate JSON against theme schema
  - Parse the JSON to ensure it's syntactically valid
  - Verify it conforms to `https://baymax.dev/schema/themes/v0.2.0.json`
  - Check all required keys are present by comparing against `one.json`'s key set
  - Verify hex color format `#RRGGBBAA` for all color values
  - _Requirements: 1.4, 1.5, 6.3_
  - _depends-on: 1_

- [ ] 3. Verify WCAG AA contrast ratios
  - Check all text-carrying tokens (`text`, `text.muted`, `text.placeholder`, `text.disabled`, `text.accent`) against their typical background surfaces
  - Check all syntax tokens against `editor.background` in both modes
  - Verify `terminal.foreground` against `terminal.background`
  - Ensure all ratios meet WCAG AA (≥4.5:1 normal text, ≥3:1 large text/UI)
  - _Requirements: 4.1, 12.1, 12.5, 8.3_
  - _depends-on: 1_

- [ ] 4. Install theme and test in Baymax
  - Copy the file to `~/.config/baymax/themes/`
  - Restart Baymax or reload themes
  - Select "Spectrum 2 Inspired Light" from the Theme Selector
  - Visually verify across:
    - TypeScript/React file (syntax highlighting, readability)
    - Rust file (syntax highlighting, readability)
    - JSON file (key-value distinction)
    - Markdown file (headings, emphasis, links)
    - Terminal (ANSI colors, readability)
    - Search panel (match highlighting)
    - Command palette (surface layering)
    - Diagnostics (error/warning/info distinction)
    - Git diff (added/deleted/modified colors)
    - Project panel (file tree, git status decorations)
  - Switch to "Spectrum 2 Inspired Dark" and verify the same surfaces
  - _Requirements: 1, 13_
  - _depends-on: 1, 2, 3_

## Notes

- No Rust code changes are required — this is purely a theme JSON data file.
- The `one.json` file remains untouched; One Dark/Light stay available as alternatives.
- Theme overrides continue to work: users can override any key with `theme_overrides` in settings.
- Contrast values are calculated against the specified backgrounds. Individual display calibration may cause minor perceptual differences.
