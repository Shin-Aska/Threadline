import { expect, test } from "@playwright/test";
import type { Page } from "@playwright/test";

async function installBrowsingDesktop(page: Page): Promise<void> {
  await page.addInitScript(() => {
    const accounts = ["BLUESKY", "MASTODON"].map(provider => ({
      id: provider, provider, handle: `${provider.toLowerCase()}.test`, displayName: provider,
      instanceUrl: "https://example.test", did: null,
      capabilities: { maxTextLength: 500, countingPolicy: "GRAPHEME", reservedUrlLength: 23, maxMediaAttachments: 4, supportedMediaTypes: [], supportsPolls: false, supportsContentWarnings: true },
    }));
    const image = "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='40' height='30'%3E%3Crect width='40' height='30' fill='gray'/%3E%3C/svg%3E";
    Object.defineProperty(window, "__TAURI_INTERNALS__", { value: { invoke: async (command: string, args: { accountId?: string; cursor?: string | null; source?: { id: string } }) => {
      const accountId = args.accountId ?? "";
      const version = localStorage.getItem("qa-browsing-version") ?? "1";
      const actor = { id: `${accountId}-author`, displayName: `${accountId} writer ${version}`, handle: `${accountId.toLowerCase()}.test`, avatarUrl: null };
      const post = (id: string) => ({
        canonicalKey: `${accountId}:${id}`, provider: accountId, remoteId: id, remoteCid: null, remoteUrl: "https://example.test/post",
        author: actor, text: `${accountId} ${id} version ${version}`, createdAt: "2026-11-06T08:00:00Z",
        media: [{ url: image, alt: "A sample image", mediaType: "image/png" }], metrics: { replies: 0, reposts: 0, likes: 2 },
        viewer: { liked: false, reposted: false, likeUri: null, repostUri: null }, replyParentId: null, replyRootId: null, replyRootCid: null,
      });
      if (command === "get_workspace") return { accounts, connectedAccountIds: accounts.map(account => account.id), mode: "LIVE" };
      if (command === "get_notifications") return { notifications: [], cursor: null };
      const key = `qa-calls-${command}-${accountId}`;
      localStorage.setItem(key, String(Number(localStorage.getItem(key) ?? "0") + 1));
      if (localStorage.getItem("qa-pending-command") === command && accountId === "BLUESKY") await new Promise<void>(resolve => window.addEventListener("qa-release-browsing", () => resolve(), { once: true }));
      if (localStorage.getItem("qa-failing-command") === command && accountId === "BLUESKY") throw new Error("Provider temporarily unavailable");
      if (command === "get_home_feed") return { posts: [post(args.cursor ? "home-more" : "home")], cursor: args.cursor ? null : "next" };
      if (command === "get_followed_sources") {
        const source = (id: string, sourceType: string) => ({ id, provider: accountId, sourceType, title: `${accountId} ${id} version ${version}`, description: null, remoteId: id });
        return { sources: [source("person", "PERSON"), source("custom", "LIST"), ...(version === "1" || localStorage.getItem("qa-stable-sources") ? [] : [source("new-custom", "LIST")])], cursor: null };
      }
      if (command === "get_source_feed") return { posts: [post(args.source?.id ?? "custom")], cursor: null };
      if (command === "get_discovery") return {
        topics: ["alpha", "beta"].map((name, index) => ({ key: name, name, postCount: Number(version) * 10 - index, sources: [{ accountId, accountHandle: actor.handle, provider: accountId }] })),
        suggestedAccounts: [actor],
        popularPosts: [{ ...post("popular"), media: [], sources: [{ accountId, accountHandle: actor.handle, provider: accountId }], capabilities: { openOriginal: true, reply: false, like: false, repost: false } }],
      };
      throw new Error(`Unexpected command: ${command}`);
    } } });
  });
}

const view = (page: Page) => page.locator(".unified-page:visible");
async function onlyBluesky(page: Page): Promise<void> {
  await view(page).getByRole("button", { name: /^Bluesky/ }).click();
  await page.getByRole("button", { name: "Only Bluesky", exact: true }).click();
}
async function filterPosts(page: Page): Promise<void> {
  await view(page).getByRole("button", { name: "Latest first", exact: true }).click();
  await page.getByRole("button", { name: "Most liked", exact: true }).click();
  await view(page).getByRole("button", { name: "Filters", exact: true }).click();
  await view(page).getByLabel("Images & video", { exact: true }).check();
  await view(page).getByLabel("Hide replies", { exact: true }).check();
}
async function expectPostFilters(page: Page, query: string): Promise<void> {
  await expect(view(page).getByRole("textbox", { name: "Search loaded posts" })).toHaveValue(query);
  await expect(view(page).getByRole("button", { name: "Most liked", exact: true })).toBeVisible();
  await expect(view(page).getByLabel("Images & video", { exact: true })).toBeChecked();
  await expect(view(page).getByLabel("Hide replies", { exact: true })).toBeChecked();
  await expect(view(page).locator(".source-controls").getByRole("button", { name: /^Mastodon/ })).toContainText("0");
}

test.beforeEach(async ({ page }) => { await installBrowsingDesktop(page); await page.goto("/"); });

test("Timeline refresh bypasses cache and preserves scope, search, sorting and filters", async ({ page }) => {
  // Given a filtered single-account timeline already cached in memory.
  await expect(view(page).getByText("BLUESKY home version 1", { exact: true })).toBeVisible();
  await onlyBluesky(page);
  await view(page).getByRole("button", { name: "Load more", exact: true }).click();
  await expect(view(page).getByText("BLUESKY home-more version 1", { exact: true })).toBeVisible();
  await view(page).getByRole("textbox", { name: "Search loaded posts" }).fill("home");
  await filterPosts(page);
  await page.evaluate(() => { localStorage.setItem("qa-browsing-version", "2"); localStorage.setItem("qa-pending-command", "get_home_feed"); });
  // When manually refreshing within the cache TTL.
  const refresh = page.getByRole("button", { name: "Refresh timeline", exact: true });
  await expect(refresh).toBeVisible();
  await refresh.click();
  await expect(refresh).toBeDisabled();
  await expect(refresh).toHaveText("Refreshing…");
  await page.evaluate(() => window.dispatchEvent(new Event("qa-release-browsing")));
  // Then fresh posts appear with all reading choices retained.
  await expect(view(page).getByText("BLUESKY home version 2", { exact: true })).toBeVisible();
  await expect(view(page).getByText("MASTODON home version 2", { exact: true })).toHaveCount(0);
  await expect(view(page).getByText("BLUESKY home-more version 1", { exact: true })).toHaveCount(0);
  await expectPostFilters(page, "home");
  await expect(refresh).toBeEnabled();
});

test("Following refresh waits for custom feeds even when the source IDs stay the same", async ({ page }) => {
  // Given a loaded collection whose membership remains unchanged.
  await page.getByRole("button", { name: "Following", exact: true }).click();
  await expect(view(page).locator(".unified-post").getByText("BLUESKY custom version 1", { exact: true })).toBeVisible();
  const previousCalls = await page.evaluate(() => Number(localStorage.getItem("qa-calls-get_source_feed-BLUESKY")));
  await page.evaluate(() => { localStorage.setItem("qa-stable-sources", "1"); localStorage.setItem("qa-browsing-version", "2"); localStorage.setItem("qa-pending-command", "get_source_feed"); });
  // When refreshing with the custom feed response delayed.
  const refresh = page.getByRole("button", { name: "Refresh following", exact: true });
  await refresh.click();
  await expect.poll(() => page.evaluate(() => Number(localStorage.getItem("qa-calls-get_source_feed-BLUESKY")))).toBe(previousCalls + 1);
  // Then the control remains busy until the unchanged source's fresh feed arrives.
  await expect(refresh).toBeDisabled();
  await expect(refresh).toHaveText("Refreshing…");
  await page.evaluate(() => window.dispatchEvent(new Event("qa-release-browsing")));
  await expect(view(page).locator(".unified-post").getByText("BLUESKY custom version 2", { exact: true })).toBeVisible();
  await expect(refresh).toBeEnabled();
});

test("Discover refresh preserves account scope, query and selected topic while reloading every panel", async ({ page }) => {
  // Given a selected topic and a single-account discovery scope.
  await page.getByRole("button", { name: "Discover", exact: true }).click();
  await expect(view(page).getByText("BLUESKY popular version 1", { exact: true })).toBeVisible();
  await onlyBluesky(page);
  await view(page).locator(".topic-list button").filter({ hasText: "#beta" }).click();
  await view(page).getByRole("textbox", { name: "Search discovery" }).fill("beta");
  await page.evaluate(() => localStorage.setItem("qa-browsing-version", "2"));
  // When refreshing the selected scope.
  const refresh = page.getByRole("button", { name: "Refresh discovery", exact: true });
  await expect(refresh).toBeVisible();
  await refresh.click();
  // Then topics, suggestions and popular posts update without resetting the selection.
  await expect(view(page).getByText("BLUESKY popular version 2", { exact: true })).toBeVisible();
  await expect(view(page).locator(".actor-copy")).toContainText("BLUESKY writer 2");
  await expect(view(page).locator(".topic-list")).toContainText("19 posts");
  await expect(view(page).locator(".detail-topic")).toHaveText("#beta");
  await expect(view(page).getByRole("textbox", { name: "Search discovery" })).toHaveValue("beta");
  await expect(view(page).locator(".source-controls").getByRole("button", { name: /^Mastodon/ })).toContainText("0");
});

test("Following refresh reloads source indexes, custom feeds and home while keeping the collection and filters", async ({ page }) => {
  // Given a filtered custom collection scoped to Bluesky.
  await page.getByRole("button", { name: "Following", exact: true }).click();
  await expect(view(page).getByText("BLUESKY home version 1", { exact: true })).toBeVisible();
  await onlyBluesky(page);
  await view(page).getByRole("button", { name: /Lists \/ feeds/ }).click();
  await expect(view(page).locator(".unified-post")).toContainText("BLUESKY custom version 1");
  await view(page).getByRole("textbox", { name: "Search loaded posts" }).fill("custom");
  await filterPosts(page);
  await page.evaluate(() => localStorage.setItem("qa-browsing-version", "2"));
  // When refreshing before either provider cache expires.
  const refresh = page.getByRole("button", { name: "Refresh following", exact: true });
  await expect(refresh).toBeVisible();
  await refresh.click();
  // Then the source list and feed data are current, with the collection and controls retained.
  await expect(view(page).locator(".unified-post").filter({ hasText: "BLUESKY custom version 2" })).toHaveCount(1);
  await expect(view(page).locator(".source-detail-list")).toContainText("BLUESKY new-custom version 2");
  await expect(view(page).locator(".collection-heading h2")).toHaveText("Lists / feeds");
  await expectPostFilters(page, "custom");
  await view(page).getByRole("textbox", { name: "Search loaded posts" }).fill("home");
  await view(page).getByRole("button", { name: /People/ }).click();
  await expect(view(page).getByText("BLUESKY home version 2", { exact: true })).toBeVisible();
});

for (const scenario of [
  { pageName: "Timeline", label: "Refresh timeline", command: "get_home_feed", content: "home" },
  { pageName: "Discover", label: "Refresh discovery", command: "get_discovery", content: "popular" },
  { pageName: "Following", label: "Refresh following", command: "get_source_feed", content: "custom" },
] as const) {
  test(`${scenario.pageName} refresh exposes provider failure and supports immediate retry`, async ({ page }) => {
    // Given loaded data and one temporarily unavailable provider.
    await page.getByRole("button", { name: scenario.pageName, exact: true }).click();
    await expect(view(page).locator(".unified-post").getByText(`BLUESKY ${scenario.content} version 1`, { exact: true })).toBeVisible();
    await page.evaluate(command => { localStorage.setItem("qa-browsing-version", "2"); localStorage.setItem("qa-failing-command", command); }, scenario.command);
    // When refreshing, then retrying after that provider recovers.
    const refresh = page.getByRole("button", { name: scenario.label, exact: true });
    await expect(refresh).toBeVisible();
    await refresh.click();
    await expect(view(page).locator('[role="alert"], [role="status"]').filter({ hasText: "Provider temporarily unavailable" })).toBeVisible();
    await expect(view(page).locator(".unified-post").getByText(`MASTODON ${scenario.content} version 2`, { exact: true })).toBeVisible();
    await expect(refresh).toBeEnabled();
    await page.evaluate(() => localStorage.removeItem("qa-failing-command"));
    await refresh.click();
    // Then fresh data replaces the failure without leaving this view.
    await expect(view(page).locator(".unified-post").getByText(`BLUESKY ${scenario.content} version 2`, { exact: true })).toBeVisible();
    await expect(view(page).locator('[role="alert"], [role="status"]').filter({ hasText: "Provider temporarily unavailable" })).toHaveCount(0);
  });
}
