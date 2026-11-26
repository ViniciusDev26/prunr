# 27. Plugin de TypeScript

## Objetivo

Um projeto TypeScript usa pacotes que o código nunca importa: o `tsconfig.json` os carrega. Sem
ler o `tsconfig`, o prunr os reporta como sem uso (o `@sindresorhus/tsconfig` do Got) e deixa
sem dono os testes de tipo (o `test-d/` do execa).

## Feito

Com `typescript` declarado ou um `tsconfig.json` na raiz, o plugin `typescript` lê os
`tsconfig*.json` da raiz, os que os scripts passam a `tsc -p`/`--project` e, a partir deles, os
que `extends` e `references` apontam (JSON com comentários e vírgula sobrando):

- `extends` com nome de pacote (texto ou lista): o pacote conta como usado;
- `compilerOptions.types`: `node` conta como `@types/node`, e `vitest/globals` como `vitest`;
- `compilerOptions.jsxImportSource`: o pacote do runtime de JSX;
- `compilerOptions.plugins[].name`: os plugins do language service.

Os testes de tipo (`test-d/**`, `*.test-d.*`) são arquivos de teste.

## Critérios de aceite

- A fixture `typescript-config` só reporta a dependência que nada usa.
