# Publishing and thread planning

## From draft to preview

The composer store holds text, image data, destination account IDs, and a publishing policy. [`usePostPreview`](../src/hooks/usePostPreview.ts) waits 250 ms after changes before requesting a native preview. Image bytes are omitted from preview requests; metadata is retained.

A preview belongs to a particular draft and workspace snapshot. Stale responses are ignored. Publishing requires a current preview, connected destinations, and completed image reads.

[`composer/mod.rs`](../src-tauri/src/composer/mod.rs) counts Unicode grapheme clusters and plans the parts for every destination:

| Policy | Behavior |
| --- | --- |
| Common limit (default) | Use the shortest selected account limit for every destination |
| Adaptive | Split separately using each account’s limit |
| Always thread | Use per-account limits and always add numbering, even for a single part |

Common limit still splits an overlong draft; it is not a hard editor cutoff. The planner prefers paragraph, sentence, whitespace, then grapheme boundaries. It reserves room for `1/N ` prefixes and recalculates until the part count fits. Image-only posts produce one empty text part.

Capability fields describe counting policies and reserved URL lengths, but the current planner uses grapheme counting rather than provider-native URL counting.

## The native write path

[`publish_to_accounts`](../src-tauri/src/commands/mod.rs) performs these steps:

1. Read accounts and re-plan the submitted draft.
2. Validate and decode media.
3. Clone available provider clients out of the shared map.
4. Publish to each destination sequentially.
5. Publish the first part, then reply to the previous part for each continuation.
6. Return a separate result for every destination, including remote IDs already created.

Images attach only to the first part. A failed destination does not prevent later destinations from being attempted. No lock is held on the provider map during network publishing.

## Images and hashtags

Images must be JPEG, PNG, or WebP, with at most four attachments and 2,000,000 bytes per image. Alt text is limited to 1,500 Unicode characters. The frontend keeps base64 data in memory; Rust checks decoded size and image signatures before uploading.

Bluesky uploads blobs and creates an image embed. Mastodon uploads media before creating statuses. Provider-specific code lives beside each provider implementation.

Hashtag suggestions use connected account data. Mastodon activity and Bluesky search-match counts are different measurements, not a combined reach estimate. Bluesky publication adds hashtag facets using UTF-8 byte offsets; link and mention facets are not generated.

## Failures and retries

The UI blocks duplicate clicks while a publish request is pending. Full success clears the draft and attachments. Failure preserves them and displays per-account results.

After a partial result, the UI removes destinations that succeeded **or created any remote posts** from the draft’s selected destinations. This reduces accidental duplicates during a retry, but it does not resume an incomplete remote thread. Check the provider before manually retrying that account.

There is no cross-network transaction, durable publish queue, or server-side idempotency guarantee. Results include a generated `canonicalId`, but neither the draft nor publication results are currently written to SQLite. Closing the app loses this session’s retry context.
