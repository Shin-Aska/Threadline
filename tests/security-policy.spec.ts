import { readFile } from "node:fs/promises";
import { join, resolve, sep } from "node:path";
import { expect, test } from "@playwright/test";

interface PolicyViolation { readonly directive: string; readonly blocked: string }
declare global {
  interface Window {
    qaPolicyViolations?: PolicyViolation[];
    qaUntrustedInline?: boolean;
    qaUntrustedRemote?: boolean;
  }
}

test.beforeEach(async ({ page }) => {
  const config: { app: { security: { csp: Record<string, string> } } } = JSON.parse(await readFile("src-tauri/tauri.conf.json", "utf8"));
  const nonce = "dGhyZWFkbGluZS1wb2xpY3ktdGVzdA==";
  const policy = { ...config.app.security.csp, "script-src": `${config.app.security.csp["script-src"]} 'nonce-${nonce}'` };
  const built = await readFile("dist/index.html", "utf8");
  const fixture = await readFile("tests/fixtures/complete-workspace.html", "utf8");
  const bridge = fixture.match(/<script>([\s\S]*?)<\/script>/)?.[1];
  if (!bridge) throw new Error("The controlled desktop bridge fixture is missing");
  // Tauri supplies nonces for its trusted bootstrap; only the test bridge receives one here.
  const html = built.replace("</head>", `<script nonce="${nonce}">${bridge}</script></head>`);
  await page.route("**/security-preview/", route => route.fulfill({
    contentType: "text/html", body: html,
    headers: { "Content-Security-Policy": Object.entries(policy).map(([directive, value]) => `${directive} ${value}`).join("; ") },
  }));
  const assets = resolve("dist/assets");
  await page.route("**/assets/**", async route => {
    const path = resolve("dist", decodeURIComponent(new URL(route.request().url()).pathname).slice(1));
    if (!path.startsWith(`${assets}${sep}`)) return route.abort();
    const contentType = path.endsWith(".js") ? "text/javascript" : path.endsWith(".css") ? "text/css" : "font/woff2";
    await route.fulfill({ contentType, body: await readFile(path) });
  });
  await page.goto("/security-preview/");
  await expect(page.getByRole("heading", { name: "Timeline", exact: true })).toBeVisible();
  await page.evaluate(() => {
    window.qaPolicyViolations = [];
    document.addEventListener("securitypolicyviolation", event => window.qaPolicyViolations?.push({ directive: event.effectiveDirective, blocked: event.blockedURI }));
  });
});

test("production policy blocks untrusted inline and remote scripts", async ({ page }) => {
  // Given a populated production build protected by the configured CSP.
  await page.route("https://scripts.example.test/untrusted.js", route => route.fulfill({ contentType: "text/javascript", body: "window.qaUntrustedRemote = true" }));
  // When untrusted content tries to add executable scripts.
  await page.evaluate(() => {
    const inline = document.createElement("script");
    inline.textContent = "window.qaUntrustedInline = true";
    const remote = document.createElement("script");
    remote.src = "https://scripts.example.test/untrusted.js";
    document.head.append(inline, remote);
  });
  // Then both attempts are blocked before execution.
  await expect.poll(() => page.evaluate(() => window.qaPolicyViolations?.filter(item => item.directive.startsWith("script-src")).map(item => item.blocked))).toEqual(expect.arrayContaining(["inline", "https://scripts.example.test/untrusted.js"]));
  expect(await page.evaluate(() => [window.qaUntrustedInline, window.qaUntrustedRemote])).toEqual([undefined, undefined]);
});

test("production policy preserves provider images, streaming reads and blob workers", async ({ page }) => {
  // Given HTTPS provider media and the app's locally bundled image support.
  await page.route("https://media.example.test/image.png", async route => route.fulfill({ contentType: "image/png", body: await readFile(join("src-tauri", "icons", "icon.png")) }));
  await page.route("https://media.example.test/stream.m3u8", route => route.fulfill({ contentType: "application/vnd.apple.mpegurl", headers: { "Access-Control-Allow-Origin": "*" }, body: "#EXTM3U\n" }));
  // When the renderer loads provider media and starts a player-style worker.
  const result = await page.evaluate(async () => {
    const image = new Image();
    const imageLoaded = new Promise<boolean>(resolve => { image.onload = () => resolve(image.naturalWidth > 0); image.onerror = () => resolve(false); });
    image.src = "https://media.example.test/image.png";
    document.body.append(image);
    const stream = await fetch("https://media.example.test/stream.m3u8").then(response => response.text());
    const url = URL.createObjectURL(new Blob(["postMessage('worker-ready')"], { type: "text/javascript" }));
    const worker = new Worker(url);
    try {
      const message = await new Promise<string>((resolve, reject) => { worker.onmessage = event => resolve(String(event.data)); worker.onerror = () => reject(new Error("Media worker blocked")); });
      return { imageLoaded: await imageLoaded, stream, message, violations: window.qaPolicyViolations };
    } finally { worker.terminate(); URL.revokeObjectURL(url); image.remove(); }
  });
  // Then provider media remains usable without broadening script execution.
  expect(result).toEqual({ imageLoaded: true, stream: "#EXTM3U\n", message: "worker-ready", violations: [] });
});
