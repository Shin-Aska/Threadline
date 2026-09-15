import { expect, test } from "@playwright/test";
import type { Page } from "@playwright/test";
import { activeHashtag } from "../src/hooks/useHashtags";
async function setup(page: Page, scenario = "ready") {
  await page.addInitScript(({ scenario }) => {
    let failed = false;
    const mastodon = { id: "masto", provider: "MASTODON", displayName: "Writer", handle: "writer@test.invalid", instanceUrl: "https://test.invalid", capabilities: { maxTextLength: 500, maxMediaAttachments: 4, supportedMediaTypes: [] } };
    const bluesky = { ...mastodon, id: "bsky", provider: "BLUESKY", handle: "writer.bsky.social", instanceUrl: "https://bsky.social" };
    Object.defineProperty(window, "__TAURI_INTERNALS__", { value: { invoke: async (command: string, args: { query?: string; accountId?: string; post?: { text: string } }) => {
      if (command === "get_workspace") return { accounts: [mastodon, bluesky], connectedAccountIds: ["masto", "bsky"], mode: "LIVE" };
      if (command === "preview_post") return { graphemeCount: 0, destinations: [{ accountId: "masto", parts: [args.post?.text ?? ""] }, { accountId: "bsky", parts: [args.post?.text ?? ""] }] };
      if (command === "lookup_hashtags") {
        const countKey = "hashtag-requests-" + args.accountId;
        sessionStorage.setItem(countKey, String(Number(sessionStorage.getItem(countKey) ?? "0") + 1));
        if (scenario === "stale" && args.query === "old") {
          sessionStorage.setItem("old-request-started", "true");
          await new Promise<void>(resolve => window.addEventListener("release-old", () => resolve(), { once: true }));
        }
        if (scenario === "error" && !failed && args.accountId === "masto") { failed = true; throw new Error("Hashtag service unavailable"); }
        if (scenario === "empty") return [];
        if (args.accountId === "bsky") return args.query ? [{ name: args.query, activity: { kind: "BLUESKY", matches: scenario === "unknown" ? null : 1250 } }] : [];
        return [{ name: args.query === "old" ? "OldTag" : "RustLang", activity: { kind: "MASTODON", uses: 42, days: 7 } }, { name: "Rust", activity: { kind: "UNAVAILABLE" } }];
      }
      throw new Error(command);
    } } });
  }, { scenario });
  await page.goto("/");
}
async function capture(page: Page, name: string) {
  await page.screenshot({ path: ".omo/evidence/hashtags/" + name + ".png" });
}
test("selecting a hashtag completion preserves text after the caret", async ({ page }) => {
  await setup(page);
  const editor = page.getByLabel("Post text", { exact: true });
  await editor.fill("Hello #ru and friends");
  await editor.press("Home");
  for (let i = 0; i < 9; i++) await editor.press("ArrowRight");
  await page.getByRole("option", { name: /^#RustLang,/ }).click();
  await expect(editor).toHaveValue("Hello #RustLang and friends");
  await expect(editor).toBeFocused();
  await expect(page.locator(".hashtag-popover")).toHaveCount(0);
});
test("keyboard selection and Escape preserve normal editor behavior", async ({ page }) => {
  await setup(page);
  const editor = page.getByLabel("Post text", { exact: true });
  await editor.fill("#ru");
  const suggestion = page.getByRole("option", { name: /^#RustLang,/ });
  await expect(suggestion).toBeVisible();
  await editor.press("Enter");
  await expect(editor).toHaveValue("#RustLang");
  await editor.fill("#ca");
  await expect(page.locator(".hashtag-popover")).toBeVisible();
  await editor.press("Escape");
  await expect(page.locator(".hashtag-popover")).toHaveCount(0);
  await expect(editor).toHaveValue("#ca");
});
test("outdated lookup responses cannot replace current hashtag results", async ({ page }) => {
  await setup(page, "stale");
  const editor = page.getByLabel("Post text", { exact: true });
  await editor.fill("#old");
  await expect.poll(() => page.evaluate(() => sessionStorage.getItem("old-request-started"))).toBe("true");
  await capture(page, "loading");
  await editor.fill("#ru");
  await expect(page.getByRole("option", { name: /^#RustLang,/ })).toBeVisible();
  await page.evaluate(() => window.dispatchEvent(new Event("release-old")));
  await expect(page.getByRole("option", { name: /^#OldTag,/ })).toHaveCount(0);
  await expect(page.getByRole("option", { name: /^#ru,/ })).toBeVisible();
});
test("provider failure is isolated and retry keeps the draft", async ({ page }) => {
  await setup(page, "error");
  const editor = page.getByLabel("Post text", { exact: true }); await editor.fill("#ru");
  await expect(page.getByText("Writer: Hashtag service unavailable", { exact: true })).toBeVisible();
  await expect(page.getByRole("option", { name: /^#ru,/ })).toBeVisible();
  await capture(page, "provider-error");
  const successfulOption = await page.getByRole("option", { name: /^#ru,/ }).elementHandle();
  await page.getByRole("button", { name: /Retry hashtags/ }).click();
  await expect(page.getByRole("option", { name: /^#RustLang,/ })).toBeVisible();
  expect(await successfulOption?.evaluate(element => element.isConnected)).toBe(true);
  expect(await page.evaluate(() => sessionStorage.getItem("hashtag-requests-bsky"))).toBe("1");
  expect(await page.evaluate(() => sessionStorage.getItem("hashtag-requests-masto"))).toBe("2");
  await expect(editor).toHaveValue("#ru");
});
test("empty results and unknown counts remain distinct", async ({ page }) => {
  await setup(page, "empty");
  await page.getByLabel("Post text", { exact: true }).fill("#unknown");
  await expect(page.getByText("No matching hashtags found.")).toHaveCount(1);
  await capture(page, "empty");
});
test("missing Bluesky statistics are not displayed as zero", async ({ page }) => {
  await setup(page, "unknown");
  await page.getByLabel("Post text", { exact: true }).fill("#ru");
  await expect(page.getByRole("option", { name: /^#ru,/ }).getByText("Count unavailable", { exact: true })).toBeVisible();
  await capture(page, "unknown-count");
});
for (const width of [375, 768, 1280]) test("hashtag activity and trends at " + width + "px", async ({ page }) => {
  await page.setViewportSize({ width, height: 1000 }); await setup(page);
  await page.getByLabel("Post text", { exact: true }).fill("Building with #ru");
  await expect(page.getByRole("option", { name: /^#RustLang,/ })).toBeVisible();
  await expect(page.getByText("≈ 1.3K matches", { exact: true })).toBeVisible();
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  await capture(page, "suggestions-" + width);
  await page.getByLabel("Post text", { exact: true }).fill("#");
  await expect(page.getByRole("listbox", { name: "Hashtag suggestions" })).toBeVisible();
  await expect(page.getByRole("option", { name: /^#RustLang,/ })).toBeVisible();
  await capture(page, "trends-" + width);
});
test("hashtag detection respects URLs, selections, Unicode and the caret", () => {
  expect(activeHashtag("https://example.test/#ru", 24, 24)).toBeNull();
  expect(activeHashtag("#rust", 1, 5)).toBeNull();
  expect(activeHashtag("🧵 #日本語", 6, 6)).toEqual({ start: 3, end: 7, query: "日本" });
  expect(activeHashtag("Hi #rust now", 6, 6)).toEqual({ start: 3, end: 8, query: "ru" });
});
