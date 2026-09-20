import { expect, test } from "@playwright/test";
import type { Page } from "@playwright/test";

async function installLifecycleDesktop(page: Page, notificationFailures = 0): Promise<void> {
  await page.addInitScript(({ notificationFailures }) => {
    const account = {
      id: "reader", provider: "MASTODON", handle: "reader@example.test", displayName: "Reader",
      instanceUrl: "https://example.test", did: null,
      capabilities: { maxTextLength: 500, countingPolicy: "GRAPHEME", reservedUrlLength: 23, maxMediaAttachments: 4, supportedMediaTypes: [], supportsPolls: true, supportsContentWarnings: true },
    };
    const actor = { id: "author", displayName: "Home author", handle: "author@example.test", avatarUrl: null };
    const post = (id: string, text: string) => ({
      canonicalKey: `MASTODON:${id}`, provider: "MASTODON", remoteId: id, remoteCid: null,
      remoteUrl: `https://example.test/@author/${id}`, author: actor, text, createdAt: "2026-09-20T08:00:00Z",
      media: [], metrics: { replies: 0, reposts: 0, likes: 0 },
      viewer: { liked: false, reposted: false, likeUri: null, repostUri: null },
      replyParentId: null, replyRootId: null, replyRootCid: null,
    });
    const people = Array.from({ length: 100 }, (_, index) => ({
      id: `person-${index}`, provider: "MASTODON", sourceType: "PERSON", title: `Person ${index}`,
      description: null, remoteId: `person-${index}`,
    }));
    const custom = { id: "list-one", provider: "MASTODON", sourceType: "LIST", title: "Local makers", description: "A custom provider list", remoteId: "list-one" };
    let documentVisible = true;
    Object.defineProperty(document, "visibilityState", { configurable: true, get: () => documentVisible ? "visible" : "hidden" });
    window.addEventListener("qa-document-hidden", () => { documentVisible = true; documentVisible = false; document.dispatchEvent(new Event("visibilitychange")); });
    window.addEventListener("qa-document-visible", () => { documentVisible = false; documentVisible = true; document.dispatchEvent(new Event("visibilitychange")); });
    const increment = (key: string) => localStorage.setItem(key, String(Number(localStorage.getItem(key) ?? "0") + 1));
    Object.defineProperty(window, "__TAURI_INTERNALS__", { value: { invoke: async (command: string, args: { accountId?: string; source?: { sourceType?: string }; cursor?: string | null }) => {
      if (command === "get_workspace") return { accounts: [account], connectedAccountIds: [account.id], mode: "LIVE" };
      if (command === "get_home_feed") {
        increment("qa-home-count");
        return args.cursor ? { posts: [post("home-more", "Second provider home page")], cursor: null } : { posts: [post("home", "Provider following stream post")], cursor: "home-next" };
      }
      if (command === "get_followed_sources") { increment("qa-following-count"); return { sources: [...people, custom], cursor: null }; }
      if (command === "get_source_feed") {
        increment(args.source?.sourceType === "PERSON" ? "qa-person-feed-count" : "qa-custom-feed-count");
        return args.cursor ? { posts: [post("custom-more", "Second custom list page")], cursor: null } : { posts: [post("custom", "Custom list post remains visible")], cursor: "custom-next" };
      }
      if (command === "get_notifications") {
        increment("qa-notification-count");
        if (Number(localStorage.getItem("qa-notification-count")) <= notificationFailures) throw new Error("Notification provider unavailable");
        return { notifications: [{ id: "notice", kind: "MENTION", createdAt: "2026-09-20T08:00:00Z", actor, post: post("mention", "A populated notification with enough text to verify responsive wrapping without clipping."), unread: true }], cursor: null };
      }
      if (command === "get_profile") return { actor, description: "", followersCount: 1, followingCount: 1, postsCount: 1, followedByMe: true, followUri: null };
      if (command === "get_profile_feed") return { posts: [post("profile", "Exact person detail")], cursor: null };
      throw new Error(`Unexpected command: ${command}`);
    } } });
  }, { notificationFailures });
}

test("Following uses one cached provider home stream for 100 people and preserves custom sources", async ({ page }) => {
  await installLifecycleDesktop(page);
  await page.goto("/");
  await expect(page.getByRole("heading", { name: "Timeline", exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Following", exact: true }).click();
  await expect(page.getByText("Custom list post remains visible", { exact: true })).toBeVisible();
  await expect(page.locator(".unified-page:visible").getByText("Provider following stream post", { exact: true })).toBeVisible();
  await page.getByRole("button", { name: /People/ }).click();
  await expect(page.locator(".collection-heading").getByText("Your provider’s following stream", { exact: false })).toBeVisible();
  await expect.poll(() => page.evaluate(() => localStorage.getItem("qa-person-feed-count") ?? "0")).toBe("0");
  await expect.poll(() => page.evaluate(() => localStorage.getItem("qa-home-count") ?? "0")).toBe("1");
  await expect.poll(() => page.evaluate(() => localStorage.getItem("qa-custom-feed-count") ?? "0")).toBe("1");
  await page.getByRole("button", { name: "All Following" }).click();
  await page.getByRole("button", { name: "Load more posts", exact: true }).click();
  await expect(page.getByText("Second provider home page", { exact: true })).toBeVisible();
  await expect(page.getByText("Second custom list page", { exact: true })).toBeVisible();
  await expect.poll(() => page.evaluate(() => localStorage.getItem("qa-home-count") ?? "0")).toBe("2");
  await expect.poll(() => page.evaluate(() => localStorage.getItem("qa-custom-feed-count") ?? "0")).toBe("2");
  await page.getByRole("button", { name: /People/ }).click();
  for (const width of [375, 768, 1280]) {
    await page.setViewportSize({ width, height: 900 });
    await page.locator("#main").evaluate(element => { element.scrollTop = 0; });
    await page.evaluate(() => { window.scrollTo(0, 0); if (document.activeElement instanceof HTMLElement) document.activeElement.blur(); });
    await page.screenshot({ path: `.omo/evidence/optimization/following-people-${width}.png`, fullPage: true });
  }
  await page.setViewportSize({ width: 1280, height: 900 });
  await page.getByRole("button", { name: /Person 0/ }).click();
  await expect(page.getByRole("button", { name: "Back", exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Back", exact: true }).click();
  await expect(page.getByRole("heading", { name: "People", exact: true })).toBeVisible();
  await expect(page.locator(".unified-page:visible").getByText("Provider following stream post", { exact: true })).toBeVisible();
});

test("notification polling runs only while active and visible and resumes once", async ({ page }) => {
  await page.clock.install({ time: new Date("2026-09-20T08:00:00Z") });
  await installLifecycleDesktop(page);
  await page.goto("/");
  await page.getByRole("button", { name: "Notifications", exact: true }).click();
  await expect.poll(() => page.evaluate(() => localStorage.getItem("qa-notification-count") ?? "0")).toBe("1");
  await page.getByRole("button", { name: "Timeline", exact: true }).click();
  await page.clock.runFor(90_000);
  await expect.poll(() => page.evaluate(() => localStorage.getItem("qa-notification-count") ?? "0")).toBe("1");
  await page.getByRole("button", { name: "Notifications", exact: true }).click();
  await expect.poll(() => page.evaluate(() => localStorage.getItem("qa-notification-count") ?? "0")).toBe("2");
  await page.evaluate(() => window.dispatchEvent(new Event("qa-document-hidden")));
  await page.clock.runFor(90_000);
  await expect.poll(() => page.evaluate(() => localStorage.getItem("qa-notification-count") ?? "0")).toBe("2");
  await page.evaluate(() => { window.dispatchEvent(new Event("qa-document-visible")); window.dispatchEvent(new Event("qa-document-visible")); });
  await expect.poll(() => page.evaluate(() => localStorage.getItem("qa-notification-count") ?? "0")).toBe("3");
  for (const width of [375, 768, 1280]) {
    await page.setViewportSize({ width, height: 900 });
    await page.locator("#main").evaluate(element => { element.scrollTop = 0; });
    await page.evaluate(() => { window.scrollTo(0, 0); if (document.activeElement instanceof HTMLElement) document.activeElement.blur(); });
    await page.screenshot({ path: `.omo/evidence/optimization/notifications-populated-${width}.png`, fullPage: true });
  }
});

test("notification polling backs off after failure and resets after success", async ({ page }) => {
  await page.clock.install({ time: new Date("2026-09-20T08:00:00Z") });
  await installLifecycleDesktop(page, 1);
  await page.goto("/");
  await page.getByRole("button", { name: "Notifications", exact: true }).click();
  await expect(page.getByRole("alert")).toContainText("Notification provider unavailable");
  await page.clock.runFor(59_000);
  expect(await page.evaluate(() => localStorage.getItem("qa-notification-count") ?? "0")).toBe("1");
  await page.clock.runFor(1_000);
  await expect.poll(() => page.evaluate(() => localStorage.getItem("qa-notification-count") ?? "0")).toBe("2");
  await page.clock.runFor(29_000);
  expect(await page.evaluate(() => localStorage.getItem("qa-notification-count") ?? "0")).toBe("2");
  await page.clock.runFor(1_000);
  await expect.poll(() => page.evaluate(() => localStorage.getItem("qa-notification-count") ?? "0")).toBe("3");
});
