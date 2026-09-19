# How Threadline works

Threadline has two halves: a React interface inside a Tauri webview, and a Rust backend that owns provider requests, credentials, storage, and thread planning.

```text
React views → desktopApi → Tauri commands → composer / provider clients
                                             │              │
                                           SQLite       OS keychain
```

## Where to look

| Path | Responsibility |
| --- | --- |
| [`src/app/App.tsx`](../src/app/App.tsx) | Startup, navigation, workspace refresh, and view composition |
| [`src/components/`](../src/components/) | Composer, account setup, browsing views, and shared controls |
| [`src/hooks/`](../src/hooks/) | Workspace loading, preview debounce, hashtags, and browsing state |
| [`src/stores/composer.ts`](../src/stores/composer.ts) | In-memory draft, attachments, destinations, and policy |
| [`src/services/desktop/`](../src/services/desktop/) | Typed adapter around Tauri `invoke` |
| [`src/services/unified/`](../src/services/unified/) | Parallel account reads, normalization aggregation, and source failures |
| [`src/types/index.ts`](../src/types/index.ts) | Frontend data contracts |
| [`src-tauri/src/lib.rs`](../src-tauri/src/lib.rs) | Native startup, shared state, and command registration |
| [`src-tauri/src/commands/`](../src-tauri/src/commands/) | Account operations, previews, publishing, and browsing commands |
| [`src-tauri/src/composer/`](../src-tauri/src/composer/) | Text counting and thread planning |
| [`src-tauri/src/providers/`](../src-tauri/src/providers/) | Mastodon and Bluesky API clients |
| [`src-tauri/src/models/`](../src-tauri/src/models/) | Rust models serialized across the bridge |
| [`src-tauri/src/database/`](../src-tauri/src/database/) | SQLite schema and account records |
| [`src-tauri/src/credentials/`](../src-tauri/src/credentials/) | OS keychain adapter |
| [`tests/`](../tests/) | Playwright scenarios with a controlled desktop bridge |

The Rust `accounts` module contains test fixtures; production account operations live in `commands`.

## Startup and accounts

1. Tauri opens `threadline.sqlite` in its application-data directory.
2. [`workspace.rs`](../src-tauri/src/workspace.rs) loads saved public account records and restores provider clients from keychain credentials. Environment-configured clients are merged in.
3. `AppState` holds the database, credential store, and a provider map keyed by account ID. The provider map is behind an `RwLock`; SQLite uses a mutex.
4. `useWorkspace` loads a snapshot through `desktopApi`. The snapshot separates saved accounts from `connectedAccountIds`.
5. With no accounts, React shows account setup. Otherwise, it opens Timeline.

A saved account can require reconnection. The navigation badge counts accounts with an available provider client, not selected publishing destinations. It is not a continuous network health check.

## State ownership

| State | Home | Survives restart? |
| --- | --- | --- |
| Draft text, images, policy, destinations | Zustand composer store | No |
| Per-view browsing account filters | Browser localStorage | Yes |
| Account metadata and capabilities | SQLite | Yes |
| App passwords and access tokens from account setup | OS keychain | Yes |
| Active provider clients | Rust memory | Recreated at startup |
| Preview and publication results | React state | No |

Composer stays mounted while hidden, so switching views does not discard the draft. Browsing filters are separate from publishing destinations.

SQLite also defines `canonical_posts` and `publications` tables, but the current publish command does **not** write to them. Their presence does not provide publication history or recovery.

## The desktop boundary

UI code calls [`desktopApi`](../src/services/desktop/index.ts), rather than invoking native commands directly. This keeps browser fallback behavior and command names in one place.

The TypeScript and Rust contracts are maintained manually. When changing a payload, update both the frontend types and Rust serialization, then adjust the controlled bridge fixtures in tests.

In a regular browser, the adapter returns an empty `BROWSER` workspace. It cannot access native credentials, plan a native preview, or publish. Screenshot and UI-test fixtures replace this bridge explicitly; the application does not ship demo accounts.

## Reading timelines

`useUnifiedTimeline` requests pages through the unified service. That service reads selected accounts in parallel, keeps individual source failures, merges posts by `canonicalKey`, and sorts them by creation time. Source attribution records which account supplied a post.

Pagination retains a cursor for each account. Deduplication depends on provider keys: a Bluesky URI identifies a post, while Mastodon keys include the instance and status ID. Do not assume identical federated copies always collapse across instances.

The desktop adapter currently accepts browsing abort signals without cancelling native requests. Stale-result handling and network cancellation are separate concerns.

## Typical changes

- **Add a view or control:** start in `components`, then use existing hooks and `desktopApi` for data.
- **Change a native operation:** update `commands`, its bridge method, both model definitions, and relevant fixtures.
- **Change splitting rules:** start in `composer`; preview and publishing use the same planner.
- **Add network behavior:** implement it in the provider layer, then normalize it before exposing it to React.

See [publishing](publishing.md) for the write path, [providers](providers.md) for network differences, and [development](development.md) for commands.
