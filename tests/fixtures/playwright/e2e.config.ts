import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./e2e",
  globalSetup: "./e2e/helpers/global-setup.ts",
  reporter: [["list"], ["playwright-ctrf-json-reporter", {}]],
  webServer: [{ command: "pnpm exec tsx e2e/mock/server.ts", port: 4000 }, { command: "pnpm dev" }],
});
