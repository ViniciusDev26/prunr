# 5. Saída para PR

## Objetivo

Mostrar os achados onde a revisão acontece: no diff do PR.

## Escopo

- `prunr check --reporter github`: anotações do GitHub Actions (`::error file=...,line=...::`),
  com linha e coluna. Exige guardar a posição (span) de cada export e import nos achados.
- `prunr check --changed <base>`: só reporta achados em arquivos alterados desde `<base>`
  (`git diff --name-only`), mais os de dependência quando o `package.json` mudou.
- Avaliar SARIF para o code scanning do GitHub.

## Critérios de aceite

- [ ] As anotações apontam a linha do export, do import ou do arquivo.
- [ ] `--changed` não esconde um achado novo causado por mudança em outro arquivo (por exemplo,
      um export que ficou sem uso porque o único consumidor foi apagado).
