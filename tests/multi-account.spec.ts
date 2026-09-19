import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";
import type { Page } from "@playwright/test";
export async function setupMulti(page: Page, scenario = "ready") {
  await page.addInitScript(({ scenario }) => {
    const accounts = [
      { id: "b1", provider: "BLUESKY", displayName: "Richard", handle: "richard.test" },
      { id: "b2", provider: "BLUESKY", displayName: "Design Notes", handle: "design.test" },
      { id: "b3", provider: "BLUESKY", displayName: "Web Experiments", handle: "web.test" },
      { id: "m1", provider: "MASTODON", displayName: "Richard", handle: "richard@mastodon.test" },
      { id: "m2", provider: "MASTODON", displayName: "Creative Corner", handle: "creative@mastodon.test" },
    ].map(account => ({ ...account, instanceUrl: account.provider === "BLUESKY" ? "https://bsky.social" : "https://mastodon.test", capabilities: { maxTextLength: account.id === "m2" ? 600 : account.provider === "BLUESKY" ? 300 : 500, maxMediaAttachments: 4, supportedMediaTypes: [] } }));
    let failed = false;
    Object.defineProperty(window, "__TAURI_INTERNALS__", { value: { invoke: async (command: string, args: { accountId?: string; query?: string; post?: { text: string; destinationAccountIds: string[] } }) => {
      if (command === "get_workspace") return { accounts, connectedAccountIds: accounts.map(a => a.id), mode: "LIVE" };
      if (command === "preview_post") return { graphemeCount: args.post?.text.length ?? 0, destinations: accounts.filter(a => args.post?.destinationAccountIds.includes(a.id)).map(a => ({ accountId: a.id, parts: scenario === "variants" && a.id === "m1" ? ["First part", "Second part"] : [args.post?.text ?? ""] })) };
      if (command === "lookup_hashtags") {
        if (scenario === "error" && args.accountId === "m1" && !failed) { failed = true; throw new Error("Search unavailable"); }
        if (args.accountId?.startsWith("b")) return args.query ? [{ name: args.query, activity: { kind: "BLUESKY", matches: 1100 } }] : [];
        return ["dra", "drawing", "drama", "dragon", "drawings", "digitalart", "drafting", "draw"].map((name, index) => ({ name, activity: { kind: "MASTODON", uses: 40 - index, days: 7 } }));
      }
      if (command === "publish_post") {
        sessionStorage.setItem("published-ids", JSON.stringify(args.post?.destinationAccountIds));
        return { canonicalId: "fixture", publications: args.post?.destinationAccountIds.map(accountId => ({ accountId, status: "PUBLISHED", remotePostIds: ["fixture"], error: null })) };
      }
      throw new Error(command);
    } } });
  }, { scenario });
  await page.goto("/");
  await page.getByRole("button", { name: "Composer", exact: true }).click();
}
test("selected accounts drive contextual discovery, grouped previews and dispatch", async ({ page }) => {
  await setupMulti(page);
  await expect(page.locator(".destination-chip")).toHaveCount(5);
  const editor = page.getByLabel("Post text", { exact: true });
  await editor.fill("Shipping a multi-account flow #dra");
  const popover = page.getByRole("listbox", { name: "Hashtag suggestions" });
  await expect(popover).toBeVisible();
  await expect(popover.getByRole("option")).toHaveCount(5);
  await expect(page.locator(".protocol-group")).toHaveCount(2);
  await expect(page.getByRole("button", { name: "Publish to 5 accounts", exact: true })).toBeEnabled();
  await page.getByRole("button", { name: "Bluesky", exact: true }).click();
  await expect(popover.getByRole("option")).toHaveCount(1);
  await page.getByRole("button", { name: "All", exact: true }).click();
  await editor.press("ArrowDown");
  await editor.press("Enter");
  await expect(editor).toHaveValue("Shipping a multi-account flow #drawing");
  await expect(popover).toHaveCount(0);
  await page.getByRole("button", { name: /^Remove destination Creative Corner/ }).click();
  await expect(page.getByRole("button", { name: "Publish to 4 accounts", exact: true })).toBeEnabled();
  await page.getByRole("button", { name: "Publish to 4 accounts", exact: true }).click();
  await expect(page.locator(".dispatch-bar")).toContainText("Published successfully");
  expect(JSON.parse(await page.evaluate(() => sessionStorage.getItem("published-ids")) ?? "[]")).toEqual(["b1", "b2", "b3", "m1"]);
});
test("merged counts deduplicate shared sources and more expands the bounded list", async ({ page }) => {
  await setupMulti(page); await page.getByLabel("Post text", { exact: true }).fill("#dra");
  const option = page.getByRole("option", { name: /^#dra,/ });
  await expect(option).toContainText("≈ 1.1K matches · 40 uses");
  await expect(option.locator(".account-overflow")).toHaveText("+2");
  await page.getByRole("button", { name: "View more hashtag suggestions" }).click();
  await expect(page.getByRole("option")).toHaveCount(8);
  const bounds = await page.locator(".hashtag-popover").boundingBox();
  expect(bounds?.height).toBeLessThanOrEqual(400);
});
test("different same-protocol thread plans remain visible inside one group", async ({ page }) => {
  await setupMulti(page, "variants"); await page.getByLabel("Post text", { exact: true }).fill("A draft");
  const group = page.getByRole("region", { name: "Mastodon previews" });
  await expect(group.locator(".protocol-variant")).toHaveCount(2);
  await expect(group).toContainText("First part");
  await expect(group).toContainText("Second part");
  await expect(page.locator(".protocol-group")).toHaveCount(2);
});
for (const width of [375, 768, 1280, 1672]) test("multi-account layouts and floating states at " + width, async ({ page }) => {
  await page.setViewportSize({ width, height: width === 1672 ? 941 : 900 }); await setupMulti(page);
  if (width === 375) {
    await page.getByRole("button", { name: "Add more", exact: true }).click();
    await page.screenshot({ path: ".omo/evidence/multi-account/identities-375.png" });
    await page.getByRole("button", { name: "Add more", exact: true }).click();
  }
  const editor = page.getByLabel("Post text", { exact: true });
  await editor.fill("Shipping a cleaner multi-account flow today!");
  const before = await editor.boundingBox();
  await page.screenshot({ path: ".omo/evidence/multi-account/composer-" + width + ".png" });
  await editor.fill("Shipping a cleaner multi-account flow today! #dra");
  await expect(page.getByRole("option").first()).toBeVisible();
  expect(await editor.boundingBox()).toEqual(before);
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  await page.screenshot({ path: ".omo/evidence/multi-account/popover-" + width + ".png" });
  await editor.press("Escape");
  await expect(page.locator(".hashtag-popover")).toHaveCount(0);
  await page.getByRole("button", { name: "Accounts & Sync", exact: true }).click();
  await page.screenshot({ path: ".omo/evidence/multi-account/accounts-" + width + ".png" });
});
test("a hashtag before multiline trailing text keeps its anchor", async ({ page }) => {
  await setupMulti(page);
  const editor = page.getByLabel("Post text", { exact: true });
  await editor.fill("Hello #dra");
  await expect(page.getByRole("option").first()).toBeVisible();
  const before = await page.locator(".hashtag-popover").boundingBox();
  await editor.fill("Hello #dra\nFollowing text on another line");
  await editor.press("Control+Home");
  for (let i = 0; i < 10; i++) await editor.press("ArrowRight");
  await expect(page.getByRole("option").first()).toBeVisible();
  await expect.poll(async () => (await page.locator(".hashtag-popover").boundingBox())?.x).toBe(before?.x);
});
test("IME and modifier keys preserve ordinary text editing", async ({ page }) => {
  await setupMulti(page);
  const editor = page.getByLabel("Post text", { exact: true }); await editor.fill("#dra");
  await expect(page.getByRole("option").first()).toBeVisible();
  await editor.press("Shift+Enter"); await expect(editor).toHaveValue("#dra\n");
  await editor.fill("#日本");
  await editor.dispatchEvent("compositionstart");
  await expect(page.locator(".hashtag-popover")).toHaveCount(0);
  await editor.dispatchEvent("compositionend");
  await expect(page.getByRole("option").first()).toBeVisible();
  await editor.press("Escape"); await expect(editor).toHaveValue("#日本");
});
test("keyboard users can reach source filters from the editor", async ({ page }) => {
  await setupMulti(page); const editor = page.getByLabel("Post text", { exact: true }); await editor.fill("#dra");
  await expect(page.getByRole("option").first()).toBeVisible();
  await editor.press("Tab"); await expect(page.getByRole("button", { name: "All", exact: true })).toBeFocused();
  await page.keyboard.press("Tab"); await page.keyboard.press("Enter");
  await expect(page.getByRole("button", { name: "Bluesky", exact: true })).toHaveAttribute("aria-pressed", "true");
  await expect(editor).toBeFocused(); await expect(page.getByRole("option")).toHaveCount(1);
});
test("filters, expansion and no selection remain one keyboard workflow", async ({ page }) => {
  await setupMulti(page); const editor = page.getByLabel("Post text", { exact: true }); await editor.fill("#dra");
  await expect(page.getByRole("option").first()).toBeVisible();
  await page.getByRole("button", { name: "Bluesky", exact: true }).click();
  await page.screenshot({ path: ".omo/evidence/multi-account/filter-bluesky.png" });
  await page.getByRole("button", { name: "Mastodon", exact: true }).click();
  await expect(page.getByRole("option")).toHaveCount(5);
  await page.screenshot({ path: ".omo/evidence/multi-account/filter-mastodon.png" });
  await page.getByRole("button", { name: "View more hashtag suggestions" }).click();
  await page.screenshot({ path: ".omo/evidence/multi-account/expanded.png" });
  await editor.press("ArrowUp"); await expect(page.getByRole("option", { selected: true })).toContainText("#draw");
  await editor.press("Enter"); await expect(editor).toHaveValue("#draw");
  await page.getByRole("button", { name: "Add more", exact: true }).click();
  for (const checkbox of await page.locator(".destination-options").getByRole("checkbox").all()) await checkbox.uncheck();
  await page.keyboard.press("Escape");
  await expect(page.getByRole("button", { name: "Publish to 0 accounts", exact: true })).toBeDisabled();
  await expect(page.locator(".protocol-group")).toHaveCount(0);
  await editor.fill("#dra");
  await expect(page.getByText("Select an account for this network.")).toBeVisible();
  await page.screenshot({ path: ".omo/evidence/multi-account/no-selection.png" });
});
test("popover follows editor scrolling and viewport resize", async ({ page }) => {
  await setupMulti(page); const editor = page.getByLabel("Post text", { exact: true });
  await editor.fill("A long draft line\n".repeat(18) + "#dra");
  await editor.press("Control+End"); await expect(page.getByRole("option").first()).toBeVisible();
  await page.screenshot({ path: ".omo/evidence/multi-account/scrolled-editor.png" });
  await page.setViewportSize({ width: 768, height: 900 });
  const popup = page.locator(".hashtag-popover"); await expect(popup).toBeVisible();
  await expect.poll(async () => { const box = await popup.boundingBox(); return box ? box.x >= 8 && box.x + box.width <= 760 && box.y >= 8 && box.y + box.height <= 892 : false; }).toBe(true);
  await page.screenshot({ path: ".omo/evidence/multi-account/resized-popover.png" });
});

test("multi-account composer and open autocomplete are accessible", async ({ page }) => {
  test.setTimeout(60000);
  await setupMulti(page);
  await page.getByLabel("Post text", { exact: true }).fill("#dra");
  await expect(page.getByRole("option").first()).toBeVisible();
  const result = await new AxeBuilder({ page }).withTags(["wcag2a", "wcag2aa", "wcag21aa"]).analyze();
  expect(result.violations).toEqual([]);
});
