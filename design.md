# rustygrep — design system

A token-first dark system. The page is a terminal that grew a coat of rust. Every decision bends toward one goal: **fewer tokens, same information** — the exact promise of the tool, applied to the site itself.

Identity: tempered rust (#e8590c) on near-black warm graphite (#0f1114). No navy, no teal, no purple, no gradient-slop. Rust is the only color. Medium-weight Inter, mono everywhere code appears. Mirrors grep's own grammar — paths, line numbers, results — as visual language.

---

## Tokens

### Canvas (warm — black is too cold for a product about warmth-per-token)

| Token | Value | Role |
|---|---|---|
| `--canvas-top` | `#0f1114` | Page start, warm near-black |
| `--canvas-bottom` | `#050608` | Page end, deepens focus |
| `--surface-1` | `#161a1e` | Cards, panels |
| `--surface-2` | `#1c2126` | Featured card |
| `--surface-3` | `#232930` | Code block wells |
| `--hairline` | `#2a3138` | Card borders, dividers |
| `--hairline-strong` | `#39414a` | Focus rings, input borders |

### Ink (warm text ladder — reads as forged metal, not print)

| Token | Value | Role |
|---|---|---|
| `--ink` | `#ece8e2` | Headlines — iron |
| `--body` | `#b6b0a8` | Reading text — worn steel |
| `--muted` | `#827c74` | Secondary — unpolished |
| `--subtle` | `#4d4category100` | Tertiary, captions |
| `--subtle` | `#59534c` | Tertiary, captions |

### Accent (rust — the one color)

| Token | Value | Role |
|---|---|---|
| `--rust` | `#e8590c` | Primary, CTAs, focus, highlights |
| `--rust-hover` | `#cf4d08` | Hover state |
| `--rust-active` | `#b34206` | Active / pressed |
| `--on-rust` | `#ffffff` | Text on rust |
| `--orange-soft` | `#ff8f4d` | Token market highlight, hover glow |
| `--good` | `#4ade80` | Token savings, success |
| `--bad` | `#f87171` | Error, over-budget |

---

## Type

| Token | Family | Size | Weight | Line | Use |
|---|---|---|---|---|---|
| Display XL | Inter | 64px / 52px | 500 | 1.05 | Hero headline |
| Display LG | Inter | 44px | 500 | 1.1 | Section heads |
| Headline | Inter | 22px | 500 | 1.3 | Card titles |
| Lead | Inter | 18px | 400 | 1.6 | Hero paragraph |
| Body | Inter | 16px | 400 | 1.6 | Reading |
| Body SM | Inter | 14px | 400 | 1.5 | Cards, lists |
| Mono SM | JetBrains Mono | 13px | 400 | 1.5 | Code, flags |
| Mono XS | JetBrains Mono | 11px | 500 | 1.3 | Eyebrows, labels, captions, badges, metadata |

Rules:
- Mono is the grammar voice — only for paths, line numbers, flags, labels, code. Never body copy.
- Inter at 500 for display, never below 400 in body. No 700+.
- No letter-spacing on body. Mono gets 0.3-1px only at caption/eyebrow.
- Em-dash banned in prose. En-dash allowed in ranges ("60-95%").

---

## Components

- **Card** — `--surface-1`, 1px `--hairline`, radius 12px, pad 24px. No shadow. No lift on hover — border brightens instead.
- **Code well** — `--surface-3`, hairline, mono 13px, radius 10px, pad 16px. Line numbers in `--subtle`, paths in `--muted`, matches in `--rust` on flat canvas.
- **Button primary** — `--rust`, `--on-rust`, radius 6px, pad 12px 18px, mono 13px 500. Hover `--rust-hover`. Always a single verb ("install", "copy").
- **Button ghost** — transparent, 1px `--hairline-strong`, `--body` text. Hover hairline → ink.
- **Badge / chip** — mono XS, hairline, radius 999px, pad 4px 10px. Tokens chip: `--good` dot + muted text.
- **Eyebrow** — mono XS, `--subtle`, uppercase, letter-spacing 1.5px, leading dot in `--rust`.
- **Install strip** — inline `code` on `--surface-2` with mono copy button on right.
- **Frosted nav** — `backdrop-filter: blur(14px)` on `rgba(15,17,20,.72)`, 56px, hairline bottom.

---

## Layout

- **Shell**: max-width 1080px, 24px gutters.
- **Spacing**: base 4 · 8 · 12 · 16 · 24 · 32 · 48. Section 96px. Cards gap 16px.
- **Grid**: features 3-up → 2-up (<=1024) → 1-up (<=640). Compare 2-up → 1-up.
- **Reading column** max 680px for prose.

---

## Motion

- The token counter: hero numbers count **down** (12,400 → 4,100) on scroll into view. `prefers-reduced-motion: reduce` → static.
- Fade-up on hero + cards: 200ms, `cubic-bezier(0.16, 1, 0.3, 1)`, one element group not `all`.
- Hover: border brightens, accent shifts 100ms. No transform lifts. No `transition: all`.

## Do / Don't

**Do** — warm black canvas · rust the only accent · mono for all metadata · 1px hairline input borders · metric first ("60-95% fewer tokens") · code-as-visual.

**Don't** — flat `#000` · multi-color palettes · purple/teal/navy · drop shadows for depth (surface ladder does it) · text-shadow glow · decorative gradients · emoji in UI · 700+ display weight.
