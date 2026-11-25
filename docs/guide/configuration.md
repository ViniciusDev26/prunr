# Configuração

Crie um `prunr.json` na raiz do projeto (`prunr init` gera um primeiro, a partir do
`package.json`) e rode `prunr check` nela.

```json
{
  "$schema": "https://unpkg.com/@prunr/prunr/schema.json",
  "entries": {
    "production": ["src/index.js", "bin/cli.js"],
    "test": ["test/**", "**/*.test.js"]
  },
  "dependencies": {
    "ignore": ["eslint-plugin-import"]
  },
  "baseline": ".prunr-baseline.json"
}
```

## Opções

| Opção | Obrigatória | O que é |
|---|---|---|
| `entries.production` | sim | Caminhos ou globs dos arquivos por onde o programa começa. Tudo o que eles alcançam, direta ou indiretamente, está vivo. Os arquivos que os `scripts` rodam já são entradas, e os de Vite, Next.js e NestJS também ([Frameworks](frameworks.md)): liste só o resto, ou deixe vazio (`[]`). |
| `$schema` | não | O JSON Schema do arquivo, para autocompletar e validar no editor. |
| `entries.test` | não | Caminhos ou globs do código de teste, incluindo helpers e mocks. Código de teste nunca é reportado, e o que só ele alcança está morto em produção. Com Jest ou Vitest, liste só o que o runner não acha sozinho. |
| `dependencies.ignore` | não | Pacotes que ficam de fora das verificações de dependência: os carregados de um jeito que nem o grafo nem os `scripts` mostram (um plugin citado no arquivo de configuração de outra ferramenta). |
| `baseline` | não | Arquivo de baseline, relativo à raiz. Veja [CI e código legado](ci.md#adotando-num-código-legado). |

Caminhos e globs são relativos à raiz; num glob, `*` não atravessa `/` e `**` atravessa. Cada
entrada precisa casar com pelo menos um arquivo-fonte (`.js`, `.cjs`, `.mjs`, `.jsx`, `.ts`, `.cts`,
`.mts` ou `.tsx`).

Uma opção só entra no arquivo quando alguma funcionalidade precisa dela. Uma chave desconhecida é
erro, para que um erro de digitação não passe em silêncio.

## Os `scripts` do `package.json`

### Os arquivos que eles rodam

O arquivo que um script roda com `node`, `tsx`, `ts-node`, `bun`, `nodemon`, `babel-node`,
`vite-node` ou `jiti` é uma entrada, sem listá-lo no `prunr.json`:

```json
{
  "scripts": {
    "start": "NODE_ENV=production node --env-file .env -r ./src/tracing.js src/server.js",
    "dev": "tsx watch ./src/server.ts",
    "seed": "cross-env SIZE=3 ts-node src/seed",
    "gen": "pnpm --filter @app/tools exec tsx src/gen.ts",
    "test:e2e": "node --test e2e/run.js"
  }
}
```

- O arquivo é o primeiro argumento que é um arquivo-fonte do projeto, com ou sem extensão; os
  argumentos seguintes são do programa. Os arquivos carregados com `-r`/`--import ./x` também
  contam.
- Os scripts cujo nome começa com `test` rodam código de teste; os outros, código de produção.
- `npx`, `pnpm exec`, `yarn`, `cross-env`, `env` e `dotenv --` são atravessados. `pnpm --filter
  <pacote>`, `npm -w <pacote>` e `yarn workspace <pacote>` rodam o arquivo no diretório daquele
  pacote do monorepo.
- `node -e` e `node -p` não rodam arquivo.

### Os pacotes que eles usam

Os `scripts` também usam pacotes, e o prunr os lê:

- o binário de cada comando (`jest`, `tsc`, `npx nodemon`, `cross-env NODE_ENV=test jest`,
  `dotenv -- jest`) conta como uso do pacote que o declara no campo `bin` (lido em
  `node_modules`; sem ele, vale o nome do pacote);
- `node -r dotenv/config` ou `--import tsx` conta como uso do pacote carregado;
- um caminho dentro de `node_modules` (`node node_modules/jest/bin/jest.js`) conta como uso do
  pacote dele.

Um pacote usado por script não é `unused-dependency` nem `misplaced-dependency`, e um binário sem
pacote declarado (`docker`, `git`) não é `missing-dependency`.

## Códigos de saída

| Código | Significado |
|---|---|
| `0` | Nenhum achado, ou só avisos. |
| `1` | Há achados novos, ou entradas obsoletas no baseline. |
| `2` | O comando não pôde rodar: `prunr.json` ausente ou inválido, `package.json` inválido, arquivo com erro de sintaxe ou argumento desconhecido. |
