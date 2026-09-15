import { expect, test } from "@playwright/test";
import type { Page } from "@playwright/test";
import type { Account, CanonicalPost } from "../src/types";
const connected: Account = {
  id: "bsky-did:plc:qa", provider: "BLUESKY", displayName: "Test writer", handle: "writer.bsky.social", instanceUrl: null, did: null,
  capabilities: { maxTextLength: 300, countingPolicy: "GRAPHEME", reservedUrlLength: null, maxMediaAttachments: 4, supportedMediaTypes: [], supportsPolls: false, supportsContentWarnings: false },
};
async function installDesktop(page: Page, scenario: "connect" | "ready" | "preview-error" | "disconnected" | "publish-error" | "custom-service" | "reconnect-preview" | "connect-error" | "workspace-error" | "publish-slow" | "publish-failed" | "publish-partial" | "publish-partial-thread") {
  await page.addInitScript(({ connected, scenario }) => {
    let accounts = scenario === "connect" || scenario === "connect-error" ? [] : [scenario === "custom-service" ? { ...connected, instanceUrl: "https://custom-pds.example" } : connected];
    if (scenario === "publish-partial" || scenario === "publish-partial-thread") accounts.push({ ...connected, id: "bsky-second", handle: "second.bsky.social" });
    let publishAttempts = 0;
    let live = scenario !== "disconnected" && scenario !== "custom-service";
    let previewAttempts = 0; let workspaceUnavailable = scenario === "workspace-error";
    window.addEventListener("qa-workspace-recovered", () => { workspaceUnavailable = false; });
    Object.defineProperty(window, "__TAURI_INTERNALS__", { value: {
      invoke: async (command: string, args: { post?: CanonicalPost; accountId?: string } = {}) => {
        if (command === "list_accounts") return accounts;
        if (command === "get_workspace") {
          if (workspaceUnavailable) throw new Error("Workspace temporarily unavailable");
          return { accounts, connectedAccountIds: live ? accounts.map(account => account.id) : [], mode: live && accounts.length ? "LIVE" : "DISCONNECTED" };
        }
        if (command === "connect_bluesky" || command === "connect_mastodon") {
          if (scenario === "connect-error") throw new Error("Credentials could not be verified");
          accounts = [connected]; live = true; return connected;
        }
        if (command === "remove_account") { accounts = accounts.filter(account => account.id !== args.accountId); return; }
        if (command === "preview_post") {
          previewAttempts += 1;
          if (scenario === "preview-error" && previewAttempts === 1) throw new Error("Planning is temporarily unavailable");
          if (scenario === "reconnect-preview" && previewAttempts === 2) throw new Error("Refreshed planning unavailable");
          const post = args.post;
          if (!post) throw new Error("Missing post");
          return { graphemeCount: post.text.length, effectiveLimit: null, limitingAccountId: null, destinations: post.destinationAccountIds.map(accountId => ({ accountId, label: "Bluesky", maxLength: 300, parts: [post.text] })) };
        }
        if (command === "publish_post") {
          localStorage.setItem("qa-publish-count", String(++publishAttempts));
          if (scenario === "publish-slow") await new Promise<void>(resolve => window.addEventListener("qa-release-publish", () => resolve(), { once: true }));
          if (scenario === "publish-failed" || scenario === "publish-partial" || scenario === "publish-partial-thread") return { canonicalId: "qa-failed", publications: accounts.map((account, index) => ({ accountId: account.id, status: scenario !== "publish-failed" && index === 0 ? "PUBLISHED" : "FAILED", remotePostIds: scenario === "publish-partial-thread" || (scenario === "publish-partial" && index === 0) ? ["test:1"] : [], error: index === 0 && scenario !== "publish-failed" ? null : "Provider unavailable" })) };
          if (scenario === "publish-error") throw new Error("Provider rejected publication");
          return { canonicalId: "qa-publication", publications: accounts.map(account => ({ accountId: account.id, status: "PUBLISHED", remotePostIds: ["test:1"], error: null })) };
        }
        throw new Error(`Unexpected command: ${command}`);
      },
    } });
  }, { connected, scenario });
}
async function draft(page: Page) {
  await page.goto("/");
  await page.getByRole("textbox", { name: "Post text" }).fill("A post for the test workspace.");
}

test("fresh browser opens setup without sample accounts or composer", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByRole("heading", { name: "Connect your first account" })).toBeVisible();
  await expect(page.getByRole("textbox", { name: "Post text" })).toHaveCount(0);
  await expect(page.getByRole("navigation")).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Connect account" })).toBeDisabled();
  await page.getByRole("button", { name: "Mastodon ActivityPub" }).click();
  await expect(page.getByLabel("Instance URL")).toHaveValue("https://mastodon.social");
  await page.getByLabel("Instance URL").fill("");
  await page.getByLabel("Instance URL").pressSequentially("https://m");
  await expect(page.getByLabel("Instance URL")).toHaveValue("https://m");
  await expect(page.getByLabel("Access token", { exact: true })).toBeVisible();
});

test("first real connection opens the composer with a selected target and empty draft", async ({ page }) => {
  await installDesktop(page, "connect"); await page.goto("/");
  await expect(page.getByRole("heading", { name: "Connect your first account" })).toBeVisible();
  await page.getByLabel("Handle", { exact: true }).fill("writer.bsky.social");
  await page.getByLabel("App password", { exact: true }).fill("test-only-password");
  await page.getByRole("button", { name: "Connect account" }).click();
  await expect(page.getByRole("textbox", { name: "Post text" })).toHaveValue("");
  await expect(page.locator(".identity-choice")).toHaveCount(1);
  await expect(page.getByRole("checkbox")).toBeChecked();
  await expect(page.getByRole("radio", { name: /Common limit/i })).toBeChecked();
  await expect(page.getByRole("button", { name: "Publish to 1 account" })).toBeDisabled();
});

test("failed first connection stays in setup and preserves public details", async ({ page }) => {
  await installDesktop(page, "connect-error"); await page.goto("/");
  await page.getByLabel("Handle", { exact: true }).fill("writer.bsky.social");
  await page.getByLabel("App password", { exact: true }).fill("test-only-password");
  await page.getByRole("button", { name: "Connect account" }).click();
  await expect(page.getByText("Credentials could not be verified", { exact: true })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Connect your first account" })).toBeVisible();
  await expect(page.getByLabel("Handle", { exact: true })).toHaveValue("writer.bsky.social");
});

test("failed preview can be retried and prevents premature publication", async ({ page }) => {
  await installDesktop(page, "preview-error"); await draft(page);
  await expect(page.getByText("Planning is temporarily unavailable", { exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: "Publish to 1 account" })).toBeDisabled();
  await page.getByRole("button", { name: "Retry preview" }).click();
  await expect(page.getByRole("button", { name: "Publish to 1 account" })).toBeEnabled();
});

test("custom service is preserved for reconnect", async ({ page }) => {
  await installDesktop(page, "custom-service"); await page.goto("/");
  await page.getByRole("button", { name: "Accounts & Sync", exact: true }).click();
  await page.getByRole("button", { name: "Reconnect writer.bsky.social" }).click();
  await expect(page.getByLabel("Service URL", { exact: true })).toHaveValue("https://custom-pds.example");
  await expect(page.getByLabel("App password", { exact: true })).toBeFocused();
});

test("legacy identity without a service URL cannot reconnect to a guessed endpoint", async ({ page }) => {
  await installDesktop(page, "disconnected"); await page.goto("/");
  await page.getByRole("button", { name: "Accounts & Sync", exact: true }).click();
  await page.getByRole("button", { name: "Reconnect writer.bsky.social" }).click();
  await expect(page.getByLabel("Service URL", { exact: true })).toHaveValue("");
  await page.getByLabel("App password", { exact: true }).fill("test-only-password");
  await expect(page.getByRole("button", { name: "Connect account" })).toBeDisabled();
});

test("publication errors survive editing and replanning without losing the draft", async ({ page }) => {
  await installDesktop(page, "publish-error"); await draft(page);
  await page.getByRole("button", { name: "Publish to 1 account" }).click();
  await expect(page.getByText("Provider rejected publication", { exact: true })).toBeVisible();
  await page.getByRole("textbox", { name: "Post text" }).fill("Keep this edited draft after the failure.");
  await expect(page.getByRole("button", { name: "Publish to 1 account" })).toBeEnabled();
  await expect(page.getByText("Provider rejected publication", { exact: true })).toBeVisible();
});

test("successful publication clears the draft and prevents accidental resubmission", async ({ page }) => {
  await installDesktop(page, "ready"); await draft(page);
  await page.getByRole("button", { name: "Publish to 1 account" }).click();
  await expect(page.getByRole("textbox", { name: "Post text" })).toHaveValue("");
  await expect(page.locator(".dispatch-bar")).toContainText("Published successfully");
  await expect(page.getByRole("button", { name: "Publish to 1 account" })).toBeDisabled();
  await expect.poll(() => page.evaluate(() => localStorage.getItem("qa-publish-count"))).toBe("1");
  await page.getByRole("textbox", { name: "Post text" }).fill("An intentional new post.");
  await page.getByRole("button", { name: "Publish to 1 account" }).click();
  await expect(page.getByRole("textbox", { name: "Post text" })).toHaveValue("");
  await expect.poll(() => page.evaluate(() => localStorage.getItem("qa-publish-count"))).toBe("2");
});

test("rapid publish clicks send only once while publication is pending", async ({ page }) => {
  await installDesktop(page, "publish-slow"); await draft(page);
  const button = page.getByRole("button", { name: "Publish to 1 account" });
  await expect(button).toBeEnabled();
  await button.evaluate(element => { if (!(element instanceof HTMLButtonElement)) throw new Error("Expected publish button"); element.click(); element.click(); });
  await expect(page.getByRole("button", { name: "Publishing…", exact: true })).toBeDisabled();
  await expect(page.getByRole("textbox", { name: "Post text" })).toBeDisabled();
  await expect.poll(() => page.evaluate(() => localStorage.getItem("qa-publish-count"))).toBe("1");
  await page.evaluate(() => window.dispatchEvent(new Event("qa-release-publish")));
  await expect(page.getByRole("textbox", { name: "Post text" })).toHaveValue("");
});

test("failed publication preserves the draft and reports failure", async ({ page }) => {
  await installDesktop(page, "publish-failed"); await draft(page);
  await page.getByRole("button", { name: "Publish to 1 account" }).click();
  await expect(page.getByRole("textbox", { name: "Post text" })).toHaveValue("A post for the test workspace.");
  await expect(page.locator(".dispatch-bar")).toContainText("Publishing failed");
  await expect(page.getByRole("button", { name: "Publish to 1 account" })).toBeEnabled();
});

for (const scenario of ["publish-partial", "publish-partial-thread"] as const) test(scenario + " preserves the draft without resending published posts", async ({ page }) => {
  await installDesktop(page, scenario); await draft(page);
  await page.getByRole("button", { name: "Publish to 2 accounts" }).click();
  await expect(page.getByRole("textbox", { name: "Post text" })).toHaveValue("A post for the test workspace.");
  await expect(page.locator(".dispatch-bar")).toContainText("Some posts were published");
  await expect(page.getByRole("checkbox").first()).not.toBeChecked();
  if (scenario === "publish-partial") await expect(page.getByRole("checkbox").nth(1)).toBeChecked();
  else {
    await expect(page.getByRole("checkbox").nth(1)).not.toBeChecked();
    await expect(page.getByRole("button", { name: "Publish to 0 accounts" })).toBeDisabled();
  }
});

test("reconnecting a selected account requires a refreshed preview before publishing", async ({ page }) => {
  await installDesktop(page, "reconnect-preview"); await draft(page);
  await expect(page.getByRole("button", { name: "Publish to 1 account" })).toBeEnabled();
  await page.getByRole("button", { name: "Accounts & Sync", exact: true }).click();
  await page.getByLabel("Handle", { exact: true }).fill("writer.bsky.social");
  await page.getByLabel("App password", { exact: true }).fill("test-only-password");
  await page.getByRole("button", { name: "Connect account" }).click();
  await expect(page.getByText("writer.bsky.social connected and verified.", { exact: true })).toBeVisible();
  await page.getByRole("button", { name: /^Composer/ }).click();
  await expect(page.getByText("Refreshed planning unavailable", { exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: "Publish to 1 account" })).toBeDisabled();
  await page.getByRole("button", { name: "Retry preview" }).click();
  await expect(page.getByRole("button", { name: "Publish to 1 account" })).toBeEnabled();
});

test("removing the last account returns to setup", async ({ page }) => {
  await installDesktop(page, "ready"); await page.goto("/");
  await page.getByRole("button", { name: "Accounts & Sync", exact: true }).click();
  await page.getByRole("button", { name: "Remove writer.bsky.social" }).click();
  await expect(page.getByRole("heading", { name: "Connect your first account" })).toBeVisible();
  await expect(page.getByRole("navigation")).toHaveCount(0);
  await expect(page.getByRole("textbox", { name: "Post text" })).toHaveCount(0);
});

test("workspace load failure has retry without presenting an empty composer", async ({ page }) => {
  await installDesktop(page, "workspace-error"); await page.goto("/");
  await expect(page.getByText("Workspace temporarily unavailable", { exact: true })).toBeVisible();
  await expect(page.getByRole("textbox", { name: "Post text" })).toHaveCount(0);
  await page.evaluate(() => window.dispatchEvent(new Event("qa-workspace-recovered")));
  await page.getByRole("button", { name: "Retry workspace" }).click();
  await expect(page.getByRole("textbox", { name: "Post text" })).toBeVisible();
});
