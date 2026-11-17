# RNG Design — one system for the RNG Studio family

Applies to CutCat, PhotoFalcon, PDFrog, DocDuck, SlideSeal, the RNG Studio launcher and Genio.
All six desktop apps are egui; the tokens below live in one file, `rng_design.rs`, copied
byte-for-byte into each app (`crates/ui-egui/src/rng_design.rs`, `launcher/src/rng_design.rs`).
Change it here first, then copy it everywhere (`scripts/sync-design.sh`).

Sources: Apple Human Interface Guidelines (Foundations: layout, typography, colour, materials,
icons, motion, accessibility; Components: toolbars, sidebars, inspectors, popovers), the way Final
Cut Pro, Pixelmator Pro, Keynote, Pages and Preview lay out pro tools, and the community
"anti-AI-slop" catalogues (Impeccable, Hallmark, wystack anti-patterns).

## 1. Principles

1. **Content first (deference).** The video, photo, page or slide is the brightest, most saturated
   thing on screen. Chrome is quiet warm graphite. Colour in the chrome means state, never decoration.
2. **Clarity.** One obvious primary action per view. Text is legible at a glance (≥4.5:1). Icons
   are recognisable without their label, and every icon-only control has a tooltip with its shortcut.
3. **Glass for chrome, solid for content (Apple Liquid Glass).** Two layers: the *content layer*
   (the video, photo, page, slide, timeline lanes: solid) and the *glass layer* that floats above
   it: toolbars, sidebars, inspectors, tab strips, popovers, HUDs, dialogs. The glass is the RNG
   signature look: translucent warm graphite, a bright specular rim on the top edge, soft depth.
   Glass is functional (it holds controls), never a decorative card. Never a card inside a card.
4. **Consistency.** Same toolbar height, same sidebar row, same inspector grid, same home screen in
   every app. Someone who knows CutCat already knows SlideSeal.
5. **Harmony (concentric shapes).** Inner radius = outer radius − padding. Things nested inside a
   rounded container share its centre of curvature.
6. **Pro means fast.** Never break a shortcut or workflow for looks. Dense where experts work
   (timeline, layers, inspector), generous where people start (Home, dialogs).

## 2. Colour tokens

Neutrals are *warm graphite* (a hint of the RNG cream), never pure `#000`/`#fff`. The brand is
cream + signal red, from the RNG Studio "S." icon and Genio.

| Token | Dark | Light | Use |
|---|---|---|---|
| `canvas` | `#0e0d0d` | `#e8e5e1` | behind documents (timeline well, pasteboard, page desk) |
| `base` | `#151414` | `#f3f1ee` | window background, title bar |
| `panel` | `#1c1b1a` | `#fbfaf8` | sidebars, inspectors, panels |
| `raised` | `#242322` | `#ffffff` | selected tab, popover body, hovered card |
| `control` | `#2c2a29` | `#efece8` | buttons, fields, segmented controls |
| `hover` | `#33312f` | `#e6e2dd` | hover fill |
| `pressed` | `#3d3a38` | `#dbd6d0` | pressed fill, selected-neutral row |
| `separator` | `#2a2827` | `#e3ded8` | hairlines between groups (1 px) |
| `border` | `#3a3735` | `#cdc7c0` | field and button outlines |
| `text` | `#f3eee8` | `#1d1b19` | primary text (14.9:1 / 16.5:1) |
| `text_secondary` | `#aaa49d` | `#5f5a54` | labels, captions (≥6.3:1 / ≥5.8:1) |
| `text_tertiary` | `#7d7871` | `#8a847c` | placeholders and disabled only (never body copy) |
| `accent` | `#ff453a` | `#d70015` | playhead, selection outline, active indicator, links |
| `accent_fill` | `#d92b20` | `#d70015` | primary button fill — white label 4.9:1 / 5.4:1 |
| `accent_text` | `#ff6961` | `#c4000f` | red text on panels |
| `accent_tint` | accent @ 16 % | accent @ 12 % | selected row / selected tool background |
| `value` | `#e8b85c` | `#946510` | timecode, scrubbable numbers (the "gold") |
| `success` / `warning` / `danger` | `#32d74b` / `#ffb340` / `#ff6961` | `#1f8f3a` / `#a05a00` / `#c4000f` | status only |

Rules: red appears at most in **one** primary button per view plus state indicators. No gradients
in chrome (the only gradient allowed is the RNG Studio window backdrop). No purple, no cyan, no
neon glow. Status colours never carry meaning alone (add an icon or text).

## 3. Typography

Inter (OFL) for UI — the closest open face to SF Pro; JetBrains Mono only for timecode/code, not
as "tech decoration". Sizes are egui points at 100 %.

| Style | Size / weight | Use |
|---|---|---|
| Large title | 26 semibold, −0.4 tracking | Home screen app name only |
| Title 1 | 20 semibold | dialog titles, Home section titles |
| Title 2 | 15 semibold | panel titles, inspector headers |
| Headline | 13 semibold | group labels, selected list item |
| Body | 13 regular | default text, menus |
| Body dense | 12 regular | pro panels (timeline, layers, inspector rows) |
| Callout | 12 regular | secondary descriptions |
| Caption | 11 regular / medium | hints, shortcuts, metadata |
| Micro | 10 medium | ruler ticks, badges — never sentences |

At least two weights per view so there is hierarchy; never more than three. Sentence case
everywhere (no ALL CAPS labels except tiny badges). Line length ≤ 72 characters in prose.

## 4. Spacing, size, shape

- **8 pt grid, 4 pt half-step**: 4, 8, 12, 16, 24, 32, 48. Window margins on Home 48; panel inner
  padding 12; group gap 16; row gap 4. Rhythm: tight inside a group, loose between groups.
- **Control heights**: 28 regular, 24 dense (pro panels), 32 large (Home, dialogs).
- **Hit targets**: ≥ 28×28 regular, ≥ 24×24 in dense toolbars (pad the icon, not the glyph).
- **Radii**: 4 = clips, chips, small dense controls · 6 = buttons, fields, rows · 10 = cards,
  menus, popovers · 14 = dialogs and floating windows. Concentric rule above.
- **Bars**: title/tool bar 40 (dense 36) · tab strip 30 · status bar 22 · sidebar 232 (min 200,
  max 320) · inspector 296 (min 260) · sidebar row 28 · inspector row 28 (dense 24).

## 5. Materials — RNG Glass

Andre's call: the family is a **glass** design. egui has no backdrop blur, so glass is built from
four parts that read as glass without hurting legibility:

1. **Backdrop** — the window background is a soft warm vignette (`canvas` → `base`, radial, very
   low contrast) so translucent chrome has something to sit on. Content areas stay solid.
2. **Tinted fill** — the glass fill is `panel`/`raised` at an alpha chosen by how much text it carries.
3. **Specular rim** — 1 px top highlight (white 16 % dark / white 70 % light) fading to a 1 px
   white 6 % side rim, plus a 1 px black 25 % bottom edge in dark mode. This is what sells "glass".
4. **Depth** — a soft shadow only on floating glass (not on docked panels).

| Glass | Fill alpha | Radius | Shadow | Use |
|---|---|---|---|---|
| `glass.bar` | 86 % | 0 (docked) or 14 pill (floating) | none / 0 8 24 @ 35 % | title bar, tool strips, transport |
| `glass.sidebar` | 90 % | 10 inset 6 from the window edge | none | sidebars, inspectors, panel frames |
| `glass.popover` | 94 % | 10 | 0 8 24 @ 40 % | menus, dropdowns, tooltips |
| `glass.hud` | 72 % (over video/canvas only) | 14 pill | 0 4 16 @ 35 % | zoom pill, floating transport, Genio HUD |
| `glass.dialog` | 96 % + 30 % scrim | 14 | 0 16 48 @ 50 % | sheets and dialogs |

Rules: text on glass is always `text` / `text_secondary` (never tertiary); the lower the alpha,
the less text. Glass never sits on glass except a popover over a bar. Content (frames, pages,
clips, layers thumbnails) is never glass. Docked panels separate by 6 px gaps of backdrop, not by
lines — that gap is what lets each panel read as its own pane of glass.

## 6. Icons

One set: **Tabler outline** (MIT; already in CutCat `assets/icons/tabler`). 24-grid glyphs drawn at
16 px (toolbar), 14 px (dense rows), 20 px (Home, sidebar large). Stroke stays ~1.5 px at display
size. Colour: `text_secondary` at rest, `text` on hover, `accent` when a tool is active. One
colour per icon — no multi-coloured icon sets, no icon "chips" above every card heading. App
icons (Magnific cat/falcon/frog/duck/seal) are the only illustrated art and appear once per screen.

## 7. Anatomy

- **Title bar / toolbar** (Final Cut, Keynote): leading = Home + workspace switch (segmented);
  centre = document name (Headline, `text_secondary` when clean, `text` + • when edited); trailing =
  ≤ 5 icon buttons + one primary action (Export / Present / Share). Menus stay as a menu bar on
  Linux/Windows, in the same 40 px row.
- **Sidebar** (source list, Pages/Keynote navigator): `panel` fill, rows 28 with 6-radius
  selection (`accent_tint` fill + `text` + accent icon), section headers Caption semibold
  `text_tertiary`, collapsible. No coloured left-border "tab" accents.
- **Inspector** (Pixelmator Pro / Final Cut): right side, tabs as a segmented control at the top,
  then disclosure groups. Two-column grid: label column 96 px `text_secondary` right-aligned,
  control column fills. Values scrubbable in `value` gold.
- **Canvas** sits on `canvas`, centred, with a soft 0 2 12 shadow under the page/frame only.
- **Status bar**: Caption `text_tertiary`; nothing critical lives only there.

## 8. Home screen (same in all five apps)

```
[icon 40] AppName                                   (Large title, left-aligned, 48 margin)
          one plain sentence of what it does         (Callout, text_secondary)

[ New Project ]  [ Open… ]  [ Import… ]             (one primary red, rest neutral, 32 high)

Start from                                           (Title 2)
[thumb] [thumb] [thumb] [thumb] …                    (real previews: templates, themes, presets)

Recent                                   [search]    (Title 2)
[thumb  name  · modified  · path]  …                 (rows or grid; empty = one line + Open…)
```

No "Welcome to X" heading + restating intro, no greeting gimmicks, no icon-card grids, no
"Recommended tools" cards inside a card. Drop anywhere to open.

## 9. States & motion

- Hover: fill one step up (`hover`), 120 ms ease-out. Press: `pressed`, instant. Selected: neutral
  `pressed` for lists, `accent_tint` + accent icon for tools/modes.
- Focus (keyboard): 2 px `accent` ring at 1 px offset, always visible when Tab-navigating.
- Disabled: 40 % opacity of the normal style, no hover.
- Durations: 120 ms (hover/press), 180 ms (popover, disclosure), 240 ms (page/view change).
  Ease-out cubic; **no bounce or elastic**; animate opacity/transform only; respect Reduce Motion
  (the apps' "Animate menus" setting → 0).

## 10. Accessibility

Text ≥ 4.5:1 (all `text*` tokens above pass except `text_tertiary`, reserved for disabled and
placeholder); UI glyphs and focus ring ≥ 3:1. Full keyboard navigation (Tab order follows reading
order, Esc closes popovers, Return = primary). Every icon-only button has a tooltip with its
shortcut. Light and dark both first-class; follow the system appearance by default. Respect UI
scale 100–200 % without clipping.

## 11. Copy

Buttons are verbs naming the result ("Create Project", "Export Video", not "OK"/"Submit"). Errors
say what happened, why, and the fix — never blame the user. Empty states always offer the next
action. Sentence case, no exclamation marks, no emoji in UI.

## 12. Anti-slop checklist (fail the review if any is true)

- Purple/blue or any decorative gradient; gradient text; neon glow on dark.
- Pure `#000` / `#fff` surfaces; grey text on coloured fills.
- Everything wrapped in cards; cards in cards; identical icon + title + text card grids.
- Glass used on content or as decorative cards (glass is for chrome only); generic big drop shadows.
- Thick coloured border on one side of a rounded element.
- Every button primary; more than one red thing competing in a view.
- Centred-everything layouts; uniform padding with no rhythm; one weight for all text.
- Big rounded icon above every heading; mixed icon sets or multicolour icons.
- Bouncy easing, animating layout size, scattered micro-animations.
- "Welcome to …" + an intro that repeats it; "Click here"; vague errors; empty state with no action.
- Leftover upstream styling: Premiere-grey light theme, blue selections, Ubuntu-font fallbacks,
  mismatched reds between apps.
