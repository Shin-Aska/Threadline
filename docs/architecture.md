# How Threadline works

Threadline is a desktop application: React owns interaction state in the Tauri webview, while Rust owns credentials, provider I/O, durable records, and scheduled dispatch. The renderer does not call provider APIs directly.

```text
React view → desktop adapter → Tauri command → provider / database / credential store
                                  │                 │
                               AppState        SQLite + OS keychain
```

## Where to look

| Path | Responsibility |
| --- | --- |
| [`src/app/App.tsx`](../src/app/App.tsx) | Workspace startup, retained page mounting, and route composition |
| [`src/hooks/useWorkspaceNavigation.ts`](../src/hooks/useWorkspaceNavigation.ts) | Browser-history route state, focus, and per-entry scroll restoration |
| [`src/hooks/useSocialFeed.ts`](../src/hooks/useSocialFeed.ts) | Multi-account feed loading, stale-result rejection, pagination, and merge inputs |
| [`src/components/NotificationsView.tsx`](../src/components/NotificationsView.tsx) | Visible-page notification polling, local filtering, paging, and read actions |
| [`src/components/FollowingView.tsx`](../src/components/FollowingView.tsx) | Following-source discovery and bounded concurrent source-feed reads |
| [`src/services/desktop/`](../src/services/desktop/) | The typed renderer boundary for desktop, OAuth, publishing, and social commands |
| [`src/services/desktop/social-cache.ts`](../src/services/desktop/social-cache.ts) | Account-scoped read de-duplication, expiry, and mutation invalidation |
| [`src-tauri/src/lib.rs`](../src-tauri/src/lib.rs) | Native startup, recovery, shared state, scheduler startup, and command registration |
| [`src-tauri/src/commands/browsing.rs`](../src-tauri/src/commands/browsing.rs) | Authoritative social-read and social-action commands |
| [`src-tauri/src/commands/publishing.rs`](../src-tauri/src/commands/publishing.rs) | Authoritative draft, history, schedule, and dispatch commands |
| [`src-tauri/src/providers/`](../src-tauri/src/providers/) | Provider contracts, network requests, and normalized social models |
| [`src-tauri/src/oauth/`](../src-tauri/src/oauth/) | OAuth validation, browser/callback coordination, and persisted sessions |
| [`src-tauri/src/database/`](../src-tauri/src/database/) | Account, notification-read, draft, publication, and schedule persistence |
| [`src-tauri/src/scheduling/mod.rs`](../src-tauri/src/scheduling/mod.rs) | The local 15-second due-schedule loop |
| [`tests/`](../tests/) | Playwright controlled-bridge scenarios; native tests live beside Rust modules |

## Startup and account state

At startup, [`lib.rs`](../src-tauri/src/lib.rs) opens `threadline.sqlite` under Tauri’s application-data directory, recovers interrupted publication work, marks missed schedules for review, restores providers from stored credentials, and starts the scheduler. `AppState` owns the database, OS credential store, OAuth coordinator, and an `RwLock`-protected provider map.

Public account metadata is durable in SQLite. App-passwords, access tokens, and serialized Bluesky OAuth sessions are held by the operating-system credential store; active provider clients are reconstructed in memory. A visible account without a restorable client is disconnected and must be reconnected. A restored client is not evidence that a remote provider will accept it at a later time.

Environment accounts are opt-in development input. They can populate a runtime provider map, but their secrets are not copied into the keychain. See [development](development.md#environment-accounts).

## Native commands are authoritative

The renderer calls `desktopApi`, `socialApi`, `publishingApi`, and OAuth adapters rather than `invoke` throughout views. Their command names are registered in [`lib.rs`](../src-tauri/src/lib.rs). The Rust command layer checks that an account exists and has a connected provider before it delegates; `get_source_feed` also rejects a source from another provider.

This boundary matters because it keeps secret material and provider-specific transport out of the webview, gives browser previews an explicit unavailable path, and makes the Rust models the authority for durable changes. When a bridge payload changes, update its TypeScript contract, Rust command/model serialization, and controlled bridge fixtures together.

The current social commands are `get_home_feed`, `get_own_feed`, `get_own_profile`, `get_profile`, `get_profile_feed`, `get_thread`, `get_tag_feed`, `get_followed_sources`, `get_source_feed`, `get_notifications`, `mark_notifications_read`, and `perform_social_action`. Draft, publication, and schedule commands are listed in [publishing](publishing.md#native-persistence-and-commands).

## Renderer state and retained pages

`App.tsx` keeps every page that has been visited mounted and hides inactive pages. This preserves the composer and loaded page state while navigating. A detail route hides the underlying page without unmounting it. Components that issue background work must therefore gate it on their `active` prop rather than assume an inactive page was unmounted.

`useWorkspaceNavigation` stores a sequence-bearing state object in browser history. Before a push it saves the current scroll position, then it focuses `#main` and scrolls to the top. `popstate` restores the saved scroll position for the history entry. Route query parameters describe the current view and target, but the validated history state is the navigation source of truth.

`useSocialFeed` reads selected accounts in parallel and keeps a revision counter so an earlier result cannot replace a later load. It allows only one initial refresh or pagination request at a time. Per-account errors remain visible while successful accounts continue to contribute posts; presentation code then merges duplicate canonical posts and applies filters.

`FollowingView` uses the ordinary home stream for its People collection and separately reads non-person followed sources. It limits source-feed requests to four concurrent calls, which avoids turning a large following list into an unbounded burst of provider requests. Source and feed requests each use a revision counter to discard stale results.

`NotificationsView` fetches all workspace accounts together only while its retained page is both active and document-visible. A disconnected saved account contributes a per-account error while other accounts can still refresh. The view refreshes on activation and visibility return, avoids overlapping requests, and schedules the next poll after completion. The normal interval is 30 seconds; consecutive failures exponentially back off to a five-minute cap and a successful cycle resets the delay. A queued refresh is run once after an in-flight request only when the page remains active and visible.

## Account-scoped social cache

[`social.ts`](../src/services/desktop/social.ts) owns a 96-entry `SocialReadCache`. Its key includes the account ID, command, and serialized arguments, so requests for different accounts never share a result. A pending request is cached with an infinite temporary expiry to coalesce concurrent callers; a successful result receives its TTL and a failed result is removed.

| Read class | TTL |
| --- | --- |
| Content feeds, profile feeds, threads, tags, and source feeds | 30 seconds |
| Notifications | 20 seconds |
| Profiles and followed-source indexes | 2 minutes |

Each cache hit is promoted to most recently used. Entries beyond 96 evict from the oldest end. `refresh: true` discards the matching key before loading. Any social mutation invalidates every cache entry for the acting account both before and after the native call, including when it fails; callers can therefore never rely on an account read surviving a mutation attempt.

## Boundaries and limits

The browser preview deliberately cannot access credentials, native preview planning, publication persistence, scheduling, or account connection. Playwright and documentation capture use an explicit controlled Tauri bridge to populate the interface; they are not live-provider verification.

The provider contract and network differences are in [providers](providers.md). Durable draft, publication, and scheduling semantics are in [publishing](publishing.md). Local commands and validation are in [development](development.md).
