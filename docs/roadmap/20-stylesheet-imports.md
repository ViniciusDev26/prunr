# 20. Imports em folhas de estilo

## Objetivo

Um pacote carregado só por CSS (`@import "tailwindcss"`, `@plugin "daisyui"`, um
`@import "normalize.css"`) não é `unused-dependency`.

## Escopo

- Quando um import de JavaScript resolve para uma folha de estilo (`.css`, `.scss`, `.sass`,
  `.less`, `.pcss`), o prunr lê a folha e as folhas locais que ela importa.
- `@import`, `@use`, `@forward`, `@plugin`, `@reference` e `@config` com aspas ou `url(...)`,
  fora de comentários.
- Um nome que aponta para um arquivo ao lado da folha é local (Sass, `postcss-import`), inclusive
  o parcial `_nome.scss`; o resto é pacote. Endereços e módulos `sass:` ficam de fora.
- O uso é atribuído ao arquivo JavaScript que importou a folha, do mesmo tipo dele.

Fora do escopo: folhas que só um HTML carrega (`<link rel="stylesheet">`), e arquivos de
configuração apontados por `@config`, que não viram arestas do grafo.

## Critérios de aceite

- [x] A fixture `css-imports` cobre CSS, Sass com parcial, `url()`, comentário e endereço.
