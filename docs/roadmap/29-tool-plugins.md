# 29. Plugins declarativos de ferramentas

## Objetivo

As ferramentas de desenvolvimento carregam pacotes pelo nome, na configuração delas: os
`extends` e `plugins` do ESLint, os `presets` do Babel, os `plugins` do Prettier, os `extends`
do commitlint. Sem ler essas configurações, cada um desses pacotes vira `unused-dependency`, e
era a maior fonte desse achado no corpus.

## Feito

Uma ferramenta assim é um dado, não código: um `Tool` em `src/plugins/tools.rs` diz o pacote
dela, os arquivos de configuração, a chave do `package.json` e as chaves que nomeiam pacotes,
com o prefixo do atalho de cada uma (`airbnb` é `eslint-config-airbnb`; `@babel/env` é
`@babel/preset-env`; `plugin:react/recommended` é `eslint-plugin-react`). Entre as expansões de
um atalho, vale a primeira que o projeto declara.

- ESLint: configuração plana (`eslint.config.*`) e legada (`.eslintrc*`, `eslintConfig`):
  `extends`, `plugins` e `parser`, em qualquer nível (`overrides`).
- Prettier: `.prettierrc*`, `prettier.config.*` e a chave `prettier`; `plugins`, ou o nome de
  uma configuração compartilhada.
- Babel: `babel.config.*`, `.babelrc*` e a chave `babel`; `presets` e `plugins` (também em
  `env`).
- commitlint: `commitlint.config.*`, `.commitlintrc*` e a chave `commitlint`; `extends` e
  `plugins`.

Um `Tool` também diz os arquivos de teste da ferramenta, as chaves cujos valores são entradas de
produção e as que são comandos, e os arquivos de comandos (os hooks do husky):

- Tailwind: `tailwind.config.*`, um arquivo da ferramenta (os plugins que ele importa).
- Storybook: `.storybook/main.*` e `preview.*`; `addons` e `framework` (também
  `{ name }`); as stories (`**/*.stories.*`) são testes.
- Cypress: `cypress.config.*`; `cypress/**` e `**/*.cy.*` são testes.
- husky: cada linha dos hooks de `.husky/` é um comando.
- lint-staged: `.lintstagedrc*`, `lint-staged.config.*` e a chave `lint-staged`; cada valor é
  um comando.
- Prisma: o `seed` da chave `prisma` é um comando.
- tsup (`entry`), Rollup (`input`) e webpack (`entry`, e os `loader`/`use` das regras): as
  entradas do bundle são entradas de produção, com ou sem extensão.

Uma configuração em JavaScript é também um arquivo que a ferramenta carrega: o que ela importa
conta como usado (o `globals` de um `eslint.config.js`). Com uma configuração, o próprio pacote
da ferramenta conta como usado, porque o editor e os hooks a rodam sem script.

No corpus, `unused-dependency` caiu de 2.865 para 1.809, e `unused-file` de 31.857 para
31.033.

## Critérios de aceite

- A fixture `tool-configs` só reporta a dependência que nada usa.
- A fixture `more-tool-configs` só reporta o arquivo e a dependência que nada usa.
