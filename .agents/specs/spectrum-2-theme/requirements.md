# Requirements — Spectrum 2 Inspired Theme

## Introduction

Baymax currently ships with "One Dark" and "One Light" as its default themes, derived from the Atom One color scheme. This feature replaces the default theme with a new "Spectrum 2 Inspired" family that inherits design ideas from Adobe Spectrum 2: approachable brightness, clear hierarchy, readable contrast, modular surfaces, subtle depth, and restrained expressive accents.

This is **not** an Adobe clone. It translates Spectrum 2's design principles into an editor context — layered surfaces, semantic color mapping, accessible contrast, and a controlled accent language — without copying Adobe branding.

## Glossary

| Term | Definition |
|---|---|
| **Baymax Theme** | A JSON file at `assets/themes/` or `~/.config/baymax/themes/` containing light and dark appearance variants with semantic color tokens (UI, editor, syntax, terminal, diagnostics). |
| **Spectrum 2** | Adobe's design system. Its principles used here: layered surfaces, token-based color scales, semantic aliases (accent, negative, informative, etc.), subdued backgrounds, restrained accent usage. |
| **Accent Language** | The intentional use of color accents. In this theme, blue is the primary accent, purple is a secondary expressive accent — both used sparingly for signal, not decoration. |
| **Layered Surface** | A visual hierarchy created by varying background lightness/darkness: main background, editor background, panel background, elevated surface (modals/popovers). |
| **Theme Overrides** | A user-facing setting (`theme_overrides`) allowing override of specific theme attributes without modifying the theme JSON file. Must remain backward compatible. |
| **Appearance Variant** | One of `"light"` or `"dark"` within a theme object. Each theme JSON contains two variants. |
| **Syntax Token** | A named capture in the theme's `syntax` object, mapping to a Tree-sitter highlight capture (e.g., `keyword`, `string`, `function`, `comment`). |

## Design Direction

### Surface Hierarchy (Layered)

Create a calm, low-noise editor with subtle elevation changes:

| Layer | Light Mode | Dark Mode | Purpose |
|---|---|---|---|
| Background (app) | `#F8F8FA` | `#15161A` | Outermost app chrome |
| Editor background | `#FFFFFF` | `#1B1C21` | Code editing surface |
| Panel/sidebar | `#F1F2F4` | `#202127` | Sidebars, project panel, terminal |
| Elevated surface | `#FFFFFF` | `#272832` | Modals, popovers, command palette |
| Border subtle | `#DCDDE1` | `#343640` | Separators, dividers |
| Border strong | `#B8BAC2` | `#4A4D59` | Focus rings, active borders |

Avoid harsh black/white contrast except for primary text. Use soft borders to separate modules.

### Accent Language

- **Blue** (primary accent): active tab indicator, focused border, cursor, selected text background, search match, primary links, active controls
- **Purple** (secondary expressive accent): function names, special keywords, secondary highlights, diagnostics/info — used sparingly
- **Red**, **green**, **orange**: reserved for semantic meaning (errors, success, warnings)

Do not make the UI rainbow-like. Spectrum 2 is expressive but controlled.

### Typography (Settings Recommendation)

The theme JSON controls colors only. Typography is configured via user settings. Recommended companion settings:

```json
{
  "buffer_font_family": "SF Mono",
  "ui_font_family": "Inter",
  "buffer_font_size": 15,
  "ui_font_size": 14
}
```

### Syntax Palette Philosophy

- Comments: readable but clearly secondary — muted, not invisible
- Keywords: purple for emphasis (matching the accent language)
- Functions/declarations: blue (mapping to primary accent)
- Strings: green (approachable, restful)
- Types: orange/warm (warm accent for structural elements)
- Operators/punctuation: neutral text color, not distracting
- Light mode: slightly warmer, higher contrast where needed
- Dark mode: slightly cooler, avoiding overly bright/neon colors

## Requirements

### Requirement 1: New Spectrum 2 Inspired Default Theme

**User Story:** As a new Baymax user, I want the default theme to feel modern, clean, and approachable, with layered surfaces and controlled accent colors, so that my first impression of the editor is polished and professional.

#### Acceptance Criteria

1. THE `spectrum-2-inspired.json` file SHALL be placed at `~/.config/baymax/themes/spectrum-2-inspired.json` for local usage.
2. THE file SHALL contain both `"appearance": "dark"` and `"appearance": "light"` variants under a single theme family named "Spectrum 2 Inspired".
3. THE file SHALL reference `https://baymax.dev/schema/themes/v0.2.0.json` as its `$schema`.
4. THE file SHALL include `"author": "Ahmad Vegah"` in the metadata.
5. THE file SHALL validate against the Baymax theme schema without errors.

### Requirement 2: Layered Surface System

**User Story:** As a user, I want the editor UI to have a clear visual hierarchy with distinct surface layers, so that I can quickly distinguish the code editing area from surrounding panels and toolbars.

#### Acceptance Criteria

1. WHEN the theme is active THEN `background` (app chrome) SHALL differ from `editor.background` (code surface) by at least 10% lightness in both modes.
2. WHEN the theme is active THEN `panel.background` SHALL be at a midpoint between `background` and `editor.background` to create a layered effect.
3. WHEN the theme is active THEN `elevated_surface.background` (modals, popovers) SHALL be the lightest surface in light mode and the lightest (least dark) in dark mode.
4. THE `border.variant` SHALL be used for subtle separators between surface layers (e.g., between the editor gutter and the code area).
5. THE `border` key SHALL be reserved for stronger structural borders (e.g., focused pane borders).

### Requirement 3: Accent Language — Blue Primary, Purple Secondary

**User Story:** As a user, I want the theme to use a restrained accent language where blue signals active/interactive elements and purple signals expressive code constructs, avoiding a rainbow effect.

#### Acceptance Criteria

1. THE `text.accent` color SHALL use a blue hue consistent with the blue accent palette.
2. THE `icon.accent` color SHALL use the same blue hue as `text.accent`.
3. THE `border.focused` color SHALL use a blue hue that is clearly visible against both panel and editor backgrounds.
4. THE `search.match_background` SHALL use a translucent blue hue.
5. THE `players` array first color (local player) SHALL use the blue accent.
6. IN the syntax palette, `function` and `variant` tokens SHALL use a blue hue.
7. IN the syntax palette, `keyword` and `keyword.control` tokens SHALL use a purple hue.
8. NO more than 6 distinct hues SHALL appear in the full color palette (blue, purple, green, orange/amber, red, gray) in order to maintain a restrained feel.

### Requirement 4: Syntax Highlighting — Readable and Distinct

**User Story:** As a developer, I want syntax highlighting to be clearly readable with sufficient contrast and distinct hue assignments per semantic category, so that I can quickly scan and understand code.

#### Acceptance Criteria

1. ALL syntax tokens SHALL have a contrast ratio of at least 4.5:1 against the editor background in both light and dark modes (WCAG AA for normal text).
2. THE comment color SHALL be muted but readable (≥4.5:1 contrast) — not faint/grayed out to the point of being hard to read.
3. THE keyword color (purple) SHALL be visually distinct from both the function color (blue) and the string color (green).
4. THE string color SHALL be green — restful and clearly distinct from code structure.
5. THE type/class name color SHALL be orange/amber — a warm accent for structural elements.
6. THE number and constant colors SHALL share a purple/violet hue (related to but distinct from keyword purple).
7. THE variable color SHALL be the primary text color (not an additional hue), keeping visually cluttered code minimal.
8. THE punctuation and operator colors SHALL be neutral (primary text or muted text), not drawing attention away from meaningful tokens.
9. THE tag color SHALL be red — distinct and intuitive for HTML/JSX markup.

### Requirement 5: Specific Syntax Palettes

**User Story:** As a theme designer, I want concrete color values specified for syntax tokens so that the result is predictable and reproducible.

#### Acceptance Criteria

1. THE light mode syntax palette SHALL use colors approximating these targets:
   - `keyword`: `#6B46C1` | `function`: `#0067B8` | `type`: `#CB5D00`
   - `string`: `#12805C` | `number`: `#8A5CF6` | `constant`: `#7D5CFF`
   - `comment`: `#777B86` | `operator`: `#555862` | `variable`: `#1D1D1F`
   - `property`: `#0F6CBD` | `tag`: `#D7373F`
2. THE dark mode syntax palette SHALL use colors approximating these targets:
   - `keyword`: `#B49CFF` | `function`: `#79B8FF` | `type`: `#F7B267`
   - `string`: `#74D99F` | `number`: `#C5A3FF` | `constant`: `#A38BFF`
   - `comment`: `#8F939E` | `operator`: `#C4C7D0` | `variable`: `#F4F4F6`
   - `property`: `#8CCBFF` | `tag`: `#FF8A90`
3. THE `comment` style SHALL support `font_style: "italic"` IF it does not reduce legibility in the specific rendering environment.
4. THE syntax `comment` and `comment.doc` tokens SHALL use the same muted gray (not distinct from each other in hue, only potentially in font style).

### Requirement 6: All Theme Keys Covered

**User Story:** As a theme developer, I want all Baymax theme keys to have explicit values in the new theme, so that no UI element falls back to potentially mismatched defaults.

#### Acceptance Criteria

1. THE light variant SHALL provide values for ALL of the following top-level keys:
   - `border`, `border.variant`, `border.focused`, `border.selected`, `border.transparent`, `border.disabled`
   - `elevated_surface.background`, `surface.background`, `background`
   - `element.background`, `element.hover`, `element.active`, `element.selected`, `element.disabled`
   - `element.selection_background`
   - `drop_target.background`, `drop_target.border`
   - `ghost_element.background`, `ghost_element.hover`, `ghost_element.active`, `ghost_element.selected`, `ghost_element.disabled`
   - `text`, `text.muted`, `text.placeholder`, `text.disabled`, `text.accent`
   - `icon`, `icon.muted`, `icon.disabled`, `icon.placeholder`, `icon.accent`
   - `debugger_accent`
   - `status_bar.background`, `title_bar.background`, `title_bar.inactive_background`, `toolbar.background`
   - `tab_bar.background`, `tab.inactive_background`, `tab.active_background`
   - `search.match_background`, `search.active_match_background`
   - `panel.background`, `panel.focused_border`, `panel.indent_guide`, `panel.indent_guide_hover`, `panel.indent_guide_active`
   - `panel.overlay_background`, `panel.overlay_hover`
   - `pane.focused_border`, `pane_group.border`
   - `scrollbar.thumb.background`, `scrollbar.thumb.hover_background`, `scrollbar.thumb.active_background`, `scrollbar.thumb.border`
   - `scrollbar.track.background`, `scrollbar.track.border`
   - `minimap.thumb.background`, `minimap.thumb.hover_background`, `minimap.thumb.active_background`, `minimap.thumb.border`
   - `editor.foreground`, `editor.background`, `editor.gutter.background`, `editor.subheader.background`
   - `editor.active_line.background`, `editor.highlighted_line.background`
   - `editor.line_number`, `editor.active_line_number`, `editor.hover_line_number`
   - `editor.invisible`, `editor.wrap_guide`, `editor.active_wrap_guide`
   - `editor.indent_guide`, `editor.indent_guide_active`
   - `editor.document_highlight.read_background`, `editor.document_highlight.write_background`, `editor.document_highlight.bracket_background`
   - `terminal.background`, `terminal.foreground`, `terminal.bright_foreground`, `terminal.dim_foreground`
   - `terminal.ansi.*` (16 colors + dim variants)
   - `link_text.hover`
   - `version_control.*` (added, deleted, modified, renamed, conflict, ignored, word_added, word_deleted, conflict_marker.ours, conflict_marker.theirs)
   - `status_colors.*` (conflict, created, deleted, error, hidden, hint, ignored, info, modified, predictive, renamed, success, unreachable, warning — each with `.background` and `.border`)
   - `players` (8 distinct player color sets)
   - `syntax.*` (all ~30+ syntax capture types)
   - `accents` array
2. THE dark variant SHALL provide values for the same set of keys.
3. NO key present in `one.json` SHALL be absent from `spectrum-2-inspired.json`.

### Requirement 7: Semantic Status and Diagnostic Colors

**User Story:** As a developer, I want status indicators (errors, warnings, git changes) to use clear semantic colors that map intuitively to their meaning and are not aggressive.

#### Acceptance Criteria

1. THE `error` token SHALL use a red hue.
2. THE `warning` token SHALL use an orange/amber hue.
3. THE `info` token SHALL use a blue hue.
4. THE `success` token SHALL use a green hue.
5. THE `created` token SHALL use a green hue consistent with git additions.
6. THE `deleted` token SHALL use a red hue consistent with git deletions.
7. THE `modified` token SHALL use a yellow/amber hue consistent with git modifications.
8. ALL diagnostic background colors SHALL use low-opacity variants so they are visible but not distracting.
9. Diagnostics SHALL be distinct from each other but not aggressive — avoid overly bright/saturated backgrounds.

### Requirement 8: Terminal ANSI Colors Coherent with Theme

**User Story:** As a terminal user, I want ANSI colors to be thematically consistent with the Spectrum-inspired palette while maintaining standard semantic mappings.

#### Acceptance Criteria

1. THE 16 main ANSI colors SHALL use hues that match the theme's overall color narrative (blue accent, purple expressive, green success, red error, orange warning, gray neutrals).
2. THE `terminal.background` SHALL match the `panel.background` of the same appearance mode for visual consistency.
3. THE `terminal.foreground` SHALL have ≥4.5:1 contrast against `terminal.background`.
4. THE bright ANSI variants SHALL be noticeably brighter than their standard counterparts (for emphasis in terminal output).
5. THE dim ANSI variants SHALL be noticeably dimmer (for deemphasized terminal output).

### Requirement 9: Collaboration Cursor Colors

**User Story:** As a collaborator using multiplayer editing, I want cursor colors to be visually distinct from each other and drawn from the theme's restrained palette.

#### Acceptance Criteria

1. THE `players` array SHALL contain 8 distinct cursor colors.
2. THE 8 hues SHALL be drawn from: blue, orange, pink, green, purple, amber, teal, red — ensuring maximum visual distance around the color wheel.
3. EACH player color SHALL have ≥3:1 contrast against both light and dark editor backgrounds.
4. THE first player (local) SHALL use the blue accent color.

### Requirement 10: Backward Compatibility

**User Story:** As an existing user with custom theme overrides, I want my `theme_overrides` configuration to continue working, so that my personalized setup is not broken.

#### Acceptance Criteria

1. THE `theme_overrides` setting SHALL continue to work with the new Spectrum 2 Inspired theme.
2. Users who have explicitly set `"theme.light": "One Light"` or `"theme.dark": "One Dark"` SHALL continue to use those themes without change.
3. THE `theme_overrides` JSON format SHALL remain unchanged.
4. Setting `"mode": "system"` with `"light": "Spectrum 2 Inspired Light"` and `"dark": "Spectrum 2 Inspired Dark"` SHALL work as expected.

### Requirement 11: Light and Dark Variant Cohesion

**User Story:** As a user who switches between light and dark mode, I want both variants to feel like the same design system, with consistent hue assignments and semantic mappings.

#### Acceptance Criteria

1. THE same accent hue (blue `#1473E6` light / `#5EA0EF` dark) SHALL be used for accent tokens across both variants.
2. THE same semantic hue mapping SHALL apply in both variants (green = success, red = error, etc.).
3. THE syntax palette SHALL maintain consistent hue-to-meaning mapping across modes (keywords are always purple, functions always blue, strings always green, etc.).
4. THE surface hierarchy approach SHALL be inverted consistently: lightest surfaces in light mode, darkest surfaces in dark mode, with the same relative ordering.

### Requirement 12: Accessibility

**User Story:** As a user with visual sensitivity, I want the theme to meet WCAG AA contrast requirements and avoid visual fatigue.

#### Acceptance Criteria

1. ALL text-carrying tokens SHALL meet WCAG AA contrast (≥4.5:1 for normal text, ≥3:1 for large text and UI components) in both modes.
2. THE dark mode SHALL avoid excessively bright or neon syntax colors that cause eye strain.
3. THE light mode SHALL avoid low-contrast "grey-on-grey" combinations that are hard to read.
4. THE `border.focused` color SHALL be clearly visible in both modes (≥3:1 against adjacent surfaces).
5. THE `text.muted` and `text.placeholder` colors SHALL still be readable (≥4.5:1 against their background).

### Requirement 13: Quality Bar

**User Story:** As a reviewer, I want clear criteria for when the theme is considered complete and polished.

#### Acceptance Criteria

1. THE editor content SHALL be more readable than the default "One" theme (qualitative assessment).
2. THE UI SHALL feel lighter, clearer, and more modular than the default theme.
3. THE active/focused states SHALL be obvious through border and background contrast.
4. THE comments SHALL remain legible at all zoom levels.
5. THE diagnostics SHALL be distinct but not visually aggressive.
6. THE git additions/deletions SHALL be immediately clear through color semantics.
7. THE light and dark variants SHALL feel like the same design system.
8. THE theme SHALL NOT look like a direct Adobe clone.

## Out of Scope

- Modifying any Rust code for theme loading, rendering, or settings.
- Changing the theme JSON schema or adding new token keys.
- Implementing Spectrum 2's component library or CSS-in-JS system.
- Modifying the Theme Builder tool or any existing documentation files.
- Forcing Adobe Clean or any specific font — typography is a user setting, not a theme concern.
- Touch/pointer detection, RTL support, or other Spectrum 2 concepts not applicable to Baymax's Rust-native UI framework.

