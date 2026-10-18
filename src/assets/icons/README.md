# Custom icons

Hand-made SVG icons live here and nowhere else. Never inline `<svg>` markup in a component or composable.

- Ready-made icons come from **Phosphor Fill** (`~icons/ph/<name>-fill`). Only draw a custom icon when Phosphor has no suitable one.
- One icon per file, named in kebab-case: `rune-mark.svg`.
- Filled, single colour, drawn on a 24x24 viewBox, using `currentColor` so it follows the text colour.
- Register it in `src/icons/index.ts` (`import IconRuneMark from '~icons/custom/rune-mark'`) under a name that describes its meaning, then use `<AppIcon name="..." />`. Components never import icons directly; the lint rule `no-restricted-imports` enforces it.