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
| `class X { static m() {} }` exportada | `X.m` é um export |
| `require(variavel)` ou template com `${}` | **aviso** `unresolved-require`, nunca ignorado em silêncio |
| `require("./nao-existe")` | **aviso** `unresolved-require` |

ESM fica fora do MVP.

### Verificações sobre o grafo

- `unused-file`: arquivo não alcançável a partir de nenhum ponto de entrada.
- `unused-export`: export que nenhum outro arquivo usa e que o próprio arquivo também não usa.
- `unused-dependency`: pacote em `dependencies`/`devDependencies` que nenhum arquivo importa.
- `missing-dependency`: pacote importado que não está declarado no `package.json`.
- `circular-import`: ciclo no grafo, reportado uma vez por componente fortemente conexo, com o
  caminho completo do ciclo mais curto que passa pelo primeiro arquivo do componente (em ordem
  alfabética). Enumerar todos os ciclos elementares pode explodir em grafos grandes.
- `unresolved-require`: aviso para um `require` que o grafo não consegue seguir. Não falha o check.

### Decisões tomadas na implementação

- **Só conta o que as entradas alcançam.** Um arquivo que nenhuma entrada alcança vira
  `unused-file`, e o que ele exige ou exporta não gera outros achados nem mantém nada vivo.
- **Os exports de uma entrada não são verificados.** São a interface com quem roda o arquivo.
- **Pacotes em `peerDependencies` e `optionalDependencies`** contam como declarados, mas não
  precisam ser usados.
- **Uso conservador:** quando um binding do `require` escapa (é passado adiante, acessado por
  chave computada etc.), o módulo inteiro conta como usado. É preferível deixar de reportar um
  export morto a reportar um vivo.

## Critérios de aceite

- [x] Cada arquivo é lido e parseado uma única vez por execução (verificado por teste).
- [x] Cada forma da tabela de CommonJS tem fixture e teste próprios.
- [x] As 5 verificações rodam a partir do mesmo grafo, num único comando: `prunr check`.
- [x] Saída legível para humanos por padrão e `--json` para máquinas.
- [x] Códigos de saída: `0` sem achados novos, `1` com achados novos, `2` erro de configuração/execução.
