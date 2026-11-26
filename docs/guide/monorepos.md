# Monorepos

Num repositório com vários pacotes, o prunr monta um grafo só e checa as dependências de cada
pacote. Ele acha os pacotes pela declaração de cada ferramenta, somando o que encontrar:

| Ferramenta | Onde declara os pacotes |
|---|---|
| npm, yarn, bun | `workspaces` do `package.json` raiz (lista ou `{ "packages": [...] }`) |
| pnpm | `packages` do `pnpm-workspace.yaml`, com exclusões `!` |
| lerna | `packages` do `lerna.json` |
| rush | `projects[].projectFolder` do `rush.json` (aceita comentários) |
| nx | as pastas com `project.json` |
| turbo | usa os workspaces do gerenciador de pacotes |

- Um import de outro pacote do repositório pelo nome (`@app/shared`, `@app/shared/x`) vira
  aresta para os arquivos dele, mesmo sem `node_modules`: pelo `exports`, `main` ou `module`, ou
  pelo fonte (`src/index`) quando eles apontam para um build que não existe. Quem importa precisa
  declarar o pacote (`workspace:*` vale), com duas exceções: um pacote que importa a si mesmo
  pelo nome, e os arquivos do pacote raiz (testes de integração, scripts de desenvolvimento),
  porque a raiz não é publicada e o gerenciador linka nela os pacotes do repositório.
- As dependências de cada pacote são checadas nele: `unused-dependency` e `misplaced-dependency`
  dizem o pacote (`left-pad (packages/shared)`), e o `prunr fix` edita o `package.json` dele. O
  que o `package.json` raiz declara vale para todos os pacotes (dependências içadas e
  ferramentas), e conta como usado por qualquer arquivo ou script do repositório, menos os de
  um pacote que declara a mesma dependência: nele, quem instala é a declaração dele, e a da raiz
  sobra. Um `@types/x` da raiz continua valendo para o `x` que um pacote declara.
- Os scripts e os [frameworks](frameworks.md) de cada pacote são lidos no diretório dele.
- As entradas de um pacote publicado (sem `"private": true`), cada subcaminho do `exports`, são
  a interface dele: seus exports não são reportados. Vale também para o pacote raiz. A de um pacote privado é checada pelos usos reais dos outros pacotes.
