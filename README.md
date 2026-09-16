# Threadline

Threadline is a desktop publishing client for writing once and posting to multiple Mastodon and Bluesky accounts. It plans each destination against that account's limits, previews any required thread split, and reports success or failure per account instead of treating a cross-post as one all-or-nothing operation.

Threadline uses the real provider APIs. There are no demo accounts and no dry-run publication mode.

## What it does

- Connects multiple Mastodon and Bluesky accounts and restores them between sessions.
- Publishes one draft to any selected accounts using native reply threads when needed.
- Supports three planning policies: the smallest common limit, adaptive per-network splitting, or always creating a thread.
- Counts Unicode grapheme clusters and splits at paragraphs, sentences, whitespace, then grapheme boundaries.
- Attaches up to four JPEG, PNG, or WebP images, including alt text, to the first post in a thread.
- Suggests hashtags from connected accounts and adds native Bluesky hashtag facets.
- Preserves the draft after failures and prevents successful destinations from being published twice during a retry.

The browser preview is useful for UI development, but account connection, native previews, and publishing require the Tauri desktop application.

## Technology

- [Tauri 2](https://v2.tauri.app/) desktop shell and Rust backend
- React 19, TypeScript, Vite 7, and Zustand frontend
- SQLite for non-sensitive local records
- The operating system credential store for account secrets
- Reqwest-based Mastodon and AT Protocol integrations

## Prerequisites

Install these on every development machine:

- Node.js `20.19+` or `22.12+`
- npm
- The stable Rust toolchain installed with [rustup](https://rustup.rs/)
- Git

Tauri also needs native build dependencies for the host operating system. See the [official Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) if your platform is not covered below.

### Linux: Debian, Ubuntu, or Linux Mint

```bash
sudo apt update
sudo apt install libwebkit2gtk-4.1-dev \
  build-essential \
  curl \
  wget \
  file \
  libxdo-dev \
  libssl-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev
```

`libwebkit2gtk-4.1-dev` is required even for Cargo project synchronization because RustRover runs Tauri build scripts during sync.

### Windows

Install:

1. Microsoft C++ Build Tools with **Desktop development with C++** selected.
2. Microsoft Edge WebView2 Runtime. It is normally already present on current Windows versions.
3. Rust's stable MSVC toolchain.

```powershell
rustup default stable-msvc
```

### macOS

Install Xcode Command Line Tools:

```bash
xcode-select --install
```

## Install and run

Install the JavaScript dependencies from the lockfile:

```bash
npm ci
```

Start the desktop application:

```bash
npm run tauri:dev
```

The first Rust build can take a while and the command intentionally remains running while Vite and the desktop application are open. Stop it with `Ctrl+C`.

For frontend-only work, start the browser preview at `http://localhost:1420`:

```bash
npm run dev
```

The browser preview cannot connect accounts or publish.

## Connect accounts

The first launch opens **Connect your first account**.

- Bluesky requires a handle, service URL, and app password. The default service is `https://bsky.social`.
- Mastodon requires an instance URL and user access token. Image publishing needs `write:media` and `write:statuses`, or the encompassing `write` scope. Hashtag lookup may also require `read:search`.

Threadline verifies credentials before saving the account. Secrets are stored in the OS credential store; SQLite stores public account metadata, capabilities, canonical posts, and per-destination publication results. Secrets are not stored in SQLite or browser storage.

For automated or local integration testing, accounts can be supplied through environment variables:

```bash
# Bluesky
export THREADLINE_BSKY_HANDLE="your-handle.bsky.social"
export THREADLINE_BSKY_APP_PASSWORD="xxxx-xxxx-xxxx-xxxx"
export THREADLINE_BSKY_SERVICE="https://bsky.social" # optional

# Mastodon
export THREADLINE_MASTODON_BASE_URL="https://mastodon.example"
export THREADLINE_MASTODON_ACCESS_TOKEN="your-user-access-token"
export THREADLINE_MASTODON_HANDLE="@you@mastodon.example" # optional label
export THREADLINE_MASTODON_MAX_LENGTH="500"               # optional
```

Do not commit credentials or a populated `.env` file.

## Build

Build the complete desktop application and the installers supported by the current operating system:

```bash
npm run tauri -- build
```

Release artifacts are written below `src-tauri/target/release/bundle/`. Tauri builds for the current host platform; producing Windows, macOS, and Linux packages normally requires a build on each platform.

To build only the web frontend:

```bash
npm run build
```

## Checks

Install Playwright's Chromium browser once before running the UI suite:

```bash
npx playwright install chromium
```

Run the frontend checks:

```bash
npm run lint
npm run check
npm run build
npm run test:ui
```

Run the Rust checks:

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --all-targets
```

## Project structure

- `src/` contains the React interface, ephemeral composer state, and the typed desktop API boundary.
- `src-tauri/src/` contains account management, credential access, SQLite storage, thread planning, media validation, provider clients, and Tauri commands.
- `tests/` contains browser-level UI and accessibility scenarios with a controlled desktop API.
- `DESIGN.md` defines the visual system, responsive behavior, and supported interface states.

The renderer talks to Rust through `src/services/desktop/`. Provider secrets stay on the native side. Publishing results remain separate per account so partial failures are visible and safe to retry.

## Current limitations

- Account setup uses Bluesky app passwords and manually created Mastodon access tokens; browser-based OAuth is not implemented.
- Mastodon instance capabilities are not discovered dynamically yet.
- Timelines, profiles, general search, notifications, moderation, polls, content warnings, scheduling, and video/audio attachments are not implemented.
- Link and mention facets are not generated yet.

## License

Threadline is licensed under the [GNU General Public License v3.0](LICENSE).
