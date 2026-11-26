# Achados

O prunr lê cada arquivo uma única vez, monta um grafo de módulos a partir das entradas e roda
todas as verificações sobre esse mesmo grafo.

| Achado | Quando |
|---|---|
| `unused-file` | Nenhuma entrada de produção alcança o arquivo. Se só os testes o alcançam, a linha diz `(only tests use it)`: apague-o junto com os testes. |
| `unused-export` | Nenhum arquivo de produção usa o export, nem o próprio arquivo onde ele está. Métodos estáticos de uma classe exportada contam como exports (`Invoice.total`). Também marca `(only tests use it)`. |
| `unused-import` | Um `require` ou `import` declara um binding que o arquivo nunca usa (`const x = require("./x")`, uma propriedade desestruturada, `import { a } from "./x"`). Um `require("./x");` ou `import "./x";` sozinho, por efeito colateral, não conta. |
| `unused-ignore` | Um comentário `prunr-ignore-next-line` ou `prunr-ignore-file` não impede nenhum achado. Veja [Ignorar no código](ignore.md). |
| `unused-dependency` | O pacote está em `dependencies` ou `devDependencies`, mas nenhum arquivo o importa, nenhum script do `package.json` o roda e nenhum pacote usado o exige como `peerDependencies`. |
| `misplaced-dependency` | O pacote está em `dependencies`, mas só os testes o importam: ele pertence a `devDependencies`. |
| `missing-dependency` | O arquivo importa um pacote que nenhum `package.json` acima dele declara: o do pacote, o da raiz, ou um aninhado com dependências próprias (um harness de testes em `tests/smoke/package.json`), como o Node procura em cada pasta acima do arquivo. Os módulos nativos do Node (`fs`, `node:fs`) e do Bun (`bun:test`) não contam. |
| `circular-import` | Há um ciclo de `require` ou `import`. Cada grupo de arquivos em ciclo é reportado uma vez, pelo caminho mais curto. Imports só de tipo (`import type`) não contam: somem na compilação. |
| `unparsed-file` | **Aviso:** um arquivo que o grafo alcança não faz parse (um erro de sintaxe, ou uma sintaxe que o prunr não lê, como Flow). O que ele importa e exporta fica desconhecido, então o resto do grafo pode estar incompleto. Não falha o check. Um arquivo quebrado que nada alcança (uma fixture de teste com erro de propósito) é só `unused-file`. |
| `unresolved-require` | **Aviso:** um `require` com caminho dinâmico (`require(nome)`), com um padrão que não alcança nenhum arquivo ou que aponta para um arquivo que não existe. Não falha o check. |

Teste não mantém código vivo: o que só os testes usam está morto em produção. Um arquivo que
nem produção nem testes alcançam não gera outros achados nem mantém nada vivo. No JSON, os
achados que só os testes usam têm `"testOnly": true`.

Um pacote que outro pacote usado exige em `peerDependencies` (o `@tiptap/pm` do
`@tiptap/react`, o `vite` do `@vitejs/plugin-react`) é usado do mesmo jeito que ele: em produção,
se o outro é usado em produção, e só nos testes, se o outro só é usado nos testes. Os peers são
lidos do `package.json` instalado em `node_modules`, inclusive os opcionais.

Na dúvida, o prunr é conservador: quando o resultado de um `require` é passado adiante ou
acessado por chave computada, o módulo inteiro conta como usado. É melhor deixar passar um
export morto do que reportar um vivo.

## Folhas de estilo

Um arquivo que importa uma folha de estilo (`import "./globals.css"`, também `.scss`, `.sass`,
`.less` e `.pcss`) usa os pacotes que ela carrega:

```css
@import "tailwindcss";
@import url("normalize.css");
@plugin "daisyui";
@import "./fonts.css"; /* lida também, e os pacotes dela contam */
```

- Contam `@import`, `@use`, `@forward`, `@plugin`, `@reference` e `@config`, com aspas ou
  `url(...)`, fora de comentários.
- Um nome sem `./` que aponta para um arquivo ao lado da folha (`@import "variables"`, o parcial
  `_mixins.scss` do Sass) é local, como no Sass e no `postcss-import`; senão é um pacote. O `~`
  do webpack é ignorado, e endereços (`https://...`) e módulos do Sass (`sass:math`) ficam de
  fora.
- O uso é do mesmo tipo do arquivo que importa: produção ou teste.

## `require` dinâmico

Um `require` montado em tempo de execução a partir de um caminho relativo vira um padrão:
`` require(`./handlers/${nome}`) `` e `require("./handlers/" + nome)` usam, inteiros, todos os
arquivos de `handlers/` (cada `${...}` vale um trecho sem `/`, e uma pasta vale pelo seu
`index`).

## ESM e TypeScript

Arquivos `.js`, `.cjs`, `.mjs`, `.jsx`, `.ts`, `.cts`, `.mts` e `.tsx` entram no mesmo grafo
(arquivos `.d.ts` ficam de fora). Um arquivo ESM pode importar um CommonJS e vice-versa:

- O `default` importado de um arquivo CommonJS é o `module.exports`, como no Node.
- `import`, `import type`, `import()` com caminho estático, `import x = require()`, `export`,
  `export default`, `export =` e os métodos estáticos de classes exportadas são lidos como no
  CommonJS. Tipos, interfaces e enums exportados sem uso são `unused-export`.
- `export { a } from`, `export * from` e `export * as ns from` repassam o uso para o arquivo de
  origem, e o export morto é reportado onde é definido, não no barrel. Os reexports de uma
  entrada de produção contam como usados: são a interface dela.
- Um tipo JSDoc `import("./x").Nome` (em `@param`, `@type`, `@typedef`...) usa `Nome` de `./x`,
  e `import("./x")` sem membro usa o módulo inteiro: um arquivo só de tipos JSDoc não é
  `unused-file`. Como imports só de tipo, eles não formam `circular-import`.
- A resolução segue o TypeScript: `./a.js` acha o `a.ts`, e valem os `paths` do `tsconfig.json`
  e os `imports` (`#interno`) do `package.json`.
- `@types/x` conta como usado quando `x` é usado; `@types/node`, quando o projeto tem TypeScript
  ou usa um módulo do Node. Importar `x` com só o `@types/x` declarado não é
  `missing-dependency`.

## Limitações

- **Um reexport que ninguém usa não é reportado** quando o export de origem tem outro uso: o
  achado fica onde o export é definido.
- **Dependências citadas só em arquivos de configuração de outras ferramentas** (plugins do ESLint,
  presets do Babel) aparecem como `unused-dependency`. Liste-as em `dependencies.ignore`.
