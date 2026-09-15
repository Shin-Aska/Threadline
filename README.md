# Threadline

Threadline is a Tauri 2 desktop application for composing once and publishing appropriately to Mastodon and Bluesky. It starts with account setup and uses real provider APIs for connected accounts. No demo accounts or dry-run publishing are included.

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
npm run test:ui
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
```

## Architecture

### Frontend/backend boundary

React components depend on the typed `DesktopApi` in `src/services/desktop`, not scattered Tauri calls. The API exchanges sanitized account IDs, canonical posts, capability-driven previews, and publication outcomes. Zustand owns only ephemeral composer state and is intentionally not persisted. New sessions default to **Common limit**; you can switch publishing policies in the composer.

Tauri commands are a thin serialization and state boundary. SQLite and business behavior remain in Rust modules so the core can be tested independently and reused by future Tauri mobile entry points.

### Account management

On first launch, **Connect your first account** appears automatically. Choose Bluesky or Mastodon and enter your connection details. Once connected, use **Accounts & Sync** to add, reconnect, or remove accounts. Bluesky connections require a handle, service URL, and app password; Mastodon connections require an instance URL and user access token. New Mastodon connections prefill `https://mastodon.social`; edit it to use another instance. Threadline verifies credentials with the provider before adding an account, keeps secrets in the OS credential store, restores providers on restart, and stores only public account metadata in SQLite.

New Mastodon account IDs include the instance so identical remote IDs on different servers remain separate. Reconnecting an existing matching legacy account preserves its ID.

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

The common aliases `BSKY_HANDLE`, `BSKY_APP_PASSWORD`, `BSKY_SERVICE`, `MASTODON_BASE_URL`, `MASTODON_URL`, `MASTODON_INSTANCE`, `MASTODON_ACCESS_TOKEN`, `MASTODON_TOKEN`, and `MASTODON_HANDLE` are also accepted. A fresh workspace without accounts opens **Connect your first account**. Previously seeded Alice, River, and Sora sample identities are removed automatically on startup. Stored real accounts whose credentials are unavailable stay disconnected and offer Reconnect. Never put credentials in a checked-in `.env` file.

### Credentials and local data

SQLite contains non-sensitive account identity, display metadata, capability JSON, canonical posts, and per-destination publications. It has no credential column. Managed secrets are stored through `CredentialStore` in the OS credential service; environment secrets remain only in native provider objects. Secrets never enter renderer storage or logs and only cross IPC once, from the password input to the native connection command.

### Providers and capabilities

`SocialProvider` exposes capabilities, publish, and reply operations. `MastodonProvider` publishes statuses with bearer authentication and idempotency keys. `BlueskyProvider` creates an ATProto session and `app.bsky.feed.post` records. Multi-part posts use native reply chains on both providers, and failures are isolated per destination.

Every account carries a capability record. Test fixtures exercise limits of 300, 500, and 5,000 graphemes; these fixtures are not loaded into the application. Future Mastodon discovery maps instance configuration into this model, while ATProto lexicon/service constraints populate the Bluesky record. Capabilities include counting policy, URL reservation, media limits/types, polls, and content warnings.

### Canonical content and publications

`CanonicalPost` is provider-neutral intent. A publish result contains one `Publication` per account, each with its own status, error, and list of remote IDs. This supports native reply threads and partial failure without collapsing a cross-post into one boolean result.

### Thread splitting

The Rust splitter counts Unicode extended grapheme clusters. It selects paragraph, then sentence, then whitespace, then grapheme boundaries. Configurable prefix/suffix numbering is included in every part's limit, and planning iterates when the part-count digit width changes. `COMMON_LIMIT` plans against the smallest selected capability; `ADAPTIVE` threads only overflowing destinations; `ALWAYS_THREAD` applies numbered planning to each destination.

## Not yet implemented

- Live instance capability discovery.
- Browser-based OAuth (manual access tokens and Bluesky app passwords are currently supported).
- Timelines, search, notifications, profiles, video/audio attachments, and moderation screens.

The browser-only Vite preview opens the account setup screen. Account connection and publishing require the desktop app.

## Next Milestone

Implement browser-based Mastodon OAuth with dynamic app registration, standards-based ATProto OAuth, and read-only home timelines. Add token refresh/revocation, live instance capability discovery, video/audio uploads, link/mention facets, content warnings, polls, and timeline/moderation surfaces.

## Interface

The composer and **Accounts & Sync** screens follow the Threadline Stitch design: compact navy panels, Geist typography, JetBrains Mono metadata, and provider-colored destinations. The design contract and source screen IDs are recorded in [DESIGN.md](DESIGN.md). Native icons in `src-tauri/icons` are generated from the same `src/assets/threadline-logo.svg`; Cargo tracks icon changes so rebuilt Windows executables embed the current logo.

Select destination accounts in the sidebar (the Accounts selector on phones), choose a publishing policy, and inspect previews grouped by protocol. Identical thread plans are shown once with their selected identities; differing plans stay visible as variants within the protocol group. The editor shows the shortest selected limit beside its grapheme count, and the dispatch bar shows the actual selected account count. The draft and selected policy stay intact when switching to account management. The layout adapts to narrow windows and mobile-width browser previews.

`npm run dev` serves the browser preview at `http://localhost:1420`. It shows the empty account setup screen, including both provider forms; connecting requires the desktop app. Use `npm run tauri dev` for account verification, native thread planning, and publishing. Timeline, OAuth, video/audio, and scheduling controls from the broader Stitch concept remain future work.

## Functional workflow

- Connecting or removing an account refreshes the native workspace, sidebar, destination list, and selection together. The first connection opens the composer with that destination selected and an empty draft. Removing the last account returns to setup.
- Disconnected accounts offer **Reconnect**, preserving their known service URL. Older accounts without a saved URL require the original endpoint to be entered.
- Native previews replan when the draft, policy, targets, or workspace changes. Publishing waits for a current plan; a failed plan offers **Retry preview**.
- Each destination reports its own success or failure; missing connections cannot report a dry-run success. Successful publication shows feedback and automatically clears the draft. Publishing blocks repeat clicks while in progress. Failed drafts stay editable; destinations that received any posts are deselected to avoid accidental duplicates when retrying.

The UI regression suite uses Chrome with a controlled desktop API boundary. Native tests cover empty startup, legacy sample cleanup, workspace restoration, thread planning, and custom-service connect/reconnect against a local server. The first-account flow was also exercised in the actual Tauri/WebView2 app against a local authentication server, including rejected credentials, successful connection, and final-account removal. Live-provider publishing requires your credentials and was not exercised during this change.

## Images and alt text

Use **Add images** in the composer to attach up to four JPEG, PNG, or WebP images, at most 2 MB each. Each image has a removable preview and an optional **Alt text** field (up to 1,500 characters). Describe the meaningful content for people using screen readers. Images appear in destination previews and attach to the first post of each thread; image-only posts are supported. Selected accounts must support the image type.

Images and descriptions stay in memory with the draft until you publish. Bluesky uploads blobs and embeds each image with its alt text; Mastodon uploads media with its description and waits for processing before creating the status. Mastodon tokens need `write:media` as well as `write:statuses` (or the encompassing `write` scope). Instance-specific restrictions may still reject an upload. Successful publication clears text and images. Failure preserves both; destinations that received posts are deselected to avoid accidental duplicates. Video and audio attachments are not supported yet.

## Hashtag suggestions and activity

Type # in the composer to open a floating autocomplete at the hashtag, without moving the page layout. All, Bluesky and Mastodon filters aggregate suggestions from selected accounts, with stacked initials showing contributing identities. Up/Down highlights a suggestion and Enter replaces only the active hashtag; Escape closes. Tab reaches the source filters, and selecting one returns focus to the editor. View more hashtag suggestions expands the same scrollable popover (up to 20 results fetched per Mastodon account). Lookup errors retry per account without blocking publishing.

Mastodon shows usage summed over the days returned by that instance; it is not a global total or a count of unique people. Bluesky search matches are labeled approximate, and missing statistics stay unavailable instead of showing zero. Identical tags are merged across accounts; shared Bluesky search-index counts and same-instance Mastodon observations are counted once. The two networks’ measures are labeled separately, and observations from different Mastodon instances can overlap. Hover a row for its account/source breakdown. Bluesky does not provide hashtag-prefix autocomplete here; its row checks the exact tag you typed.

Requests go through the native account provider after a 600 ms typing pause and send only the active tag, never the rest of your draft. Results cache in memory for up to one minute; Bluesky reuses a native search session for up to twenty minutes. Mastodon search can require a token with read:search. No hashtag data or search sessions are persisted.

Bluesky publications include native hashtag facets with UTF-8 byte offsets calculated separately for each final thread part, including numbering. Unicode hashtags and leading emoji are supported. Link and mention facets remain future work.
