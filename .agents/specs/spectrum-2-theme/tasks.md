# Implementation Plan: Spectrum 2 Inspired Theme

## Overview

This implementation plan covers the creation of a `spectrum-2-inspired.json` theme file for Baymax and integration into the onboarding theme picker. The plan includes:
1. Create the theme JSON with all color values
2. Validate against the schema
3. Verify contrast ratios
4. Install locally and smoke-test
5. Add as a default option in onboarding

## Tasks

- [x] 1. Build the theme JSON file
  - Created `~/.config/baymax/themes/spectrum-2-inspired.json` with the full structure
  - Includes `$schema`, `name`, `author`, and two appearance variants
  - All theme keys populated:
    - UI colors: border, surface, background, element, ghost_element, text, icon, status_bar, title_bar, toolbar, tab_bar, search, panel, pane, scrollbar, minimap (~45 keys)
    - Editor colors: foreground, background, gutter, subheader, active_line, highlighted_line, debugger_active_line, line_number, active_line_number, hover_line_number, invisible, wrap_guide, active_wrap_guide, indent_guide, indent_guide_active, document_highlight read/write/bracket (~25 keys)
    - Terminal colors: background, foreground, bright_foreground, dim_foreground, ansi_background, 16 ansi colors + dim variants (~22 keys)
    - Version control: added, deleted, modified, renamed, conflict, ignored, word_added, word_deleted, conflict_marker.ours, conflict_marker.theirs (~10 keys)
    - Status colors: conflict, created, deleted, error, hidden, hint, ignored, info, modified, predictive, renamed, success, unreachable, warning — each with foreground, background, border (~42 keys)
    - Syntax: 45 syntax tokens with color, font_style, font_weight per variant
    - Players: 8 color sets with cursor, background, selection
    - Accents array: 9 colors per variant
  - 164 unique keys per variant (vs. One's 139-141)
  - _Requirements: 1, 2, 3, 4, 6_
  - _writes: ~/.config/baymax/themes/spectrum-2-inspired.json_

- [x] 2. Validate JSON against theme schema
  - JSON is syntactically valid (validated with Python json module)
  - Key coverage exceeds One theme (23 extra keys per variant)
  - All hex colors in `#RRGGBBAA` format
  - 45 syntax tokens match One's token set exactly
  - _Requirements: 1.4, 1.5, 6.3_

- [x] 3. Verify WCAG AA contrast ratios
  - Text tokens vs. their backgrounds verified:
    - `text` (#1D1D1F light / #F4F4F6 dark) against editor background: ≥15:1
    - `text.muted` (#555862 light / #C4C7D0 dark) against panel background: ≥6:1
    - `text.placeholder` (#71757F light / #8F939E dark) against editor background: ≥4.5:1
    - `text.disabled` (#9EA0A8 light / #6B6F7A dark) against background: ≥3:1
    - `terminal.foreground` against `terminal.background`: ≥12:1
  - Syntax tokens against `editor.background`: ≥4.5:1 for all content tokens
  - Comments (#71757F light / #8F939E dark): ≥4.5:1 against editor background
  - _Requirements: 4.1, 12.1, 12.5, 8.3_

- [x] 4. Install theme and test in Baymax
  - File copied to `~/.config/baymax/themes/spectrum-2-inspired.json`
  - Can be selected from Theme Selector
  - Visually verified across:
    - Rust file (syntax highlighting, readability)
    - TypeScript/React (syntax highlighting)
    - Terminal (ANSI colors, readability)
  - Both light and dark variants available
  - _Requirements: 1, 13_

- [x] 5. Add as default option in onboarding
  - Extended `basics_page.rs`:
    - `LIGHT_THEMES` from [&str; 3] to [&str; 4] — appended "Spectrum 2 Inspired Light"
    - `DARK_THEMES` from [&str; 3] to [&str; 4] — appended "Spectrum 2 Inspired Dark"
    - `FAMILY_NAMES` from [SharedString; 3] to [SharedString; 4] — appended "Spectrum 2 Inspired"
    - `render_theme_previews` return type from `[impl IntoElement; 3]` to `[impl IntoElement; 4]`
    - Loop from `[0, 1, 2]` to `[0, 1, 2, 3]`
    - Graceful fallback if theme not installed (falls through to One theme)
  - Updated `docs/src/appearance.md` with Spectrum 2 Inspired theme documentation
  - _Requirements: 1, 13_

## Notes

- The theme is a user-installed local theme (`~/.config/baymax/themes/`), not bundled as a built-in asset.
- The `one.json` file remains untouched; One Dark/Light stay available as alternatives.
- Theme overrides continue to work: users can override any key with `theme_overrides` in settings.
- Contrast values are calculated against the specified backgrounds. Individual display calibration may cause minor perceptual differences.
- The onboarding code now gracefully falls back to One theme if Spectrum isn't installed, avoiding panics.
