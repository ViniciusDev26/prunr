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

## Feito

Os membros de um enum exportado são exports aninhados (`Color.Blue`), como os métodos estáticos
de uma classe, e o que sobra sem uso é `unused-enum-member`. Um membro lido no próprio arquivo
fica, e ler o enum de outro jeito que não por um membro usa todos. O `prunr fix` tira o membro
da lista, e `prunr-ignore-next-line` o mantém.

## Critérios de aceite

- [x] Uma fixture com um membro lido, um sem uso e um enum lido por chave.
