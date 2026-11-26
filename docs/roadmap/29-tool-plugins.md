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

Uma configuração em JavaScript é também um arquivo que a ferramenta carrega: o que ela importa
conta como usado (o `globals` de um `eslint.config.js`). Com uma configuração, o próprio pacote
da ferramenta conta como usado, porque o editor e os hooks a rodam sem script.

No corpus, `unused-dependency` caiu de 2.865 para 1.944.

## Critérios de aceite

- A fixture `tool-configs` só reporta a dependência que nada usa.
