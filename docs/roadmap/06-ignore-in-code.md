# 6. Ignorar no código

## Objetivo

Marcar no próprio código um export que é API pública de propósito, sem mexer no `prunr.json`.

## Escopo

- Um comentário `// prunr-ignore-next-line` (ou a tag JSDoc `@public`) antes do export.
- Um comentário `/* prunr-ignore-file */` no topo deixa o arquivo fora de `unused-file` e de
  `unused-export`.
- Um comentário que não ignora nada é reportado, como o baseline velho, para não acumular.

## Critérios de aceite

- [ ] Uma fixture com cada forma de comentário.
- [ ] Um comentário de ignorar sem efeito falha o check.
