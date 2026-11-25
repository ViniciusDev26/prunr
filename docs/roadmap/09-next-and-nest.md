# 9. Next.js e NestJS

## Objetivo

Os pontos de entrada vêm da convenção do framework, sem listar cada arquivo no `prunr.json`.

## Escopo

- **Next.js:** `pages/**`, `app/**/{page,layout,route,loading,error,not-found,template,default}.*`,
  `middleware.*`, `instrumentation.*` e `next.config.*` são entradas. Os exports que o Next lê por
  convenção (`default`, `getServerSideProps`, `generateMetadata`, `metadata`, `revalidate` etc.)
  não são reportados.
- **NestJS:** `main.ts` é a entrada; classes registradas em `@Module({ imports, controllers,
  providers })` são usadas mesmo que nenhum código as chame diretamente.
- Um plugin para cada um ([item 11](11-plugins.md)), ativado pelo pacote declarado (`next`,
  `@nestjs/core`), como os de Jest e Vitest.
- Depende do [item 1](01-esm-and-typescript.md).

## Critérios de aceite

- [ ] Uma fixture Next (app router e pages router) e uma Nest.
- [ ] Um provider que nenhum módulo registra é reportado.
