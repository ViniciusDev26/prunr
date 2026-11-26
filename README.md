<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/brand/logo-dark.svg">
    <img src="assets/brand/logo.svg" alt="prunr" width="360">
  </picture>
</p>

<p align="center">
  Dead code finder for Node.js, with CommonJS as a first-class citizen.
</p>

<p align="center">
  <a href="https://www.npmjs.com/package/@prunr/prunr"><img src="https://img.shields.io/npm/v/@prunr/prunr" alt="npm"></a>
  <a href="LICENSE"><img src="https://img.shields.io/npm/l/@prunr/prunr" alt="MIT license"></a>
</p>

<p align="center">
  <a href="https://github.com/ViniciusDev26/prunr/blob/main/README.md"><img src="https://img.shields.io/badge/lang-English-blue" alt="Read in English"></a>
  <a href="https://github.com/ViniciusDev26/prunr/blob/main/README.pt-BR.md"><img src="https://img.shields.io/badge/lang-Portugu%C3%AAs%20(BR)-green" alt="Leia em português"></a>
</p>

---

prunr builds a module graph from your project's entry points and finds what nothing uses: files,
exports, imports and dependencies. From the same graph, it points out undeclared dependencies,
circular imports and `require` calls it cannot follow.

It is one Rust binary, one config file and one command, in place of depcheck + madge + custom
scripts.

- **Real CommonJS:** `module.exports`, `exports.x`, destructured `require`, static methods and
  `require` with a built path (`` require(`./handlers/${name}`) ``).
- **ESM and TypeScript in the same graph**, with `tsconfig.json` `paths` and CJS/ESM interop the
  way Node does it.
- **Knows your stack** with no setup: Jest, Vitest, Mocha, `node --test`, tap, AVA, Playwright,
  Cypress, Storybook, Vite, React, Next.js, NestJS, Nuxt, SvelteKit, Astro, React Router/Remix,
  TypeScript (`tsconfig`), `.vue`/`.svelte`/`.astro`/`.mdx` files, ESLint, Prettier, Babel,
  commitlint, husky, lint-staged, Tailwind, Prisma, tsup, Rollup, webpack, Drizzle Kit, PostCSS
  and monorepos (npm, yarn, pnpm, bun, lerna, rush, nx, turbo).
- **Fixes it for you:** `prunr fix` deletes what is dead and cleans up `package.json`.
- **Built for CI:** annotations on the PR diff, only new findings with `--since`, and a baseline
  for adopting it on legacy code.

## Quick start

```sh
npm i -D @prunr/prunr   # or pnpm add -D / yarn add -D / bun add -d
npx prunr init          # creates a prunr.json from package.json
npx prunr check
```

```console
$ npx prunr check
unused-file           src/legacy-report.js
unused-export         src/invoices.js: cancelInvoice
unused-export         src/money.js: Money.fromCents
unused-export         src/money.js: Money.invoiceFor
unused-dependency     left-pad
missing-dependency    src/index.js: dotenv
circular-import       src/invoices.js -> src/money.js -> src/invoices.js
unresolved-require    src/index.js: require(process.env.PLUGIN) (warning)

errors: 7, warnings: 1
```

(That is the output for the project in [`tests/fixtures/example/`](tests/fixtures/example/).)

The package downloads only the binary for your platform (Linux x64/arm64, macOS x64/arm64,
Windows x64), with no install script. The binaries are also attached to every
[release](https://github.com/ViniciusDev26/prunr/releases), each with a `.sha256`.

## Configuration

`prunr.json` says where the program starts. Everything those entries reach is alive; what only
the tests reach is dead in production.

```json
{
  "$schema": "https://unpkg.com/@prunr/prunr/schema.json",
  "entries": {
    "production": ["src/index.js", "bin/cli.js"],
    "test": ["test/**"]
  }
}
```

The files the `scripts` of `package.json` run (`node src/server.js`, `tsx watch src/index.ts`)
are entries already, and with Jest, Vitest, Vite, Next.js or NestJS prunr finds most of the rest
on its own. Every option is in [Configuration](docs/guide/configuration.md).

## Commands

| Command | What it does |
|---|---|
| `prunr check` | Lists the findings. `--json` for machines, `--github` for PR annotations, `--sarif` for code scanning, `--codeclimate` for GitLab Code Quality, `--markdown` for a PR comment, `--since <ref>` for only what is new, `--production` for only what ships. |
| `prunr fix` | Removes dead code and unused dependencies. `--dry-run` shows the changes without making them. |
| `prunr why <target>` | Explains a file (the chain of imports from an entry), a `file:export` (who uses it) or a package (the files, scripts and tools that need it). |
| `prunr init` | Creates a `prunr.json` from `package.json`. |
| `prunr baseline` | Records the current findings, to adopt prunr without fixing everything at once. |
| `prunr schema` | Prints the JSON Schema of `prunr.json`. |

`check` exits with `0` when there are no findings (or only warnings), `1` when there are
findings, and `2` when it cannot run.

## What it finds

| Finding | When |
|---|---|
| `unused-file` | No production entry reaches the file. |
| `unused-export` | No production code uses the export. |
| `unused-import` | A `require` or `import` brings in a name the file never uses. |
| `unused-dependency` | Nothing imports the package and no script runs it. |
| `misplaced-dependency` | The package is in `dependencies`, but only the tests use it. |
| `missing-dependency` | The code imports a package `package.json` does not declare. |
| `circular-import` | There is an import cycle. |
| `unused-ignore` | A `prunr-ignore` comment that no longer ignores anything. |
| `unresolved-require` | Warning: a `require` prunr cannot follow. |
| `unparsed-file` | Warning: a file the graph reaches does not parse, so what it uses is unknown. |

When the only use comes from the tests, the finding says `(only tests use it)`. The details of
each are in [Findings](docs/guide/checks.md).

## In CI

```yaml
- uses: actions/checkout@v4
  with:
    fetch-depth: 0
- run: npm ci
- run: npx prunr check --github --since origin/${{ github.base_ref }}
```

Each new finding becomes an annotation on the right line of the diff. For legacy code, start
with `prunr baseline`: what is already there does not fail, and anything that shows up later
does. See [CI and legacy code](docs/guide/ci.md).

## Guide

The guide is in Portuguese for now.

- [Configuration](docs/guide/configuration.md): `prunr.json` and the `scripts` of `package.json`.
- [Findings](docs/guide/checks.md): what each one means, ESM and TypeScript, dynamic `require`.
- [`prunr fix`](docs/guide/fix.md): what it removes and what it leaves to you.
- [Frameworks and tools](docs/guide/frameworks.md): test runners, frameworks, single-file
  components and the tool configs prunr reads.
- [Monorepos](docs/guide/monorepos.md): how packages are found and checked.
- [Ignoring in code](docs/guide/ignore.md): exports that are public on purpose.
- [CI and legacy code](docs/guide/ci.md): annotations, `--since` and the baseline.

## What it leaves out

- **Unused variables and private members inside a file** are a linter's job. Turn on
  `noUnusedVariables` and `noUnusedPrivateClassMembers` in [Biome](https://biomejs.dev/):
  together with prunr's `unused-import`, they cover what `tsc`'s `noUnusedLocals` catches.
- **Formatting, style and types:** use Biome and `tsc --noEmit`.
- **Dependencies named only in the config of a tool prunr does not know** show up as
  `unused-dependency`. List them in `dependencies.ignore`.

What comes next is in the [roadmap](docs/roadmap/).

## Why prunr, and not Knip or other tools

[Knip](https://knip.dev) is the reference for ESM and TypeScript, and tools like `ts-prune` and
`unimported` were archived pointing to it. prunr exists for the case Knip treats as secondary:
**Node.js code in CommonJS**, alone or mixed with ESM.

**CommonJS as it is actually written.** Knip asks code to follow a convention so it can
understand the exports: according to its
[CommonJS guide](https://knip.dev/guides/working-with-commonjs), only `module.exports.x =` and an
object with shorthand keys only (`{ a, b }`) are named exports; any other object in
`module.exports` becomes a single default export, and importing the whole module makes every
named export show up as unused. prunr reads the code you already have, without asking you to
change it:

```js
// invoices.js
module.exports = {
  total,
  cancel: () => {},
  format(value) { return value; },
};

// index.js
const invoices = require("./invoices");
invoices.format(invoices.total());
const handler = require(`./handlers/${name}`);
```

prunr reports only `cancel`: it reads each key of the object, follows the use through
`invoices.x`, and the `require` built at runtime keeps the files in `handlers/` alive. When in
doubt (the module passed along, a computed key), it treats everything as used: it is better to
let a dead export through than to tell you to delete a live one.

This is not a hypothetical case. prunr was born in a production Node.js project, all in
CommonJS, that adopted Knip, fixed its entry points, deleted what it pointed out, and ended up
removing it because of too many false positives. In the same project, prunr pointed out 26 real
problems (dead files and exports, unused dependencies and a queue shutdown that was never
called), all of them fixed, and it now runs in CI with no pending findings.

**One tool in place of several.** Unused files, exports, imports and dependencies, undeclared
or misplaced dependencies and circular imports all come out of the same graph, in one command.
That is the work usually split across `depcheck`, `madge`, `unimported` and custom scripts.

**Fast, and it runs nothing.** It is a native binary, written in Rust, that reads and resolves
files in parallel; the npm package only downloads the binary for your platform, with no install
script. On a monorepo of 13,000 files, the check takes 0.6 s. Tool configs (Jest, Vitest,
Playwright, Vite...) are read as text, never executed.

**Built to adopt on existing code.** A baseline so you do not have to fix everything at once,
`--since` and diff annotations to block only what is new in the PR, and a `prunr fix` that
deletes dead code and cleans up `package.json`.

**When Knip is the better choice.** On a project that is only ESM and TypeScript, Knip has many
more plugins (prunr knows about 40 tools and frameworks; Knip, over a hundred) and more mileage. prunr pays off when there is CommonJS in the mix, or when speed
and a single tool matter more.

## Contributing

prunr is built with TDD and 100% test coverage. See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

[MIT](LICENSE)
