# 38. GitHub Action

## Objetivo

Rodar o prunr num PR hoje exige baixar o binário e montar o comando. Uma Action oficial faz isso
numa linha e deixa os achados como anotações no diff.

## Escopo

- `action.yml` na raiz do repositório: baixa o binário da release (com o mesmo instalador do
  npm), roda `prunr check` e publica o SARIF no code scanning ou, sem permissão para isso, o
  Markdown no resumo do job.
- Entradas: a versão, a pasta do projeto e o formato.

## Critérios de aceite

- [ ] Um workflow do próprio repositório usa a Action contra uma fixture.
