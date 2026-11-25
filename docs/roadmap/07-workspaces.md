# 7. Workspaces e monorepo

## Objetivo

Analisar um repositório com vários pacotes (`workspaces` do npm/yarn, `pnpm-workspace.yaml`) num
só grafo.

## Escopo

- Cada workspace tem seu `package.json` e suas dependências, checados separadamente.
- Um import de outro workspace pelo nome do pacote vira aresta para os arquivos dele, e o
  workspace importado precisa estar declarado como dependência.
- Os exports do ponto de entrada de um workspace usado por outro são checados pelos usos reais.
- `prunr.json` ganha entradas por workspace só se a convenção (`main`, `exports`) não bastar.

## Critérios de aceite

- [ ] Uma fixture com dois workspaces, um importando o outro.
- [ ] Uma dependência declarada no workspace errado é reportada no workspace certo.
