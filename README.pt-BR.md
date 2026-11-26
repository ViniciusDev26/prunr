<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/brand/logo-dark.svg">
    <img src="assets/brand/logo.svg" alt="prunr" width="360">
  </picture>
</p>

<p align="center">
  Detector de código morto para Node.js, com CommonJS como cidadão de primeira classe.
</p>

<p align="center">
  <a href="https://www.npmjs.com/package/@prunr/prunr"><img src="https://img.shields.io/npm/v/@prunr/prunr" alt="npm"></a>
  <a href="LICENSE"><img src="https://img.shields.io/npm/l/@prunr/prunr" alt="licença MIT"></a>
</p>

<p align="center">
  <a href="https://github.com/ViniciusDev26/prunr/blob/main/README.md"><img src="https://img.shields.io/badge/lang-English-blue" alt="Read in English"></a>
  <a href="https://github.com/ViniciusDev26/prunr/blob/main/README.pt-BR.md"><img src="https://img.shields.io/badge/lang-Portugu%C3%AAs%20(BR)-green" alt="Leia em português"></a>
</p>

---

O prunr monta um grafo de módulos a partir dos pontos de entrada do projeto e encontra o que
nada usa: arquivos, exports, imports e dependências. No mesmo grafo, ele aponta dependências não
declaradas, imports circulares e `require` que não dá para seguir.

É um binário em Rust, um arquivo de configuração e um comando, no lugar da combinação de
depcheck + madge + scripts próprios.

- **CommonJS de verdade:** `module.exports`, `exports.x`, desestruturação de `require`, métodos
  estáticos e `require` com caminho montado (`` require(`./handlers/${nome}`) ``).
- **ESM e TypeScript no mesmo grafo**, com `paths` do `tsconfig.json` e interop CJS/ESM como no
  Node.
- **Entende o seu stack** sem configuração: Jest, Vitest, Mocha, `node --test`, tap, AVA,
  Playwright, Cypress, Storybook, Vite, React, Next.js, NestJS, Nuxt, SvelteKit, Astro, React
  Router/Remix, TypeScript (`tsconfig`), arquivos `.vue`/`.svelte`/`.astro`/`.mdx`, ESLint,
  Prettier, Babel, commitlint, husky, lint-staged, Tailwind, Prisma, tsup, Rollup, webpack,
  Drizzle Kit, PostCSS e monorepos (npm, yarn, pnpm, bun, lerna, rush, nx, turbo).
- **Conserta sozinho:** `prunr fix` apaga o que está morto e limpa o `package.json`.
- **Feito para CI:** anotações no diff do PR, só os achados novos com `--since`, e um baseline
  para adotar em código legado.

## Início rápido

```sh
npm i -D @prunr/prunr   # ou pnpm add -D / yarn add -D / bun add -d
npx prunr init          # cria um prunr.json a partir do package.json
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

(É a saída do projeto em [`tests/fixtures/example/`](tests/fixtures/example/).)

O pacote baixa só o binário da sua plataforma (Linux x64/arm64, macOS x64/arm64, Windows x64),
sem script de instalação. Os binários também estão em cada
[release](https://github.com/ViniciusDev26/prunr/releases), com `.sha256`.

## Configuração

O `prunr.json` diz por onde o programa começa. Tudo o que essas entradas alcançam está vivo; o
que só os testes alcançam está morto em produção.

```json
{
  "$schema": "https://unpkg.com/@prunr/prunr/schema.json",
  "entries": {
    "production": ["src/index.js", "bin/cli.js"],
    "test": ["test/**"]
  }
}
```

Os arquivos que os `scripts` do `package.json` rodam (`node src/server.js`, `tsx watch
src/index.ts`) já são entradas, e com Jest, Vitest, Vite, Next.js ou NestJS boa parte do resto o
prunr acha sozinho. Todas as opções estão
em [Configuração](docs/guide/configuration.md).

## Comandos

| Comando | O que faz |
|---|---|
| `prunr check` | Lista os achados. `--json` para máquinas, `--github` para anotações no PR, `--sarif` para code scanning, `--codeclimate` para o Code Quality do GitLab, `--markdown` para um comentário de PR, `--since <ref>` para só o que é novo, `--production` para só o que vai para produção. |
| `prunr fix` | Remove o código morto e as dependências sem uso. `--dry-run` mostra sem mudar nada. |
| `prunr why <alvo>` | Explica um arquivo (a cadeia de imports desde uma entrada), um `arquivo:export` (quem o usa) ou um pacote (os arquivos, scripts e ferramentas que precisam dele). |
| `prunr init` | Cria um `prunr.json` a partir do `package.json`. |
| `prunr baseline` | Grava os achados atuais, para adotar o prunr sem consertar tudo de uma vez. |
| `prunr schema` | Imprime o JSON Schema do `prunr.json`. |

O `check` sai com `0` sem achados (ou só com avisos), `1` com achados e `2` quando não consegue
rodar.

## O que ele encontra

| Achado | Quando |
|---|---|
| `unused-file` | Nenhuma entrada de produção alcança o arquivo. |
| `unused-export` | Nenhum código de produção usa o export. |
| `unused-import` | Um `require` ou `import` traz um nome que o arquivo nunca usa. |
| `unused-dependency` | Nada importa o pacote e nenhum script o roda. |
| `misplaced-dependency` | O pacote está em `dependencies`, mas só os testes o usam. |
| `missing-dependency` | O código importa um pacote que o `package.json` não declara. |
| `circular-import` | Há um ciclo de imports. |
| `unused-ignore` | Um comentário `prunr-ignore` que não ignora mais nada. |
| `unresolved-require` | Aviso: um `require` que não dá para seguir. |
| `unparsed-file` | Aviso: um arquivo que o grafo alcança não faz parse, então o que ele usa é desconhecido. |

Quando o único uso vem dos testes, o achado diz `(only tests use it)`. Os detalhes de cada um
estão em [Achados](docs/guide/checks.md).

## No CI

```yaml
- uses: actions/checkout@v4
  with:
    fetch-depth: 0
- run: npm ci
- run: npx prunr check --github --since origin/${{ github.base_ref }}
```

Cada achado novo vira uma anotação na linha certa do diff. Para um código legado, comece com
`prunr baseline`: o que já existe não falha, e o que surgir depois falha. Veja
[CI e código legado](docs/guide/ci.md).

## Guia

- [Configuração](docs/guide/configuration.md): o `prunr.json` e os `scripts` do `package.json`.
- [Achados](docs/guide/checks.md): o que cada um significa, ESM e TypeScript, `require` dinâmico.
- [`prunr fix`](docs/guide/fix.md): o que ele remove e o que deixa para você.
- [Frameworks e ferramentas](docs/guide/frameworks.md): test runners, frameworks, componentes de
  arquivo único e as configurações de ferramentas que o prunr lê.
- [Monorepos](docs/guide/monorepos.md): como os pacotes são encontrados e checados.
- [Ignorar no código](docs/guide/ignore.md): exports públicos de propósito.
- [CI e código legado](docs/guide/ci.md): anotações, `--since` e baseline.

## O que fica de fora

- **Variáveis e membros privados sem uso dentro de um arquivo** são trabalho de linter. Ligue
  `noUnusedVariables` e `noUnusedPrivateClassMembers` no [Biome](https://biomejs.dev/): junto com
  o `unused-import` do prunr, elas cobrem o que o `noUnusedLocals` do `tsc` pega.
- **Formatação, estilo e tipos:** use Biome e `tsc --noEmit`.
- **Dependências citadas só na configuração de uma ferramenta que o prunr não conhece** aparecem
  como `unused-dependency`. Liste-as em `dependencies.ignore`.

O que vem depois está no [roadmap](docs/roadmap/).

## Por que o prunr, e não o Knip ou outras ferramentas

O [Knip](https://knip.dev) é a referência para ESM e TypeScript, e ferramentas como o
`ts-prune` e o `unimported` foram arquivadas recomendando ele. O prunr existe para o caso que o
Knip trata como secundário: **código Node.js em CommonJS**, sozinho ou misturado com ESM.

**CommonJS escrito como ele é escrito de verdade.** O Knip pede que o código siga uma convenção
para entender os exports: pelo [guia de CommonJS dele](https://knip.dev/guides/working-with-commonjs),
só `module.exports.x =` e um objeto com chaves curtas (`{ a, b }`) são exports nomeados;
qualquer outro objeto em `module.exports` vira um único export default, e importar o módulo
inteiro faz todos os exports nomeados aparecerem como sem uso. O prunr lê o código existente,
sem pedir que ele mude:

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

O prunr reporta só `cancel`: lê cada chave do objeto, segue o uso por `invoices.x`, e o
`require` montado em tempo de execução mantém vivos os arquivos de `handlers/`. Na dúvida (o
módulo passado adiante, uma chave computada), ele considera tudo usado: é melhor deixar passar
um export morto do que mandar apagar um vivo.

Não é um caso teórico. O prunr nasceu num projeto Node.js de produção, todo em CommonJS, que
adotou o Knip, ajustou os entry points, apagou o que ele apontou, e acabou removendo a ferramenta
por excesso de falsos positivos. No mesmo projeto, o prunr apontou 26 problemas reais (arquivos e
exports mortos, dependências sem uso e um encerramento de fila que nunca era chamado), todos
resolvidos, e hoje roda no CI sem achados pendentes.

**Uma ferramenta no lugar de várias.** Arquivos, exports, imports e dependências sem uso,
dependências não declaradas ou no lugar errado e imports circulares saem do mesmo grafo, num
comando. É o trabalho que costuma ficar dividido entre `depcheck`, `madge`, `unimported` e
scripts próprios.

**Rápido e sem executar nada.** É um binário nativo, em Rust, que lê e resolve os arquivos em
paralelo; o pacote npm só baixa o binário da sua plataforma, sem script de instalação. Num
monorepo de 13 mil arquivos, o check leva 0,6 s. As configurações das ferramentas (Jest,
Vitest, Playwright, Vite...) são lidas como texto, nunca executadas.

**Feito para adotar num código que já existe.** Baseline para não ter que resolver tudo de uma
vez, `--since` e anotações no diff para barrar só o que é novo no PR, e um `prunr fix` que apaga
o código morto e limpa o `package.json`.

**Quando o Knip é a melhor escolha.** Num projeto só em ESM e TypeScript, o Knip tem bem mais
plugins (o prunr conhece uns 40 frameworks e ferramentas; o Knip, mais de cem) e mais tempo de
estrada. O prunr vale a pena quando há CommonJS no meio, ou quando a
velocidade e uma ferramenta só pesam mais.

## Contribuindo

O prunr é feito com TDD e 100% de cobertura de testes. Veja [CONTRIBUTING.md](CONTRIBUTING.md).

## Licença

[MIT](LICENSE)
