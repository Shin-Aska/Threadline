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
| `npm run test:ui` | Builds the frontend and runs controlled-bridge Playwright scenarios |
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

cargo fmt --manifest-path src-tauri/Cargo.toml --all --check
cargo clippy --locked --manifest-path src-tauri/Cargo.toml --workspace --all-targets --no-default-features -- -D warnings
cargo test --locked --manifest-path src-tauri/Cargo.toml --workspace --all-targets --no-default-features
```

### Security checks

Run `npm run audit:security` to check every locked npm dependency, including development tooling. The Vite minimum includes the 7.3.7 security fixes. A targeted override moves `@typescript-eslint/typescript-estree` to minimatch 11.0.1 or later because its upstream dependency range still selects minimatch 10, affected by [GHSA-g3hv-75wg-m249](https://github.com/isaacs/minimatch/security/advisories/GHSA-g3hv-75wg-m249). Keep this override until the upstream range includes a patched version, and run lint and the UI suite when updating it.

For native dependencies, install the audit tool with `cargo install cargo-audit --locked --version 0.22.2`, then run `cargo audit --file src-tauri/Cargo.lock`. Security CI runs both audits for pull requests, main-branch pushes, release checks, and a weekly schedule. Dependabot checks npm, Cargo, and pinned GitHub Actions weekly. Audit failures require investigation; the workflow does not suppress advisories or automatically merge dependency updates.

The November 6, 2026 maintenance pass updated quinn-proto to 0.11.19, rustls to 0.23.46, rustls-webpki to 0.103.16, webpki-roots to 1.0.11, and Tokio to 1.54.1. These resolve ten RustSec vulnerability advisories and the Tokio I/O-safety warning present in the previous lockfile.

The crypto-bigint advisory [RUSTSEC-2026-0387](https://rustsec.org/advisories/RUSTSEC-2026-0387) is resolved by a local Atrium OAuth 0.1.7 patch using p256 0.14 / elliptic-curve 0.14 / crypto-bigint 0.7.10. The published Atrium OAuth 0.2.0 still selects the vulnerable stack, so upgrading that release alone is insufficient. The patch retains the saved JWK/session format, validates imported keys, and tests ES256/DPoP signatures with an independent backend. Its provenance, scope and removal criteria are in [the patch notes](../src-tauri/vendor/atrium-oauth/THREADLINE-PATCH.md). No advisory exception is configured.

Rust checks include the patched dependency's unit tests with `--workspace`. `--no-default-features` selects the app's existing Rustls transport configuration instead of enabling the dependency's optional native-TLS default client.

RustSec also reports upstream informational warnings for glib 0.18.5 (`VariantStrIter` unsoundness), lru 0.16.4 (panic safety; brought in only for Atrium's wasm32 target), proc-macro-error, and five `unic-*` crates (unmaintained). These remain visible in audit output. The repository targets desktop Linux, Windows, and macOS; no warning has been globally ignored or treated as proof of a reachable exploit.

The packaged webview uses an explicit Content Security Policy. Scripts and fonts load from bundled assets; provider images and media can use HTTPS, with data/blob sources retained for authored previews and player workers. Streaming reads allow HTTPS alongside Tauri IPC. Inline styles remain necessary for measured popover placement and image transforms. Frames, objects, base URL changes, and form submissions are blocked. The separate development policy additionally permits Vite's inline bootstrap and loopback HTTP/WebSocket connections on port 1420; these script and loopback allowances are absent from production.

`npm run test:ui` builds the frontend first: the security-policy scenarios serve `dist/` under the production policy with a controlled, nonced desktop bridge. They exercise blocked untrusted scripts and allowed provider images, streaming reads, and workers. Tauri adds its own trusted bootstrap nonces and hashes during native builds. When invoking Playwright directly with `npx playwright test`, run `npm run build` first.

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
