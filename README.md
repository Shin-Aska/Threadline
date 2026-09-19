# Threadline

A desktop client for Bluesky and Mastodon. Read your timelines together, write one draft, and preview how it will publish to each account.

![Threadline Composer with two selected accounts and per-network previews](docs/images/composer.png)

*Screenshots show the actual interface with example accounts and posts.*

## What you can do

- Read a combined home timeline with account filters and source attribution.
- Publish to multiple accounts, with thread splitting for each destination’s text limit.
- Choose a common limit, adapt to each account, or always number your thread.
- Attach up to four images with alt text and get hashtag suggestions as you type.
- See publication results per account, including partial failures.
- Keep account credentials in your operating system’s credential store.

Threadline opens on **Timeline**. Composer keeps your draft while you move between views; the **Accounts & Sync** badge shows how many accounts have a connected client.

<details>
<summary>Timeline and account management</summary>

### Timeline

![Combined Bluesky and Mastodon timeline](docs/images/timeline.png)

### Accounts & Sync

![Account management with two example identities](docs/images/accounts.png)

</details>

## Run locally

You’ll need Node.js 22.12+ (or 20.19+), npm, stable Rust, and the [Tauri prerequisites for your OS](https://v2.tauri.app/start/prerequisites/).

```bash
npm ci
npm run tauri:dev
```

On first launch, connect a Bluesky account with an app password or a Mastodon account with an instance access token. After setup, the app opens your timeline.

For frontend-only work, `npm run dev` opens a browser preview at `http://localhost:1420`. Account connection and publishing require the desktop app.

## Current scope

Threadline is early-stage. Timeline reading and publishing work; Discover currently shows Mastodon trending tags. Following lists Mastodon followed tags and Bluesky pinned feeds, but does not fetch an individual tag or feed timeline yet.

Drafts and publication results are not saved across restarts. There is no scheduling, OAuth sign-in, notification inbox, or in-app liking/reposting. Media support is JPEG, PNG, and WebP; polls, content warnings, video, and audio are not supported.

## Development

Built with **Tauri 2 · Rust · React 19 · TypeScript · SQLite**.

- [Development setup and checks](docs/development.md)
- [Releases and download verification](docs/releases.md)
- [How the codebase works](docs/architecture.md)
- [Publishing, thread planning, and retries](docs/publishing.md)
- [Provider support and extension points](docs/providers.md)
- [Visual system](DESIGN.md)

## License

[GNU General Public License v3.0](LICENSE).
