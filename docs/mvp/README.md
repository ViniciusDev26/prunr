# Definição de MVP

O MVP inicial só está pronto quando **os 7 pontos** abaixo forem atendidos, cada um com seus
critérios de aceite:

- [x] 1. [Um grafo só](01-single-graph.md)
- [x] 2. [Teste não mantém código vivo](02-tests-dont-keep-code-alive.md)
- [x] 3. [Baseline que só diminui](03-shrinking-baseline.md)
- [x] 4. [Pontos de entrada num arquivo de configuração JSON](04-entry-points-config.md)
- [ ] 5. [Variáveis locais delegadas ao Biome](05-locals-delegated-to-biome.md)
- [ ] 6. [CI de testes e release automático](06-ci-and-auto-release.md)
- [ ] 7. [Todo o MVP construído com TDD](07-tdd.md)

**Teste de aceite final:** rodar o prunr num projeto CommonJS real de referência e comparar com
a saída da composição de ferramentas que ele usa hoje (exceto variáveis locais). Toda diferença
precisa ser explicada: um falso positivo/negativo das ferramentas antigas ou um bug do prunr,
que nesse caso vira teste antes da correção.
