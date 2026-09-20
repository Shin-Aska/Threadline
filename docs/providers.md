# Provider support and contracts

[`SocialProvider`](../src-tauri/src/providers/mod.rs) is the native provider boundary. It exposes normalized feeds, profiles, threads, tags, followed sources, notifications, social actions, capabilities, and thread publication. `capabilities`, `publish`, and `reply` are required; unsupported optional reads/actions return an explicit provider error. React receives the normalized models from [`providers/social.rs`](../src-tauri/src/providers/social.rs), never provider credentials or raw transport responses.

The authoritative bridge commands are in [`commands/browsing.rs`](../src-tauri/src/commands/browsing.rs). They resolve the public account and connected provider together before invoking the trait. A source-feed request must also name the same provider as its acting account.

## Implemented provider behavior

| Capability | Bluesky | Mastodon |
| --- | --- | --- |
| Credential fallback | Handle, service URL, and app password | HTTPS instance URL and user access token |
| Browser sign-in | Localhost during local development; hosted public-client OAuth in packaged builds | Instance registration, PKCE, and loopback callback |
| Home / profile / thread / tag reads | AT Protocol XRPC endpoints | Mastodon REST endpoints |
| Followed-source index | Followed people, owned lists, and saved feeds | Followed people, tags, and lists |
| Source feed | Person profile, list, or saved feed | Person profile, followed tag, or list |
| Notifications | Provider unread state and `updateSeen` | Provider events with locally persisted read IDs |
| Social actions | Like, repost, follow, reply and their undo forms where applicable | Like, repost, follow, reply and their undo forms where applicable |
| Publishing | AT Protocol post records, reply references, media embeds, and hashtag facets | Statuses, reply IDs, media upload, and a per-request idempotency key |
| Discovery | Provider discovery result | Provider discovery result |

Source collection is a provider index, not a claim that every source’s posts are already loaded. The Following page uses the selected accounts’ home stream for the People collection, then fetches selected tags, lists, and feeds as separate sources. The exact remote data and permission requirements remain provider-dependent.

## Sessions, credentials, and account restoration

Account connection validates the submitted credential before saving public account metadata in SQLite and a serialized secret in the OS keychain. Forms clear their submitted secret after a successful connection. At startup, [`workspace.rs`](../src-tauri/src/workspace.rs) rebuilds connected clients from credentials; missing or unreadable secrets leave the public account record visible but disconnected.

For app-password Bluesky accounts, [`BlueskyProvider`](../src-tauri/src/providers/bluesky/mod.rs) keeps an in-memory bearer session for a conservative 20 minutes. A 401 invalidates that session; GET requests create a replacement session and retry once, while writes are not retried automatically because the remote outcome could be ambiguous. The session itself is not written to SQLite. Browser OAuth uses the OAuth runtime and persists its serialized session through the credential store instead.

Environment accounts are only for local integration work and do not move their environment secrets into the keychain. Their documented variables and limits are in [development](development.md#environment-accounts).

## OAuth configuration

Mastodon browser sign-in accepts only a credential-free HTTPS instance URL, registers Threadline on that instance, and completes an authorization-code flow with PKCE through a loopback callback. The required provider permissions depend on the requested operations.

Bluesky browser sign-in uses a localhost callback in local development. A packaged build needs the public HTTPS metadata URL supplied by the `threadline-bluesky-client-metadata` HTML meta tag. The checked metadata must describe a native public DPoP client, have no client secret, support `authorization_code` and `refresh_token`, return `code`, and include `atproto` plus `transition:generic` scopes. Its first redirect must either be HTTPS on the metadata client’s origin or a custom scheme formed from the reversed metadata hostname and followed by `:/`. The native command rejects a token response that lacks the required scopes.

This checkout does not itself prove that a hosted metadata document and registered callback are available in every packaged environment. Use the app-password fallback when browser sign-in is unavailable; do not treat local or controlled-bridge tests as live OAuth verification.

## Notifications

The provider read is one page per account. Bluesky reads up to 50 notification records and resolves related subject posts in batches of at most 25 identifiers; failure to resolve a related batch does not discard the notification page. Mastodon requests up to 40 notification records and stores local read IDs in SQLite because its API response is treated as unread on retrieval.

The renderer has a separate 20-second cache and a page-lifecycle poller; see [architecture](architecture.md#account-scoped-social-cache) and [architecture](architecture.md#renderer-state-and-retained-pages). Marking read is an account-scoped mutation and invalidates that account’s cached social reads.

## Adding or changing a provider operation

1. Add the provider-specific request and normalization in [`src-tauri/src/providers/`](../src-tauri/src/providers/).
2. Extend `SocialProvider` only when both the bridge model and command need a new operation; return a meaningful unavailable error for networks that do not implement it.
3. Add or update the Tauri command in [`commands/browsing.rs`](../src-tauri/src/commands/browsing.rs) or [`commands/publishing.rs`](../src-tauri/src/commands/publishing.rs).
4. Update the TypeScript desktop adapter and shared TypeScript/Rust model contracts together.
5. Test provider parsing/failure behavior in Rust and the renderer behavior through a controlled bridge scenario.

Provider tests verify the local implementation’s request/response handling. They do not establish that an account, remote instance, or hosted OAuth metadata is live at documentation time.
