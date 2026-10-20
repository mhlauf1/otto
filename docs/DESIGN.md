# Design

> Rules (SPEC §6): references come **only** from Mobbin, the system font only (no serif), no orange or terracotta, no generic "AI app" look. Every value is a token in `src/design/tokens.ts`.

---

## Brand — Otto (M0, 2026-10-20)

### Idea

Otto exists so one person can be two identities without them ever touching. The brand says that in one shape.

- **The mark, "the divide":** a single ring (the _o_ in Otto) split by a hard vertical line. It's one person and two identities, with a clean boundary between them. `src/assets/brand/otto-mark.svg`
- **The wordmark:** _otto_ is a palindrome, and the wordmark leans into it. The two *t*s share one crossbar (Otto spans both worlds), and their tails turn toward each other and **stop short**, leaving the same gap as the mark. `src/assets/brand/otto-wordmark.svg`
- **The mark is functional, not decoration.** In the app, the active workspace lights its half: **left = Playbook** (cobalt) and **right = Lauf** (jade), and the other half dims. The logo doubles as the "which identity am I in?" indicator (SPEC §4.5). Component: `src/brand/OttoMark.tsx`.

### Assets

| File                                 | Use                                                       |
| ------------------------------------ | --------------------------------------------------------- |
| `src/assets/brand/otto-mark.svg`     | The mark, `currentColor`, 64-unit grid                    |
| `src/assets/brand/otto-wordmark.svg` | The wordmark, `currentColor`, 7.5-unit stroke             |
| `src/assets/brand/otto-lockup.svg`   | Horizontal lockup                                         |
| `src/assets/brand/app-icon.svg`      | macOS app icon source → `pnpm icons` → `src-tauri/icons/` |

### Construction

- **Mark:** outer radius 26, inner radius 15 and a 6-unit gap on a 64 grid. Ring weight (11) ≈ 2× the gap, so the split reads at 16px.
- **Wordmark:** geometric, built from circles (r 16) and straight strokes, x-height = o diameter, crossbar on the x-height line. It's drawn as paths rather than set in a font, so it never depends on the installed fonts.
- **App icon:** macOS grid (824px body on a 1024 canvas, radius 185). A graphite body with a slight vertical falloff (#3A3A40 → #17171A) and a hairline top rim, with a white mark at 8× scale. The subtle depth follows the app-icon conventions in the Craft and Cursor references below. There are no colour gradients.
- **Clear space:** ½ the mark's height on every side. Minimum mark size is 14px.
- **Don't:** recolour halves outside the workspace mapping, close the gap, add effects, or set the wordmark in a typeface.

### Colour

- **Neutrals: graphite.** Cool greys from `#111114` to `#F5F5F7` (light) and from `#161618` to `#F2F2F4` (dark). Never warm greys.
- **Workspace accents** (one each, used sparingly: the mark's half, the active dot, the title-bar name, emphasis text):
  - Playbook: **Cobalt** `#4062F0` light / `#6C86FF` dark
  - Lauf: **Jade** `#13875F` light / `#34C08F` dark
- A Vitest test fails if any accent hue falls in the orange band.
- **Open question for M3:** jade sits close to the "success" green. Identity Doctor statuses must therefore never rely on hue alone: always pair colour with a glyph (✓ / ✕) and a label.

### Type

SF Pro via `-apple-system` (Text at small sizes, Display at 20px and up), and SF Mono via `ui-monospace` for terminals and code. The ramp is macOS-native: body 13px, and tracking tightens as size grows.

### Voice

Calm, plain and precise. Short sentences and no exclamation marks. Say what's true about identity ("You're in Playbook."), not what's exciting. Tagline: **"Every identity, signed in. Nothing crossing over."**

---

## Mobbin references

### Brand (M0)

| Reference                                                                                             | Borrowed                                                                                                                |
| ----------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------- |
| [Cursor — brand guidelines](https://mobbin.com/sites/sections/8f2aefcb-c254-4701-8ddd-3ffe5aeca749)   | A monochrome geometric mark on a near-black app-icon body, and restraint: the mark does all the work                    |
| [Beside — brand assets](https://mobbin.com/sites/sections/347fc27a-e5ae-4651-86a1-4cac1c3b1d61)       | System structure: mark, wordmark and lockup as separate assets, black/white only, "don't recolour" rules                |
| [Craft — app icon](https://mobbin.com/sites/sections/62fcdcb6-3868-4a1a-8cfb-475c0756f3b1)            | macOS icon depth: a dark body with a soft vertical falloff and a rim highlight, rather than a flat fill                 |
| [Framer — app icon & colours](https://mobbin.com/sites/sections/822d4a02-7bc6-475f-90ff-d02bc887c2ee) | A tight palette: black, white and only two saturated blues, as the precedent for "neutrals plus one accent per context" |

### Shell (M0 hello window, full pass in M3)

| Reference                                                                                             | Borrowed                                                                                               |
| ----------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------ |
| [Linear — workspace switcher](https://mobbin.com/screens/e8fe60b4-6db4-4485-99b8-3395d69ea7db)        | Workspace switching with numbered shortcuts, and quiet grey selection fills instead of coloured ones   |
| [Linear — sidebar](https://mobbin.com/screens/3e4c96cc-b973-44eb-a665-c1045c5a39c4)                   | Small caption section labels, 28px rows, and a coloured dot per team as the only colour in the sidebar |
| [PlanetScale — console](https://mobbin.com/screens/a1fa1590-52b6-4554-b83c-03e998fe6537)              | A calm light shell framing a dark terminal surface (for M2)                                            |
| [LangChain — sidebar & environments](https://mobbin.com/screens/ef3381e2-522c-48f4-94af-96e5c26004b2) | Identity at the bottom of the sidebar (avatar and email), for the M3 Identity Doctor entry point       |

---

## Tokens

Source: `src/design/tokens.ts` → `pnpm tokens` → `src/design/tokens.css`. A test fails if the CSS is stale. Stylelint rejects raw colours and spacing in components.

| Group  | Values                                                                                    |
| ------ | ----------------------------------------------------------------------------------------- |
| Space  | 4px grid (half steps 2px and 6px for hairline gaps)                                       |
| Radius | 4, 6, 8, 12, 16, full                                                                     |
| Type   | caption 11, footnote 12, body 13, callout 14, title3 15, title2 17, title1 22, display 34 |
| Motion | 80 / 140 / 220 / 360ms with a standard ease. All zero under Reduce Motion                 |
| Layout | sidebar 248px, title bar 52px                                                             |

## Screens

### M0 — hello window

Native overlay title bar with traffic lights at (18, 22), a translucent sidebar over native `sidebar` vibrancy, and a solid content surface. The sidebar holds the lockup, a Workspaces section (dot, name, ⌘ shortcut) and a milestone footer. The centre holds the large mark lit for the active workspace, the tagline, and a pill showing the round trip to the Rust core (`Otto 0.1.0 · macos · aarch64`).

Screenshots: `pnpm test:e2e` writes `test-results/hello-{light,dark,lauf}.png`. CI uploads them as an artifact.

### M1 — Identity Doctor inspector

This is a functional panel on the right (320px), toggled with ⌘⇧I or the title-bar "Identity" button. Each check shows a **glyph, a word (for VoiceOver) and a colour**, never colour alone, which resolves the jade-vs-success concern above. The sidebar shows ✓/✕ per workspace for push safety. It follows the Linear and LangChain references above; the full visual pass happens in M3.

### M2 — Terminal sessions

- **Terminal surface:** always graphite-dark (`terminal` tokens), inset in the content area with a 12px radius and a medium shadow. This is the "calm light shell framing a dark terminal" from the PlanetScale reference. The ANSI palette is tuned to the brand: blue is the dark-mode cobalt and green is near the success green. There's no orange.
- **Sidebar:** a Sessions section per workspace with a running dot in the workspace accent, plus "+ Claude / + Codex / + Shell" chips.
- **Identity Doctor:** a red Claude or Codex row shows an accent "Sign in to … as <Workspace>" button that opens the sign-in flow in a terminal.
