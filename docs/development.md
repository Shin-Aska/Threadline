# Development

## Requirements

- Node.js 22.12+ or Node.js 20.19+, and npm (Vite’s supported versions).
- Stable Rust, installed with [rustup](https://rustup.rs/).
- Native dependencies from the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/): WebKitGTK on Linux, C++ Build Tools and WebView2 on Windows, or Xcode Command Line Tools on macOS.

On Debian/Ubuntu-based Linux:

```bash
sudo apt update
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file \
  libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

The native libraries are also needed when an IDE runs Tauri’s Cargo build scripts during project synchronization.

## Run and build

From the repository root:

```bash
npm ci
npm run tauri:dev
```

The first Rust build takes longer. The command keeps Vite and the app running; stop it with `Ctrl+C`.

| Command | Result |
| --- | --- |
| `npm run dev` | Browser preview at `http://localhost:1420` |
| `npm run build` | Type-check and build the frontend into `dist/` |
| `npm run tauri -- build` | Build desktop bundles for the host platform |

Desktop artifacts are under `src-tauri/target/release/bundle/` unless you override Cargo’s target directory. Build separately on each target OS for its native packages.

The browser preview cannot connect accounts or publish. The UI tests and documentation screenshots use a controlled Tauri bridge to exercise populated states.

## Connect an account

First launch shows account setup. Later, use **Accounts & Sync**.

- **Bluesky:** enter the service URL, handle, and an app password. The service defaults to `https://bsky.social`.
- **Mastodon:** enter the instance URL and a user access token authorized for the operations you use. Publishing needs status and media write permissions; browsing and hashtag search need the corresponding read permissions.

Account setup validates credentials before saving. Public account metadata goes to SQLite; credentials go to the OS keychain under service `social.threadline.app`, keyed by account ID.

### Environment accounts

For local integration work, launch Tauri from a shell with these variables exported. The Rust configuration does not automatically load a `.env` file.

| Variable | Required / default |
| --- | --- |
| `THREADLINE_BSKY_HANDLE` | Required together with the app password |
| `THREADLINE_BSKY_APP_PASSWORD` | Required together with the handle |
| `THREADLINE_BSKY_SERVICE` | Optional; `https://bsky.social` |
| `THREADLINE_MASTODON_BASE_URL` | Required together with the access token |
| `THREADLINE_MASTODON_ACCESS_TOKEN` | Required together with the base URL |
| `THREADLINE_MASTODON_HANDLE` | Optional display handle; defaults to the base URL |
| `THREADLINE_MASTODON_MAX_LENGTH` | Optional; `500` |

These use real provider APIs and publish real posts. Do not commit credentials. Environment secrets are not copied to the keychain; a later launch without them may show the saved account as disconnected.

## Checks

The Playwright configuration selects Google Chrome (`channel: "chrome"`). Install it if unavailable:

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

Playwright starts Vite on port 4174. Its tests cover workspace loading, aggregation, media, hashtags, multiple accounts, and Composer layout/accessibility. They use controlled responses, not live account credentials. Rust tests cover native behavior; neither suite replaces a live-provider integration check.

## Refresh the screenshots

The README images are captures of the actual React interface with fictional accounts and posts. The fixture supports reads and a short single-part preview only; it rejects publishing and account mutations. It does not validate native publishing behavior.

Start the frontend in one terminal:

```bash
npm run dev -- --host 127.0.0.1 --port 4174
```

Then capture in another:

```bash
node scripts/capture-docs.mjs
```

This writes Composer, Timeline, and Accounts & Sync PNGs into [`docs/images/`](images/). Set `THREADLINE_DOCS_URL` to use a different local preview URL. Review the images before committing them; font rendering can differ by OS.

## IDE debugging

The checked-in [VS Code launch configurations](../.vscode/launch.json) include:

- **Tauri: Dev** — complete application through the Tauri CLI.
- **Tauri: Debug Rust Core** — native breakpoints through CodeLLDB; starts Vite automatically.
- **Frontend: Browser Preview** — Chrome debugging; starts Vite automatically.

Install the workspace’s [recommended extensions](../.vscode/extensions.json) for Rust debugging and Tauri support.
