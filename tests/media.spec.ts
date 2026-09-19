import { expect, test } from "@playwright/test";
import type { Page } from "@playwright/test";
import type { CanonicalPost } from "../src/types";
import { readFileSync } from "node:fs";
const image = { name: "threadline.png", mimeType: "image/png", buffer: readFileSync("src-tauri/icons/icon.png") };
async function setup(page: Page, failure = false) {
  await page.addInitScript(({ failure }) => {
    const account = { id: "image-test", provider: "BLUESKY", displayName: "Test writer", handle: "writer.test", capabilities: { maxTextLength: 300, maxMediaAttachments: 4, supportedMediaTypes: ["image/png"] } };
    Object.defineProperty(window, "__TAURI_INTERNALS__", { value: { invoke: async (command: string, args: { post?: CanonicalPost }) => {
      if (command === "get_workspace") return { accounts: [account], connectedAccountIds: [account.id], mode: "LIVE" };
      const post = args.post;
      if (!post) throw new Error("Missing post");
      if (command === "preview_post") {
        if (post.media.some(image => image.dataBase64)) throw new Error("Preview should not transfer image bytes");
        return { graphemeCount: post.text.length, destinations: [{ accountId: account.id, parts: [post.text] }] };
      }
      if (command === "publish_post") {
        sessionStorage.setItem("published-draft", JSON.stringify(post));
        await new Promise<void>(resolve => window.addEventListener("release-publish", () => resolve(), { once: true }));
        return { canonicalId: "qa", publications: [{ accountId: account.id, status: failure ? "FAILED" : "PUBLISHED", remotePostIds: failure ? [] : ["qa:1"], error: failure ? "Upload rejected" : null }] };
      }
      throw new Error(command);
    } } });
  }, { failure });
  await page.goto("/");
  await page.getByRole("button", { name: "Composer", exact: true }).click();
}
async function capture(page: Page, name: string) {
  await page.evaluate(() => { if (document.activeElement instanceof HTMLElement) document.activeElement.blur(); window.scrollTo(0, 0); });
  await page.screenshot({ path: `.omo/evidence/media/${name}.png`, fullPage: true });
}
test("image-only publish sends bytes and alt text, locks controls and clears on success", async ({ page }) => {
  await setup(page);
  await capture(page, "empty");
  await page.getByLabel("Choose images").setInputFiles(image);
  await page.getByLabel("Alt text for image 1").fill("Blue and violet Threadline logo 🧵");
  await expect(page.locator(".preview-images img")).toHaveAttribute("alt", "Blue and violet Threadline logo 🧵");
  await page.getByRole("button", { name: "Publish to 1 account" }).click();
  await expect(page.getByLabel("Alt text for image 1")).toBeDisabled();
  await expect(page.getByRole("button", { name: "Remove threadline.png" })).toBeDisabled();
  await capture(page, "publishing");
  const sent = await page.evaluate(() => sessionStorage.getItem("published-draft"));
  expect(sent).toContain(image.buffer.toString("base64"));
  expect(sent).toContain("Blue and violet Threadline logo 🧵");
  await page.evaluate(() => window.dispatchEvent(new Event("release-publish")));
  await expect(page.locator(".attachment-card")).toHaveCount(0);
  await expect(page.locator(".dispatch-bar")).toContainText("Published successfully");
  await capture(page, "success");
  await expect(page.getByRole("button", { name: "Publish to 1 account" })).toBeDisabled();
});
test("upload failure keeps text, image and alt text for retry", async ({ page }) => {
  await setup(page, true);
  await page.getByLabel("Post text", { exact: true }).fill("Keep my draft");
  await page.getByLabel("Choose images").setInputFiles(image);
  await page.getByLabel("Alt text for image 1").fill("Keep this description");
  await page.getByRole("button", { name: "Publish to 1 account" }).click();
  await page.evaluate(() => window.dispatchEvent(new Event("release-publish")));
  await expect(page.locator(".dispatch-bar")).toContainText("Publishing failed");
  await expect(page.getByLabel("Post text", { exact: true })).toHaveValue("Keep my draft");
  await capture(page, "failure");
  await expect(page.getByLabel("Alt text for image 1")).toHaveValue("Keep this description");
  await expect(page.getByRole("button", { name: "Publish to 1 account" })).toBeEnabled();
});
test("invalid files and excessive counts preserve existing images; removing permits reselection", async ({ page }) => {
  await setup(page);
  const picker = page.getByLabel("Choose images");
  await picker.setInputFiles(image);
  await picker.setInputFiles({ name: "large.png", mimeType: "image/png", buffer: Buffer.alloc(2_000_001) });
  await expect(page.getByRole("alert")).toContainText("up to 2 MB");
  await expect(page.locator(".attachment-card")).toHaveCount(1);
  await picker.setInputFiles([image, image, image, image]);
  await expect(page.getByRole("alert")).toContainText("up to four");
  await picker.setInputFiles({ name: "invalid.png", mimeType: "image/png", buffer: Buffer.from("not an image") });
  await expect(page.getByRole("alert")).toContainText("could not be opened");
  await capture(page, "invalid-image");
  await page.getByRole("button", { name: "Remove threadline.png" }).click();
  await picker.setInputFiles(image);
  await expect(page.locator(".attachment-card")).toHaveCount(1);
});
for (const width of [375, 768, 1280]) test(`attachment layout at ${width}px`, async ({ page }) => {
  await page.setViewportSize({ width, height: 1000 }); await setup(page);
  await page.getByLabel("Post text", { exact: true }).fill("A little color for your timeline.");
  await page.getByLabel("Choose images").setInputFiles([image, { ...image, name: "another-image.png" }]);
  await page.getByLabel("Alt text for image 1").fill("A white Threadline mark on a blue and violet background.");
  await expect(page.locator(".preview-images img")).toHaveCount(2);
  await expect(page.getByRole("button", { name: "Publish to 1 account" })).toBeEnabled();
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  await capture(page, `final-attachments-${width}`);
  await page.locator(".images-panel").evaluate(element => window.scrollTo(0, element.getBoundingClientRect().top + window.scrollY - 12));
  await page.screenshot({ path: `.omo/evidence/media/final-detail-${width}.png` });
});
