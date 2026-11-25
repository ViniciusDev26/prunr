# prunr

Detector de código morto para projetos Node.js, com **CommonJS como cidadão de primeira classe**.

Uma ferramenta, um arquivo de configuração e um grafo de módulos substituem a composição de
ferramentas que um projeto CommonJS costuma montar para detectar código morto:

| Hoje | Verifica | No prunr |
|---|---|---|
| dependency-cruiser | arquivos órfãos | `unused-file` |
| script próprio + baseline | exports sem uso em produção | `unused-export` |
| depcheck | dependências sem uso ou não declaradas | `unused-dependency`, `missing-dependency` |
| madge | imports circulares | `circular-import` |
| `tsc` com `noUnusedLocals` + `grep` | variáveis e imports locais | **fora do escopo**: Biome ([ponto 5](docs/mvp/05-locals-delegated-to-biome.md)) |

## Por que não o Knip

O Knip não lida bem com CommonJS, e a comunidade considera que não vale o esforço de dar suporte.
O prunr parte do caminho oposto: começa por Node puro em CommonJS com Jest e depois expande para
ESM, Vitest, Next, Nest etc.

## Definição de MVP

O MVP inicial só está pronto quando **os 7 pontos** abaixo forem atendidos, cada um com seus
critérios de aceite:

- [ ] 1. [Um grafo só](docs/mvp/01-single-graph.md)
- [ ] 2. [Teste não mantém código vivo](docs/mvp/02-tests-dont-keep-code-alive.md)
- [ ] 3. [Baseline que só diminui](docs/mvp/03-shrinking-baseline.md)
- [ ] 4. [Pontos de entrada num arquivo de configuração JSON](docs/mvp/04-entry-points-config.md)
- [ ] 5. [Variáveis locais delegadas ao Biome](docs/mvp/05-locals-delegated-to-biome.md)
- [ ] 6. [CI de testes e release automático](docs/mvp/06-ci-and-auto-release.md)
- [ ] 7. [Todo o MVP construído com TDD](docs/mvp/07-tdd.md)

**Teste de aceite final:** rodar o prunr num projeto CommonJS real de referência e comparar com
a saída da composição de ferramentas que ele usa hoje (exceto variáveis locais). Toda diferença
precisa ser explicada: um
falso positivo/negativo das ferramentas antigas ou um bug do prunr, que nesse caso vira teste
antes da correção.

## Stack (proposta)

- Rust, com `oxc_parser` (AST) e `oxc_resolver` (resolução no estilo do Node).
- Distribuído pelo npm como dependência de desenvolvimento, com um pacote de binário por
  plataforma em `optionalDependencies`, sem `postinstall`.
- Nome `prunr` livre no npm e no crates.io (conferido em 2026-11-24).
