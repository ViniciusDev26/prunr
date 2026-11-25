# 17. Os arquivos que os scripts rodam são entradas

## Objetivo

Um projeto cujos pontos de entrada estão nos `scripts` (`tsx ./src/index.ts`, `node
scripts/x.mjs`) roda no prunr sem repetir esses arquivos em `entries.production`. Num monorepo
grande de referência, isso levou os `unused-file` de 2075 para 308.

## Escopo

- Os runners que rodam um arquivo: `node`, `tsx`, `ts-node`, `bun`, `nodemon`, `babel-node`,
  `vite-node`, `jiti` e similares. O arquivo é o primeiro argumento que é um arquivo-fonte do
  projeto (com ou sem extensão); os valores das flags que recebem valor ficam de fora, e
  `-e`/`-p` não rodam arquivo.
- Os arquivos de `-r`/`--import ./x` também são entradas.
- Scripts `test*` rodam código de teste; os outros, de produção.
- Wrappers e gerenciadores: `npx`, `pnpm exec`, `yarn`, `cross-env`, `env`, `dotenv --`, e
  `pnpm --filter`, `npm -w` e `yarn workspace`, que rodam no diretório de outro pacote.
- O `prunr init` deixa de propor esses arquivos, porque o check já os encontra.

## Critérios de aceite

- [x] A fixture `script-entries` cobre cada forma acima, e o argumento que não é o arquivo
  rodado (`node scripts/lint.mjs input.js`) não vira entrada.
