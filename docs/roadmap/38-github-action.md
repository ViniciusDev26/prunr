# 38. GitHub Action

## Objetivo

Rodar o prunr num PR hoje exige baixar o binário e montar o comando. Uma Action oficial faz isso
numa linha e deixa os achados como anotações no diff.

## Escopo

- `action.yml` na raiz do repositório: roda o pacote do npm, que traz o binário, e publica as
  anotações, o SARIF no code scanning ou o Markdown no resumo do job.
- Entradas: a versão, a pasta do projeto e o formato.

## Feito

[`action.yml`](../../action.yml) roda o `@prunr/prunr` do npm (com `npx`, sem instalar o
projeto) e entrega os achados como anotações, como SARIF para o code scanning ou como uma tabela
no resumo do job. O workflow `action` do repositório a roda contra uma fixture limpa e uma com
achados.

## Critérios de aceite

- [x] Um workflow do próprio repositório usa a Action contra uma fixture.
