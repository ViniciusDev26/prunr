# 19. Playwright, Drizzle Kit, PostCSS e Vitest `projects`

## Objetivo

As ferramentas que apareceram num monorepo grande de referência rodam no prunr sem
`entries.test` nem `dependencies.ignore` para elas.

## Escopo

- **Playwright:** `testDir` inteiro e `testMatch` (também por projeto) são teste; `globalSetup`,
  `globalTeardown` e reporters de fora contam; `webServer.command` é lido como um script de
  teste. A `Contribution` ganha `scripts`, os comandos que uma ferramenta roda.
- **Drizzle Kit:** a configuração é ferramenta, e o `schema` é entrada de produção.
- **PostCSS:** liga com o pacote declarado ou com uma configuração no projeto; os plugins citados
  pelo nome contam como uso de produção, e a configuração é entrada de produção.
- **Vitest `projects`:** cada configuração listada (arquivo, pasta ou glob de pastas) é lida, com
  os caminhos relativos à pasta dela.

Fora do escopo: a chave `postcss` do `package.json` e as configurações montadas por função (o
item de strings em configs, ainda em aberto).

## Critérios de aceite

- [x] Uma fixture por ferramenta (`playwright`, `drizzle`, `postcss`, `vitest-projects`).
