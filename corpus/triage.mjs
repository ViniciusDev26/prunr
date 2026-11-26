#!/usr/bin/env node
// Triage of the corpus: every `missing-dependency` and `unresolved-require` of the last run,
// one by one, with the category it falls in and the evidence on disk for it.
//
//   node corpus/run.mjs && node corpus/triage.mjs > triage.md
//
// Reads $PRUNR_CORPUS_DIR (default: ~/.cache/prunr-corpus).

import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { homedir } from "node:os";
import { dirname, join, relative } from "node:path";

const cache = process.env.PRUNR_CORPUS_DIR ?? join(homedir(), ".cache", "prunr-corpus");
const rows = JSON.parse(readFileSync(join(cache, "results.json"), "utf8"));

/** Every package.json of a repository, outside node_modules, by directory. */
function manifests(root) {
  const found = new Map();
  const walk = (dir) => {
    for (const entry of readdirSync(dir, { withFileTypes: true })) {
      if (entry.name === "node_modules" || entry.name === ".git") continue;
      const path = join(dir, entry.name);
      if (entry.isDirectory()) walk(path);
      else if (entry.name === "package.json") {
        try {
          found.set(relative(root, dir), JSON.parse(readFileSync(path, "utf8").replace(/^\uFEFF/, "")));
        } catch {}
      }
    }
  };
  walk(root);
  return found;
}

/** The paths of the git submodules of a repository. */
function submodules(root) {
  try {
    const text = readFileSync(join(root, ".gitmodules"), "utf8");
    return [...text.matchAll(/^\s*path\s*=\s*(.+)$/gm)].map((match) => match[1].trim());
  } catch {
    return [];
  }
}

const fields = ["dependencies", "devDependencies", "peerDependencies", "optionalDependencies"];
const extensions = ["", ".js", ".cjs", ".mjs", ".jsx", ".ts", ".cts", ".mts", ".tsx", ".d.ts", ".json", "/index.js", "/index.ts"];
const isFile = (path) => existsSync(path) && statSync(path).isFile();

function missing(root, packages, finding) {
  const name = finding.package;
  const sibling = [...packages].find(([, manifest]) => manifest.name === name);
  if (sibling) {
    return ["sibling package, undeclared", `${sibling[0] || "."}/package.json is "${name}"`];
  }
  const declarer = [...packages].find(([, manifest]) =>
    fields.some((field) => manifest[field] && name in manifest[field]),
  );
  if (declarer) {
    return ["declared by another package of the repository", `${declarer[0] || "."}/package.json`];
  }
  if (name.startsWith("@std/") || existsSync(join(root, "deno.json"))) {
    return ["Deno/JSR import", "no package.json declares it; a Deno import map does"];
  }
  const local = [...walkNames(root)].find((file) => file.replace(/\.[^.]+$/, "") === name);
  if (local) {
    return ["local module a tool alias resolves", `a source file named ${local} exists`];
  }
  return ["undeclared", "no package.json of the repository declares it"];
}

function* walkNames(root) {
  const stack = [root];
  while (stack.length > 0) {
    const dir = stack.pop();
    for (const entry of readdirSync(dir, { withFileTypes: true })) {
      if (entry.name === "node_modules" || entry.name === ".git") continue;
      if (entry.isDirectory()) stack.push(join(dir, entry.name));
      else yield entry.name;
    }
  }
}

function unresolved(root, finding) {
  const specifier = finding.specifier;
  if (/^(require|import)\(/.test(specifier) || specifier.includes("`") || specifier.includes("${")) {
    return ["computed path", "the path is built at runtime"];
  }
  if (/(^|\/)(dist|build|built|generated|lib|out|cjs|esm)(\/|$)/.test(specifier) || specifier.includes("package-info")) {
    return ["build output", "the folder a build writes to"];
  }
  if (specifier.includes("!")) {
    return ["bundler loader syntax", "webpack's inline loader (`loader!file`)"];
  }
  const base = join(root, dirname(finding.file), specifier);
  const submodule = submodules(root).find((path) => relative(root, base).startsWith(path));
  if (submodule) {
    return ["git submodule not cloned", `${submodule} is a submodule; the corpus clones shallow`];
  }
  const target = extensions.map((extension) => base + extension).find(isFile);
  if (target) {
    return ["exists, not a source file", relative(root, target)];
  }
  if (!base.startsWith(root)) {
    return ["outside the repository", relative(root, base)];
  }
  return ["does not exist", `nothing at ${relative(root, base)}`];
}

const lines = ["# Corpus triage", ""];
const totals = new Map();
const table = ["| repo | finding | file | target | category | evidence |", "|---|---|---|---|---|---|"];
for (const row of rows) {
  const root = join(cache, "repos", row.name);
  let findings;
  try {
    findings = JSON.parse(readFileSync(join(cache, "results", row.name, "prunr.json"), "utf8")).findings;
  } catch {
    continue;
  }
  const packages = manifests(root);
  for (const finding of findings) {
    let category;
    let evidence;
    if (finding.kind === "missing-dependency") [category, evidence] = missing(root, packages, finding);
    else if (finding.kind === "unresolved-require") [category, evidence] = unresolved(root, finding);
    else continue;
    const key = `${finding.kind}: ${category}`;
    totals.set(key, (totals.get(key) ?? 0) + 1);
    const target = finding.package ?? finding.specifier;
    table.push(`| ${row.name} | ${finding.kind} | ${finding.file} | \`${target.replaceAll("|", "\\|")}\` | ${category} | ${evidence} |`);
  }
}
lines.push("| finding: category | count |", "|---|---|");
for (const [key, count] of [...totals].sort()) lines.push(`| ${key} | ${count} |`);
lines.push("", ...table);
console.log(lines.join("\n"));
