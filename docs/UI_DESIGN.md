# UI Design

## Direction

Windows 11 native-ish, quiet, high information density, single accent color.
No neon, no dashboard-template feel, no ambient animation. Translucency only
where it is cheap (Mica-like surfaces later via window effects, not CSS blur
stacking).

## Design tokens (src/styles/tokens.css)

All components consume CSS variables; nothing hard-codes colors or spacing.

- Surfaces: `--bg`, `--surface`, `--surface-hover`, `--surface-active`
- Text: `--text-primary`, `--text-secondary`, `--text-tertiary`
- Accent: `--accent`, `--accent-soft` (single hue; user-configurable in M7)
- Lines: `--border`, `--border-strong`; depth: `--shadow`
- Shape: `--radius-s|ml|l`; Space: `--space-1..6`
- Type: `--font-size-s|m|l|xl`, Segoe UI Variable stack
- Motion: `--duration-fast` (120ms) / `--duration-normal` (200ms),
  `--ease-out`; disabled under `prefers-reduced-motion`

Theme switching is attribute-driven (`:root[data-theme="light"|"dark"]`).
"System" follows `prefers-color-scheme` live via matchMedia. The preference is
persisted through the settings repository (key `ui.theme`).

## Layout grammar

- Left sidebar (216px): brand, nav, version footer. Content column centered
  with a readable max width (720px for text-heavy pages).
- Page header pattern: small overline/greeting + one clear h1. Actions on the
  right, never floating.
- Cards: 1px border + tiny shadow; they are containers, not decoration.

## Page blueprints (V1)

- **Today**: date + greeting, today's tasks (top 5), next event, focus
  summary line, quick actions (start focus, new task). No charts in V1.
- **Desktop**: virtual organizer grid of desktop items; collection rail;
  drag/drop assignment; search box pinned top-right.
- **Focus**: large timer, preset chips (25/5, 50/10, custom), task binding
  select, session note; mini timer floats bottom-right while running.
- **Calendar**: agenda list default; week grid with drag-to-time-block;
  month grid secondary.
- **Tasks**: single list, inline create, status triad, priority dot, due
  chips; keyboard-first.
- **Settings**: grouped rows; segmented controls; no nested tab jungle.

## Accessibility & input

- Focus-visible rings using accent; full keyboard navigation; `aria-current`
  on active nav. Reduced motion respected globally (base.css).
