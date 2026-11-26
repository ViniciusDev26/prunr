#!/usr/bin/env node
// The validation corpus: runs prunr (and, optionally, Knip) on real open source projects pinned
// to a commit, and writes what each one finds and how long it takes.
//
//   node corpus/run.mjs [--only express,koa] [--install] [--knip] [--prunr path/to/prunr]
//
// Checkouts and results live outside the repository, in $PRUNR_CORPUS_DIR
// (default: ~/.cache/prunr-corpus). See corpus/README.md.

import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, writeFileSync, copyFileSync } from "node:fs";
import { homedir } from "node:os";
import { join, resolve } from "node:path";
import { performance } from "node:perf_hooks";

const here = new URL(".", import.meta.url).pathname;
const args = process.argv.slice(2);
const flag = (name) => args.includes(`--${name}`);
const option = (name) => {
  const index = args.indexOf(`--${name}`);
  return index >= 0 ? args[index + 1] : undefined;
};

const cache = process.env.PRUNR_CORPUS_DIR ?? join(homedir(), ".cache", "prunr-corpus");
const prunr = resolve(option("prunr") ?? join(here, "..", "target", "release", "prunr"));
const knipVersion = option("knip-version") ?? "5";
const only = option("only")?.split(",");
const { repositories } = JSON.parse(readFileSync(join(here, "repos.json"), "utf8"));
const selected = repositories.filter((repo) => !only || only.includes(repo.name));

if (!existsSync(prunr)) {
  console.error(`prunr not found at ${prunr}: run cargo build --release, or pass --prunr`);
  process.exit(2);
}

/** Runs a command and returns its output, exit code and duration in seconds. */
function run(command, commandArgs, cwd, extra = {}) {
  const start = performance.now();
  const result = spawnSync(command, commandArgs, {
    cwd,
    encoding: "utf8",
    maxBuffer: 1 << 30,
    ...extra,
  });
  const seconds = (performance.now() - start) / 1000;
  return { ...result, seconds };
}

function git(cwd, ...gitArgs) {
  const result = run("git", gitArgs, cwd);
  if (result.status !== 0) {
    throw new Error(`git ${gitArgs.join(" ")}: ${result.stderr}`);
  }
  return result.stdout.trim();
}

/** A shallow checkout of the repository at its pinned commit. */
function checkout(repo) {
  const dir = join(cache, "repos", repo.name);
  if (!existsSync(join(dir, ".git"))) {
    mkdirSync(dir, { recursive: true });
    git(dir, "init", "-q");
    git(dir, "remote", "add", "origin", `https://github.com/${repo.repo}.git`);
  }
  if (run("git", ["rev-parse", "HEAD"], dir).stdout.trim() !== repo.commit) {
    git(dir, "fetch", "-q", "--depth", "1", "origin", repo.commit);
    git(dir, "checkout", "-q", "--force", "FETCH_HEAD");
    git(dir, "clean", "-q", "-fdx", "-e", "node_modules");
  }
  return dir;
}

/** Installs the dependencies with the project's package manager, without running scripts. */
function install(dir) {
  if (existsSync(join(dir, "node_modules"))) return "present";
  const manager = existsSync(join(dir, "pnpm-lock.yaml"))
    ? ["npx", ["-y", "pnpm@10", "install", "--ignore-scripts", "--no-frozen-lockfile"]]
    : existsSync(join(dir, "yarn.lock"))
      ? ["npx", ["-y", "yarn@1", "install", "--ignore-scripts", "--ignore-engines"]]
      : ["npm", ["install", "--ignore-scripts", "--no-audit", "--no-fund", "--allow-remote=all"]];
  const result = run(...manager, dir, { env: { ...process.env, CI: "1" } });
  return result.status === 0 ? "installed" : `failed: ${result.stderr.slice(-400)}`;
}

/** The prunr.json of the corpus for this repository, or one `prunr init` writes. */
function configure(repo, dir) {
  const config = join(here, "configs", `${repo.name}.json`);
  if (existsSync(config)) {
    copyFileSync(config, join(dir, "prunr.json"));
    return "corpus";
  }
  if (existsSync(join(dir, "prunr.json"))) return "existing";
  const result = run(prunr, ["init"], dir);
  return result.status === 0 ? "init" : `init failed: ${result.stderr.trim()}`;
}

/** prunr's findings, keyed the way the comparison needs them. */
function prunrFindings(output) {
  const findings = JSON.parse(output).findings;
  const live = findings.filter((finding) => !finding.testOnly);
  const pick = (kind, key) =>
    new Set(live.filter((finding) => finding.kind === kind).map(key));
  return {
    counts: count(findings.map((finding) => finding.kind)),
    files: pick("unused-file", (finding) => finding.file),
    exports: pick("unused-export", (finding) => `${finding.file}:${finding.symbol}`),
    dependencies: pick("unused-dependency", (finding) => dependency(finding.package, finding.workspace)),
    unlisted: pick("missing-dependency", (finding) => `${finding.file}:${finding.package}`),
  };
}

/** Knip's findings (its JSON reporter), keyed like prunr's. */
function knipFindings(output) {
  const report = JSON.parse(output);
  const files = new Set(report.files ?? []);
  const exports = new Set();
  const dependencies = new Set();
  const unlisted = new Set();
  const counts = {};
  for (const issue of report.issues ?? []) {
    if (issue.files) for (const file of [].concat(issue.files)) files.add(file.name ?? issue.file);
    for (const [type, items] of Object.entries(issue)) {
      if (!Array.isArray(items)) continue;
      counts[type] = (counts[type] ?? 0) + items.length;
      for (const item of items) {
        if (["exports", "types"].includes(type)) exports.add(`${issue.file}:${item.name}`);
        if (["dependencies", "devDependencies"].includes(type)) {
          const workspace = issue.file.includes("/") ? issue.file.replace(/\/package\.json$/, "") : undefined;
          dependencies.add(dependency(item.name, workspace));
        }
        if (type === "unlisted") unlisted.add(`${issue.file}:${item.name}`);
      }
    }
  }
  counts.files = files.size;
  return { counts, files, exports, dependencies, unlisted };
}

const dependency = (name, workspace) => (workspace ? `${name} (${workspace})` : name);

function count(items) {
  const counts = {};
  for (const item of items) counts[item] = (counts[item] ?? 0) + 1;
  return counts;
}

/** What only one of the two tools reports, per kind. */
function compare(ours, theirs) {
  const diff = {};
  for (const kind of ["files", "exports", "dependencies", "unlisted"]) {
    const a = ours[kind];
    const b = theirs[kind];
    diff[kind] = {
      both: [...a].filter((item) => b.has(item)).length,
      onlyPrunr: [...a].filter((item) => !b.has(item)).sort(),
      onlyKnip: [...b].filter((item) => !a.has(item)).sort(),
    };
  }
  return diff;
}

const rows = [];
for (const repo of selected) {
  const results = join(cache, "results", repo.name);
  mkdirSync(results, { recursive: true });
  const row = { name: repo.name, kind: repo.kind };
  try {
    process.stderr.write(`${repo.name}: checkout`);
    const dir = checkout(repo);
    if (flag("install")) {
      process.stderr.write(", install");
      row.install = install(dir);
    }
    row.config = configure(repo, dir);
    process.stderr.write(", prunr");
    const ours = run(prunr, ["check", "--json"], dir);
    writeFileSync(join(results, "prunr.json"), ours.stdout);
    row.prunrSeconds = ours.seconds;
    if (ours.status === 2) {
      row.error = ours.stderr.trim();
    } else {
      const found = prunrFindings(ours.stdout);
      row.prunr = found.counts;
      if (flag("knip") && existsSync(join(dir, "node_modules"))) {
        process.stderr.write(", knip");
        const theirs = run(
          "npx",
          ["-y", `knip@${knipVersion}`, "--reporter", "json", "--no-exit-code", "--no-progress"],
          dir,
        );
        writeFileSync(join(results, "knip.json"), theirs.stdout);
        row.knipSeconds = theirs.seconds;
        try {
          const knip = knipFindings(theirs.stdout);
          row.knip = knip.counts;
          row.diff = compare(found, knip);
          writeFileSync(join(results, "diff.json"), JSON.stringify(row.diff, null, 2));
        } catch {
          row.knipError = (theirs.stderr || theirs.stdout).trim().slice(-400);
        }
      }
    }
  } catch (error) {
    row.error = String(error.message ?? error);
  }
  process.stderr.write(row.error ? `: error\n` : `: ${row.prunrSeconds.toFixed(2)}s\n`);
  rows.push(row);
}

writeFileSync(join(cache, "results.json"), JSON.stringify(rows, null, 2));
writeFileSync(join(cache, "report.md"), report(rows));
console.log(report(rows));

function report(rows) {
  const kinds = [
    "unused-file",
    "unused-export",
    "unused-import",
    "unused-dependency",
    "misplaced-dependency",
    "missing-dependency",
    "circular-import",
    "unresolved-require",
  ];
  const short = (kind) =>
    ({
      "unused-file": "files",
      "unused-export": "exports",
      "unused-import": "imports",
      "unused-dependency": "deps",
      "misplaced-dependency": "misplaced",
      "missing-dependency": "missing",
      "circular-import": "cycles",
      "unresolved-require": "unresolved",
    })[kind];
  const lines = [
    "# Corpus report",
    "",
    `| repo | kind | config | prunr s | ${kinds.map(short).join(" | ")} | knip s | knip files/exports/deps |`,
    `|---|---|---|---|${kinds.map(() => "---").join("|")}|---|---|`,
  ];
  for (const row of rows) {
    if (row.error) {
      lines.push(`| ${row.name} | ${row.kind} | ${row.config ?? ""} | error | ${kinds.map(() => "").join(" | ")} | | ${row.error.split("\n")[0].slice(0, 80)} |`);
      continue;
    }
    const knip = row.knip
      ? `${row.knip.files ?? 0}/${(row.knip.exports ?? 0) + (row.knip.types ?? 0)}/${(row.knip.dependencies ?? 0) + (row.knip.devDependencies ?? 0)}`
      : row.knipError
        ? "error"
        : "";
    lines.push(
      `| ${row.name} | ${row.kind} | ${row.config} | ${row.prunrSeconds.toFixed(2)} | ${kinds.map((kind) => row.prunr[kind] ?? 0).join(" | ")} | ${row.knipSeconds?.toFixed(2) ?? ""} | ${knip} |`,
    );
  }
  const compared = rows.filter((row) => row.diff);
  if (compared.length > 0) {
    lines.push("", "## Agreement with Knip", "", "| repo | files both/prunr/knip | exports both/prunr/knip | deps both/prunr/knip |", "|---|---|---|---|");
    for (const row of compared) {
      const cell = (kind) => {
        const d = row.diff[kind];
        return `${d.both}/${d.onlyPrunr.length}/${d.onlyKnip.length}`;
      };
      lines.push(`| ${row.name} | ${cell("files")} | ${cell("exports")} | ${cell("dependencies")} |`);
    }
  }
  return `${lines.join("\n")}\n`;
}
