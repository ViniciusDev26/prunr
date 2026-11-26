# 39. `prunr init` a partir do Knip

## Objetivo

Quem já usa o Knip tem as entradas e os ignores escritos num `knip.json`. O `prunr init` passa a
ler esse arquivo, para a migração não começar do zero.

## Escopo

- Com `knip.json`, `knip.jsonc`, `knip.ts` ou a chave `knip` do `package.json`, o `init`
  traduz `entry`, `ignoreDependencies` e os `workspaces`. `project` e `ignore` não têm
  equivalente: o prunr lê todo arquivo-fonte.
- O que não tem equivalente (opções de plugins, `rules`) é listado na saída, para o usuário
  decidir.

## Feito

O `prunr init` lê a config do Knip (JSON, JSONC, a chave do `package.json` ou TypeScript e
JavaScript, sem executar) e traduz `entry`, as entradas de `workspaces` e `ignoreDependencies`.
O resto sai listado como não traduzido.

## Critérios de aceite

- [x] Uma fixture com um `knip.json` gera o `prunr.json` esperado e avisa o que ficou de fora.
