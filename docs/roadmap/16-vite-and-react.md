# 16. Vite e React

## Objetivo

Um app React criado com Vite (`npm create vite`) roda no prunr sem listar entradas no
`prunr.json`, com os mesmos achados que um projeto configurado à mão.

## Escopo

- Plugin `vite`: as entradas de produção são os `<script type="module" src>` das páginas
  `.html`; o `vite.config.*` é código de ferramenta; um import que começa com `/` é procurado no
  `publicDir` e no `root`, como o Vite faz.
- Plugin `react`: o `react` conta como usado, porque o JSX automático o importa sem que o código
  mostre.
- O núcleo ganha, na `Contribution`, as pastas onde um especificador absoluto é procurado
  (`absolute_roots`), e os plugins passam a contribuir antes da resolução dos imports.

Fora do escopo, até um caso real pedir: `jsxImportSource` (Preact, Emotion), scripts inline,
`build.rollupOptions.input` e o modo biblioteca (`build.lib`), e `import.meta.glob`.

## Critérios de aceite

- [x] A fixture `vite-react`, igual ao template `react-ts` do Vite, reporta só o componente e o
  export mortos, e o import absoluto quebrado como aviso.
- [x] `react` e `@types/react` não são `unused-dependency` sem nenhum `import` de `react`.
