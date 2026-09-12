# Threadline

Threadline is a Tauri 2 desktop foundation for composing once and publishing appropriately to multiple Mastodon and Bluesky accounts. This milestone is deliberately a **simulation**: preview and thread planning are real Rust core behavior, but no remote post is created.

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

### Credentials and local data

SQLite contains non-sensitive account identity, display metadata, capability JSON, canonical posts, and per-destination publications. It has no credential column. `CredentialStore` is a Rust-only interface with an `OsKeychainCredentialStore` implementation built on the native credential service. Secrets never cross IPC, enter renderer storage, or appear in logs. Environment credentials are intentionally not read by this milestone.

### Providers and capabilities

`SocialProvider` exposes capabilities, publish, and reply operations. `MastodonProvider` and `BlueskyProvider` are distinct implementations and own `reqwest` clients ready for future DTO mapping. Their network methods currently return explicit “not implemented” errors rather than pretending to publish.

Every account carries a capability record. The seeded records demonstrate Bluesky at 300 graphemes and two independently configurable Mastodon servers at 500 and 5,000; these are mock records, not universal platform constants. Future Mastodon discovery maps instance configuration into this model, while ATProto lexicon/service constraints populate the Bluesky record. Capabilities include counting policy, URL reservation, media limits/types, polls, and content warnings.

### Canonical content and publications

`CanonicalPost` is provider-neutral intent. A publish result contains one `Publication` per account, each with its own status, error, and list of remote IDs. This supports native reply threads and partial failure without collapsing a cross-post into one boolean result.

### Thread splitting

The Rust splitter counts Unicode extended grapheme clusters. It selects paragraph, then sentence, then whitespace, then grapheme boundaries. Configurable prefix/suffix numbering is included in every part's limit, and planning iterates when the part-count digit width changes. `COMMON_LIMIT` plans against the smallest selected capability; `ADAPTIVE` threads only overflowing destinations; `ALWAYS_THREAD` applies numbered planning to each destination.

## What is mocked

- The three local account records and their capability discovery results.
- OAuth/session acquisition and account management.
- Publishing. The button returns clearly labelled, sanitized simulated remote IDs.
- Timelines, search, notifications, profiles, media upload, and moderation screens.

The browser-only Vite fallback can display accounts, but it intentionally rejects preview/publish because those must use the Rust core.

## Next Milestone

Implement end-to-end account connection and read-only home timelines: Mastodon OAuth with dynamic app registration and instance capability discovery, plus standards-based ATProto OAuth/session handling. Store credentials exclusively through `CredentialStore`, map external DTOs into domain types, add token refresh/revocation, and exercise provider conformance with mocked HTTP servers before enabling carefully staged real publishing.
