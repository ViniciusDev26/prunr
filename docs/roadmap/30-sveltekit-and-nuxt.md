# 30. SvelteKit e Nuxt

## Objetivo

Num app SvelteKit ou Nuxt, quase nada é importado: o framework acha as rotas, os layouts, os
componentes e os composables pela pasta em que estão. Sem conhecer essas convenções, o prunr
reportaria o app inteiro como arquivos sem uso, e por isso o Vue ficava desligado no Nuxt e o
Svelte no SvelteKit ([28](28-single-file-components.md)).

## Feito

- `@sveltejs/kit`: as rotas (`src/routes/**/+*`), os hooks (`src/hooks.*`), o service worker,
  os param matchers (`src/params/**`) e os arquivos servidos como estão (`static/**`) são
  entradas de produção. `svelte.config.js` é um arquivo da ferramenta. `$lib` e os nomes do
  `kit.alias` são aliases; `$app`, `$env`, `$service-worker` e os tipos de cada rota
  (`./$types`) são módulos que o framework gera.
- `nuxt`: sob a pasta do código (`app/` no Nuxt 4, a raiz antes, ou o `srcDir`), `app.vue`,
  `error.vue`, `app.config.*`, `pages/`, `layouts/`, `components/`, `composables/`, `utils/`,
  `plugins/` e `middleware/` são entradas de produção, e na raiz `server/`, `modules/` e
  `shared/`. `nuxt.config.*` é um arquivo da ferramenta, e os `modules` dele contam como usados.
  `~`/`@` apontam para a pasta do código, `~~`/`@@` para a raiz, e `#imports`, `#app`,
  `#components`... são módulos que o Nuxt gera.

O Vue liga também num projeto Nuxt (que não precisa declarar `vue`), e o Svelte, num projeto
SvelteKit.

## Critérios de aceite

- As fixtures `sveltekit` e `nuxt` só reportam o arquivo órfão e o export sem uso.
