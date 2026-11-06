import { expect, test } from "@playwright/test";
import type { Page } from "@playwright/test";

async function installRefreshDesktop(page: Page): Promise<void> {
  await page.addInitScript(() => {
    const accounts = ["BLUESKY", "MASTODON"].map(provider => ({
      id: provider, provider, handle: `${provider.toLowerCase()}.test`, displayName: provider === "BLUESKY" ? "Blue reader" : "Masto reader",
      instanceUrl: "https://example.test", did: null,
      capabilities: { maxTextLength: 500, countingPolicy: "GRAPHEME", reservedUrlLength: 23, maxMediaAttachments: 4, supportedMediaTypes: [], supportsPolls: false, supportsContentWarnings: true },
    }));
    const actor = { id: "author", displayName: "A reader", handle: "reader.test", avatarUrl: null };
    const post = (provider: string) => ({
      canonicalKey: `${provider}:own`, provider, remoteId: "own", remoteCid: null, remoteUrl: "https://example.test/post",
      author: actor, text: `${provider} profile version ${localStorage.getItem("qa-profile-version") ?? "1"}`,
      createdAt: "2026-11-06T08:00:00Z", media: [], metrics: { replies: 0, reposts: 0, likes: 0 },
      viewer: { liked: false, reposted: false, likeUri: null, repostUri: null }, replyParentId: null, replyRootId: null, replyRootCid: null,
    });
    const read = new Set<string>();
    Object.defineProperty(window, "__TAURI_INTERNALS__", { value: { invoke: async (command: string, args: { accountId?: string }) => {
      const accountId = args.accountId ?? "";
      if (command === "get_workspace") return { accounts, connectedAccountIds: accounts.map(account => account.id), mode: "LIVE" };
      if (command === "get_home_feed") return { posts: [], cursor: null };
      if (command === "get_own_feed") {
        if (localStorage.getItem("qa-profile-failure") === accountId) throw new Error("Profile temporarily unavailable");
        return { posts: [post(accountId)], cursor: null };
      }
      if (command === "get_own_profile") return { actor, description: `Profile version ${localStorage.getItem("qa-profile-version") ?? "1"}`, followersCount: 1, followingCount: 1, postsCount: 1, followedByMe: false, followUri: null };
      if (command === "get_notifications") {
        if (localStorage.getItem("qa-notification-pending") === accountId) {
          localStorage.setItem("qa-notification-started", accountId);
          await new Promise<void>(resolve => window.addEventListener("qa-release-notifications", () => resolve(), { once: true }));
        }
        if (localStorage.getItem("qa-notification-failure") === accountId) throw new Error("Notifications temporarily unavailable");
        const notification = { id: accountId, kind: "FOLLOW", createdAt: "2026-11-06T08:00:00Z", actor, post: null, unread: !read.has(accountId) };
        const notifications = [notification];
        if (localStorage.getItem("qa-new-notification") === accountId) notifications.push({ ...notification, id: `${accountId}-new`, kind: "LIKE", unread: true });
        return { notifications, cursor: null };
      }
      if (command === "mark_notifications_read") {
        localStorage.setItem(`qa-mark-count-${accountId}`, String(Number(localStorage.getItem(`qa-mark-count-${accountId}`) ?? "0") + 1));
        if (localStorage.getItem("qa-mark-pending") === accountId) await new Promise<void>(resolve => window.addEventListener("qa-release-mark", () => resolve(), { once: true }));
        if (localStorage.getItem("qa-mark-failure") === accountId) throw new Error("Mark read temporarily unavailable");
        read.add(accountId);
        return;
      }
      throw new Error(`Unexpected command: ${command}`);
    } } });
  });
}

test.beforeEach(async ({ page }) => { await installRefreshDesktop(page); await page.goto("/"); });

test("unified profiles fetch new posts on return without switching accounts", async ({ page }) => {
  // Given both profiles have loaded and their provider data changes within the cache TTL.
  await page.getByRole("button", { name: "My profiles", exact: true }).click();
  await expect(page.getByText("BLUESKY profile version 1", { exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Timeline", exact: true }).click();
  await page.evaluate(() => localStorage.setItem("qa-profile-version", "2"));
  // When returning directly to the retained unified view.
  await page.getByRole("button", { name: "My profiles", exact: true }).click();
  // Then both providers show their latest posts.
  await expect(page.getByText("BLUESKY profile version 2", { exact: true })).toBeVisible();
  await expect(page.getByText("MASTODON profile version 2", { exact: true })).toBeVisible();
});

test("profile refresh bypasses cached feeds and exposes recoverable account failures", async ({ page }) => {
  // Given an already loaded unified profile and one failing provider.
  await page.getByRole("button", { name: "My profiles", exact: true }).click();
  await expect(page.getByText("MASTODON profile version 1", { exact: true })).toBeVisible();
  await page.evaluate(() => { localStorage.setItem("qa-profile-version", "2"); localStorage.setItem("qa-profile-failure", "BLUESKY"); });
  // When refreshing explicitly.
  await page.getByRole("button", { name: "Refresh profiles", exact: true }).click();
  // Then the healthy account updates and the failed account can be retried.
  await expect(page.getByRole("alert")).toContainText("Blue reader");
  await expect(page.getByText("MASTODON profile version 2", { exact: true })).toBeVisible();
  await page.evaluate(() => localStorage.removeItem("qa-profile-failure"));
  await page.getByRole("button", { name: "Refresh profiles", exact: true }).click();
  await expect(page.getByText("BLUESKY profile version 2", { exact: true })).toBeVisible();
  await expect(page.getByRole("alert")).toHaveCount(0);
});

test("switching back to Unified bypasses cached posts from both accounts", async ({ page }) => {
  // Given Unified and the Bluesky scope have loaded before the provider data changes.
  await page.getByRole("button", { name: "My profiles", exact: true }).click();
  await expect(page.getByText("MASTODON profile version 1", { exact: true })).toBeVisible();
  await page.locator(".profile-scope").getByRole("button", { name: "Bluesky", exact: true }).click();
  await expect(page.getByText("Profile version 1", { exact: true })).toBeVisible();
  await page.evaluate(() => localStorage.setItem("qa-profile-version", "2"));
  // When selecting Unified directly within the cache TTL.
  await page.locator(".profile-scope").getByRole("button", { name: "Unified", exact: true }).click();
  // Then both accounts are current without separately visiting Mastodon.
  await expect(page.getByText("BLUESKY profile version 2", { exact: true })).toBeVisible();
  await expect(page.getByText("MASTODON profile version 2", { exact: true })).toBeVisible();
});

test("background notification reads keep mark-all disabled until fresh activity arrives", async ({ page }) => {
  // Given a loaded inbox and a delayed next background read.
  await page.clock.install();
  await page.getByRole("button", { name: "Notifications", exact: true }).click();
  await expect(page.locator(".notification-row")).toHaveCount(2);
  await page.evaluate(() => localStorage.setItem("qa-notification-pending", "BLUESKY"));
  // When the polling timer starts that read.
  await page.clock.runFor(30_000);
  await expect.poll(() => page.evaluate(() => localStorage.getItem("qa-notification-started"))).toBe("BLUESKY");
  // Then read actions cannot silently collide with it, and incoming activity is retained.
  await expect(page.getByRole("button", { name: "Mark all read", exact: true })).toBeDisabled();
  await page.evaluate(() => { localStorage.setItem("qa-new-notification", "BLUESKY"); window.dispatchEvent(new Event("qa-release-notifications")); });
  await expect(page.locator(".notification-row")).toHaveCount(3);
  await expect(page.getByRole("button", { name: "Mark all read", exact: true })).toBeEnabled();
});

test("notifications can refresh immediately after a provider failure", async ({ page }) => {
  // Given an inbox with a provider that fails the next read.
  await page.getByRole("button", { name: "Notifications", exact: true }).click();
  await expect(page.locator(".notification-row")).toHaveCount(2);
  await page.evaluate(() => localStorage.setItem("qa-notification-failure", "BLUESKY"));
  // When refreshing, then recovering and retrying without waiting for the polling timer.
  await page.getByRole("button", { name: "Refresh notifications", exact: true }).click();
  await expect(page.getByRole("alert")).toContainText("Blue reader");
  await page.evaluate(() => { localStorage.removeItem("qa-notification-failure"); localStorage.setItem("qa-new-notification", "BLUESKY"); });
  await page.getByRole("button", { name: "Refresh notifications", exact: true }).click();
  // Then new activity is visible and the failure clears.
  await expect(page.locator(".notification-row")).toHaveCount(3);
  await expect(page.getByRole("alert")).toHaveCount(0);
});

test("mark all read locks repeat actions and preserves partial success for retry", async ({ page }) => {
  // Given one delayed failing Bluesky read action and a healthy Mastodon account.
  await page.getByRole("button", { name: "Notifications", exact: true }).click();
  await expect(page.locator(".notification-row.unread")).toHaveCount(2);
  await page.evaluate(() => { localStorage.setItem("qa-mark-pending", "BLUESKY"); localStorage.setItem("qa-mark-failure", "BLUESKY"); });
  // When marking every account read.
  await page.getByRole("button", { name: "Mark all read", exact: true }).click();
  // Then the pending action cannot be resubmitted and successful accounts stay read.
  await expect(page.getByRole("button", { name: "Marking read…", exact: true })).toBeDisabled();
  await page.evaluate(() => window.dispatchEvent(new Event("qa-release-mark")));
  await expect(page.getByRole("alert")).toContainText("Blue reader");
  await expect(page.locator(".notification-row.unread")).toHaveCount(1);
  await page.evaluate(() => { localStorage.removeItem("qa-mark-pending"); localStorage.removeItem("qa-mark-failure"); });
  await page.getByRole("button", { name: "Mark all read", exact: true }).click();
  await expect(page.locator(".notification-row.unread")).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Notifications", exact: true })).toHaveAccessibleDescription("0 unread notifications");
  expect(await page.evaluate(() => localStorage.getItem("qa-mark-count-MASTODON"))).toBe("1");
});
