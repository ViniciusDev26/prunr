# 36. Exports duplicados

## Objetivo

`export { a, a as b }` ou `export default a` junto de `export { a }`: o mesmo valor exportado com
dois nomes. Quem importa escolhe um, e o outro vira API sem motivo.

## Escopo

- Um achado `duplicate-export` (`arquivo: a, b`) quando dois exports ESM de um arquivo são o
  mesmo binding local, fora de uma entrada.
- `prunr-ignore-next-line` o mantém (um apelido de compatibilidade é proposital).

## Feito

`duplicate-export` aponta os exports ESM de um arquivo que são o mesmo binding. No corpus, a
primeira versão deu 159 achados, quase todos de propósito: os `GET` e `POST` de uma rota do Next
(o framework pede os dois nomes), os apelidos de compatibilidade do helmet, `export default`
junto do nome, e tabelas CommonJS (`{ "<a>": f, "<b>": f }` no jsdom). Ficaram de fora as
entradas, o `default` e o CommonJS; sobraram 13, apelidos de tipo e de parser que quem mantém
decide.

## Critérios de aceite

- [x] Uma fixture com um export duplicado e um apelido ignorado.
