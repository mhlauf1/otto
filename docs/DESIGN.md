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

---

## M3 — Shell and design pass

### Mobbin references

| Reference                                                                                     | Borrowed                                                                                                 |
| --------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------- |
| [Supabase — command palette](https://mobbin.com/screens/9b4725f5-e038-4fc7-98ad-64a59f74b4d7) | Sectioned groups with small caption headings and shortcut hints right-aligned in bordered keycaps        |
| [Magnific — command palette](https://mobbin.com/screens/e22e26e2-f813-4f1e-beda-43c9ecf26419) | A footer of key hints (↑↓ navigate · ↵ select · esc close) and a soft filled row highlight               |
| [Navattic — command palette](https://mobbin.com/screens/c65971cf-c4ea-45e9-99a5-555930cb5d73) | Calm light panel, generous radius, a quiet dimmed backdrop rather than a heavy blur                      |
| [Ferndesk — agent composer](https://mobbin.com/screens/b2c10835-8ff6-4283-9462-0e2a5b38552d)  | A "Connected to" pill in the composer header, which becomes Otto's identity pill (`Playbook · mikelauf`) |
| [Cursor — agent composer](https://mobbin.com/screens/924cf1be-b47e-4957-8ac7-426c93ee8835)    | A slim controls row under the field, with a round send button on the right                               |
| [Langdock — composer](https://mobbin.com/screens/b66870ae-8d47-493d-b1b6-e6c01173ad96)        | A tall multi-line field that grows with the text, suited to long dictated prompts                        |
| [LangChain — sidebar footer](https://mobbin.com/screens/ef3381e2-522c-48f4-94af-96e5c26004b2) | The identity block (avatar, name, email) pinned to the bottom of the sidebar                             |

### What was built

- **Command palette (⌘K):**
  - Radix Dialog, a fuzzy match (prefix > word start > subsequence), and groups when browsing (New in <workspace>, Workspaces, Sessions, Identity). Search results are one ranked list.
  - Full keyboard support, and a combobox/listbox ARIA pattern for VoiceOver.
- **Composer:**
  - Sits under the active terminal and grows to 240px.
  - The border and glow take the workspace accent on focus.
  - **⌘↵** sends as a bracketed paste followed by Return, so dictated multi-line prompts arrive whole. **esc** returns to the terminal, and **⌘L** focuses the composer.
  - **⌥↵ (queue) ships with the queue in M5**; it isn't stubbed here.
- **Sidebar:** workspaces with ✓/✕ push safety, sessions with a running dot, new-session chips, and an identity footer in the workspace accent.

### Design QA (SPEC §6.5)

`pnpm test:e2e` writes `hello-{light,dark,lauf}.png` and `palette-{light,dark}.png`. Notes against the references:

- **Palette:** matches Supabase's grouping and keycaps, and Magnific's footer. The first QA pass found the global focus outline clipped under the input as a thick accent bar; it was removed, since the open palette is the focus indicator.
- **Identity panel:** the first pass wrapped the email mid-word. Now the user sits on one line and the email truncates with the full value in a tooltip.
- **Dark mode:** the panel uses `surfaceRaised` over a darker scrim (0.45), so it lifts without a border glow.

### M3 revision after Mike's first use (2026-10-20)

Two inputs (Claude's own `❯` line plus Otto's composer) were confusing, and ⌘↵ wasn't discoverable.

- **The terminal is the everyday input.** The composer is now an on-demand sheet: ⌘L or the title-bar "Compose" button. ↵ sends, ⇧↵ adds a line, esc closes and returns focus to the terminal. It doesn't send while dictation or IME text is still composing.
- **One "+ New session" menu** (Radix DropdownMenu) replaces three equal chips: Claude Code (⌘T), Codex, and Shell (described as "Terminal signed in as this workspace").
- **Known gap:** sessions open in the home folder. A project picker is next on the list.

## M3.5 — Shape and labels (2026-10-20)

Mike still found the composer confusing after the M3 revision, and couldn't tell how workspaces, projects and sessions related. This pass gives every part one clear word (SPEC §2.1) and one place.

### Mobbin references

| Reference                                                                                     | What Otto takes from it                                                                                                   |
| --------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------- |
| [ChatGPT — projects sidebar](https://mobbin.com/screens/8b37adf6-385f-4d04-afb8-99ce66b1d17f) | Chats listed under their project, one step in. Otto lists sessions under their project the same way                       |
| [Height — team sections](https://mobbin.com/screens/62cabfa8-fa79-43f7-8f02-11ee362b4609)     | Caption-label sections with indented children, so groups like Clients and Studio read as headings, not rows               |
| [Wrike — spaces tree](https://mobbin.com/screens/d750f820-568f-4a4f-8540-1692449871de)        | A chevron to expand, plus "+" and "···" that appear only on hover, so the tree stays quiet                                |
| [Plane — project overview](https://mobbin.com/screens/b40e6a3a-28da-4f4d-86ba-3bd593a9fdf0)   | A left-aligned landing page with a header and a properties side card. This became the Overview and its "What's what" card |
| [Coda — workspace home](https://mobbin.com/screens/ba6f5461-ff84-4795-b8f6-a11d5a7526a9)      | The workspace name as the page title, with a plain recent-items list under it                                             |

### What was built

- **Workspace switcher:** a segmented control. Each segment shows its own dot, name and ✓/✕, and the active one lifts on the raised surface. It replaces the Workspaces list.
- **Tree:** groups (caption labels; ··· gives Rename, Move up or down, Delete), then projects (chevron, name, a `local` tag, and "+" and "···" on hover), then sessions indented under them.
  - "Other projects" holds anything ungrouped. Playbook has no groups yet, so its section is labelled "Projects".
  - **General** comes last.
  - The project new sessions open in is shown in bold, and "+ New session" names it ("New session in Lauf · stoc").
- **Title bar:** a workspace · project · session breadcrumb, with the workspace in its accent.
- **Overview:** the workspace name, a ✓ All clear or ✕ Pushes blocked pill that opens Identity, Running now, Projects (hover for "Open Claude Code"), the "What's what" glossary card, and the core status.
- **Knowledge:** a placeholder that says plainly what M4 adds.
- **Removed:** the composer and the title-bar Compose button (ADR-018).

### Design QA

`pnpm test:e2e` writes `overview-{light,dark}.png`, `sidebar-lauf-{light,dark}.png` and `knowledge-{light,dark}.png`. In the first pass the switcher's dot floated away from the name, because the name stretched to fill the segment. The segment now centres its contents.

### Iteration after Mike's first look (2026-10-20)

- **Adding is visible, not hinted.** "Import projects…" and "New group…" are full 28px rows with a "+", in secondary text rather than tertiary. Every group heading has an always-visible "+" with a menu: move an existing project in, or "Import new projects…" straight into that group.
- **Empty groups offer an action.** "+ Add a project" replaces the "No projects yet. Use ··· …" hint.
- **Names, not folders.** Rename a project from its ··· menu. The tooltip still shows the folder name.
- **Chevron:** a drawn SVG that rotates, replacing the ▸ text glyph, which rendered like a dot at caption size.

## M4 — Knowledge (2026-10-20)

### Mobbin references

| Reference                                                                                               | What Otto takes from it                                                                                                                                            |
| ------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| [hilos — docs with "Agents read this"](https://mobbin.com/screens/3426d6f0-cef4-4139-ada2-95a6cb0e9ead) | A docs list beside the document, plus a plain label saying whether agents read it. This became the "Agents read this" and "Private: agents don't read this" badges |
| [Cofounder — Preview and Source](https://mobbin.com/screens/8b26b489-9c60-4b6a-984a-e796e271bfc7)       | A two-way toggle above the document. This became Read and Edit, with Markdown source in Edit                                                                       |
| [Obvious — notes list](https://mobbin.com/screens/a65b9010-7f2d-4a52-995c-fdb9d731f9d6)                 | List rows with a title plus a one-line snippet, which Otto takes from each note's first sentence                                                                   |
| [Evernote — notes beside a note](https://mobbin.com/screens/77b58346-67f9-4f5d-bd8c-4e2c837841ce)       | A middle list with the selected note filling the rest                                                                                                              |
| [Lightfield — record notes](https://mobbin.com/screens/ef271621-2ea1-4bc8-a501-dbfe9ac56ef8)            | A calm reading column with properties above the body                                                                                                               |

### What was built

- **Page:** a "Knowledge" title with "What Lauf's agents know. Kept on this Mac, never in a repo." On the right sit "Include private" and an accent-filled **Ask Lauf** button.
- **List (260px):** "+ New note", then Shared (About Mike), Notes ("Every session gets an index of these"), Private (lock icon and "+"), From your projects (folding per project) and Assets ("+" adds images).
- **Reader:** the title, an area badge (the accent-tinted "Agents read this" for notes), a save status, and a Read/Edit segmented control. Read renders GitHub Markdown, with note headings one level below the page's h1. Mermaid diagrams load only when needed and follow light or dark. Edit is a plain monospace field that saves 600 ms after typing stops.
- **Assets** show the image and a copyable `![name](../assets/name)` snippet for using it in a note.

### Design QA

`pnpm test:e2e` writes `knowledge-{light,dark}.png` and waits for the diagram to draw. The first pass matched Mermaid's off-screen measuring element, so the screenshot caught "Drawing diagram…". The test now waits for the SVG inside the reader.

### Rework after Mike's first look (2026-10-20)

Mike found the first layout clashing: the list sat in a bordered card with a second grey inside the page, and snippets ran out of the column.

- **One flat surface.** There's no card: the list column and the reader sit directly on the page, split by a single hairline (Evernote, Obvious).
- **The column carries its own header.** "Knowledge" with "On this Mac, never in a repo." and a bordered "+" for a new note. **Ask** and "Include private notes" moved to the foot of the column, like a composer.
- **Rows** show the title on one line and a snippet clamped to two lines, clipped inside the column (`min-width: 0` all the way down).
- **The reader header** is a breadcrumb ("Notes / Clients") plus the badge. The note's own title is the one big heading.

## M5a — Session status and paused sessions (2026-10-20)

- **Session rows show their status in words** (never colour alone), with a dot:
  - **Working:** a breathing accent dot (no motion under Reduce Motion).
  - **Needs you:** an accent dot with a focus ring, and an accent "Needs you" label.
  - **Ended:** a grey dot.
- **Paused** (open when Otto last quit): a hollow accent dot, secondary text, and "Paused". Hovering shows × to forget it. Clicking resumes it where it left off.
- **Notifications:**
  - Title "Lauf · stoc · Claude Code", with the agent's own message (for example "Claude needs your permission to use Bash"), else "Finished. Your turn." or "Needs you."
  - They're sent only when Mike isn't already looking at that session.
- **"From your projects" removed** (Mike: too many files). The list is now just Shared, Notes, Private and Assets.

## M5b — Up next and parallel tasks (2026-10-20)

- **Up next** sits under the terminal and is quiet until used. It's a single "+ Up next" text button. Once items exist, a caption reads "Up next · runs when Claude finishes", with numbered rows (tabular figures) that show × on hover.
- **The add field** is a raised card with a borderless field, a key hint line ("↵ add · ⇧↵ new line · ⌘↵ in parallel · esc cancel"), a secondary **Run in parallel** button and an accent **Add to Up next** button. It never looks like a chat input: no avatar, no send arrow, and it closes after adding.
- **Parallel sessions** show their branch as a small monospace tag (without "otto/") on the session row.
- **The project ··· menu** gains "Parallel worktrees". Each one shows why it can't be removed yet ("A session is running in it", "Has uncommitted changes") or that "The branch stays".
