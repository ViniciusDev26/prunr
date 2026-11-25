#!/usr/bin/env node
// Runs the prunr binary of this platform, from its @prunr/<platform> package.
"use strict";

const { spawnSync } = require("node:child_process");
const fs = require("node:fs");

const platform = `${process.platform}-${process.arch}`;
const executable = process.platform === "win32" ? "prunr.exe" : "prunr";

function fail(message) {
  console.error(`prunr: ${message}`);
  process.exit(2);
}

let binary;
try {
  binary = require.resolve(`@prunr/${platform}/${executable}`);
} catch {
  fail(
    `no binary for ${platform}. Supported: linux-x64, linux-arm64, darwin-x64, darwin-arm64, ` +
      "win32-x64. If yours is one of them, reinstall without --omit=optional.",
  );
}

try {
  fs.accessSync(binary, fs.constants.X_OK);
} catch {
  fs.chmodSync(binary, 0o755);
}

const result = spawnSync(binary, process.argv.slice(2), { stdio: "inherit" });
if (result.error) {
  fail(result.error.message);
}
process.exit(result.status ?? 1);
