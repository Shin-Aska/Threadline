import { expect, test, type Page } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";

async function installCacheBridge(page: Page, sizeMode: "ready" | "pending" | "fail" | "unsupported" = "ready", bytes = 26_214_400): Promise<void> {
  await page.addInitScript(({ sizeMode, bytes }) => {
    localStorage.setItem("threadline:post-cache-limit", "50");
    localStorage.setItem("threadline:external-link-warning", "off");
    sessionStorage.setItem("qa-cache-bytes", String(bytes));
    Object.defineProperty(window, "__TAURI_INTERNALS__", { value: { invoke: async (command: string) => {
      if (command === "get_browser_cache_size") {
        const call = Number(sessionStorage.getItem("qa-cache-size-calls") ?? "0") + 1;
        sessionStorage.setItem("qa-cache-size-calls", String(call));
        if (sizeMode === "fail") throw new Error("Cache size is unavailable");
        if (sizeMode === "unsupported") return null;
        if (sizeMode === "pending") return new Promise<number | null>((resolve, reject) => window.addEventListener(`qa-cache-size-result-${call}`, event => {
          if (!(event instanceof CustomEvent)) return;
          const value: unknown = event.detail;
          if (value === null || typeof value === "number") resolve(value);
          else reject(new Error("Cache size is unavailable"));
        }, { once: true }));
        return Number(sessionStorage.getItem("qa-cache-bytes"));
      }
      if (command !== "clear_browser_cache") throw new Error(`Unexpected command: ${command}`);
      sessionStorage.setItem("qa-cache-calls", String(Number(sessionStorage.getItem("qa-cache-calls") ?? "0") + 1));
      await new Promise<void>((resolve, reject) => window.addEventListener("qa-cache-result", event => {
        if (event instanceof CustomEvent && event.detail === "fail") reject("Cache storage is unavailable");
        else resolve();
      }, { once: true }));
    } } });
  }, { sizeMode, bytes });
}

async function captureSettings(page: Page, state: string): Promise<void> {
  for (const width of [375, 768, 1280]) {
    await page.setViewportSize({ width, height: 900 });
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
    if (width === 375) {
      const bounds = await page.getByRole("region", { name: "Browser cache", exact: true }).getByRole("button").boundingBox();
      expect(bounds?.height).toBeGreaterThanOrEqual(44);
    }
    await page.screenshot({ path: `.omo/evidence/browser-cache/settings-${state}-${width}.png`, fullPage: true });
  }
}

test("clearing browser cache waits for native completion and preserves preferences", async ({ page }) => {
  // Given saved preferences and a native clear operation awaiting completion.
  await installCacheBridge(page);
  await page.goto("/tests/fixtures/settings.html");
  const panel = page.getByRole("region", { name: "Browser cache", exact: true });
  const button = panel.getByRole("button", { name: "Clear browser cache", exact: true });
  await expect(panel).toBeVisible();
  await expect(panel).toContainText("Disk cache used: 25 MiB");
  await captureSettings(page, "ready");

  // When the user starts clearing the browser cache.
  await button.focus();
  await page.keyboard.press("Enter");

  // Then clearing stays pending until native completion and changes no preferences.
  await expect(panel.getByRole("button")).toBeDisabled();
  await expect(panel.getByRole("status")).toContainText("Clearing browser cache");
  await page.keyboard.press("Enter");
  expect(await page.evaluate(() => sessionStorage.getItem("qa-cache-calls"))).toBe("1");
  await captureSettings(page, "clearing");
  await page.evaluate(() => {
    sessionStorage.setItem("qa-cache-bytes", "4096");
    window.dispatchEvent(new CustomEvent("qa-cache-result", { detail: "success" }));
  });
  await expect(panel.getByRole("status")).toContainText("Browser cache cleared");
  await expect(button).toBeEnabled();
  await expect(panel).toContainText("Disk cache used: 4 KiB");
  expect(await page.evaluate(() => sessionStorage.getItem("qa-cache-size-calls"))).toBe("2");
  expect(await page.evaluate(() => sessionStorage.getItem("qa-cache-calls"))).toBe("1");
  expect(await page.evaluate(() => [localStorage.getItem("threadline:post-cache-limit"), localStorage.getItem("threadline:external-link-warning")])).toEqual(["50", "off"]);
  await captureSettings(page, "cleared");
});

test("an empty disk cache reports zero bytes", async ({ page }) => {
  await installCacheBridge(page, "ready", 0);
  await page.goto("/tests/fixtures/settings.html");
  await expect(page.getByRole("region", { name: "Browser cache", exact: true })).toContainText("Disk cache used: 0 B");
  await captureSettings(page, "empty");
});

for (const sizeMode of ["fail", "unsupported"] as const) {
  test(`unavailable cache size (${sizeMode}) keeps clearing available`, async ({ page }) => {
    await installCacheBridge(page, sizeMode);
    await page.goto("/tests/fixtures/settings.html");
    const panel = page.getByRole("region", { name: "Browser cache", exact: true });
    await expect(panel).toContainText("Disk cache size unavailable.");
    await expect(panel).not.toContainText("0 B");
    await panel.getByRole("button", { name: "Clear browser cache", exact: true }).click();
    await page.evaluate(() => window.dispatchEvent(new CustomEvent("qa-cache-result", { detail: "success" })));
    await expect(panel.getByRole("status")).toContainText("Browser cache cleared");
    await expect(panel).toContainText("Disk cache size unavailable.");
    await expect(panel.getByRole("button")).toBeEnabled();
    await captureSettings(page, `size-${sizeMode}`);
  });
}

test("cache size refresh ignores an older measurement arriving after clearing", async ({ page }) => {
  await installCacheBridge(page, "pending");
  await page.goto("/tests/fixtures/settings.html");
  const panel = page.getByRole("region", { name: "Browser cache", exact: true });
  await expect(panel).toContainText("Calculating disk cache size…");
  await expect.poll(() => page.evaluate(() => sessionStorage.getItem("qa-cache-size-calls"))).toBe("1");
  await captureSettings(page, "size-loading");
  await panel.getByRole("button", { name: "Clear browser cache", exact: true }).click();
  await page.evaluate(() => window.dispatchEvent(new CustomEvent("qa-cache-result", { detail: "success" })));
  await expect.poll(() => page.evaluate(() => sessionStorage.getItem("qa-cache-size-calls"))).toBe("2");
  await page.evaluate(() => window.dispatchEvent(new CustomEvent("qa-cache-size-result-2", { detail: 512 })));
  await expect(panel).toContainText("Disk cache used: 512 B");
  await page.evaluate(() => window.dispatchEvent(new CustomEvent("qa-cache-size-result-1", { detail: 26_214_400 })));
  await expect(panel).toContainText("Disk cache used: 512 B");
  await expect(panel).not.toContainText("25 MiB");
});

test("browser cache failures remain actionable and can be retried", async ({ page }) => {
  // Given a native cache operation that fails.
  await installCacheBridge(page);
  await page.goto("/tests/fixtures/settings.html");
  const panel = page.getByRole("region", { name: "Browser cache", exact: true });
  const button = panel.getByRole("button", { name: "Clear browser cache", exact: true });

  // When clearing fails and the user retries.
  await button.click();
  await page.evaluate(() => window.dispatchEvent(new CustomEvent("qa-cache-result", { detail: "fail" })));
  await expect(panel.getByRole("alert")).toContainText("Cache storage is unavailable");
  await expect(button).toBeEnabled();
  await expect(panel.getByRole("status")).toHaveCount(0);
  await captureSettings(page, "failed");
  await button.click();
  await page.evaluate(() => window.dispatchEvent(new CustomEvent("qa-cache-result", { detail: "success" })));

  // Then successful retry replaces the error with truthful completion feedback.
  await expect(panel.getByRole("alert")).toHaveCount(0);
  await expect(panel.getByRole("status")).toContainText("Browser cache cleared");
  expect(await page.evaluate(() => sessionStorage.getItem("qa-cache-calls"))).toBe("2");
});

test("browser preview cannot claim to clear the desktop cache", async ({ page }) => {
  // Given Settings in a browser without a native bridge.
  await page.goto("/tests/fixtures/settings.html");

  // When the cache control is presented.
  const panel = page.getByRole("region", { name: "Browser cache", exact: true });

  // Then it is disabled and explains the desktop requirement.
  await expect(panel.getByRole("button", { name: "Clear browser cache", exact: true })).toBeDisabled();
  await expect(panel).toContainText("Available in the desktop app");
  await expect(panel.getByRole("status")).toHaveCount(0);
  await captureSettings(page, "browser");
});

test("Settings navigation exposes the accessible desktop cache control", async ({ page }) => {
  // Given the real application shell with connected fixture accounts.
  await page.goto("/tests/fixtures/complete-workspace.html");

  // When the user navigates to Settings.
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  const panel = page.getByRole("region", { name: "Browser cache", exact: true });

  // Then the cache control is reachable and follows the existing layout contract.
  await expect(panel.getByRole("button", { name: "Clear browser cache", exact: true })).toBeEnabled();
  expect((await new AxeBuilder({ page }).include('[aria-labelledby="settings-browser-heading"]').analyze()).violations).toEqual([]);
  await captureSettings(page, "workspace");
});
