# Publishing, history, and schedules

Publishing is native and durable. The renderer edits a durable draft through [`publishingApi`](../src/services/desktop/publishing.ts); [`commands/publishing.rs`](../src-tauri/src/commands/publishing.rs) is the authoritative command layer for drafts, publication history, and schedules.

## Native persistence and commands

`save_draft`, `list_drafts`, `get_draft`, and `delete_draft` persist drafts in SQLite. A save advances the draft revision. `create_schedule` reads the selected draft and stores its post payload and draft revision in the schedule row, so later edits to the draft cannot silently alter the scheduled publication. The schedule also persists its chosen IANA time zone and an optimistic-concurrency revision.

`publish_draft` creates or reuses a ledger keyed by the current durable draft revision, so repeated manual dispatch requests for that revision share one idempotency scope. `publish_scheduled` is internal scheduler work that uses a separate schedule-scoped ledger and its captured immutable post snapshot. `list_publications` and `delete_publication` manage the history; `list_schedules`, `reschedule_publication`, `cancel_schedule`, and `send_schedule_now` manage scheduled work. Reschedule and cancel require the current schedule revision and apply only to queued or needs-attention records.

## Native dispatch and recovery

[`publishing/mod.rs`](../src-tauri/src/publishing/mod.rs) writes the publication ledger before provider I/O. It records each destination and each segment as pending, in flight, published, failed, blocked, or uncertain. A destination is claimed before sending; each thread segment is claimed before its provider call. This gives restart recovery a durable answer about what was being attempted even when a provider response is lost.

For each eligible destination, native code re-plans the captured post against current account capabilities, prepares media, clones connected providers out of shared state, and sends segments in order. The first segment carries media; later segments reply to the preceding remote post. A missing/disconnected account is recorded as blocked. A provider failure after a segment starts is recorded as uncertain because the remote outcome may be unknown. On startup, in-flight destinations and segments become uncertain with a recovery explanation.

There is no cross-provider transaction and no guarantee that a manual retry will be idempotent across every provider. Review recorded remote IDs and uncertain results before retrying. Publication history is durable until the user deletes it.

## Scheduling semantics

The desktop process checks due schedules every 15 seconds. Automatic dispatch only claims an item no more than 60 seconds after its scheduled instant. A schedule beyond that grace period becomes **Needs attention** instead of sending late. Startup marks queued items whose time elapsed while Threadline was closed as **Needs attention**, and interrupted dispatch is also recovered for review.

Use **Send now** for a missed item, or reschedule/cancel it with the record’s current revision. Scheduling is local to the desktop installation; it does not run while the app is closed and does not provide a hosted delivery service.

## Planning and media

The native composer planner counts Unicode grapheme clusters and produces destination-specific thread parts. The UI requests a preview after a short debounce, but native dispatch re-plans rather than trusting a cached preview. The policy selects a shared shortest limit, destination-adaptive limits, or forced numbering. The planner prefers paragraph, sentence, whitespace, then grapheme boundaries and reserves numbering space until the part count fits.

Threadline accepts up to four JPEG, PNG, or WebP images, 2 MB each, or one MP4 video. Images and video cannot be mixed; alt text is limited to 1,500 characters. Provider capability limits remain authoritative: Bluesky advertises a 300 MB MP4 limit subject to provider allowance and processing, while Mastodon capability discovery supplies usable video limits when available. Polls, content warnings, audio, and other video formats are not composer-supported.

## Verification boundaries

The controlled browser bridge used by Playwright can demonstrate draft and UI lifecycles but cannot validate a native publication or a live provider result. Run the Rust checks for durable native behavior and use real accounts only when intentionally performing integration work; see [development](development.md#checks).
