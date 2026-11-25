# 1. Um grafo só

## Objetivo

Parsear e resolver cada arquivo **uma única vez**, montar um grafo de módulos e rodar todas as
verificações sobre ele. A composição usual monta o grafo várias vezes (dependency-cruiser,
depcheck, madge e um script de exports), cada ferramenta com seus próprios ignores e suas próprias
divergências.

## Comportamento

O grafo guarda, por arquivo:
- os módulos que ele importa (resolvidos para um arquivo do projeto ou um pacote do `node_modules`);
- os nomes que ele exporta;
- os nomes que ele usa de cada módulo importado.

### CommonJS (obrigatório no MVP)

| Forma | Tratamento |
|---|---|
| `require("./x")`, `require("pkg")` com string literal | aresta no grafo |
| `const { a, b } = require("./x")` | usa `a` e `b` de `./x` |
| `const x = require("./x"); x.a()` | usa `a` de `./x` |
| `module.exports = { a, b }` | exporta `a` e `b` |
| `exports.a = ...` / `module.exports.a = ...` | exporta `a` |
| `module.exports = algoNaoObjeto` | export padrão único |
| `class X { static m() {} }` exportada | `m` é um export (paridade com o script atual) |
| `require(variavel)` ou template string | **aviso** `unresolved-require`, nunca ignorado em silêncio |

ESM fica fora do MVP.

### Verificações sobre o grafo

- `unused-file`: arquivo não alcançável a partir de nenhum ponto de entrada.
- `unused-export`: export que nenhum outro arquivo usa e que o próprio arquivo também não usa.
- `unused-dependency`: pacote em `dependencies`/`devDependencies` que nenhum arquivo importa.
- `missing-dependency`: pacote importado que não está declarado no `package.json`.
- `circular-import`: ciclo no grafo, reportado uma vez por ciclo, com o caminho completo.

## Critérios de aceite

- [ ] Cada arquivo é lido e parseado uma única vez por execução (verificado por teste).
- [ ] Cada forma da tabela de CommonJS tem fixture e teste próprios.
- [ ] As 5 verificações rodam a partir do mesmo grafo, num único comando: `prunr check`.
- [ ] Saída legível para humanos por padrão e `--json` para máquinas.
- [ ] Códigos de saída: `0` sem achados novos, `1` com achados novos, `2` erro de configuração/execução.
