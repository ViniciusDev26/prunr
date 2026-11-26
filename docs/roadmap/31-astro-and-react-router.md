# 31. Astro e React Router/Remix

## Objetivo

Completar os frameworks da fase 2: o Astro, além das páginas, carrega coleções de conteúdo,
middleware e actions sem import; o React Router (modo framework) e o Remix acham as rotas pela
pasta `app/`.

## Feito

- `astro`: além de `src/pages/**`, são entradas de produção as coleções (`src/content/**` e a
  config delas), o middleware, as actions e o `astro.config.*` (o `astro add` põe as
  integrações em `dependencies`). `astro:content`, `astro:assets`... são URLs com esquema, que o
  prunr já ignora.
- `@react-router/dev` e `@remix-run/dev`: sob a pasta do app (`app/`, ou o `appDirectory`), a
  rota raiz, os módulos de rota, o `routes.ts` e as entradas de cliente e servidor são entradas
  de produção. Os tipos gerados de cada rota (`./+types/home`) são módulos do framework; os
  pacotes que as entradas padrão importam (`react-dom`, `isbot`, o adaptador de servidor) e o
  Vite contam como usados.

## Critérios de aceite

- A fixture `astro` só reporta o componente órfão, e a `react-router` só o arquivo órfão e o
  export sem uso.
