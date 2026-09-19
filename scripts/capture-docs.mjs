import { mkdir } from "node:fs/promises";
import process from "node:process";
import { fileURLToPath, URL } from "node:url";
import { chromium, expect } from "@playwright/test";

// Capture the real frontend with example data; no provider requests or writes.
const accounts = [
  { id: "bsky", provider: "BLUESKY", handle: "alex.example", displayName: "Alex Rivera", instanceUrl: "https://bsky.social", did: "did:plc:example" },
  { id: "masto", provider: "MASTODON", handle: "alex@social.example", displayName: "Alex Rivera", instanceUrl: "https://social.example", did: null },
].map(account => ({ ...account, capabilities: { maxTextLength: account.provider === "BLUESKY" ? 300 : 500, countingPolicy: "GRAPHEME", reservedUrlLength: null, maxMediaAttachments: 4, supportedMediaTypes: ["image/jpeg", "image/png", "image/webp"], supportsPolls: false, supportsContentWarnings: false } }));
const examples = [
  ["bsky", "Maya Chen", "maya.example", "Small things that make a good writing space: an open window, a notebook, and enough quiet to finish a thought.", 12],
  ["masto", "Sam Brooks", "sam@social.example", "Spent the morning walking the long way home. Found a tiny bookshop, bought a very large book. A fair trade.", 28],
  ["bsky", "Jordan Lee", "jordan.example", "A reminder for anyone building something this weekend: the first version only needs to help one person. Start there.", 45],
  ["masto", "Robin Ellis", "robin@social.example", "The community garden is waking up again. Peas are planted, the tools are shared, and someone brought coffee. #Gardening", 63],
];
const now = new Date("2026-09-19T08:00:00Z");
const posts = examples.map(([accountId, displayName, handle, text, minutes], index) => {
  const account = accounts.find(account => account.id === accountId);
  return { canonicalKey: `${account.provider}:example-${index}`, provider: account.provider, remoteId: `example-${index}`, remoteUrl: "https://example.invalid/post", author: { id: handle, displayName, handle, avatarUrl: null }, text, createdAt: new Date(now.getTime() - minutes * 60_000).toISOString(), media: [], sources: [{ accountId, accountHandle: account.handle, provider: account.provider }], metrics: { replies: index + 1, reposts: index * 2, likes: 12 + index * 7 }, capabilities: { openOriginal: true, reply: false, like: false, repost: false } };
});
const browser = await chromium.launch({ channel: "chrome" });
try {
  const page = await browser.newPage({ viewport: { width: 1536, height: 1024 }, deviceScaleFactor: 1 });
  await page.clock.setFixedTime(now);
  await page.addInitScript(({ accounts, posts }) => {
    Object.defineProperty(globalThis, "__TAURI_INTERNALS__", { value: { invoke: async (command, args) => {
      if (command === "get_workspace") return { accounts, connectedAccountIds: accounts.map(account => account.id), mode: "LIVE" };
      if (command === "get_timeline") return { posts: posts.filter(post => post.sources.some(source => source.accountId === args.accountId)), cursor: null };
      if (command === "preview_post") {
        const selected = accounts.filter(account => args.post.destinationAccountIds.includes(account.id));
        return { graphemeCount: args.post.text.length, effectiveLimit: 300, limitingAccountId: "bsky", destinations: selected.map(account => ({ accountId: account.id, label: account.handle, maxLength: account.capabilities.maxTextLength, parts: [args.post.text] })) };
      }
      throw new Error(`Documentation fixture does not support ${command}`);
    } } });
  }, { accounts, posts });
  await page.goto(process.env.THREADLINE_DOCS_URL ?? "http://127.0.0.1:4174");
  await expect(page.getByText(examples[0][3], { exact: true })).toBeVisible();
  await page.evaluate(() => globalThis.document.fonts.ready);
  await mkdir(new URL("../docs/images/", import.meta.url), { recursive: true });
  const capture = name => page.screenshot({ path: fileURLToPath(new URL(`../docs/images/${name}.png`, import.meta.url)), fullPage: true, animations: "disabled" });
  await capture("timeline");
  await page.getByRole("button", { name: "Composer", exact: true }).click();
  await page.getByLabel("Post text", { exact: true }).fill("A quiet Saturday, a hot coffee, and a list of ideas.\n\nGrateful for this corner of the internet and the people who make it feel like home.");
  await expect(page.getByRole("button", { name: "Publish to 2 accounts", exact: true })).toBeEnabled();
  await page.getByLabel("Post text", { exact: true }).blur();
  await capture("composer");
  await page.getByRole("button", { name: "Accounts & Sync", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Your identities", exact: true })).toBeVisible();
  await capture("accounts");
  process.stdout.write("Captured Composer, Timeline, and Accounts & Sync in docs/images/.\n");
} finally {
  await browser.close();
}
