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

- **Nomes que o Jest resolve sozinho:** o `moduleNameMapper` (`"^@/(.*)$": "<rootDir>/src/$1"`,
  `"\\.css$": "identity-obj-proxy"`) e os arquivos das pastas `__mocks__`, cujo nome vira o do
  módulo (`require("createRuntime")` chega em `__mocks__/createRuntime.js`; o mock de um pacote
  declarado continua sendo do pacote).

Só valores literais são lidos. Uma configuração montada em tempo de execução (uma função, uma
variável importada) cai nos padrões do runner; cubra o resto com `entries.test`. `testRegex`
não é lido.

## Mocha

Com `mocha` declarado, o prunr lê a configuração do Mocha (`.mocharc.js`, `.cjs`, `.yaml`,
`.yml`, `.jsonc` ou `.json`, a chave `mocha` do `package.json`, ou a que um script passa com
`--config`) e a linha de comando de cada script que roda `mocha`:

- **Arquivos de teste:** o `spec` da configuração e os caminhos (pastas, arquivos, globs) que o
  script passa; sem nenhum, o padrão do Mocha (`test/*`, ou `test/**` com `recursive`).
- **Carregados antes:** `require` e `file` (também `-r`/`--require` e `--file` no script) são
  código de teste quando são arquivos do projeto, e uso de teste quando são pacotes
  (`ts-node/register`, `should`).
- **Reporters** que não vêm com o Mocha contam como uso de teste.

Os argumentos que um script repassa a outro (`npm run unit -- --config x`) contam como do script
que roda o Mocha, e no repositório do próprio Mocha o arquivo do `bin` (`node bin/mocha.js`) é o
`mocha`. O mesmo vale para os runners abaixo.

## `node --test`, tap, AVA, lab e borp

Um script que roda `node --test`, `tap`, `ava`, `lab` ou `borp` (os quatro últimos quando o
pacote está declarado) roda como teste os caminhos que passa: pastas, arquivos e globs. Sem
nenhum, valem os padrões de cada runner (no `node --test`, `*.test.*`, `*-test.*`, `test.*` e
`test/**`, entre outros). Um argumento só conta como caminho se existe ou tem glob, então o
valor de uma flag (`--import tsx`) não esconde os padrões. O `-a` do lab (a biblioteca de
asserções) conta como uso.

### Vitest com `projects`

Os itens de `test.projects` são lidos também: um arquivo de configuração, uma pasta (ou um glob
de pastas, `packages/*`) com o seu `vitest.config.*` ou `vite.config.*`, ou um projeto escrito
na própria lista (`{ test: { include: [...] } }`). Os globs e os arquivos de setup de cada
projeto são relativos à pasta da configuração dele, e o `include` parte do `test.dir` quando ele
existe.

## Playwright

Com `@playwright/test` declarado, o prunr lê o `playwright.config.*` (ou o que um script passa
com `-c`/`--config`):

- **Arquivos de teste:** tudo o que está no `testDir` (helpers, fixtures, servidores de mock),
  mais os arquivos de `testMatch` (por padrão, `*.spec.*` e `*.test.*`), também os de cada item
  de `projects`.
- **Arquivos carregados:** `globalSetup` e `globalTeardown`.
- **Reporters** que não vêm com o Playwright contam como uso de teste.
- **`webServer.command`** é lido como um script de teste: o arquivo que ele roda é código de
  teste e o binário conta como uso (veja [Configuração](configuration.md#os-scripts-do-packagejson)).

## Drizzle Kit

Com `drizzle-kit` declarado, o `drizzle.config.*` (ou o que um script passa com `--config`) é
código de ferramenta, e os arquivos do `schema` (um arquivo, uma pasta ou um glob) são entradas
de produção: é deles que as migrations são geradas.

## PostCSS

Com `postcss` declarado, ou com um `postcss.config.*` ou `.postcssrc*` no projeto (o Next.js e o
Vite carregam a configuração sem o projeto declarar o `postcss`), os plugins que a configuração
cita pelo nome (`"@tailwindcss/postcss": {}`, `["autoprefixer"]`) e o `parser`, `syntax` e
`stringifier` contam como uso de produção: a configuração roda no build, e é comum essas
dependências ficarem em `dependencies`.

## Vite

Com `vite` declarado, as entradas de produção são os `<script type="module" src="...">` das
páginas `.html` do projeto (o `index.html`, e as outras de um app com várias páginas), sem
listá-las no `prunr.json`. Um `src` que começa com `/` parte do `root` do `vite.config.*`; um
relativo, da pasta da página. Scripts de outro host e scripts sem `src` ficam de fora.

O `vite.config.*` é código de ferramenta: os plugins que ele importa (`@vitejs/plugin-react`...)
contam como dependências de desenvolvimento usadas.

Um import que começa com `/` (`import logo from "/vite.svg"`) é procurado como o Vite faz: no
`publicDir` (`public/` por padrão) e depois no `root`. Os nomes do `resolve.alias` (um objeto,
ou uma lista de `{ find, replacement }` com `find` em texto) também: um destino que começa com
`/` parte do `root`, e um relativo, da configuração.

## React

Com `react` declarado, o `react` conta como usado em produção: o JSX vira imports de
`react/jsx-runtime` que o código não escreve, então um projeto pode não ter nenhum
`import React`.

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

Num projeto Vite, Next.js ou NestJS, `entries.production` pode ficar vazio (`[]`).

## TypeScript

Com `typescript` declarado, ou um `tsconfig.json` na raiz, os `tsconfig*.json` da raiz e os que
os scripts passam a `tsc -p` são lidos, seguindo `extends` e `references`. Contam como usados:

- os pacotes do `extends` (`@tsconfig/node20`, `@sindresorhus/tsconfig`...);
- os de `compilerOptions.types` (`node` é `@types/node`; `vitest/globals` é `vitest`);
- o de `compilerOptions.jsxImportSource`;
- os plugins de `compilerOptions.plugins`.

Os testes de tipo (`test-d/**` e `*.test-d.*`, do tsd ou do próprio `tsc`) são código de teste.

## Vue, Svelte, Astro e MDX

Com `vue`, `svelte`, `astro` ou um compilador de MDX (`@mdx-js/*`, `@next/mdx`, `@astrojs/mdx`)
declarado, os arquivos `.vue`, `.svelte`, `.astro` e `.mdx` fazem parte do projeto: os scripts
de cada um (os `<script>`, o frontmatter do Astro, os blocos de `import`/`export` do MDX) entram
no grafo, e um componente que nada importa é reportado como arquivo sem uso.

Os imports de um desses arquivos nunca são reportados, porque o template pode usar o nome. As
páginas do Astro (`src/pages/**`) são entradas de produção. Num projeto Nuxt ou SvelteKit, os
`.vue` e `.svelte` ainda ficam de fora: os componentes e as rotas desses frameworks são achados
por convenção, sem import.

## ESLint, Prettier, Babel e commitlint

Os pacotes que a configuração dessas ferramentas nomeia contam como usados, também no atalho
de cada uma: os `extends`, `plugins` e `parser` do ESLint (`airbnb` é `eslint-config-airbnb`,
`plugin:react/recommended` é `eslint-plugin-react`), os `plugins` do Prettier, os `presets` e
`plugins` do Babel (`@babel/env` é `@babel/preset-env`) e os `extends` e `plugins` do
commitlint. A configuração pode estar no arquivo da ferramenta (JavaScript, JSON ou YAML) ou no
`package.json` (`eslintConfig`, `prettier`, `babel`, `commitlint`).

Uma configuração em JavaScript é um arquivo da ferramenta: o que ela importa conta como usado.
Com uma configuração, o pacote da ferramenta (`eslint`, `prettier`, `@babel/core`,
`@commitlint/cli`) também conta, mesmo sem script que o rode.
