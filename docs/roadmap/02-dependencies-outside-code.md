# 2. Dependências usadas fora do código

## Objetivo

Um pacote que só aparece nos `scripts` do `package.json` (`jest`, `eslint`, `tsx`, `nodemon`) não
é `unused-dependency`. Hoje ele precisa ir para `dependencies.ignore`.

## Escopo

- **Binários:** cada comando de cada script é lido (separado por `&&`, `||`, `;` e `|`), sem as
  atribuições de ambiente (`NODE_ENV=test`) e sem os executores (`npx`, `pnpm exec`, `yarn`,
  `bunx`, `cross-env`, `dotenv --`). O binário que sobra aponta para o pacote que o declara no
  campo `bin`, lido em `node_modules/<pacote>/package.json`. Sem `node_modules`, vale o nome do
  pacote (sem o escopo) como nome do binário.
- **Módulos carregados pela linha de comando:** `node -r x`, `--require x`, `--import x` e
  `--loader x` usam o pacote `x`.
- **Uso por script não é classificado como produção nem teste:** um pacote usado só por script
  não é `unused-dependency` nem `misplaced-dependency`.
- **Binário sem pacote declarado não é `missing-dependency`:** pode ser uma ferramenta do sistema
  (`docker`, `git`).

## Critérios de aceite

- [ ] Uma fixture com `scripts` cobrindo binário direto, `npx`, `cross-env`, ambiente, `&&`,
      `node -r` e um pacote com binário de nome diferente do pacote (via `node_modules`).
- [ ] `dependencies.ignore` deixa de ser necessário para os casos acima, e a documentação diz isso.
