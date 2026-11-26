# 34. Membros de enum sem uso

## Objetivo

Um `enum` exportado e usado pode ter membros que ninguém lê. O Knip reporta esses membros; o
prunr só reporta o enum inteiro.

## Escopo

- Um achado `unused-enum-member` (`arquivo: Enum.Membro`) para o membro de um enum exportado que
  nenhum arquivo alcançado lê, nem o próprio arquivo.
- Ler o enum inteiro (`Object.values(E)`, `for (k in E)`, `E[x]`, passar `E` adiante) usa todos
  os membros.
- `prunr fix` apaga o membro, e `prunr-ignore-next-line` o mantém.

## Critérios de aceite

- [ ] Uma fixture com um membro lido, um sem uso e um enum lido por chave.
