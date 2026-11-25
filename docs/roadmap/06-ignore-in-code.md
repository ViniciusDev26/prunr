# 6. Ignorar no código

## Objetivo

Marcar no próprio código um export que é API pública de propósito, sem mexer no `prunr.json`.

## Escopo

- `// prunr-ignore-next-line` (ou em bloco, com um motivo depois) antes de um export: ignora os
  exports feitos na linha seguinte (declaração `export`, `exports.x =`, chave de
  `module.exports = { ... }`, método estático).
- A tag JSDoc `@public` faz o mesmo para o export que documenta. Por ser documentação, nunca é
  reportada.
- `/* prunr-ignore-file */` em qualquer lugar do arquivo deixa o arquivo fora de `unused-file` e
  seus exports fora de `unused-export`.
- Um comentário de ignorar que não impede nenhum achado é `unused-ignore`, como o baseline velho,
  para não acumular.

## Critérios de aceite

- [x] Uma fixture com cada forma de comentário.
- [x] Um comentário de ignorar sem efeito falha o check.
