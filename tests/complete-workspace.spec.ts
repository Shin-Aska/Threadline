import { expect, test } from "@playwright/test";
import { Buffer } from "node:buffer";

test("approved fixture orders loaded posts by discussion count", async ({ page }) => {
  await page.goto("/tests/fixtures/complete-workspace.html");
  await expect(page.getByRole("heading", { name: "Timeline", exact: true })).toBeVisible();
  await page.getByRole("button", { name: /Latest first/ }).click();
  await page.getByRole("button", { name: "Most discussed", exact: true }).click();
  await expect(page.locator(".unified-post .author-link")).toHaveText(["Marco Santos", "Clara Chen"]);
  await expect(page.locator(".unified-post footer button").nth(0)).toContainText("8");
});

test("Back restores Timeline filters and loaded presentation state", async ({ page }) => {
  await page.goto("/tests/fixtures/complete-workspace.html");
  await page.getByRole("button", { name: /Latest first/ }).click();
  await page.getByRole("button", { name: "Most discussed", exact: true }).click();
  await page.getByPlaceholder("Search loaded posts, people, or topics…").fill("Marco");
  await page.locator(".unified-post time button").click();
  await expect(page.getByRole("heading", { name: "Conversation", exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Back", exact: true }).click();
  await expect(page.getByPlaceholder("Search loaded posts, people, or topics…")).toHaveValue("Marco");
  await expect(page.getByRole("button", { name: /Most discussed/ })).toBeVisible();
  await expect(page.locator(".unified-post .author-link")).toHaveText(["Marco Santos"]);
});

test("approved fixture distinguishes own replies and topic feeds", async ({ page }) => {
  await page.goto("/tests/fixtures/complete-workspace.html");
  await page.getByRole("button", { name: "My profiles", exact: true }).click();
  await page.locator(".unified-page:visible .author-link").first().click();
  await expect(page.getByRole("heading", { name: "My profiles", exact: true })).toBeVisible();
  await expect(page.getByText("@richard.example · Bluesky", { exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: /Follow as/ })).toHaveCount(0);
  await page.getByRole("button", { name: "Replies", exact: true }).click();
  await expect(page.getByText("The replacement screen came from a donor handheld.")).toBeVisible();
  await expect(page.getByText("A few finds from the weekend.")).toHaveCount(0);
  await page.getByRole("button", { name: "Discover", exact: true }).click();
  await page.getByRole("button", { name: "Browse topic", exact: true }).first().click();
  await expect(page.locator(".unified-page:visible .unified-post")).toContainText("#retrogaming");
});

test("Following paginates source collections and their feeds", async ({ page }) => {
  await page.goto("/tests/fixtures/complete-workspace.html");
  await page.getByRole("button", { name: "Following", exact: true }).click();
  await expect(page.locator(".source-detail-list .actor-row")).toHaveCount(4);
  await page.getByRole("button", { name: "Load more sources" }).click();
  await expect(page.locator(".source-detail-list .actor-row")).toHaveCount(6);
  await page.getByRole("button", { name: "Load more posts" }).click();
  await expect(page.getByText(/A second loaded page for/)).toHaveCount(2);
  await expect(page.getByRole("alert")).toContainText("Fixture source page temporarily unavailable");
});

test("Notifications deduplicate paginated activity and preserve read state", async ({ page }) => {
  await page.goto("/tests/fixtures/complete-workspace.html");
  await page.getByRole("button", { name: "Notifications", exact: true }).click();
  await expect(page.locator(".notification-row")).toHaveCount(3);
  await page.getByRole("button", { name: "Load more" }).click();
  await expect(page.locator(".notification-row")).toHaveCount(5);
  await expect(page.getByText(/Updated Jules Park/)).toBeVisible();
  await expect(page.getByText(/Updated Clara Chen/)).toBeVisible();
  await expect(page.locator(".notification-row.unread").filter({ hasText: "Updated" })).toHaveCount(0);
  await page.getByRole("button", { name: "Mark all read" }).click();
  await expect(page.locator(".notification-row.unread")).toHaveCount(0);
});

test("approved fixture validates, previews, and restores an MP4 draft", async ({ page }) => {
  await page.goto("/tests/fixtures/complete-workspace.html");
  await page.getByRole("button", { name: "Composer", exact: true }).click();
  const base64 = await page.evaluate(async () => {
    const canvas = document.createElement("canvas"); canvas.width = 64; canvas.height = 64;
    const context = canvas.getContext("2d")!; context.fillStyle = "#6364ff"; context.fillRect(0, 0, 64, 64);
    const stream = canvas.captureStream(12);
    const recorder = new MediaRecorder(stream, { mimeType: "video/mp4" });
    const chunks: Blob[] = [];
    recorder.ondataavailable = event => chunks.push(event.data);
    const stopped = new Promise<void>(resolve => { recorder.onstop = () => resolve(); });
    recorder.start(); await new Promise(resolve => setTimeout(resolve, 400)); recorder.stop(); await stopped;
    stream.getTracks().forEach(track => track.stop());
    const bytes = new Uint8Array(await new Blob(chunks, { type: "video/mp4" }).arrayBuffer());
    let binary = ""; for (const byte of bytes) binary += String.fromCharCode(byte); return btoa(binary);
  });
  await page.getByLabel("Choose an MP4 video").setInputFiles({ name: "fixture.mp4", mimeType: "video/mp4", buffer: Buffer.from(base64, "base64") });
  await expect(page.locator(".attachment-card video")).toHaveAttribute("controls", "");
  await expect(page.locator(".preview-images video")).toHaveCount(2);
  await page.waitForTimeout(850);
  await page.getByRole("tab", { name: /Drafts/ }).click();
  await page.getByRole("button", { name: /1 media attachment/ }).click();
  await expect(page.locator(".attachment-card video")).toBeVisible();
  expect(await page.locator(".attachment-card video").evaluate(video => Number.isFinite((video as HTMLVideoElement).duration))).toBe(true);
});

test("social actions use the chosen account and roll back provider failure", async ({ page }) => {
  await page.addInitScript(() => {
    const capabilities = { maxTextLength: 300, maxMediaAttachments: 4, supportedMediaTypes: [], countingPolicy: "GRAPHEME", reservedUrlLength: null, supportsPolls: false, supportsContentWarnings: false };
    const accounts = [{ id: "reader-one", provider: "BLUESKY", handle: "one.test", displayName: "Personal", instanceUrl: "https://bsky.social", did: null, capabilities }, { id: "reader-two", provider: "BLUESKY", handle: "two.test", displayName: "Work", instanceUrl: "https://bsky.social", did: null, capabilities }];
    const post = { canonicalKey: "BLUESKY:shared", provider: "BLUESKY", remoteId: "shared", remoteCid: "cid-shared", remoteUrl: "https://example.test/shared", author: { id: "author", displayName: "Shared author", handle: "shared.test", avatarUrl: null }, text: "A post visible to both accounts.", createdAt: "2026-09-20T08:00:00Z", media: [], metrics: { replies: 1, reposts: 2, likes: 3 }, viewer: { liked: false, reposted: false, likeUri: null, repostUri: null }, replyParentId: null, replyRootId: null, replyRootCid: null };
    let attempts = 0;
    Object.defineProperty(window, "__TAURI_INTERNALS__", { value: { invoke: async (command: string, args: { accountId?: string }) => {
      if (command === "get_workspace") return { accounts, connectedAccountIds: accounts.map(account => account.id), mode: "LIVE" };
      if (command === "get_home_feed") return { posts: [post], cursor: null };
      if (command === "perform_social_action") { sessionStorage.setItem("acting-account", args.accountId ?? ""); attempts += 1; if (attempts === 1) throw new Error("Provider rejected the action"); return { targetId: "shared", viewer: { ...post.viewer, liked: true }, followed: null, recordId: "like:shared", createdPost: null }; }
      throw new Error(`Unexpected command ${command}`);
    } } });
  });
  await page.goto("/");
  const card = page.locator(".unified-post");
  await card.getByLabel("Act as").selectOption("reader-two");
  const like = card.locator("footer button").nth(2);
  await like.click();
  await expect(card.getByRole("alert")).toHaveText("Provider rejected the action");
  await expect(like).toHaveAttribute("aria-pressed", "false");
  expect(await page.evaluate(() => sessionStorage.getItem("acting-account"))).toBe("reader-two");
  await like.click();
  await expect(like).toHaveAttribute("aria-pressed", "true");
  await expect(like).toContainText("4");
});

test("successful heart updates the visible count without refreshing", async ({ page }) => {
  await page.goto("/tests/fixtures/complete-workspace.html");
  const card = page.locator(".unified-page:visible .unified-post").first();
  const heart = card.locator("footer button").nth(2);
  await expect(heart).toContainText("24");
  await heart.click();
  await expect(heart).toHaveAttribute("aria-pressed", "true");
  await expect(heart).toContainText("25");
  await page.getByPlaceholder("Search loaded posts, people, or topics…").fill("Marco");
  await expect(heart).toHaveAttribute("aria-pressed", "true");
  await heart.click();
  await expect(heart).toHaveAttribute("aria-pressed", "false");
  await expect(heart).toContainText("24");
});

test("successful reply updates the conversation and timeline without refreshing", async ({ page }) => {
  await page.goto("/tests/fixtures/complete-workspace.html");
  const timelinePost = page.locator(".unified-page:visible .unified-post").first();
  await timelinePost.locator("time button").click();
  const conversation = page.locator(".unified-page:visible");
  await conversation.locator("#thread-reply").fill("A new reply from this account.");
  await conversation.getByRole("button", { name: "Reply", exact: true }).click();
  await expect(conversation.getByText("A new reply from this account.")).toBeVisible();
  await expect(conversation.locator(".unified-post").first().locator("footer button").first()).toContainText("9");
  await conversation.getByRole("button", { name: "Back", exact: true }).click();
  await expect(timelinePost.locator("footer button").first()).toContainText("9");
});
