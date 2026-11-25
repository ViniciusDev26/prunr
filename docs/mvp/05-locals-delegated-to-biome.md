# 5. Variáveis locais com o Biome, imports com o prunr

## Objetivo

Um símbolo sem uso **dentro de um arquivo** (variável, função, membro privado) é trabalho de
linter, e o Biome já resolve isso. O prunr **não** implementa essa verificação.

**Imports são a exceção**: um `require` sem uso afeta o grafo. Ele mantém o arquivo exigido
"vivo" e faz a dependência parecer usada, escondendo `unused-file`, `unused-export` e
`unused-dependency`. Por isso o prunr reporta `unused-import`.

Isso elimina o truque comum de rodar `tsc` com `noUnusedLocals` num `tsconfig` dedicado e filtrar
os códigos de erro com `grep`.

## `unused-import`

- `const x = require("./x")` com `x` nunca referenciado → `unused-import file.js: x from ./x`.
- `const { a, b } = require("./x")` com `b` nunca referenciado → `unused-import ...: b from ./x`.
- Um binding sem uso **não conta como uso** do export: o export que ele nomeia também aparece como
  `unused-export`. A aresta do grafo continua, porque o `require` ainda executa o módulo; ao
  apagar o import, a próxima execução mostra o que mais morreu.
- `require("./x");` sozinho, por efeito colateral, é intencional e não é reportado.
- Padrões que o prunr não acompanha (`...rest`, desestruturação aninhada, valor padrão) são
  tratados de forma conservadora: contam como uso e não são reportados.
- Vale para arquivos de produção e de teste: um import esquecido num teste também é lixo.

## Configuração recomendada (Biome 2)

```json
{
  "linter": {
    "rules": {
      "correctness": {
        "noUnusedVariables": "error",
        "noUnusedPrivateClassMembers": "error"
      }
    }
  }
}
```

### Verificação contra o `tsc`

Uma sonda com 10 tipos de símbolo local sem uso (`require` inteiro e desestruturado, `const`,
`let`, função, classe, variável interna, elemento de array desestruturado, campo e método
`#privados`) foi rodada com `tsc` (`checkJs` + `noUnusedLocals`, os códigos que o script antigo
filtrava) e com o Biome:

- O `tsc` acusou os 10.
- O Biome com as regras acima acusou os mesmos 10, e também um `catch (error)` sem uso.
- Sem `noUnusedPrivateClassMembers`, os dois membros `#privados` escapariam.
- Os dois `require` sem uso também são `unused-import` no prunr; o Biome os acusa como variáveis.

O projeto de referência está limpo nos dois hoje (0 achados no `tsc`), então a troca não muda o
resultado do gate dele.

## Critérios de aceite

- [x] O README do prunr tem uma seção "O que o prunr não faz" citando o Biome para variáveis locais.
- [x] O prunr não emite achado sobre variáveis locais que não são imports (fixture
      `unused-locals` → nenhum achado).
- [x] Um `require` cujo binding nunca é usado é `unused-import`, e um `require` por efeito
      colateral não é (fixture `unused-import`).
- [x] No projeto de referência, as regras do Biome acima cobrem o que o `tsc` + `grep` pegava.
