# Requirements — Spectrum 2 Inspired UI Language

## Introduction

This specification defines the typography, iconography, spacing, shapes, and editor UI behavior for a Spectrum 2-inspired visual language in Baymax. While the companion [Spectrum 2 Theme](../spectrum-2-theme/requirements.md) spec covers **color**, this spec covers everything else that makes the editor feel like a cohesive, approachable, modern tool.

Spectrum 2's design principles center on being **approachable, bright, rounded-feeling, and clear**. This spec translates those principles into concrete Baymax settings, theme extensions, and code changes across typography, icons, spacing, shapes, and editor chrome.

## Glossary

- **Typography Scale**: The system of font sizes, weights, line heights, and font families used across the UI and editor buffers
- **Icon Theme**: A Baymax extension or built-in theme that maps file types and UI states to SVG icon files
- **Dynamic Spacing**: Baymax's spacing system using `DynamicSpacing::BaseNN` rems-based spacing tokens
- **Element Shape**: The border radius, corner smoothing, and visual outline of UI elements (buttons, panels, tabs, inputs)
- **Editor Chrome**: The non-editing UI surrounding the buffer — gutter, scrollbar, minimap, tabs, breadcrumbs, status bar
- **Spectrum 2 (S2)**: Adobe's design system — see [Introducing Spectrum 2](https://adobe.design/ideas/introducing-spectrum-2)
- **Content Density**: The amount of whitespace around UI elements; Spectrum 2 favors moderate density with clear breathing room
- **Corner Smoothing**: The visual treatment of rounded corners beyond simple border-radius, inspired by S2's "rounded-feeling" shapes
- **Focus Indicator**: The visual halo/ring shown when an element receives keyboard focus
- **Active Tab Indicator**: The visual accent (line, fill, or background) on the currently selected tab

## Design Direction

### Typography

Spectrum 2 uses [Adobe Clean](https://fonts.adobe.com/fonts/adobe-clean) as its primary typeface — a humanist sans-serif with approachable, warm curves. For Baymax:

| Setting | Recommendation | Rationale |
|---|---|---|
| `ui_font_family` | `"Inter"` | Humanist sans-serif, excellent legibility, free, similar warmth to Adobe Clean |
| `buffer_font_family` | `"SF Mono"` / `"JetBrains Mono"` | Coding font with clear distinction between similar chars |
| `ui_font_size` | `14px` | Spectrum 2 uses moderately sized UI text for approachability |
| `buffer_font_size` | `15px` | Slightly larger than default for readability |
| `buffer_line_height` | `"standard"` (1.3) | Tighter than "comfortable" for more code on screen; Spectrum 2 favors moderate density |

Font features should enable:
- `calt` (contextual alternates) for Inter's distinctive italic forms
- `liga` (standard ligatures) for coding ligatures where appropriate
- `cvXX` stylistic sets where Inter offers Spectrum 2-like glyph shapes (e.g., single-story `a`, `g`)

### Iconography

Spectrum 2 uses a [consistent, rounded, stroke-based icon set](https://react-spectrum.adobe.com/react-spectrum/icons.html) with:
- 18px default UI icon size
- 1.5px stroke width
- Rounded line caps and joins
- Clear, minimal designs with no filled variants for default states

For Baymax, this translates to:
- A new Spectrum 2-inspired icon theme as a Baymax extension (or bundled theme)
- Redesigned SVG file icons with rounded strokes
- Spectrum 2-style chevrons and folder icons
- Spectrum 2-style UI icons (search, settings, close, etc.) — though these are shared across all themes

### Spacing

Spectrum 2 uses an 8px grid with a 4px sub-unit for fine adjustments. Baymax already uses a `DynamicSpacing` system. The recommendation is:

| Token | Current Default | S2-Inspired Value | Usage |
|---|---|---|---|
| `DynamicSpacing::Base00` | `0px` | `0px` | No spacing |
| `DynamicSpacing::Base01` | `2px` | `2px` | Micro spacing |
| `DynamicSpacing::Base02` | `4px` | `4px` | Dense padding (icon containers) |
| `DynamicSpacing::Base03` | `6px` | `6px` | Tight gaps |
| `DynamicSpacing::Base04` | `8px` | `8px` | **Base unit** — default gaps |
| `DynamicSpacing::Base05` | `10px` | `10px` | Slightly relaxed |
| `DynamicSpacing::Base06` | `12px` | `12px` | Panel padding, list items |
| `DynamicSpacing::Base07` | `14px` | `14px` | Section spacing |
| `DynamicSpacing::Base08` | `16px` | `16px` | Card padding, modal padding |
| `DynamicSpacing::Base09` | `20px` | `20px` | Large gaps |
| `DynamicSpacing::Base10` | `24px` | `24px` | Section margins |
| `DynamicSpacing::Base11` | `28px` | `28px` | Wide spacing |
| `DynamicSpacing::Base12` | `32px` | `32px` | Page padding |

The key principle: elements in the same group should be `Base04` (8px) apart; related groups should be `Base06` (12px) apart; sections should be `Base10` (24px) apart.

### Shapes (Border Radius)

Spectrum 2 is described as "rounded-feeling." Key shape guidelines:

| Element | S2-Inspired Radius | Baymax Current (approx) |
|---|---|---|
| Buttons (default) | `8px` | `6px` |
| Input fields | `6px` | `4px` |
| Panels / Cards | `12px` | `8px` |
| Modals / Dialogs | `16px` | `12px` |
| Tabs (active indicator) | `6px` top corners | `0px` (line only) |
| Dropdowns / Menus | `8px` | `6px` |
| Tooltips | `6px` | `4px` |
| Scrollbar thumb | `4px` | `2px` |
| Badges / Tags | `4px` | `2px` |
| Preview tiles (onboarding) | `12px` | `8px` (already set) |

### Editor UI Behavior

Spectrum 2 principles applied to editor chrome:

| Behavior | S2 Direction | Implementation |
|---|---|---|
| **Cursor style** | Block cursor by default (approachable, visible) | Change default cursor from `bar` to `block` in S2 variants |
| **Cursor blink** | Softer blink with smooth opacity transition | Animate cursor blink with opacity 0.3→1.0 (not binary on/off) |
| **Active line** | Subtle background highlight, not full-width border | Use `editor.active_line.background` only (already set in theme) |
| **Scrollbar** | Thinner, rounded, auto-hide in editor, always-show in panels | Set scrollbar width to 6px, thumb radius to 3px |
| **Minimap** | Show as a block overview, not character-precise | No Baymax setting change — minimap rendering is built-in |
| **Tab sizing** | Equal-width tabs with fixed min/max; active tab has bottom accent | The theme already sets accent via `tab.active_background` |
| **Gutter** | Show line numbers only (no breakpoint gutter when no breakpoints) | No change needed — Baymax already hides breakpoint gutter when empty |
| **Autocomplete** | Compact, rounded menu with clear hierarchy | Adjust autocomplete menu border radius to 8px |
| **Command palette** | Full-width, elevated surface with clear search | Already elevated via `elevated_surface.background` in theme |
| **Focus ring** | Use `border.focused` (blue accent) with 2px width | The theme already sets `border.focused` to the accent blue |
| **Active tab** | Bottom accent line + slightly elevated background | Theme already has `tab.active_background` differentiated |
| **File tree indent** | 16px per level (matches Base08 spacing) | Currently 12px in list items — change to 16px for S2 variant |

## Requirements

### Requirement 1: Spectrum 2 Inspired Default Typography Settings

**User Story:** As a Baymax user selecting the Spectrum 2 Inspired theme, I want the typography to feel approachable and modern like Spectrum 2, so that the text in both UI and editor buffers is comfortable to read.

#### Acceptance Criteria

1. WHEN the user selects "Spectrum 2 Inspired Light" or "Spectrum 2 Inspired Dark" THEN THE system SHALL apply recommended font settings (Inter for UI, SF Mono/JetBrains Mono for buffers, 14px UI font, 15px buffer font, 1.3 line height)
2. IF the user has not explicitly set `buffer_font_family` or `ui_font_family` THEN THE system SHALL use the Spectrum 2-inspired defaults when the Spectrum 2 Inspired theme is active
3. IF Inter is not installed on the system THEN THE system SHALL fall back to the default Baymax UI font stack
4. IF SF Mono is not installed THEN THE system SHALL fall back to JetBrains Mono, then to the default Baymax buffer font
5. THE Spectrum 2 typography settings SHALL NOT override explicit user font preferences

### Requirement 2: Typography Scale and Features

**User Story:** As a developer, I want the font rendering (weight, features, line height) to match Spectrum 2's warm, approachable feel while maintaining code readability.

#### Acceptance Criteria

1. WHEN rendering UI text with Inter THEN THE system SHALL use font weight 400 (Regular) for body text and 500 (Medium) for headings/labels
2. WHEN rendering buffer text with SF Mono or JetBrains Mono THEN THE system SHALL use font weight 400 (Regular) for body text and 600 (Semi-Bold) for headings in markdown
3. WHEN Inter is the UI font THEN THE system SHALL enable `calt` (contextual alternates) for improved readability
4. WHEN a coding font is used THEN THE system SHALL enable `liga` (standard ligatures) for coding ligatures like `->`, `=>`, `::`

### Requirement 3: Spectrum 2 Inspired Icon Theme

**User Story:** As a Spectrum 2 Inspired theme user, I want file icons and folder icons that match the rounded, stroke-based, approachable visual style of Spectrum 2.

#### Acceptance Criteria

1. THE system SHALL provide a "Spectrum 2 Inspired" icon theme selectable from the Icon Theme Selector
2. THE icon theme SHALL use 1.5px stroke width for all file and folder icons
3. THE icon theme SHALL use rounded line caps (`stroke-linecap="round"`) and rounded line joins (`stroke-linejoin="round"`)
4. ALL file type icons SHALL be redesigned with minimal, clear shapes — no complex fills, no excessive detail
5. Folder icons SHALL use an open-book or folder shape with rounded corners and a subtle fold
6. Chevron icons SHALL use a simple V-shape with rounded endpoints (not filled triangles)
7. THE icon theme SHALL be distributed as a Baymax extension that users can install from the Extensions page

### Requirement 4: Icon Sizing and Theming

**User Story:** As a user, I want icons to be consistently sized and colocated with their labels, with clear visual hierarchy.

#### Acceptance Criteria

1. WHEN rendering file icons in the project panel THEN THE system SHALL display them at 16x16px
2. WHEN rendering file icons in editor tabs THEN THE system SHALL display them at 14x14px
3. WHEN rendering UI icons (search, settings, etc.) THEN THE system SHALL display them at 18x18px (Spectrum 2 standard)
4. WHEN a file icon has no custom SVG THEN THE system SHALL fall back to a generic rounded-file icon, not a blank space
5. THE icon color SHALL be derived from the active theme's `icon`, `icon.muted`, and `icon.accent` colors
6. Folder icons SHALL use the theme's accent blue when open, and `icon.muted` when closed

### Requirement 5: Spectrum 2 Spacing Scale

**User Story:** As a user, I want UI elements to have consistent, breathable spacing that feels approachable and organized.

#### Acceptance Criteria

1. WHEN rendering list items in the project panel, command palette, or search results THEN THE system SHALL use `DynamicSpacing::Base06` (12px) horizontal padding
2. WHEN grouping related elements in a toolbar or header THEN THE system SHALL use `DynamicSpacing::Base04` (8px) gaps between elements
3. WHEN separating unrelated sections (e.g., between project panel sections) THEN THE system SHALL use `DynamicSpacing::Base10` (24px) spacing
4. WHEN rendering editor tabs THEN THE system SHALL use `DynamicSpacing::Base04` (8px) horizontal padding inside each tab
5. WHEN rendering the command palette or modal dialogs THEN THE system SHALL use `DynamicSpacing::Base08` (16px) internal padding
6. WHEN indenting nested items in the file tree THEN THE system SHALL use 16px per level (matching Spectrum 2's 8px grid doubled)

### Requirement 6: Element Shapes and Border Radius

**User Story:** As a user, I want UI elements to feel "rounded-feeling" and approachable, with consistent corner radii across the interface.

#### Acceptance Criteria

1. WHEN rendering buttons THEN THE system SHALL apply a border radius of 8px
2. WHEN rendering input fields, search bars, and text areas THEN THE system SHALL apply a border radius of 6px
3. WHEN rendering panels, sidebars, and cards THEN THE system SHALL apply a border radius of 12px
4. WHEN rendering modal dialogs and popovers THEN THE system SHALL apply a border radius of 16px
5. WHEN rendering tooltips THEN THE system SHALL apply a border radius of 6px
6. WHEN rendering scrollbar thumbs THEN THE system SHALL apply a border radius of 3px with a width of 6px
7. WHEN rendering the autocomplete/prompt menu THEN THE system SHALL apply a border radius of 8px
8. WHEN rendering onboarding theme preview tiles THEN THE system SHALL use 12px border radius (already set)

### Requirement 7: Editor UI Behavior — Cursor and Active Line

**User Story:** As a user of the Spectrum 2 Inspired theme, I want the editor cursor and active line indicator to feel smooth, visible, and approachable.

#### Acceptance Criteria

1. WHEN the Spectrum 2 Inspired theme is active THEN THE default cursor SHALL be a block cursor (more visible and approachable than a thin bar)
2. WHEN the cursor blinks THEN IT SHALL use a smooth opacity transition (0.3 → 1.0 over 530ms) rather than a binary on/off toggle
3. WHEN the cursor is on a line THEN THE active line background SHALL be subtly highlighted using `editor.active_line.background` (already set in theme)
4. WHEN the editor is not focused THEN THE cursor SHALL be rendered at reduced opacity (0.3) to indicate inactive state
5. IF the user has explicitly set cursor preferences THEN THE Spectrum 2 defaults SHALL NOT override them

### Requirement 8: Editor UI Behavior — Scrollbar and Minimap

**User Story:** As a user, I want the scrollbar and minimap to be unobtrusive but functional, with Spectrum 2's rounded, minimal aesthetic.

#### Acceptance Criteria

1. WHEN rendering the editor scrollbar THEN THE system SHALL use a 6px wide thumb with 3px border radius
2. WHEN the mouse is not over the scrollbar THEN THE thumb SHALL be at 40% opacity
3. WHEN the mouse hovers over the scrollbar THEN THE thumb SHALL be at 70% opacity
4. WHEN the scrollbar is actively dragged THEN THE thumb SHALL be at 90% opacity
5. WHEN rendering the minimap THEN IT SHALL use a 4px wide thumb with 2px border radius

### Requirement 9: Focus Indicators and Selection States

**User Story:** As a keyboard-driven user, I want focus indicators to be obvious, consistent, and visually aligned with Spectrum 2's accent language.

#### Acceptance Criteria

1. WHEN an element receives keyboard focus THEN THE system SHALL display a 2px wide focus ring using the theme's `border.focused` color (blue accent)
2. WHEN focus is moved away THEN THE focus ring SHALL disappear with a 150ms fade-out transition
3. WHEN text is selected in the editor THEN THE selection background SHALL use `editor.selection.background` (already set in theme)
4. WHEN an element is in active/hover state THEN IT SHALL use the element hover/active colors from the theme

### Requirement 10: Tabs and Navigation

**User Story:** As a user, I want editor tabs and navigation elements to feel consistent with Spectrum 2's approachable, clear hierarchy.

#### Acceptance Criteria

1. WHEN rendering the active editor tab THEN IT SHALL have a bottom accent line in the theme's accent blue
2. WHEN rendering inactive tabs THEN THEY SHALL be visually compressed (lower opacity, no bottom accent)
3. WHEN the user hovers over an inactive tab THEN IT SHALL show a subtle background change using `element.hover`
4. WHEN the tab bar has many tabs THEN tabs SHALL shrink to a minimum width of 80px, not disappear
5. WHEN rendering breadcrumbs in the editor header THEN THEY SHALL use `text.muted` for parent paths and `text` for the current file

### Requirement 11: Backward Compatibility

**User Story:** As an existing Baymax user, I want the Spectrum 2 UI language changes to not break my existing setup.

#### Acceptance Criteria

1. WHEN a user does not have the Spectrum 2 Inspired theme selected THEN ALL typography, spacing, and shape defaults SHALL remain unchanged
2. WHEN a user has configured custom font settings THEN Spectrum 2 defaults SHALL NOT override them
3. IF a user uninstalls the Spectrum 2 Inspired icon theme THEN the system SHALL fall back to the default Baymax icon theme
4. THE Spectrum 2 Inspired icon theme SHALL be distributed as an installable extension, not a core dependency

### Requirement 12: Accessibility

**User Story:** As a user with visual accessibility needs, I want the UI language enhancements to maintain or improve readability.

#### Acceptance Criteria

1. WHEN rendering UI text at the recommended font size (14px) THEN IT SHALL meet WCAG AA contrast ratios against its background
2. WHEN rendering focus indicators THEN THEY SHALL have a contrast ratio of at least 3:1 against the adjacent background
3. WHEN rendering selected/focused states THEN THEY SHALL NOT rely solely on color — shape changes (border, background shift) SHALL also indicate state
4. THE block cursor SHALL provide sufficient contrast (≥3:1) against the editor background at all times

### Requirement 13: Theme-Aware Settings

**User Story:** As a theme developer, I want the UI language settings (typography, spacing, shapes) to be theme-aware so that switching themes changes the UI feel holistically.

#### Acceptance Criteria

1. THE typography defaults (font family, size, line height) SHALL be overridable per theme via theme settings or theme-override JSON keys
2. THE spacing scale SHALL remain consistent across themes (not overridable per theme)
3. THE border radius values SHALL be overridable per theme via a new theme JSON key (e.g., `ui.border_radius` or similar)
4. THE cursor style (block vs bar) SHALL be overridable per theme
5. IF a theme JSON does not specify UI language overrides THEN the system-wide defaults SHALL apply

## Out of Scope

- Redesigning core Baymax SVG icons (search, settings, close, etc.) — these are shared across all themes and controlled by the app
- Changing the Baymax DynamicSpacing tokens themselves (the values are fine; what changes is how elements use them)
- Creating a full Spectrum 2 icon set for every file type (the initial spec covers the most common ~30 file types)
- Animations or micro-interactions beyond cursor blink
- Changing the layout of the editor (gutter position, tab bar position, panel docking) — only visual styling
- Touch/mobile adaptations
- Accessibility of third-party content (file contents, web views)
