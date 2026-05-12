# Icon Design Spec — zxcv

## Purpose

A README hero icon placed at the top of `README.md` to attract developers by visually communicating the app's core concept: natural language input transforms into a shell one-liner.

## Decisions

| Decision | Choice | Rationale |
|---|---|---|
| Usage | GitHub README header | Shown at top of doc, no small-size constraints |
| Style | Modern minimal | Concept-first, no decoration |
| Color | Terminal green | Classic CLI signal; GitHub dark-mode native |
| Text | None | README `# zxcv` heading provides the name |
| Container | None (floating marks) | Airy, works on transparent bg, dark & light |

## Visual Spec

**Concept:** Speech bubble → green arrow → terminal block

**Canvas:** `280 × 130` viewBox, no background fill (transparent)

### Speech bubble (left)
- Shape: `rect` at `(4,10)`, size `92×66`, `rx=16`
- Fill: `#161b22`, stroke: `#30363d` 2.5px
- Tail: `path M22 76 L12 98 L42 76 Z`, same fill/stroke
- Content: three dots `cx=34,50,66` `cy=43` `r=5` fill `#6e7681`

### Arrow (center)
- Shaft: `line (106,43)→(144,43)`, stroke `#3fb950` 3.5px round cap
- Head: `polygon 144,35 160,43 144,51`, fill `#3fb950`

### Terminal block (right)
- Shape: `rect` at `(168,6)`, size `108×74`, `rx=13`
- Fill: `#0a1a0e`, stroke: `#3fb950` 2.5px
- Chrome dots: 3× circles at `cx=184,195,206` `cy=22` `r=3.5` fill `#3fb950` opacity `0.45`
- Chrome divider: `line (168,32)→(276,32)` stroke `#3fb950` 1px opacity `0.25`
- Prompt `$`: monospace bold, `(180,55)` fill `#3fb950` font-size 15
- Command `ls -la`: monospace, `(196,55)` fill `#3fb950` font-size 12 opacity `0.9`
- Cursor: `rect (240,43)` size `9×15` rx=2 fill `#3fb950` opacity `0.9`

## Output File

`assets/icon.svg` — placed in repo root-level `assets/` directory.

## README Embed

```markdown
<p align="center">
  <img src="assets/icon.svg" alt="zxcv icon" width="320" />
</p>
```

Centered, fixed width 320px — renders well at any viewport width.

## Light Mode Compatibility

The transparent background means the icon renders on both GitHub dark (`#0d1117`) and light (`#ffffff`) modes. The terminal block's green palette reads well on light bg; the speech bubble's `#161b22` fill contrasts sufficiently on white.
