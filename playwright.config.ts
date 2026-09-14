import { defineConfig } from "@playwright/test";
export default defineConfig({
  testDir: "./tests",
  timeout: 20_000,
  use: { baseURL: "http://127.0.0.1:4174", browserName: "chromium", channel: "chrome", headless: true, viewport: { width: 1280, height: 900 } },
  webServer: { command: "npm run dev -- --host 127.0.0.1 --port 4174", url: "http://127.0.0.1:4174", reuseExistingServer: !process.env.CI },
  reporter: "list",
  outputDir: ".omo/evidence/functional-ui/test-results",
});
