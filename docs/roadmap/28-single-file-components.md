# 28. Vue, Svelte, Astro e MDX

## Objetivo

Os componentes de um projeto Vue, Svelte ou Astro, e os documentos MDX, importam módulos e
pacotes como qualquer arquivo. Sem lê-los, o prunr não vê esses imports, reporta como sem uso o
que só eles alcançam e nunca diz que um componente ficou órfão.

## Feito

Um plugin pode registrar um compilador para uma extensão: os arquivos com ela passam a ser
fontes do projeto, e o compilador transforma cada um no módulo que ele contém. O que não é
script vira espaço, byte a byte, mantendo as quebras de linha: linhas, colunas e posições do
achado (e as edições do `prunr fix`) são as do arquivo original.

- `vue`: os `<script>` e `<script setup>` de cada `.vue`.
- `svelte`: os `<script>` de cada `.svelte`; `svelte` é usado em produção (o runtime que o
  componente compilado importa).
- `astro`: o frontmatter (entre os `---` do topo) e os `<script>` de cada `.astro`, como
  TypeScript; as páginas (`src/pages/**`) são entradas de produção, e `astro` é usado em produção.
- `@mdx-js/mdx` (ligado por `@mdx-js/*`, `@next/mdx` ou `@astrojs/mdx`): os blocos de `import`
  e `export` de cada `.mdx`, fora dos blocos de código.

`lang="ts"` faz o script ser lido como TypeScript. Um import de um arquivo compilado nunca é
reportado como sem uso, e o que ele importa conta como usado: o template pode usar o nome.

Cada pacote de um monorepo compila só os próprios arquivos: um pacote com Svelte não faz das
rotas de um app SvelteKit vizinho fontes do projeto. No script de instância de um componente
Svelte, `export let` declara uma prop, não um export do módulo. O Nuxt e o SvelteKit têm plugins
próprios ([30](30-sveltekit-and-nuxt.md)).

## Critérios de aceite

- A fixture `single-file-components` reporta só o componente órfão e o export sem uso.
