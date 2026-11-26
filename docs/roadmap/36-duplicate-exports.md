# 36. Exports duplicados

## Objetivo

`export { a, a as b }` ou `export default a` junto de `export { a }`: o mesmo valor exportado com
dois nomes. Quem importa escolhe um, e o outro vira API sem motivo.

## Escopo

- Um achado `duplicate-export` (`arquivo: a, b`) quando dois exports de um arquivo são o mesmo
  binding local.
- `prunr-ignore-next-line` o mantém (um apelido de compatibilidade é proposital).

## Critérios de aceite

- [ ] Uma fixture com um export duplicado e um apelido ignorado.
