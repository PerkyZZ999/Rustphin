---
name: Obsidian Telemetry OS
colors:
  surface: '#0f131c'
  surface-dim: '#0f131c'
  surface-bright: '#353943'
  surface-container-lowest: '#0a0e17'
  surface-container-low: '#181b25'
  surface-container: '#1c1f29'
  surface-container-high: '#262a34'
  surface-container-highest: '#31353f'
  on-surface: '#dfe2ef'
  on-surface-variant: '#c7c4d8'
  inverse-surface: '#dfe2ef'
  inverse-on-surface: '#2c303a'
  outline: '#918fa1'
  outline-variant: '#464555'
  surface-tint: '#c3c0ff'
  primary: '#c3c0ff'
  on-primary: '#1d00a5'
  primary-container: '#4f46e5'
  on-primary-container: '#dad7ff'
  inverse-primary: '#4d44e3'
  secondary: '#7bd0ff'
  on-secondary: '#00354a'
  secondary-container: '#00a6e0'
  on-secondary-container: '#00374d'
  tertiary: '#4edea3'
  on-tertiary: '#003824'
  tertiary-container: '#006e4b'
  on-tertiary-container: '#67f4b7'
  error: '#ffb4ab'
  on-error: '#690005'
  error-container: '#93000a'
  on-error-container: '#ffdad6'
  primary-fixed: '#e2dfff'
  primary-fixed-dim: '#c3c0ff'
  on-primary-fixed: '#0f0069'
  on-primary-fixed-variant: '#3323cc'
  secondary-fixed: '#c4e7ff'
  secondary-fixed-dim: '#7bd0ff'
  on-secondary-fixed: '#001e2c'
  on-secondary-fixed-variant: '#004c69'
  tertiary-fixed: '#6ffbbe'
  tertiary-fixed-dim: '#4edea3'
  on-tertiary-fixed: '#002113'
  on-tertiary-fixed-variant: '#005236'
  background: '#0f131c'
  on-background: '#dfe2ef'
  surface-variant: '#31353f'
typography:
  headline-xl:
    fontFamily: Space Grotesk
    fontSize: 2.25rem
    fontWeight: '700'
    lineHeight: 2.75rem
    letterSpacing: -0.03em
  headline-lg:
    fontFamily: Space Grotesk
    fontSize: 1.75rem
    fontWeight: '600'
    lineHeight: 2.25rem
    letterSpacing: -0.02em
  headline-md:
    fontFamily: Space Grotesk
    fontSize: 1.25rem
    fontWeight: '600'
    lineHeight: 1.75rem
    letterSpacing: -0.01em
  headline-sm:
    fontFamily: Space Grotesk
    fontSize: 1rem
    fontWeight: '600'
    lineHeight: 1.5rem
    letterSpacing: 0em
  body-lg:
    fontFamily: JetBrains Mono
    fontSize: 0.9375rem
    fontWeight: '400'
    lineHeight: 1.5rem
    letterSpacing: -0.01em
  body-md:
    fontFamily: JetBrains Mono
    fontSize: 0.8125rem
    fontWeight: '400'
    lineHeight: 1.25rem
    letterSpacing: 0em
  body-sm:
    fontFamily: JetBrains Mono
    fontSize: 0.75rem
    fontWeight: '400'
    lineHeight: 1.125rem
    letterSpacing: 0.01em
  label-md:
    fontFamily: JetBrains Mono
    fontSize: 0.75rem
    fontWeight: '500'
    lineHeight: 1rem
    letterSpacing: 0.04em
  label-sm:
    fontFamily: JetBrains Mono
    fontSize: 0.6875rem
    fontWeight: '500'
    lineHeight: 0.875rem
    letterSpacing: 0.06em
  label-xs:
    fontFamily: JetBrains Mono
    fontSize: 0.625rem
    fontWeight: '600'
    lineHeight: 0.75rem
    letterSpacing: 0.08em
spacing:
  gutter: 0.75rem
  gutter-desktop: 1rem
  margin: 0.75rem
  margin-desktop: 1.25rem
  space-xs: 0.25rem
  space-sm: 0.5rem
  space-md: 0.75rem
  space-lg: 1rem
  space-xl: 1.5rem
  space-2xl: 2rem
---

## Brand & Style

This design system establishes an ultra-precise, cyber-utilitarian command deck and desktop shell. Engineered for systems engineers, technical operators, and demanding power users, the environment prioritizes uncompromised density, structural geometry, and clinical legibility over decorative embellishments.

The visual ethos fuses high-performance telemetry console architecture with an obsidian backdrop. Pure right-angled edges, stark borders, and concentrated electric sapphire energy corridors define the aesthetic. Every view functions as an instrument: stark monolithic surfaces, precise hairline boundaries, optical scans, and glowing functional focal points deliver immediate orientation across vast hierarchies of structured data and disk volumes.

## Colors

The palette is anchored in cold, lightless obsidian and carbon tones, punctuated by high-potency photon luminescences:

- **Canvas & Shell Base (`#05070B` to `#090D16`):** Deep, light-absorbent backdrops that recede entirely into physical displays, reducing ocular strain across extended operational cycles.
- **Surface Panels (`#0F172A` with 70–85% alpha overlays):** Structured panes for file trees, split panes, terminal docks, and telemetry inspectors.
- **Primary Energy (`#4F46E5` / Royal Electric Indigo):** Denotes execution paths, affirmative focus, active partition selection, and persistent system states.
- **Secondary Telemetry (`#38BDF8` / Electric Cyan):** Reserved for live read/write indicators, active symlinks, transit meters, and dynamic metadata filters.
- **Tertiary Diagnostic (`#10B981` / Emerald Status):** Reflects parity verification, clean integrity states, and online filesystem daemons.
- **Hairline Structural Borders (`rgba(99, 102, 241, 0.18)` to `rgba(255, 255, 255, 0.08)`): Crisp, un-aliased demarcations mapping panel divisions.

## Typography

The typographic hierarchy implements an uncompromising division of labor:

- **Display & Section Headers (`Space Grotesk`):** Brings mechanical rhythm and geometric authority to volume labels, window framing, system dialogues, and top-level navigation hubs.
- **Core Telemetry, System Readouts, and Body Data (`JetBrains Mono`):** Standardizes all file paths, binary checksums, byte dimensions, permission flags (`rwxr-xr-x`), and content tables. Monospaced character alignment guarantees tabular precision across vertical data stacks, preventing visual drift during real-time sorting and filtering.
- **Case Conventions:** Metadata badges, status tags, and hardware readouts utilize uppercase styling with expanded letter spacing to ensure rapid scanning under low-light operating conditions.

## Layout & Spacing

Layout geometry follows an exact 4px modular base grid optimized for multi-pane split viewing, pinned telemetry decks, and file matrix displays:

- **Grid Architecture:** Desktop workspace structures depend on a docked pane engine: primary left navigation rail (collapsible between 56px and 240px), dynamic multi-column file workspace (fluid), and a contextual inspector flyout (fixed 320px).
- **Gutter & Border Discipline:** Internal panel splits utilize fixed 1px partition dividers with explicit `0.75rem` or `1rem` inner guttering. Dense data arrangements preserve screen real estate, letting high-volume directory structures sit alongside concurrent inspection windows.
- **Adaptive Reflow:** When canvas constraints contract below 1024px, the inspector collapses into an overlay drawer, and breadcrumb trees abbreviate leading directories via progressive root notation (`~/…/src/lib`).

## Elevation & Depth

Elevation eschews soft, diffuse dropshadows in favor of sharp, planar stacking, crisp edge lighting, and optical translucent depth:

- **Surface Layering:**
  - *Tier 0 (Root Workspace Canvas):* Solid `#05070B` matte base.
  - *Tier 1 (Explorer Windows & Split Panels):* Obsidian tint `rgba(11, 15, 23, 0.82)` with `backdrop-filter: blur(20px)` and a top-edge 1px specular line `rgba(255, 255, 255, 0.08)`.
  - *Tier 2 (Floating Popovers, Context Menus & Command Palette):* Elevated obsidian `rgba(15, 23, 42, 0.94)` with `backdrop-filter: blur(28px)` bounded by an energetic 1px edge highlight `rgba(79, 70, 229, 0.45)`.
- **Glow & Photon Bleed:** Hover states, active node selections, and focused command entries cast a tight 0px blur, 1px spread border paired with an exterior luminescence: `0 0 16px -2px rgba(79, 70, 229, 0.35)`.

## Shapes

The design system is strictly zero-radius (`0px`). Curves are deliberately excluded.

Every element—including windows, search inputs, contextual menus, status tags, selection bounding boxes, and action buttons—is rendered with razor-sharp orthogonal corners. This reinforces mechanical instrument precision, maximizes usable pixel surface area for tabular directory streams, and delivers a cold, futuristic workstation aesthetic.

## Components

### Buttons & Action Controls
- **Primary Execution:** Solid `#4F46E5` fill, `#FFFFFF` mono text, 0px corners, enclosed in a 1px border `rgba(255, 255, 255, 0.2)`. Focus emits a sharp `#38BDF8` perimeter glow.
- **Secondary Shell Action:** Translucent fill `rgba(255, 255, 255, 0.04)` framed by a 1px hairline `rgba(255, 255, 255, 0.12)`. On hover, background shifts to `rgba(79, 70, 229, 0.18)` with edge illumination.
- **Micro Toolbar Triggers:** 28px square hitboxes, raw icon glyphs centered, transitioning to `rgba(255, 255, 255, 0.08)` on active interaction.

### Contextual Chips & Telemetry Tags
- **Filter Tags:** Rectangular zero-radius tags with background `rgba(15, 23, 42, 0.85)` and border `rgba(79, 70, 229, 0.3)`. Selected filters activate a solid `#4F46E5` 2px left-indicator accent stripe.
- **System Attributes:** File flags (`HIDDEN`, `SYSTEM`, `SYMLINK`, `READ-ONLY`) display in `label-xs` micro-caps mono typography with tinted borders reflecting node permissions.

### File Explorer Grid & Directory Lists
- **Item Rows:** Alternating hairline demarcations or hover-activated horizontal raster bars with `rgba(79, 70, 229, 0.1)`. Selected items trigger a sharp 1px `#4F46E5` outer border and left inset tick mark.
- **Tree View Connectors:** Monospaced structural guide-wires connecting nested folders, preserving visual parent-child lineage across deep hierarchies.

### Form Inputs & Omnibar Search
- **Search Command Field:** Flat obsidian box framed with a 1px border `rgba(255, 255, 255, 0.1)`. Focus instantly swaps border to `#4F46E5` accompanied by an inner cyan coordinate prompt indicator.
- **Toggle Switches & Checkboxes:** Sharp square checkboxes (14x14px) with hairline outer frames. Checked state renders a solid `#4F46E5` core with high-contrast glyph center.

### Inspector Panels & Glassmorphic Cards
- **Telemetry Inspector:** Docked modular card containers featuring distinct header strips, mono key-value metadata pairings (`SIZE`, `MODIFIED`, `SHA-256`, `PERMISSIONS`), and inline live hex/ASCII previewers bordered by crisp obsidian lines.