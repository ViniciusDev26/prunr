# 5. Saída para PR

## Objetivo

Mostrar os achados onde a revisão acontece: no diff do PR.

## Escopo

- `prunr check --github`: anotações do GitHub Actions (`::error file=...,line=...,title=...::`),
  na linha do export, do import, do `require`, do comentário de ignorar ou da dependência no
  `package.json`. Um arquivo sem uso é anotado no arquivo, sem linha.
- `prunr check --since <ref>`: só os achados que o commit `<ref>` não tinha. O commit é analisado
  num worktree temporário do git (com o `node_modules` do projeto) e com o mesmo `prunr.json`.
- Combina com `--json` e com o baseline.
- SARIF (code scanning do GitHub) fica de fora: as anotações já aparecem no diff, sem exigir
  permissão de escrita em *security events*.

## Critérios de aceite

- [x] As anotações apontam a linha do export, do import ou do arquivo.
- [x] `--since` não esconde um achado novo causado por mudança em outro arquivo (por exemplo,
      um export que ficou sem uso porque o único consumidor foi apagado).
