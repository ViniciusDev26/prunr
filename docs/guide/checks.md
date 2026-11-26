# Achados

O prunr lê cada arquivo uma única vez, monta um grafo de módulos a partir das entradas e roda
todas as verificações sobre esse mesmo grafo.

| Achado | Quando |
|---|---|
| `unused-file` | Nenhuma entrada de produção alcança o arquivo. Se só os testes o alcançam, a linha diz `(only tests use it)`: apague-o junto com os testes. |
| `unused-export` | Nenhum arquivo de produção usa o export, nem o próprio arquivo onde ele está. Métodos estáticos de uma classe exportada contam como exports (`Invoice.total`). Também marca `(only tests use it)`. |
| `unused-enum-member` | Nenhum arquivo de produção lê o membro de um enum exportado (`Color.Blue`), nem o próprio arquivo. Ler o enum inteiro (`Object.values(Color)`, `Color[chave]`, passá-lo adiante) usa todos os membros. Também marca `(only tests use it)`. |
| `unused-class-member` | Um método público de uma classe exportada (`Cart.legacyTotal`) que nenhum arquivo de produção chama. Sem tipos, a análise é conservadora: qualquer leitura de uma propriedade com o mesmo nome, em qualquer objeto (`cache.clear()`), conta como uso. Ficam de fora os métodos de uma classe que estende ou implementa outra (podem ser chamados pelo que sobrescrevem), os com decorator, os privados e protegidos, o `constructor`, os de nome convencional (`toString`, `toJSON`, `then`...), os de uma classe sem uso (já reportada inteira) ou de uma entrada, e todos os de um pacote publicado: lá, uma classe que o código alcança pode chegar a quem instala o pacote. Também marca `(only tests use it)`. |
| `duplicate-export` | Dois ou mais exports ESM de um arquivo são o mesmo binding (`export { format, format as formatDate }`): quem importa escolhe um nome, e os outros são interface à toa. `default` junto de um nome (`export default a; export { a }`) não conta, nem os exports de uma entrada (os `GET` e `POST` de uma rota, um apelido de compatibilidade de uma lib) ou de um objeto CommonJS, que costuma ser uma tabela. Um `prunr-ignore-next-line` antes de um dos nomes mantém o apelido. |
| `unused-import` | Um `require` ou `import` declara um binding que o arquivo nunca usa (`const x = require("./x")`, uma propriedade desestruturada, `import { a } from "./x"`). Um `require("./x");` ou `import "./x";` sozinho, por efeito colateral, não conta. |
| `unused-ignore` | Um comentário `prunr-ignore-next-line` ou `prunr-ignore-file` não impede nenhum achado. Veja [Ignorar no código](ignore.md). |
| `unused-dependency` | O pacote está em `dependencies` ou `devDependencies`, mas nenhum arquivo o importa, nenhum script do `package.json` o roda e nenhum pacote usado o exige como `peerDependencies`. |
| `misplaced-dependency` | O pacote está em `dependencies`, mas só os testes o importam: ele pertence a `devDependencies`. |
| `missing-dependency` | O arquivo importa um pacote que nenhum `package.json` acima dele declara: o do pacote, o da raiz, ou um aninhado com dependências próprias (um harness de testes em `tests/smoke/package.json`), como o Node procura em cada pasta acima do arquivo. Os módulos nativos do Node (`fs`, `node:fs`) e do Bun (`bun`, `bun:test`), os especificadores com esquema (`data:...`, `virtual:...`) e os pacotes que o repositório versiona em `node_modules` (um helper em `test/node_modules/`) não contam. |
| `circular-import` | Há um ciclo de `require` ou `import`. Cada grupo de arquivos em ciclo é reportado uma vez, pelo caminho mais curto. Imports só de tipo (`import type`) não contam: somem na compilação. |
| `unparsed-file` | **Aviso:** um arquivo que o grafo alcança não faz parse (um erro de sintaxe, ou uma sintaxe que o prunr não lê, como Flow). O que ele importa e exporta fica desconhecido, então o resto do grafo pode estar incompleto. Não falha o check. Um arquivo quebrado que nada alcança (uma fixture de teste com erro de propósito) é só `unused-file`. |
| `unresolved-require` | **Aviso:** um `require` com caminho dinâmico (`require(nome)`), com um padrão que não alcança nenhum arquivo ou que aponta para um arquivo que não existe. Não falha o check. |

Os arquivos de uma pasta de fixtures de uma pasta de teste (`test/fixtures/`, `__tests__/__fixtures__/`,
`test/samples/`) são dados que os testes leem: o que eles importam conta como uso, mas nada
neles é reportado.

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

## Por que algo está vivo: `prunr why`

`prunr why` responde com o mesmo grafo do check, antes de você apagar (ou manter) algo:

```console
$ prunr why src/money.js
src/money.js is used in production:
  src/index.js (entry)
  -> src/money.js (src/index.js:4)

$ prunr why src/money.js:Money.format
src/money.js: Money.format is used by:
  src/invoices.js:1 (production)

$ prunr why express
express is used by:
  src/index.js:1 (production)
```

- Um arquivo: a cadeia mais curta de imports desde uma entrada de produção, ou desde um teste
  quando só os testes o alcançam; se nada o alcança, quem o importa.
- `arquivo:export` (com `.` para um método estático, `Money.format`): os arquivos que o usam,
  cada um marcado `production`, `test` ou `unreached`, e `through <arquivo>` quando o uso chega
  por um arquivo que reexporta. Também diz quando o export é de uma entrada (a interface dela)
  ou usado no próprio arquivo, os dois casos que nunca são reportados.
- Um pacote: os arquivos que o importam, os scripts que o rodam, as ferramentas e frameworks
  que o carregam e os pacotes que o exigem como peer. Um `@types/x` fica enquanto `x` é usado.

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
