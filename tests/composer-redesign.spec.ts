import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";
import type { Page } from "@playwright/test";
import type { Account, CanonicalPost } from "../src/types";

const draftText = "A quiet Saturday, a hot coffee, and a list of ideas.\n\nGrateful for this corner of the internet and the people who make it feel like home.";
const writer: Account = {
  id: "bsky", provider: "BLUESKY", displayName: "Alex Rivera", handle: "alexrivera.bsky.social", instanceUrl: "https://bsky.social", did: null,
  capabilities: { maxTextLength: 300, countingPolicy: "GRAPHEME", reservedUrlLength: null, maxMediaAttachments: 4, supportedMediaTypes: [], supportsPolls: false, supportsContentWarnings: false },
};
async function setup(page: Page, connectedIds: readonly string[] = ["bsky", "masto"]) {
  await page.addInitScript(({ writer, connectedIds }) => {
    let accounts: Account[] = [writer, { ...writer, id: "masto", provider: "MASTODON", handle: "alexrivera@mastodon.social", instanceUrl: "https://mastodon.social" }, { ...writer, id: "archive", displayName: "Archive", handle: "archive.bsky.social" }];
    let connectedAccountIds = [...connectedIds];
    Object.defineProperty(window, "__TAURI_INTERNALS__", { value: { invoke: async (command: string, args: { accountId?: string; post?: CanonicalPost }) => {
      if (command === "get_workspace") return { accounts, connectedAccountIds, mode: connectedAccountIds.length ? "LIVE" : "DISCONNECTED" };
      if (command === "get_timeline") return { posts: [], cursor: null };
      if (command === "get_discovery") return { topics: [], suggestedAccounts: [], popularPosts: [] };
      if (command === "get_following_sources") return [];
      if (command === "preview_post") return { graphemeCount: args.post?.text.length ?? 0, destinations: args.post?.destinationAccountIds.map(accountId => ({ accountId, parts: [args.post?.text ?? ""] })) };
      if (command === "remove_account") { accounts = accounts.filter(account => account.id !== args.accountId); connectedAccountIds = connectedAccountIds.filter(id => id !== args.accountId); return; }
      if (command === "connect_bluesky") { connectedAccountIds = [...connectedAccountIds, "archive"]; return accounts.find(account => account.id === "archive"); }
      throw new Error(`Unexpected command ${command}`);
    } } });
  }, { writer, connectedIds });
  await page.goto("/");
}

test("connected badge follows workspace connections, not post selection", async ({ page }) => {
  await setup(page);
  const navigation = page.getByRole("button", { name: "Accounts & Sync", exact: true });
  await expect(page.getByRole("heading", { name: "Timeline", exact: true })).toBeVisible();
  await expect(navigation).toHaveAccessibleDescription("2 connected accounts");
  await expect(navigation.locator(".connected-count")).toHaveText("2");
  await expect(page.locator(".sidebar").getByRole("checkbox")).toHaveCount(0);
  await page.getByRole("button", { name: "Composer", exact: true }).click();
  await page.getByRole("button", { name: /^Remove destination Alex Rivera · Bluesky/ }).click();
  await expect(navigation.locator(".connected-count")).toHaveText("2");
  await expect(page.locator(".destination-chip")).toHaveCount(1);
  await navigation.click();
  await page.getByRole("button", { name: "Remove alexrivera.bsky.social", exact: true }).click();
  await expect(navigation).toHaveAccessibleDescription("1 connected account");
  await expect(navigation.locator(".connected-count")).toHaveText("1");
  await page.getByRole("button", { name: "Reconnect archive.bsky.social", exact: true }).click();
  await page.getByLabel("App password", { exact: true }).fill("test-only-password");
  await page.getByRole("button", { name: "Connect account", exact: true }).click();
  await expect(navigation).toHaveAccessibleDescription("2 connected accounts");
});

test("destination picker supports removal, reselection, Escape and disconnected accounts", async ({ page }) => {
  await setup(page);
  await page.getByRole("button", { name: "Composer", exact: true }).click();
  const add = page.getByRole("button", { name: "Add more", exact: true });
  await page.getByRole("button", { name: /^Remove destination Alex Rivera · Bluesky/ }).click();
  await expect(add).toBeFocused();
  await add.press("Enter");
  await page.getByRole("checkbox", { name: "Alex Rivera · Bluesky · alexrivera.bsky.social", exact: true }).check();
  await page.keyboard.press("Escape");
  await expect(add).toBeFocused();
  await expect(add).toHaveAttribute("aria-expanded", "false");
  await expect(page.locator(".destination-chip")).toHaveCount(2);
  await add.click();
  await page.getByRole("checkbox", { name: "Archive · Bluesky · archive.bsky.social", exact: true }).check();
  await page.getByLabel("Post text", { exact: true }).fill(draftText);
  await expect(add).toHaveAttribute("aria-expanded", "false");
  await expect(page.getByText("Some selected accounts need reconnecting.", { exact: false })).toBeVisible();
  await expect(page.getByRole("button", { name: "Publish to 3 accounts", exact: true })).toBeDisabled();
  await page.getByRole("button", { name: /^Remove destination Archive/ }).click();
  await expect(page.getByRole("button", { name: "Publish to 2 accounts", exact: true })).toBeEnabled();
  await page.getByRole("button", { name: "Timeline", exact: true }).click();
  await page.getByRole("button", { name: "Composer", exact: true }).click();
  await expect(page.getByLabel("Post text", { exact: true })).toHaveValue(draftText);
});

test("saved disconnected accounts show a zero connected badge", async ({ page }) => {
  await setup(page, []);
  const navigation = page.getByRole("button", { name: "Accounts & Sync", exact: true });
  await expect(navigation).toHaveAccessibleDescription("0 connected accounts");
  await expect(navigation.locator(".connected-count")).toHaveText("0");
  await page.getByRole("button", { name: "Composer", exact: true }).click();
  await expect(page.locator(".destination-chip")).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Publish to 0 accounts", exact: true })).toBeDisabled();
});

for (const width of [375, 768, 1280, 1536]) test(`approved Composer layout and destination picker at ${width}px`, async ({ page }) => {
  await page.setViewportSize({ width, height: 1024 });
  await setup(page);
  await page.getByRole("button", { name: "Composer", exact: true }).click();
  await page.getByLabel("Post text", { exact: true }).fill(draftText);
  await expect(page.getByRole("button", { name: "Publish to 2 accounts", exact: true })).toBeEnabled();
  await page.getByLabel("Post text", { exact: true }).blur();
  await page.screenshot({ path: `.omo/evidence/composer-redesign/composer-${width}.png`, fullPage: true });
  await page.getByRole("button", { name: "Add more", exact: true }).click();
  await expect(page.getByRole("checkbox")).toHaveCount(3);
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  await page.screenshot({ path: `.omo/evidence/composer-redesign/destinations-${width}.png`, fullPage: true });
  await page.keyboard.press("Escape");
  for (const name of ["Timeline", "Discover", "Following", "Accounts & Sync"]) {
    await page.getByRole("button", { name, exact: true }).click();
    await page.screenshot({ path: `.omo/evidence/composer-redesign/${name.split(" ")[0]?.toLowerCase()}-${width}.png`, fullPage: true });
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  }
});

test("destination picker and empty composer meet accessibility checks", async ({ page }) => {
  test.setTimeout(60000);
  await setup(page);
  await page.getByRole("button", { name: "Composer", exact: true }).click();
  await page.getByRole("button", { name: "Add more", exact: true }).click();
  expect((await new AxeBuilder({ page }).withTags(["wcag2a", "wcag2aa", "wcag21aa"]).analyze()).violations).toEqual([]);
});
