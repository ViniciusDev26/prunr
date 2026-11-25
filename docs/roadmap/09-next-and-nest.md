# 9. Next.js e NestJS

## Objetivo

Os pontos de entrada vêm da convenção do framework, sem listar cada arquivo no `prunr.json`.

## Escopo

- Um plugin para cada ([item 11](11-plugins.md)), ativado pelo pacote declarado (`next`,
  `@nestjs/core`). A `Contribution` dos plugins ganha entradas e pacotes de produção.
- **Next.js:** `pages/**`, os arquivos do app router (`page`, `layout`, `route`, `loading`,
  `error`, `global-error`, `not-found`, `template`, `default`, imagens de Open Graph, ícones,
  `sitemap`, `robots`, `manifest`), `middleware`, `instrumentation` e `next.config`, também
  dentro de `src/`, são entradas. Como os exports de uma entrada são a interface dela, o que o
  Next.js lê (`default`, `getServerSideProps`, `metadata`, `GET`...) não é reportado sem lista
  especial. `react` e `react-dom` contam como usados.
- **NestJS:** a entrada é o `main` do `nest-cli.json` (`sourceRoot`/`entryFile`, e o de cada
  projeto num monorepo do Nest), `src/main.ts` sem ele. As classes registradas em `@Module` já
  estão no grafo pelos `import`. Testes e2e (`test/**`, `*.e2e-spec.*`) são código de teste;
  `@nestjs/cli`, a coleção de schematics, `reflect-metadata` e `rxjs` contam como usados.

## Critérios de aceite

- [x] Uma fixture Next (app router e pages router) e uma Nest.
- [x] Um provider que nenhum módulo registra é reportado.
- [x] `prunr init` aceita um projeto cujas entradas vêm só do framework.
