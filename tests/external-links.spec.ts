import { expect, test } from "@playwright/test";
import type { Page } from "@playwright/test";

async function recordBrowserLaunches(page: Page, failFirst = false): Promise<void> {
  await page.evaluate(({ failFirst }) => {
    const bridge = (window as unknown as { __TAURI_INTERNALS__: { invoke: (command: string, args: Record<string, unknown>) => Promise<unknown> } }).__TAURI_INTERNALS__;
    const invoke = bridge.invoke;
    let attempts = 0;
    bridge.invoke = async (command, args) => {
      if (command !== "open_external_url") return invoke(command, args);
      attempts += 1;
      localStorage.setItem("qa-browser-attempts", String(attempts));
      if (failFirst && attempts === 1) throw new Error("Default browser unavailable");
      localStorage.setItem("qa-browser-url", String(args.url));
    };
  }, { failFirst });
}

test("external links launch through the desktop bridge after confirmation and when warnings are disabled", async ({ page }) => {
  await page.goto("/tests/fixtures/complete-workspace.html");
  await recordBrowserLaunches(page);
  const original = page.locator(".unified-post").first().getByRole("link", { name: "Original", exact: true });
  const destination = await original.getAttribute("href");
  await original.click();
  const dialog = page.getByRole("dialog", { name: "Open an external link?" });
  await expect(dialog).toBeVisible();
  expect(await page.evaluate(() => localStorage.getItem("qa-browser-url"))).toBeNull();
  await dialog.getByRole("button", { name: "Cancel" }).click();
  await expect(dialog).not.toBeVisible();
  expect(await page.evaluate(() => localStorage.getItem("qa-browser-url"))).toBeNull();
  await original.click();
  await dialog.getByRole("checkbox", { name: "Don’t warn me again" }).check();
  await dialog.getByRole("link", { name: "Continue" }).click();
  await expect(dialog).not.toBeVisible();
  expect(await page.evaluate(() => localStorage.getItem("qa-browser-url"))).toBe(destination);
  expect(await page.evaluate(() => localStorage.getItem("threadline:external-link-warning"))).toBe("off");
  await original.click({ button: "middle" });
  await expect.poll(() => page.evaluate(() => localStorage.getItem("qa-browser-attempts"))).toBe("2");
  await expect(dialog).not.toBeVisible();
  await expect(page.getByRole("heading", { name: "Timeline", exact: true })).toBeVisible();
});

test("failed browser launches show an error and keep confirmation preferences until a successful retry", async ({ page }) => {
  await page.goto("/tests/fixtures/complete-workspace.html");
  await recordBrowserLaunches(page, true);
  await page.locator(".unified-post").first().getByRole("link", { name: "Original", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "Open an external link?" });
  await dialog.getByRole("checkbox", { name: "Don’t warn me again" }).check();
  await dialog.getByRole("link", { name: "Continue" }).click();
  await expect(dialog.getByRole("alert")).toContainText("Default browser unavailable");
  expect(await page.evaluate(() => localStorage.getItem("threadline:external-link-warning"))).toBeNull();
  await dialog.getByRole("link", { name: "Continue" }).click();
  await expect(dialog).not.toBeVisible();
  expect(await page.evaluate(() => localStorage.getItem("qa-browser-attempts"))).toBe("2");
  expect(await page.evaluate(() => localStorage.getItem("threadline:external-link-warning"))).toBe("off");
});

test("failed browser launches still offer a retry with warnings disabled", async ({ page }) => {
  await page.goto("/tests/fixtures/complete-workspace.html");
  await recordBrowserLaunches(page, true);
  await page.evaluate(() => localStorage.setItem("threadline:external-link-warning", "off"));
  await page.locator(".unified-post").first().getByRole("link", { name: "Original", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "Open an external link?" });
  await expect(dialog.getByRole("alert")).toContainText("Default browser unavailable");
  await dialog.getByRole("link", { name: "Continue" }).click();
  await expect(dialog).not.toBeVisible();
  expect(await page.evaluate(() => localStorage.getItem("qa-browser-attempts"))).toBe("2");
});
