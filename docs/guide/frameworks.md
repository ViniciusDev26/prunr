# Frameworks

Cada framework é ligado sozinho quando o pacote dele está declarado no `package.json`. O prunr lê
as configurações sem executá-las.

## Jest e Vitest

Quando `jest` ou `vitest` está declarado, o prunr lê a configuração do runner (`jest.config.*`, a
chave `jest` do `package.json`, `vitest.config.*` ou `vite.config.*`):

- **Arquivos de teste:** os de `testMatch` (Jest) ou `test.include` (Vitest), ou os padrões de
  cada runner quando a opção não existe, mais as pastas `__mocks__` do Jest. Eles se somam a
  `entries.test`.
- **Configurações passadas por script:** `jest --config jest.e2e.config.js` (ou `-c`, também
  com `node node_modules/jest/bin/jest.js`) é lida como as outras.
- **Arquivos carregados:** a própria configuração e os arquivos de `setupFiles`,
  `setupFilesAfterEnv`, `globalSetup` e `globalTeardown` (Jest) ou `test.setupFiles` e
  `test.globalSetup` (Vitest) são código de teste.
- **Pacotes citados como texto** (`preset`, `testEnvironment`, `transform`, `reporters`,
  `snapshotSerializers`, `watchPlugins`; no Vitest, `test.environment`, `test.reporters` e
  `test.coverage.provider`) contam como uso de teste: não são `unused-dependency` e, se estiverem
  em `dependencies`, são `misplaced-dependency`.

Só valores literais são lidos. Uma configuração montada em tempo de execução (uma função, uma
variável importada) cai nos padrões do runner; cubra o resto com `entries.test`. `testRegex`
não é lido.

## Next.js

Com `next` declarado, as rotas são entradas de produção por convenção, sem listá-las no
`prunr.json`: `pages/**`, os arquivos do app router (`page`, `layout`, `route`, `loading`,
`error`, `not-found`, `template`, `default`, `sitemap`, `robots`...), `middleware`,
`instrumentation` e `next.config`, também dentro de `src/`.

Os exports de uma entrada são a interface dela, então o que o Next.js lê (`default`,
`getServerSideProps`, `metadata`, `generateStaticParams`, `GET`...) nunca é reportado. `react` e
`react-dom` contam como usados, mesmo sem `import React`.

## NestJS

Com `@nestjs/core` declarado, a entrada é o `main` que o CLI do Nest inicia (`src/main.ts`, ou o
`sourceRoot`/`entryFile` de cada projeto do `nest-cli.json`). As classes que um `@Module`
registra já estão no grafo pelos `import` do módulo.

Os testes e2e (`test/**`, `*.e2e-spec.*`) são código de teste. `@nestjs/cli` e a coleção de
schematics contam como usados, e `reflect-metadata` e `rxjs` também, porque o Nest os exige sem
que o código os importe.

Num projeto Next.js ou NestJS, `entries.production` pode ficar vazio (`[]`).
