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
- **Entende o seu stack** sem configuração: Jest, Vitest, Next.js, NestJS e monorepos (npm, yarn,
  pnpm, bun, lerna, rush, nx, turbo).
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

Com Jest, Vitest, Next.js ou NestJS, boa parte disso o prunr acha sozinho. Todas as opções estão
em [Configuração](docs/guide/configuration.md).

## Comandos

| Comando | O que faz |
|---|---|
| `prunr check` | Lista os achados. `--json` para máquinas, `--github` para anotações no PR, `--since <ref>` para só o que é novo. |
| `prunr fix` | Remove o código morto e as dependências sem uso. `--dry-run` mostra sem mudar nada. |
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
- [Frameworks](docs/guide/frameworks.md): Jest, Vitest, Next.js e NestJS.
- [Monorepos](docs/guide/monorepos.md): como os pacotes são encontrados e checados.
- [Ignorar no código](docs/guide/ignore.md): exports públicos de propósito.
- [CI e código legado](docs/guide/ci.md): anotações, `--since` e baseline.

## O que fica de fora

- **Variáveis e membros privados sem uso dentro de um arquivo** são trabalho de linter. Ligue
  `noUnusedVariables` e `noUnusedPrivateClassMembers` no [Biome](https://biomejs.dev/): junto com
  o `unused-import` do prunr, elas cobrem o que o `noUnusedLocals` do `tsc` pega.
- **Formatação, estilo e tipos:** use Biome e `tsc --noEmit`.
- **Dependências citadas só na configuração de outra ferramenta** (plugins do ESLint, presets do
  Babel) aparecem como `unused-dependency`. Liste-as em `dependencies.ignore`.

O que vem depois está no [roadmap](docs/roadmap/).

## Por que não o Knip

O Knip é ótimo para ESM e TypeScript, mas não lida bem com CommonJS, e suportá-lo não está nos
planos dele. O prunr começou pelo caminho oposto: Node puro em CommonJS primeiro, e ESM,
TypeScript e frameworks depois, no mesmo grafo.

## Contribuindo

O prunr é feito com TDD e 100% de cobertura de testes. Veja [CONTRIBUTING.md](CONTRIBUTING.md).

## Licença

[MIT](LICENSE)
