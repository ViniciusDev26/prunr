# 39. `prunr init` a partir do Knip

## Objetivo

Quem já usa o Knip tem as entradas e os ignores escritos num `knip.json`. O `prunr init` passa a
ler esse arquivo, para a migração não começar do zero.

## Escopo

- Com `knip.json`, `knip.jsonc` ou a chave `knip` do `package.json`, o `init` traduz `entry`,
  `project` (para o que o prunr não deve ler), `ignore`, `ignoreDependencies` e os `workspaces`.
- O que não tem equivalente (opções de plugins, `rules`) é listado na saída, para o usuário
  decidir.

## Critérios de aceite

- [ ] Uma fixture com um `knip.json` gera o `prunr.json` esperado e avisa o que ficou de fora.
