# Development

## Requirements

- Node.js 22.12+ or Node.js 20.19+, and npm.
- Stable Rust installed with [rustup](https://rustup.rs/).
- Native dependencies from the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/): WebKitGTK on Linux, C++ Build Tools and WebView2 on Windows, or Xcode Command Line Tools on macOS.

On Debian/Ubuntu-based Linux:

```bash
sudo apt update
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file \
  libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

The native libraries are also required when an IDE runs Tauri’s Cargo build scripts during project synchronization.

## Run and build

From the repository root:

```bash
npm ci
npm run tauri:dev
```

The first Rust build takes longer. The command keeps Vite and the desktop app running; stop it with `Ctrl+C`.

| Command | Result |
| --- | --- |
| `npm run dev` | Browser preview at `http://localhost:1420` |
| `npm run check` | Type-checks the app and controlled-browser test contracts |
| `npm run lint` | Runs ESLint with warnings treated as failures |
| `npm run build` | Type-checks and builds the frontend into `dist/` |
| `npm run test:ui` | Runs controlled-bridge Playwright scenarios |
| `npm run tauri -- build` | Builds desktop bundles for the host platform |

Desktop artifacts are under `src-tauri/target/release/bundle/` unless Cargo’s target directory is overridden. Build separately on each target OS for native packages.

The browser preview cannot connect accounts, access desktop persistence, schedule work, or publish. UI tests and documentation screenshots use a controlled Tauri bridge to exercise populated states; that is not live-provider verification.

## Connect an account

First launch shows account setup. Later, use **Accounts & Sync**.

- **Bluesky credential fallback:** enter a service URL, handle, and app password. The service defaults to `https://bsky.social`.
- **Mastodon credential fallback:** enter an HTTPS instance URL and a user access token authorized for the operations in use.
- **Browser sign-in:** Mastodon registers a local app and uses PKCE with a loopback callback. Bluesky uses a localhost callback in local development; packaged browser sign-in needs the hosted public metadata configured by the build.

Account setup validates credentials before saving. Public account metadata goes to SQLite; secrets go to the OS keychain under service `social.threadline.app`, keyed by account ID. The OAuth metadata and callback requirements are documented in [providers](providers.md#oauth-configuration).

### Environment accounts

For intentional local integration work, launch Tauri from a shell with these variables exported. The Rust configuration does not automatically load a `.env` file.

| Variable | Required / default |
| --- | --- |
| `THREADLINE_BSKY_HANDLE` | Required together with the app password |
| `THREADLINE_BSKY_APP_PASSWORD` | Required together with the handle |
| `THREADLINE_BSKY_SERVICE` | Optional; `https://bsky.social` |
| `THREADLINE_MASTODON_BASE_URL` | Required together with the access token |
| `THREADLINE_MASTODON_ACCESS_TOKEN` | Required together with the base URL |
| `THREADLINE_MASTODON_HANDLE` | Optional display handle; defaults to the base URL |
| `THREADLINE_MASTODON_MAX_LENGTH` | Optional; `500` |

These values can call real provider APIs and can publish real posts. Do not commit credentials. Environment secrets are not copied to the keychain, so a later launch without them can show the saved account as disconnected.

## Checks

Playwright selects Google Chrome (`channel: "chrome"`). Install it if unavailable:

```bash
npx playwright install chrome
```

See [Playwright’s browser documentation](https://playwright.dev/docs/browsers#google-chrome--microsoft-edge) for browser and system dependency setup.

```bash
npm run lint
npm run check
npm run build
npm run test:ui

cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --all-targets
```

Playwright starts Vite on port 4174. Its controlled scenarios cover workspace loading, durable draft/publishing flows, social cache behavior, retained-page lifecycle, media, multiple accounts, and Composer accessibility. Rust tests cover provider parsing, OAuth validation, persistence, scheduling, and native dispatch. Neither suite proves that a remote provider account, hosted metadata document, or callback registration is live.

## Refresh screenshots

The README images capture the actual React interface with fictional accounts and posts. The fixture supports reads and a short single-part preview only; it rejects publishing and account mutations.

Start the frontend in one terminal:

```bash
npm run dev -- --host 127.0.0.1 --port 4174
```

Then capture in another:

```bash
node scripts/capture-docs.mjs
```

This writes Composer, Timeline, and Accounts & Sync PNGs into [`docs/images/`](images/). Set `THREADLINE_DOCS_URL` to use a different local preview URL. Review images before committing them because font rendering can differ by OS.

## IDE debugging

The checked-in [VS Code launch configurations](../.vscode/launch.json) include:

- **Tauri: Dev** runs `npm run tauri:dev` in VS Code’s `node-terminal` debug terminal.
- **Tauri: Debug Rust Core** starts the native binary through CodeLLDB and starts Vite first.
- **Frontend: Browser Preview** starts Chrome debugging and starts Vite first.

When **Tauri: Dev** prints a Node “Debugger listening” or “Debugger attached” message, it is informational: VS Code’s debug terminal or its Auto Attach setting can attach to Node even though the package script has no inspect flag. For a quiet run, use a plain non-debug terminal. If VS Code Auto Attach is the source, turn it off with **Debug: Toggle Auto Attach** and open a new terminal. Do not remove environment settings in `package.json` or disable a developer’s debugger to suppress that message. See [VS Code’s Node.js debugging documentation](https://code.visualstudio.com/docs/nodejs/nodejs-debugging) for Auto Attach behavior.

A Rust unused-import diagnostic is a compiler warning about that Rust source file, separate from Node debugger output. Fix the unused import in the affected Rust code; do not use debugger settings as a workaround.

Install the workspace’s [recommended extensions](../.vscode/extensions.json) for Rust debugging and Tauri support.
