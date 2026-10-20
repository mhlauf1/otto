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
