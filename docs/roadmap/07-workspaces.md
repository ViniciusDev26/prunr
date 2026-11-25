# 7. Workspaces e monorepo

## Objetivo

Analisar um repositório com vários pacotes num só grafo, qualquer que seja a ferramenta que
declara os pacotes.

## Escopo

- **Descoberta dos pacotes**, pela declaração de cada ferramenta, somando o que houver:
  - npm, yarn (clássico e berry) e bun: campo `workspaces` do `package.json` raiz, como lista de
    globs ou como `{ "packages": [...] }`;
  - pnpm: `packages` do `pnpm-workspace.yaml` (com os padrões `!` de exclusão);
  - lerna: `packages` do `lerna.json`;
  - rush: `projects[].projectFolder` do `rush.json`;
  - turbo e nx usam os workspaces do gerenciador de pacotes; o nx também conhece projetos só com
    `project.json`, que entram como pacote quando têm um `package.json` ao lado ou nenhum.
- Cada pacote tem seu `package.json` e suas dependências, checados separadamente: um
  `unused-dependency` ou `missing-dependency` diz em qual pacote.
- Um import de outro pacote do repositório pelo nome (`@app/shared`) vira aresta para os arquivos
  dele (pelo `main`/`exports`/`module`, ou pelo link em `node_modules`), e o pacote importado
  precisa estar declarado como dependência de quem importa (`workspace:*` vale).
- Os exports do ponto de entrada de um pacote usado por outro são checados pelos usos reais; os
  de um pacote publicado (sem `"private": true`) são a interface dele e não são reportados.
- As entradas e os runners (Jest, Vitest) de cada pacote vêm do `package.json` dele, como num
  repositório de um pacote só; o `prunr.json` ganha entradas por pacote só se a convenção não
  bastar.

## Critérios de aceite

- [ ] Uma fixture por ferramenta (npm/yarn/bun, pnpm, lerna, rush) com dois pacotes, um
      importando o outro.
- [ ] Uma dependência declarada no pacote errado é reportada no pacote certo.
- [ ] Um export do pacote compartilhado que nenhum outro pacote usa é reportado.
