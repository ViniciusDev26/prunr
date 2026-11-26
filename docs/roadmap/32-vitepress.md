# 32. VitePress

## Objetivo

Num site de docs em VitePress, os componentes do tema são importados pelo `theme/index` ou pelo
`<script setup>` das páginas Markdown, e a config importa helpers e plugins. Sem conhecer essas
convenções, o corpus mostrava os componentes de docs do Vite e do Rollup como arquivos sem uso.

## Feito

Com `vitepress` declarado, em cada pasta com `.vitepress/` (a raiz, a pasta que um script passa
a `vitepress dev`, ou `docs/`), `.vitepress/theme/index.*` e `.vitepress/config.*` são entradas
de produção. Os `.md` são compilados para os `<script>` deles, fora dos blocos de código e do
código inline, e todo `.md` do pacote é uma entrada: Markdown nunca é arquivo sem uso. Os `.vue`
do tema são compilados mesmo sem `vue` declarado.

No corpus, sumiram os 32 componentes e os 6 helpers de config do Vite e do Rollup. Ficam os
aliases que o tema externo do Vite registra (`@components/*`), que só existem rodando o plugin
de Vite dele.

## Critérios de aceite

- A fixture `vitepress` só reporta o componente órfão; o `<script>` dentro de blocos de código e
  de código inline é texto.
