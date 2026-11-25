// Publishes the npm packages of one release: a @prunr/<platform> package per binary, then
// @prunr/prunr, which depends on them. Run by .github/workflows/release.yml with npm trusted
// publishing (OIDC), so no token is involved.
//
// usage: node npm/publish.mjs <version> <artifacts dir>
// With DRY_RUN=1 it only packs each package and lists its files.
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

const [version, artifacts] = process.argv.slice(2);
if (!version || !artifacts) {
  throw new Error("usage: node npm/publish.mjs <version> <artifacts dir>");
}

const root = path.resolve(import.meta.dirname, "..");
const main = JSON.parse(fs.readFileSync(path.join(root, "npm/prunr/package.json"), "utf8"));
const platforms = {
  "darwin-arm64": { os: "darwin", cpu: "arm64" },
  "darwin-x64": { os: "darwin", cpu: "x64" },
  "linux-arm64": { os: "linux", cpu: "arm64" },
  "linux-x64": { os: "linux", cpu: "x64" },
  "win32-x64": { os: "win32", cpu: "x64" },
};

function published(name) {
  try {
    execFileSync("npm", ["view", `${name}@${version}`, "version"], { stdio: "pipe" });
    return true;
  } catch {
    return false;
  }
}

const dryRun = process.env.DRY_RUN === "1";

function publish(directory, name) {
  if (dryRun) {
    execFileSync("npm", ["pack", "--dry-run"], { cwd: directory, stdio: "inherit" });
    return;
  }
  if (published(name)) {
    console.log(`${name}@${version} is already published`);
    return;
  }
  execFileSync("npm", ["publish", "--access", "public", "--provenance"], {
    cwd: directory,
    stdio: "inherit",
  });
}

const common = {
  version,
  license: main.license,
  repository: main.repository,
  homepage: main.homepage,
};

for (const [platform, { os: system, cpu }] of Object.entries(platforms)) {
  const name = `@prunr/${platform}`;
  const executable = system === "win32" ? "prunr.exe" : "prunr";
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), "prunr-"));
  fs.copyFileSync(path.join(artifacts, `prunr-${platform}`, executable), path.join(directory, executable));
  fs.chmodSync(path.join(directory, executable), 0o755);
  const manifest = {
    name,
    ...common,
    description: `The prunr binary for ${platform}`,
    os: [system],
    cpu: [cpu],
    files: [executable],
  };
  fs.writeFileSync(path.join(directory, "package.json"), `${JSON.stringify(manifest, null, 2)}\n`);
  publish(directory, name);
}

const directory = fs.mkdtempSync(path.join(os.tmpdir(), "prunr-"));
fs.cpSync(path.join(root, "npm/prunr"), directory, { recursive: true });
for (const file of ["README.md", "LICENSE", "schema.json"]) {
  fs.copyFileSync(path.join(root, file), path.join(directory, file));
}
const manifest = { ...main, version };
for (const dependency of Object.keys(manifest.optionalDependencies)) {
  manifest.optionalDependencies[dependency] = version;
}
fs.writeFileSync(path.join(directory, "package.json"), `${JSON.stringify(manifest, null, 2)}\n`);
publish(directory, main.name);
