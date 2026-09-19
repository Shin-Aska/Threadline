# Provider support

[`SocialProvider`](../src-tauri/src/providers/mod.rs) is the native interface for capabilities, publishing, replies, hashtags, timelines, discovery, and following sources. Implementations own their network-specific requests and normalize responses before they reach the frontend.

## Implemented behavior

| Feature | Bluesky | Mastodon |
| --- | --- | --- |
| Authentication | Handle and app password | Instance URL and user access token |
| Home timeline | `app.bsky.feed.getTimeline` | `/api/v1/timelines/home` |
| Thread publishing | Post records with reply references | Statuses with reply IDs |
| Images | Uploaded blobs and image embeds | Uploaded media attached to statuses |
| Hashtag suggestions | Exact-tag lookup and approximate search matches | Tag search and trends |
| Discover | Empty result currently | Trending tags |
| Following sources | Pinned saved feeds from preferences | Followed tags |
| Reactions | Read-only counts | Read-only counts |

Following is not a full list of followed people. Selecting a collection currently filters loaded home posts by its source account IDs; it does not request that individual feed or filter posts by actual tag membership.

Discovery’s text filter operates on loaded results. General remote search, notifications, follow/unfollow, and in-app reply/like/repost actions are not implemented. Posts link to their original network.

## Account lifecycle

Account setup verifies credentials with the provider before saving public metadata to SQLite and the secret to the OS keychain. Forms briefly hold submitted secrets in renderer memory and clear them after a successful connection.

At startup, saved credentials restore provider clients. Missing or unreadable credentials leave the public account visible with reconnection required. A restored client does not prove that credentials are still valid remotely.

Environment-configured accounts are also supported; see [development](development.md#environment-accounts). Their public metadata is saved, but that path does not copy environment secrets into the keychain.

Default text capabilities are 300 graphemes for Bluesky and 500 for Mastodon. Mastodon instance capabilities are not automatically discovered. The environment configuration can override its text limit.

## Adding provider behavior

1. Implement the operation in [`providers/`](../src-tauri/src/providers/) and normalize its result. Keep credentials and HTTP details on the Rust side.
2. Add or update the native command if the operation is not already exposed.
3. Update [`desktopApi`](../src/services/desktop/index.ts) and the shared TypeScript/Rust contracts.
4. Preserve source account IDs, cursors, and per-account errors for browsing operations.
5. Cover provider parsing and failure cases in Rust, and add a controlled bridge scenario for the UI behavior.

For a new network, also extend provider enums, account setup, credential restoration, icons/labels, and capabilities. Unsupported operations should return an explicit unavailable result or error, rather than imply support through placeholder data.
