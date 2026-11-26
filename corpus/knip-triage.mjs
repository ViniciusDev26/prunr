#!/usr/bin/env node
// Triage of what only Knip reports in the last run (`node corpus/run.mjs --knip`): every unused
// file, export and dependency Knip reports and prunr does not, with the category it falls in and
// the evidence for it, from `prunr why` and the files on disk.
//
//   node corpus/knip-triage.mjs [--jobs 32] > knip-triage.md
//
// Reads $PRUNR_CORPUS_DIR (default: ~/.cache/prunr-corpus) and runs target/release/prunr.

import { spawn } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { availableParallelism, homedir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const cache = process.env.PRUNR_CORPUS_DIR ?? join(homedir(), ".cache", "prunr-corpus");
const prunr = join(here, "..", "target", "release", "prunr");
const jobsAt = process.argv.indexOf("--jobs");
const jobs = jobsAt > 0 ? Number(process.argv[jobsAt + 1]) : availableParallelism();

/** prunr's rule for the data tests read: a fixtures folder inside a test folder. */
const TEST_FOLDERS = ["test", "tests", "__tests__", "spec", "e2e"];
const FIXTURE_FOLDERS = ["fixtures", "__fixtures__", "fixture", "samples"];
function inTestFixture(file) {
  const folders = file.split("/").slice(0, -1);
  const at = folders.findIndex((folder) => TEST_FOLDERS.includes(folder));
  return at >= 0 && folders.slice(at + 1).some((folder) => FIXTURE_FOLDERS.includes(folder));
}

function why(repo, target) {
  return new Promise((resolve) => {
    const child = spawn(prunr, ["why", target], { cwd: join(cache, "repos", repo) });
    let output = "";
    child.stdout.on("data", (data) => (output += data));
    child.stderr.on("data", (data) => (output += data));
    child.on("close", () => resolve(output.trim()));
  });
}

const read = (repo, file) => {
  try {
    return readFileSync(join(cache, "repos", repo, file), "utf8");
  } catch {
    return "";
  }
};
const escape = (name) => name.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
const mentions = (text, name) =>
  (text.match(new RegExp(`(?<![\\w$])${escape(name)}(?![\\w$])`, "g")) ?? []).length;
const isDeclaration = (file) => /\.d\.[cm]?ts$/.test(file);

/** The users `prunr why` lists: `file:line (kind[, through x])`. */
function users(answer) {
  return answer
    .split("\n")
    .slice(1)
    .map((line) => line.trim().match(/^(.+):(\d+) \((\w+)(?:, through (.+))?\)$/))
    .filter(Boolean)
    .map(([, file, line, kind, through]) => ({ file, line: Number(line), kind, through }));
}

function file(repo, path, answer, knipFiles) {
  if (inTestFixture(path)) return ["in a test fixtures folder, never reported", path];
  if (answer.includes("is not a file of the project")) {
    return isDeclaration(path)
      ? ["a declaration file, not part of the graph", path]
      : ["not a source file prunr reads", "behind a symbolic link or ignored"];
  }
  const lines = answer.split("\n");
  if (lines[0].endsWith("is used only by tests:")) {
    return lines.length === 2
      ? ["a test file", lines[1].trim()]
      : ["reached only by tests: reported as test-only", lines[1].trim()];
  }
  if (lines[0].endsWith("is used in production:")) {
    const entry = lines[1].trim().replace(/ \(entry\)$/, "");
    if (knipFiles.has(entry)) return ["reached from an entry Knip reports as unused", entry];
    return ["reached through an import Knip does not follow", lines.at(-1).trim()];
  }
  return ["uncategorized", lines[0]];
}

async function exported(repo, target, answer, findings) {
  const at = target.lastIndexOf(":");
  const [path, name] = [target.slice(0, at), target.slice(at + 1)];
  if (inTestFixture(path)) return ["in a test fixtures folder, never reported", path];
  if (answer.includes("is not a file of the project")) {
    return ["in a declaration file, not part of the graph", path];
  }
  if (answer.includes("an entry: its exports are its interface")) {
    return ["an export of an entry: its interface", path];
  }
  if (answer.includes("  its own file")) return ["used in its own file", path];
  const found = users(answer);
  const production = found.filter((user) => user.kind === "production");
  if (production.length > 0) {
    const through = production.find((user) => user.through);
    if (through) return ["used through a reexport", `${through.file}:${through.line} via ${through.through}`];
    const byName = production.find((user) => mentions(read(repo, user.file), name) > 0);
    if (byName) return ["used by name", `${byName.file}:${byName.line}`];
    return ["the whole module is used (passed on, or read by computed key)", `${production[0].file}:${production[0].line}`];
  }
  if (found.some((user) => user.kind === "test")) return ["used only by tests: reported as test-only", `${found[0].file}:${found[0].line}`];
  if (found.length > 0) return ["used only by files no entry reaches: reported as unused files", found[0].file];
  if (findings.some((finding) => finding.kind === "unused-file" && finding.file === path)) {
    return ["in a file reported as unused", path];
  }
  if (findings.some((finding) => finding.kind === "unused-export" && finding.file === path && finding.symbol === name)) {
    return ["reported as test-only", path];
  }
  const source = read(repo, path);
  if (new RegExp(`export\\s*(type\\s*)?\\{[^}]*\\b${escape(name)}\\b[^}]*\\}\\s*from`).test(source)) {
    return ["a reexport: prunr reports it where it is declared", path];
  }
  if (name === "default" && /module\.exports\s*=/.test(source)) {
    return ["module.exports, which Knip calls default: prunr checks its properties", path];
  }
  const of = (await why(repo, path)).split("\n");
  if (of.length === 2 && of[1].trim().endsWith("(test entry)")) {
    return ["an export of a test file: tests are not checked for exports", path];
  }
  // `var x = exports.x = ...` read as `x`, or `exports` rebound to `module.exports`.
  if (mentions(source, name) >= 2) return ["used in its own file", path];
  return ["uncategorized", answer.split("\n")[0]];
}

function dependency(answer) {
  const lines = answer.split("\n").map((line) => line.trim());
  const reasons = [
    [/\(production\)$|\(test\)$|\(unreached\)$/, "imported by a file"],
    [/^a tool configured in/, "loaded by a tool's configuration"],
    [/^a framework of/, "loaded by a framework"],
    [/^a script of/, "run by a script"],
    [/^a peer dependency of/, "a peer dependency of a used package"],
    [/^the types of/, "the types of a used package"],
  ];
  for (const [pattern, category] of reasons) {
    const line = lines.slice(1).find((candidate) => pattern.test(candidate));
    if (line) return [category, line];
  }
  return ["uncategorized", lines[0]];
}

async function pool(items, work) {
  const results = [];
  let next = 0;
  const worker = async () => {
    while (next < items.length) {
      const index = next++;
      results[index] = await work(items[index]);
    }
  };
  await Promise.all(Array.from({ length: jobs }, worker));
  return results;
}

const rows = JSON.parse(readFileSync(join(cache, "results.json"), "utf8"));
const items = [];
for (const row of rows) {
  const diffPath = join(cache, "results", row.name, "diff.json");
  if (!existsSync(diffPath)) continue;
  const diff = JSON.parse(readFileSync(diffPath, "utf8"));
  let findings = [];
  try {
    findings = JSON.parse(readFileSync(join(cache, "results", row.name, "prunr.json"), "utf8")).findings;
  } catch {}
  const knipFiles = new Set(diff.files.onlyKnip);
  for (const path of diff.files.onlyKnip) items.push({ repo: row.name, kind: "file", target: path, knipFiles });
  for (const target of diff.exports.onlyKnip) items.push({ repo: row.name, kind: "export", target, findings });
  for (const target of diff.dependencies.onlyKnip) {
    items.push({ repo: row.name, kind: "dependency", target: target.replace(/ \(.*\)$/, ""), shown: target });
  }
}

const triaged = await pool(items, async (item) => {
  // The fixtures rule needs no graph.
  let answer = item.kind === "file" && inTestFixture(item.target) ? "" : await why(item.repo, item.target);
  // A property of `module.exports = X` is `default.name` for prunr and `name` for Knip.
  if (item.kind === "export" && answer.endsWith("is used by no file.")) {
    const at = item.target.lastIndexOf(":");
    const named = `${item.target.slice(0, at)}:default.${item.target.slice(at + 1)}`;
    const retried = await why(item.repo, named);
    if (!retried.endsWith("is used by no file.")) answer = retried;
  }
  const [category, evidence] =
    item.kind === "file"
      ? file(item.repo, item.target, answer, item.knipFiles)
      : item.kind === "export"
        ? await exported(item.repo, item.target, answer, item.findings)
        : dependency(answer);
  return { ...item, category, evidence };
});

const totals = new Map();
for (const item of triaged) {
  const key = `${item.kind}\t${item.category}`;
  totals.set(key, (totals.get(key) ?? 0) + 1);
}
const lines = ["# What only Knip reports", "", "| kind | category | count |", "|---|---|---|"];
for (const [key, count] of [...totals].sort((a, b) => a[0].localeCompare(b[0]) || b[1] - a[1])) {
  const [kind, category] = key.split("\t");
  lines.push(`| ${kind} | ${category} | ${count} |`);
}
lines.push("", "| repo | kind | target | category | evidence |", "|---|---|---|---|---|");
const cell = (text) => String(text).replaceAll("|", "\\|");
for (const item of triaged) {
  if (item.category === "in a test fixtures folder, never reported") continue;
  lines.push(`| ${item.repo} | ${item.kind} | \`${cell(item.shown ?? item.target)}\` | ${item.category} | ${cell(item.evidence)} |`);
}
console.log(lines.join("\n"));
