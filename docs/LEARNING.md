# Learning Otto

Plain-English notes for a React and TypeScript developer learning desktop apps, Tauri and Rust. There is one chapter per milestone, and new concepts are added as they appear.

---

## Chapter 0 — The shape of the thing (M0)

### What Otto is made of

Otto has two halves that talk to each other:

- **The webview (your React app).** Tauri doesn't bundle a browser like Electron does. It uses the browser engine macOS already has (WKWebView, the engine behind Safari). Your React code runs there exactly as it would on the web, which is why the app stays small (a few MB instead of 100+) and starts fast.
- **The core (Rust).** This is a normal native program. It can do things a web page can't: start processes, open terminals, read the Keychain, write files. In Otto it's the part that keeps workspaces isolated.

### IPC: how the two halves talk

**IPC** (inter-process communication) is how the webview asks the core to do something. In Tauri you mark a Rust function with `#[tauri::command]`, and the frontend calls it like an async function. It feels like `fetch` to a local API, but without HTTP.

Otto uses **tauri-specta**, which reads the Rust function signatures and generates `src/bindings.ts`. You call `commands.appInfo()` and TypeScript knows the exact return and error types. If someone renames the Rust function, your TypeScript build breaks, which is what you want.

### Two package managers

- **pnpm** manages JavaScript dependencies (`package.json`, `pnpm-lock.yaml`).
- **cargo** manages Rust dependencies (`Cargo.toml`, `Cargo.lock`) and also builds and tests Rust code. Think of it as npm, tsc and Vitest rolled into one.

### Why two Rust crates?

A **crate** is Rust's unit of code, roughly an npm package. Otto has `otto-core` (pure logic, no Tauri) and `src-tauri` (the app shell). Keeping the logic Tauri-free means it can be tested quickly with `cargo test`, with no window, which matters for the isolation tests in M1.

### Capabilities and CSP

Tauri gives the webview **no power by default**. Each permission (for example "may open a file dialog") is granted in a capabilities file. Otto grants almost nothing, so even if malicious content got into the webview, it couldn't start processes. The **Content Security Policy (CSP)** adds another lock: the page may only load scripts and styles bundled with the app, never anything from the internet.

### Design tokens → CSS variables

`tokens.ts` is a plain TypeScript object, the single source of truth. `pnpm tokens` turns it into `tokens.css`, a list of CSS custom properties like `--otto-space-4: 16px`. Components use CSS Modules (`App.module.css`, where class names are scoped to the component automatically, a bit like styled-components without the runtime) and only ever write `var(--otto-…)`. Dark mode is just a second set of variable values inside `@media (prefers-color-scheme: dark)`, so components never branch on theme.

The workspace tint works the same way: `<div data-workspace="lauf">` swaps `--otto-accent` for everything inside it. Switching workspace is one attribute change, which is why it can be well under the 100ms budget.

### Why the tint is an attribute, not an effect

The first version set the tint in a `useEffect`. Effects run _after_ the browser paints, so the first frame was neutral and the colour animated in. Rendering `data-workspace` straight into the JSX makes it correct on the first frame. Rule of thumb: if it changes what's on screen, render it; reach for effects only to sync with things outside React.

### Vibrancy

Vibrancy is macOS's frosted-glass material, the blurred desktop showing through a sidebar. Tauri asks the OS for it (`windowEffects: sidebar`) and makes the window transparent. Our CSS then paints only a _translucent_ sidebar colour so the material shows through. In a normal browser there's no material, so it falls back to a solid colour.

### Supply-chain hygiene (why some things are "pinned")

- **Exact versions** (`save-exact`): `react@19.3.1`, never `^19`. The lockfile pins everything below it.
- **pnpm 10 blocks install scripts** unless allowlisted. Only esbuild may run one, because it needs to fetch its native binary.
- **GitHub Actions pinned to commit SHAs:** a tag like `v7` can be moved to point at different code, but a SHA can't.

### Rust you've now seen

- `Result<T, E>` is how Rust returns "either a value or an error". It's like a typed `try/catch` that the compiler forces you to handle. tauri-specta turns it into `{ status: "ok", data } | { status: "error", error }` in TypeScript.
- `#[derive(Serialize, Type)]` is a macro that writes code for you at compile time. Here it generates the JSON conversion (serde) and the TypeScript type (specta).
- `unwrap()` means "crash if this is an error". It's banned outside tests (Clippy enforces it) because a crash in the core would take down every session.

### What to read next

- Tauri 2 "What is Tauri?" and "Process model" pages (tauri.app/start, tauri.app/concept/process-model).
- _The Rust Book_, chapters 1–4 (ownership is the big one).

---

## Chapter 1 — The isolation core (M1)

### What "isolation" actually means here

Every CLI (git, gh, claude, codex) decides who you are by reading **environment variables** and **config files**. Otto controls both. When it starts a process, it first wipes the environment (`env_clear()`), then adds back only an allowlist. `CLAUDE_CONFIG_DIR`, for example, points at that workspace's own folder. A Playbook process literally has no path to anything in Lauf's folder.

### The guarantee vs. the guardrail

- **Guarantee:** each workspace holds _only its own_ GitHub token. Even if an agent bypassed every check, git could only show GitHub the Playbook token from a Playbook process. A test proves this by asking git what it _would_ send.
- **Guardrail:** git **hooks** are shell scripts git runs at certain moments. `pre-commit` checks the author email, and `pre-push` checks where the push is going and who wrote every commit. They catch honest mistakes early, but `--no-verify` can skip them, which is why they aren't the guarantee.

### Testing without the internet

"GitHub" in tests is a **bare repo**: a folder git can push to, with no working files. A git setting called `insteadOf` rewrites `https://github.com/mikelauf/project.git` to that folder, so the hooks see a real GitHub URL while the bytes land on disk. One test accidentally reached the real GitHub, and that's how we found that git talks to the remote _before_ `pre-push` runs.

### Rust concepts you've now met

- **Integration tests** live in `crates/otto-core/tests/`. Each file is compiled as if it were an outside user of the library, so it can only use the public API.
- **`async` vs blocking:** SQLite calls are `async`, so they wait without freezing anything. Identity Doctor runs slow external commands, so Tauri runs it on a separate worker thread (`spawn_blocking`) and the window stays responsive.
- **Migrations** (`migrations/0001_init.sql`) are numbered SQL files that sqlx applies once, in order, on startup. It's how the database schema evolves without losing data.
- **`#[serde(tag = "kind", content = "message")]`** turns a Rust error enum into `{ kind: "refused", message: "…" }` in TypeScript, so the UI can show friendly text and still branch on `kind`.

### What to read next

- _Pro Git_, chapter 8.3 "Git Hooks" (git-scm.com/book)
- _The Rust Book_, chapter 11 "Writing Automated Tests"

---

## Chapter 2 — Terminals (M2)

### What a PTY is

Programs like `claude` and `codex` expect a _terminal_, not a pipe. They draw colour, move the cursor and react to window size. A **PTY** (pseudo-terminal) is the OS giving a program a fake terminal: the program writes to one end, and Otto reads the other end and forwards everything to the window. Keystrokes travel the other way.

### xterm.js

xterm.js is the terminal emulator from VS Code. It turns the raw byte stream (text plus escape codes like `\x1b[2m`) into a grid of characters. The **WebGL addon** draws that grid on the GPU, which keeps typing latency low. The **fit addon** works out how many columns and rows fit in the panel, and Otto tells the PTY so programs re-layout.

### Streaming without breaking characters

Output arrives in chunks of bytes, and a chunk can end halfway through a multi-byte character like `→`. `Utf8Stream` keeps the incomplete tail and prepends it to the next chunk, so the UI only ever sees whole characters.

### Why terminals live outside React

React components unmount and remount, for example when you switch workspace. If the terminal lived inside a component, you'd lose its history. So each `Terminal` lives in a small store (`registry.ts`), and the component just plugs it into the page when visible.

### Channels

A Tauri command returns once. A **Channel** is a pipe the frontend hands to a command, so Rust can keep sending messages (output, then an exit event) for as long as the session lives.
