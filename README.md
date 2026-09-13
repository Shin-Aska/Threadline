# Threadline

Threadline is a Tauri 2 desktop application for composing once and publishing appropriately to Mastodon and Bluesky. It uses real provider APIs when credentials are configured and retains a clearly identified simulation mode for local UI development without credentials.

## Run

Prerequisites are Node 20+, Rust stable, and the [Tauri 2 system dependencies](https://v2.tauri.app/start/prerequisites/) for your OS.

```bash
npm install
npm run tauri dev
```

Checks:

```bash
npm run lint
npm run check
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
```

## Architecture

### Frontend/backend boundary

React components depend on the typed `DesktopApi` in `src/services/desktop`, not scattered Tauri calls. The API exchanges sanitized account IDs, canonical posts, capability-driven previews, and publication outcomes. Zustand owns only ephemeral composer state and is intentionally not persisted.

Tauri commands are a thin serialization and state boundary. SQLite and business behavior remain in Rust modules so the core can be tested independently and reused by future Tauri mobile entry points.

### Account management

Use the **Accounts** panel in the desktop app to connect or remove accounts. Bluesky connections require a handle, service URL, and app password; Mastodon connections require an instance URL and user access token. Threadline verifies credentials with the provider before adding an account, keeps secrets in the OS credential store, restores providers on restart, and stores only public account metadata in SQLite.

Environment credentials can also bootstrap either or both providers before starting Threadline, which is useful for automated connection testing:

```bash
# Bluesky (an app password is strongly recommended)
export THREADLINE_BSKY_HANDLE="your-handle.bsky.social"
export THREADLINE_BSKY_APP_PASSWORD="xxxx-xxxx-xxxx-xxxx"
# Optional for a self-hosted PDS; defaults to https://bsky.social
export THREADLINE_BSKY_SERVICE="https://your-pds.example"

# Mastodon
export THREADLINE_MASTODON_BASE_URL="https://mastodon.example"
export THREADLINE_MASTODON_ACCESS_TOKEN="your-user-access-token"
export THREADLINE_MASTODON_HANDLE="@you@mastodon.example" # optional label
export THREADLINE_MASTODON_MAX_LENGTH="500"               # optional
```

The common aliases `BSKY_HANDLE`, `BSKY_APP_PASSWORD`, `BSKY_SERVICE`, `MASTODON_BASE_URL`, `MASTODON_URL`, `MASTODON_INSTANCE`, `MASTODON_ACCESS_TOKEN`, `MASTODON_TOKEN`, and `MASTODON_HANDLE` are also accepted. If no managed or complete environment account is present, Threadline loads mock accounts and simulated publishing. Never put credentials in a checked-in `.env` file.

### Credentials and local data

SQLite contains non-sensitive account identity, display metadata, capability JSON, canonical posts, and per-destination publications. It has no credential column. Managed secrets are stored through `CredentialStore` in the OS credential service; environment secrets remain only in native provider objects. Secrets never enter renderer storage or logs and only cross IPC once, from the password input to the native connection command.

### Providers and capabilities

`SocialProvider` exposes capabilities, publish, and reply operations. `MastodonProvider` publishes statuses with bearer authentication and idempotency keys. `BlueskyProvider` creates an ATProto session and `app.bsky.feed.post` records. Multi-part posts use native reply chains on both providers, and failures are isolated per destination.

Every account carries a capability record. The seeded records demonstrate Bluesky at 300 graphemes and two independently configurable Mastodon servers at 500 and 5,000; these are mock records, not universal platform constants. Future Mastodon discovery maps instance configuration into this model, while ATProto lexicon/service constraints populate the Bluesky record. Capabilities include counting policy, URL reservation, media limits/types, polls, and content warnings.

### Canonical content and publications

`CanonicalPost` is provider-neutral intent. A publish result contains one `Publication` per account, each with its own status, error, and list of remote IDs. This supports native reply threads and partial failure without collapsing a cross-post into one boolean result.

### Thread splitting

The Rust splitter counts Unicode extended grapheme clusters. It selects paragraph, then sentence, then whitespace, then grapheme boundaries. Configurable prefix/suffix numbering is included in every part's limit, and planning iterates when the part-count digit width changes. `COMMON_LIMIT` plans against the smallest selected capability; `ADAPTIVE` threads only overflowing destinations; `ALWAYS_THREAD` applies numbered planning to each destination.

## What remains mocked

- Account records and capability discovery when no live credentials are configured.
- Browser-based OAuth (manual access tokens and Bluesky app passwords are currently supported).
- Timelines, search, notifications, profiles, media upload, and moderation screens.

The browser-only Vite fallback can display accounts, but it intentionally rejects preview/publish because those must use the Rust core.

## Next Milestone

Implement browser-based Mastodon OAuth with dynamic app registration, standards-based ATProto OAuth, and read-only home timelines. Add token refresh/revocation, live instance capability discovery, media upload, facets, content warnings, polls, and timeline/moderation surfaces.
