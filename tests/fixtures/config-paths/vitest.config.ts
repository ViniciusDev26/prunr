import { defineConfig } from "vitest/config";

// Built by a function at runtime: prunr cannot read the projects, only the paths written in
// them.
function suites() {
  return [
    {
      test: {
        setupFiles: ["./packages/kit/setup.ts"],
        globalSetup: "./packages/kit/global.ts",
      },
    },
  ];
}

export default defineConfig(() => ({ test: { projects: suites() } }));
