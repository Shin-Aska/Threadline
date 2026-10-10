import { expect, test } from "@playwright/test";
import { Buffer } from "node:buffer";

test("image viewer keeps close visible and reveals long descriptions", async ({ page }) => {
  await page.goto("/tests/fixtures/complete-workspace.html");
  const thumbnail = page.locator(".unified-post .post-media button").first();
  const alt = await thumbnail.locator("img").getAttribute("alt");
  await thumbnail.click();

  const viewer = page.getByRole("dialog", { name: "Image viewer" });
  const close = viewer.getByRole("button", { name: "Close image viewer" });
  const description = viewer.getByRole("button", { name: "Description", exact: true });
  const panel = viewer.getByRole("region", { name: "Image description" });
  for (const width of [1280, 375]) {
    await page.setViewportSize({ width, height: 900 });
    await expect(close).toBeInViewport();
    await expect(description).toBeInViewport();
    await description.click();
    await expect(panel).toBeVisible();
    await expect(panel.locator("p")).toHaveText(alt ?? "");
    await page.keyboard.press("Escape");
    await expect(panel).toBeHidden();
    await expect(viewer).toBeVisible();
    await expect(description).toBeFocused();
  }

  await description.click();
  await panel.getByRole("button", { name: "Hide description" }).click();
  await expect(panel).toBeHidden();
  await expect(description).toBeFocused();
  await page.keyboard.press("Escape");
  await expect(viewer).toHaveCount(0);
  await expect(thumbnail).toBeFocused();
});

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
  const navigation = page.getByRole("button", { name: "Notifications", exact: true });
  await expect(navigation.locator(".connected-count")).toHaveText("2");
  await expect(navigation).toHaveAccessibleDescription("2 unread notifications");
  await page.getByRole("button", { name: "Notifications", exact: true }).click();
  await expect(page.locator(".notification-row")).toHaveCount(3);
  await expect(page.locator(".notification-row").filter({ hasText: "Clara Chen" })).toContainText("Content warning: Story spoilers");
  await expect(page.locator(".notification-row").filter({ hasText: "Clara Chen" })).not.toContainText("Finally got this handheld working again");
  await page.getByRole("button", { name: "Load more" }).click();
  await expect(page.locator(".notification-row")).toHaveCount(5);
  await expect(page.getByText(/Updated Jules Park/)).toBeVisible();
  await expect(page.getByText(/Updated Clara Chen/)).toBeVisible();
  await expect(page.locator(".notification-row.unread").filter({ hasText: "Updated" })).toHaveCount(0);
  await expect(navigation.locator(".connected-count")).toHaveText("2");
  await page.getByRole("button", { name: "Mark all read" }).click();
  await expect(page.locator(".notification-row.unread")).toHaveCount(0);
  await expect(navigation.locator(".connected-count")).toHaveCount(0);
  await expect(navigation).toHaveAccessibleDescription("0 unread notifications");
  await page.getByRole("button", { name: "Timeline", exact: true }).click();
  await expect(navigation.locator(".connected-count")).toHaveCount(0);
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

test("Mastodon content warnings hide post text and sensitive media until revealed", async ({ page }, testInfo) => {
  await page.addInitScript(() => {
    const capabilities = { maxTextLength: 500, maxMediaAttachments: 4, supportedMediaTypes: ["image/png"], countingPolicy: "GRAPHEME", reservedUrlLength: 23, supportsPolls: true, supportsContentWarnings: true };
    const account = { id: "reader", provider: "MASTODON", handle: "reader@social.test", displayName: "Reader", instanceUrl: "https://social.test", did: null, capabilities };
    const post = { canonicalKey: "MASTODON:42", provider: "MASTODON", remoteId: "42", remoteCid: null, remoteUrl: "https://social.test/@alice/42", author: { id: "alice", displayName: "Alice", handle: "alice", avatarUrl: null }, text: "The hidden ending", contentWarning: "Story spoilers", sensitive: true, createdAt: "2026-09-20T08:00:00Z", media: [{ url: "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg'/%3E", alt: "Ending illustration", mediaType: "image" }], metrics: { replies: 0, reposts: 0, likes: 0 }, viewer: { liked: false, reposted: false, likeUri: null, repostUri: null }, replyParentId: null, replyRootId: null, replyRootCid: null };
    Object.defineProperty(window, "__TAURI_INTERNALS__", { value: { invoke: async (command: string) => {
      if (command === "get_workspace") return { accounts: [account], connectedAccountIds: [account.id], mode: "LIVE" };
      if (command === "get_home_feed") return { posts: [post], cursor: null };
      throw new Error(`Unexpected command ${command}`);
    } } });
  });
  await page.goto("/");
  const card = page.locator(".unified-post").first();
  await expect(card.getByText("Story spoilers")).toBeVisible();
  await expect(card.getByText("The hidden ending")).toHaveCount(0);
  await expect(card.locator(".post-media img")).toHaveCount(0);
  await page.screenshot({ path: testInfo.outputPath("warning-collapsed-desktop.png") });
  const toggle = card.getByRole("button", { name: "Show post" });
  await toggle.click();
  await expect(card.getByText("The hidden ending")).toBeVisible();
  await expect(card.locator(".post-media img")).toHaveCount(1);
  await page.screenshot({ path: testInfo.outputPath("warning-expanded-desktop.png") });
  await expect(card.getByRole("button", { name: "Hide post" })).toHaveAttribute("aria-expanded", "true");
  await card.getByRole("button", { name: "Hide post" }).click();
  await expect(card.getByText("The hidden ending")).toHaveCount(0);
  await page.setViewportSize({ width: 375, height: 812 });
  await expect(card.getByRole("button", { name: "Show post" })).toBeInViewport();
  await page.screenshot({ path: testInfo.outputPath("warning-collapsed-phone.png") });
});

test("Bluesky video posts expose an HLS player with its thumbnail and alt text", async ({ page }, testInfo) => {
  let releasePlaylist: () => void = () => {};
  const playlistGate = new Promise<void>(resolve => { releasePlaylist = resolve; });
  await page.route("https://video.bsky.test/clip.m3u8", async route => {
    await playlistGate;
    await route.fulfill({
      contentType: "application/vnd.apple.mpegurl",
      headers: { "access-control-allow-origin": "*" },
      body: "#EXTM3U\n#EXT-X-VERSION:3\n#EXT-X-TARGETDURATION:2\n#EXT-X-MEDIA-SEQUENCE:0\n#EXT-X-ENDLIST\n",
    });
  });
  await page.addInitScript(() => {
    const capabilities = { maxTextLength: 300, maxMediaAttachments: 4, supportedMediaTypes: ["image/png", "video/mp4"], countingPolicy: "GRAPHEME", reservedUrlLength: 23, supportsPolls: false, supportsContentWarnings: false };
    const account = { id: "reader", provider: "BLUESKY", handle: "reader.bsky.social", displayName: "Reader", instanceUrl: "https://bsky.social", did: "did:plc:reader", capabilities };
    const thumbnail = "data:image/svg+xml," + encodeURIComponent("<svg xmlns='http://www.w3.org/2000/svg' width='640' height='360'><rect width='640' height='360' fill='#264779'/><circle cx='320' cy='180' r='75' fill='#87a9e9'/></svg>");
    const post = { canonicalKey: "BLUESKY:video", provider: "BLUESKY", remoteId: "at://did:plc:alice/app.bsky.feed.post/video", remoteCid: "video-cid", remoteUrl: "https://bsky.app/profile/alice.bsky.social/post/video", author: { id: "did:plc:alice", displayName: "Alice", handle: "alice.bsky.social", avatarUrl: null }, text: "A clip", contentWarning: null, sensitive: false, createdAt: "2026-09-20T08:00:00Z", media: [{ url: "https://video.bsky.test/clip.m3u8", alt: "A small dog playing", mediaType: "video/hls", thumbnail }], metrics: { replies: 0, reposts: 0, likes: 0 }, viewer: { liked: false, reposted: false, likeUri: null, repostUri: null }, replyParentId: null, replyRootId: null, replyRootCid: null };
    Object.defineProperty(window, "__TAURI_INTERNALS__", { value: { invoke: async (command: string) => {
      if (command === "get_workspace") return { accounts: [account], connectedAccountIds: [account.id], mode: "LIVE" };
      if (command === "get_home_feed") return { posts: [post], cursor: null };
      throw new Error(`Unexpected command ${command}`);
    } } });
  });
  const playlist = page.waitForRequest("https://video.bsky.test/clip.m3u8");
  await page.goto("/");
  const card = page.locator(".unified-post").first();
  const video = card.locator("video[aria-label=\"A small dog playing\"]");
  await expect(video).toBeVisible();
  await expect(video).toHaveAttribute("poster", /data:image\/svg\+xml/);
  await page.screenshot({ path: testInfo.outputPath("bluesky-video-desktop.png") });
  await page.setViewportSize({ width: 375, height: 812 });
  await expect(video).toBeInViewport();
  await page.screenshot({ path: testInfo.outputPath("bluesky-video-phone.png") });
  releasePlaylist();
  await playlist;
  const fallback = card.getByRole("link", { name: "Video playback unavailable. Open original post" });
  await expect(fallback).toBeVisible();
  await fallback.click();
  await expect(page.getByRole("dialog", { name: "Open an external link?" })).toContainText("bsky.app");
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
