<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/brand/logo-dark.svg">
    <img src="assets/brand/logo.svg" alt="prunr" width="360">
  </picture>
</p>

# prunr

Detector de código morto para projetos Node.js, com **CommonJS como cidadão de primeira classe**,
e ESM e TypeScript no mesmo grafo.

O prunr lê cada arquivo uma única vez, monta um grafo de módulos a partir dos pontos de entrada
do projeto e roda todas as verificações sobre esse mesmo grafo: arquivos e exports sem uso,
dependências sem uso ou não declaradas, imports circulares e `require` que não dá para seguir.
Ele substitui a composição comum de dependency-cruiser + depcheck + madge + scripts próprios por
uma ferramenta, um arquivo de configuração e um comando.

> **Estado:** MVP publicado (`0.1.0`). O que entrou nele está em [`docs/mvp/`](docs/mvp/README.md).

## Por que não o Knip

O Knip não lida bem com CommonJS, e a comunidade considera que não vale o esforço de dar suporte.
O prunr parte do caminho oposto: começa por Node puro em CommonJS e depois expande para ESM,
Vitest, Next, Nest etc.

## Instalação

Como dependência de desenvolvimento, fixada por projeto:

```sh
npm i -D @prunr/prunr      # ou: pnpm add -D @prunr/prunr / yarn add -D @prunr/prunr
npx prunr check
```

O pacote `@prunr/prunr` não tem binário próprio nem script de instalação: ele declara um pacote por
plataforma (`@prunr/linux-x64`, `@prunr/linux-arm64`, `@prunr/darwin-x64`,
`@prunr/darwin-arm64`, `@prunr/win32-x64`) como dependência opcional, e o gerenciador de pacotes
baixa só o da sua máquina. Os binários de Linux são estáticos (musl), então rodam em qualquer
distribuição.

Os binários também ficam anexados a cada [release](https://github.com/ViniciusDev26/prunr/releases),
com um `.sha256` ao lado. Para compilar do código: `cargo install --path .`.

## Uso

Crie um `prunr.json` na raiz do projeto (`prunr init` gera um primeiro, a partir do
`package.json`) e rode `prunr check` nela:

```json
{
  "$schema": "https://unpkg.com/@prunr/prunr/schema.json",
  "entries": {
    "production": ["src/index.js", "bin/cli.js"],
    "test": ["test/**", "**/*.test.js"]
  },
  "dependencies": {
    "ignore": ["eslint-plugin-import"]
  },
  "baseline": ".prunr-baseline.json"
}
```

```sh
prunr init           # cria um prunr.json a partir do package.json
prunr check          # saída para pessoas
prunr check --json   # saída para máquinas
prunr baseline       # grava os achados atuais no arquivo de baseline
prunr schema         # o JSON Schema do prunr.json
```

### `prunr.json`

| Opção | Obrigatória | O que é |
|---|---|---|
| `entries.production` | sim | Caminhos ou globs dos arquivos por onde o programa começa. Tudo o que eles alcançam, direta ou indiretamente, está vivo. |
| `$schema` | não | O JSON Schema do arquivo, para autocompletar e validar no editor. |
| `entries.test` | não | Caminhos ou globs do código de teste, incluindo helpers e mocks. Código de teste nunca é reportado, e o que só ele alcança está morto em produção. Com Jest ou Vitest, liste só o que o runner não acha sozinho (veja [Jest e Vitest](#jest-e-vitest)). |

| `dependencies.ignore` | não | Pacotes que ficam de fora das verificações de dependência: os carregados de um jeito que nem o grafo nem os `scripts` mostram (um plugin citado no arquivo de configuração de outra ferramenta). |

| `baseline` | não | Arquivo de baseline, relativo à raiz. Veja [Adotando num código legado](#adotando-num-código-legado). |

Caminhos e globs são relativos à raiz; num glob, `*` não atravessa `/` e `**` atravessa. Cada
entrada precisa casar com pelo menos um arquivo-fonte (`.js`, `.cjs`, `.mjs`, `.jsx`, `.ts`, `.cts`,
`.mts` ou `.tsx`).

Uma opção só entra no arquivo quando alguma funcionalidade precisa dela. Uma chave desconhecida é
erro, para que um erro de digitação não passe em silêncio.

### O que o prunr verifica

| Achado | Quando |
|---|---|
| `unused-file` | Nenhuma entrada de produção alcança o arquivo. Se só os testes o alcançam, a linha diz `(only tests use it)`: apague-o junto com os testes. |
| `unused-export` | Nenhum arquivo de produção usa o export, nem o próprio arquivo onde ele está. Métodos estáticos de uma classe exportada contam como exports (`Invoice.total`). Também marca `(only tests use it)`. |
| `unused-import` | Um `require` ou `import` declara um binding que o arquivo nunca usa (`const x = require("./x")`, uma propriedade desestruturada, `import { a } from "./x"`). Um `require("./x");` ou `import "./x";` sozinho, por efeito colateral, não conta. |
| `unused-ignore` | Um comentário `prunr-ignore-next-line` ou `prunr-ignore-file` não impede nenhum achado: o que ele ignorava passou a ser usado, ou ele não está antes de um export. Veja [Ignorar no código](#ignorar-no-código). |
| `unused-dependency` | O pacote está em `dependencies` ou `devDependencies`, mas nenhum arquivo o importa e nenhum script do `package.json` o roda. |
| `misplaced-dependency` | O pacote está em `dependencies`, mas só os testes o importam: ele pertence a `devDependencies`. |
| `missing-dependency` | O arquivo importa um pacote que o `package.json` não declara. |
| `circular-import` | Há um ciclo de `require` ou `import`. Cada grupo de arquivos em ciclo é reportado uma vez, pelo caminho mais curto. Imports só de tipo (`import type`) não contam: somem na compilação. |
| `unresolved-require` | **Aviso:** um `require` com caminho dinâmico (`require(nome)`), com um padrão que não alcança nenhum arquivo ou que aponta para um arquivo que não existe. Não falha o check. |

Teste não mantém código vivo: o que só os testes usam está morto em produção. Um arquivo que
nem produção nem testes alcançam não gera outros achados nem mantém nada vivo. No JSON, os
achados que só os testes usam têm `"testOnly": true`.

Os `scripts` do `package.json` também usam pacotes: o binário de cada comando (`jest`,
`tsc`, `npx nodemon`, `cross-env NODE_ENV=test jest`, `dotenv -- jest`) conta como uso do pacote
que o declara no campo `bin` (lido em `node_modules`; sem ele, vale o nome do pacote),
`node -r dotenv/config` ou `--import tsx` conta como uso do pacote carregado, e um caminho
dentro de `node_modules` (`node node_modules/jest/bin/jest.js`) conta como uso do pacote dele. Um pacote usado por
script não é `unused-dependency` nem `misplaced-dependency`, e um binário sem pacote declarado
(`docker`, `git`) não é `missing-dependency`.

### ESM e TypeScript

Arquivos `.js`, `.cjs`, `.mjs`, `.jsx`, `.ts`, `.cts`, `.mts` e `.tsx` entram no mesmo grafo
(arquivos `.d.ts` ficam de fora). Um arquivo ESM pode importar um CommonJS e vice-versa:

- O `default` importado de um arquivo CommonJS é o `module.exports`, como no Node.
- `import`, `import type`, `import()` com caminho estático, `import x = require()`, `export`,
  `export default`, `export =` e os métodos estáticos de classes exportadas são lidos como no
  CommonJS. Tipos, interfaces e enums exportados sem uso são `unused-export`.
- `export { a } from`, `export * from` e `export * as ns from` repassam o uso para o arquivo de
  origem, e o export morto é reportado onde é definido, não no barrel. Os reexports de uma
  entrada de produção contam como usados: são a interface dela.
- A resolução segue o TypeScript: `./a.js` acha o `a.ts`, e valem os `paths` do `tsconfig.json`
  e os `imports` (`#interno`) do `package.json`.
- `@types/x` conta como usado quando `x` é usado; `@types/node`, quando o projeto tem TypeScript
  ou usa um módulo do Node. Importar `x` com só o `@types/x` declarado não é
  `missing-dependency`.

### Jest e Vitest

Quando `jest` ou `vitest` está declarado no `package.json`, o prunr lê a configuração do runner
(`jest.config.*`, a chave `jest` do `package.json`, `vitest.config.*` ou `vite.config.*`) sem
executá-la:

- **Arquivos de teste:** os de `testMatch` (Jest) ou `test.include` (Vitest), ou os padrões de
  cada runner quando a opção não existe, mais as pastas `__mocks__` do Jest. Eles se somam a
  `entries.test`.
- **Configurações passadas por script:** `jest --config jest.e2e.config.js` (ou `-c`, também
  com `node node_modules/jest/bin/jest.js`) é lida como as outras.
- **Arquivos carregados:** a própria configuração e os arquivos de `setupFiles`,
  `setupFilesAfterEnv`, `globalSetup` e `globalTeardown` (Jest) ou `test.setupFiles` e
  `test.globalSetup` (Vitest) são código de teste.
- **Pacotes citados como texto** (`preset`, `testEnvironment`, `transform`, `reporters`,
  `snapshotSerializers`, `watchPlugins`; no Vitest, `test.environment`, `test.reporters` e
  `test.coverage.provider`) contam como uso de teste: não são `unused-dependency` e, se estiverem
  em `dependencies`, são `misplaced-dependency`.

Só valores literais são lidos. Uma configuração montada em tempo de execução (uma função, uma
variável importada) cai nos padrões do runner; cubra o resto com `entries.test`. `testRegex`
não é lido.

Na dúvida, o prunr é conservador: quando o resultado de um `require` é passado adiante ou
acessado por chave computada, o módulo inteiro conta como usado. É melhor deixar passar um
export morto do que reportar um vivo.

Um `require` montado em tempo de execução a partir de um caminho relativo vira um padrão:
`` require(`./handlers/${nome}`) `` e `require("./handlers/" + nome)` usam, inteiros, todos os
arquivos de `handlers/` (cada `${...}` vale um trecho sem `/`, e uma pasta vale pelo seu
`index`).

### Ignorar no código

Para um export que é API pública de propósito (usado por quem instala o pacote, carregado por
nome por um sistema de plugins), marque-o no próprio código:

```js
// prunr-ignore-next-line
export const register = () => {};

/**
 * Parte da API pública.
 * @public
 */
export function createClient() {}

module.exports = {
  // prunr-ignore-next-line: chamado pelo agendador
  hook() {},
};
```

- `// prunr-ignore-next-line` (ou `/* ... */`, com um motivo depois, se quiser) ignora os exports
  feitos na linha seguinte: uma declaração `export`, um `exports.x =`, uma chave de
  `module.exports = { ... }` ou um método estático.
- A tag `@public` num comentário JSDoc faz o mesmo para o export que ele documenta.
- `/* prunr-ignore-file */` em qualquer lugar do arquivo o deixa fora de `unused-file` e de
  `unused-export`.

Um comentário de ignorar que não impede nenhum achado é `unused-ignore`, para não acumular: o
export voltou a ser usado, o comentário não está antes de um export, ou o arquivo ignorado não
tem nada a esconder. A tag `@public` é documentação e nunca é reportada.

### Adotando num código legado

Com `"baseline": ".prunr-baseline.json"` no config, `prunr baseline` grava os achados atuais
nesse arquivo (versione-o). A partir daí:

- os achados listados no baseline não falham o check;
- um achado **novo** falha;
- uma entrada do baseline que deixou de ser achada aparece como `stale-baseline` e também falha,
  até você rodar `prunr baseline` de novo. Assim a lista só diminui.

Cada entrada é identificada pelo tipo, arquivo e símbolo (ou pacote, ou ciclo), sem número de
linha: mover código não invalida o baseline. No JSON, as entradas obsoletas vêm em `"stale"`.

### Exemplo

[`tests/fixtures/example/`](tests/fixtures/example/) tem todos os tipos de achado de uma vez:

```console
$ cd tests/fixtures/example && prunr check
unused-file           src/legacy-report.js
unused-export         src/invoices.js: cancelInvoice
unused-export         src/money.js: Money.fromCents
unused-export         src/money.js: Money.invoiceFor
unused-dependency     left-pad
missing-dependency    src/index.js: dotenv
circular-import       src/invoices.js -> src/money.js -> src/invoices.js
unresolved-require    src/index.js: require(process.env.PLUGIN) (warning)

errors: 7, warnings: 1
```

### Códigos de saída

| Código | Significado |
|---|---|
| `0` | Nenhum achado, ou só avisos. |
| `1` | Há achados novos, ou entradas obsoletas no baseline. |
| `2` | O comando não pôde rodar: `prunr.json` ausente ou inválido, `package.json` inválido, arquivo com erro de sintaxe ou argumento desconhecido. |

## O que o prunr não faz

- **Variáveis, funções e membros privados sem uso dentro de um arquivo.** Isso é trabalho de
  linter, e o [Biome](https://biomejs.dev/) já faz. Ative estas regras no `biome.json`:

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

  Junto com o `unused-import` do prunr, elas cobrem tudo o que `tsc` com `noUnusedLocals` pega
  num projeto JavaScript, e substituem o truque de rodar `tsc` num `tsconfig` dedicado e filtrar
  os erros com `grep`.

  Imports sem uso são do prunr, e não só do linter: um `require` esquecido mantém o arquivo
  exigido "vivo" e faz a dependência parecer usada, escondendo outros achados.
- **Formatação, estilo e tipos.** Use Biome e `tsc --noEmit`.

## Limitações atuais

- **Um reexport que ninguém usa não é reportado** quando o export de origem tem outro uso: o
  achado fica onde o export é definido.
- **Dependências citadas só em arquivos de configuração de outras ferramentas** (plugins do ESLint,
  presets do Babel) aparecem como `unused-dependency`. Liste-as em `dependencies.ignore`.

O que vem depois está no [roadmap](docs/roadmap/).

## Licença

[MIT](LICENSE).

## Desenvolvimento

O prunr é construído com **TDD e 100% de cobertura de testes**. As regras estão em
[`CLAUDE.md`](CLAUDE.md).

```sh
mise install                    # Rust e cargo-llvm-cov nas versões do projeto
cargo test
cargo clippy --all-targets -- -D warnings
cargo llvm-cov --fail-under-lines 100 --fail-under-functions 100
```

- `src/`: `config` (prunr.json), `facts` (o que um arquivo exige, importa e exporta),
  `resolve` (para onde aponta um `require` ou `import`), `scripts` (o que os `scripts` do
  `package.json` rodam), `project` (o grafo), `checks`, `report` e `cli`.
- `tests/fixtures/<caso>/`: um mini projeto por comportamento, rodado por `tests/check.rs`.
- `npm/`: o pacote `@prunr/prunr`, o launcher que executa o binário da plataforma e o script que
  publica tudo.

### Releases

Os commits seguem Conventional Commits, e o [release-please](https://github.com/googleapis/release-please)
mantém um PR de release aberto com a próxima versão e o CHANGELOG. Ao fazer merge dele, o workflow
`release`:

1. compila os 5 binários e confere que os de Linux rodam em Alpine e Debian 11;
2. anexa os binários e os `.sha256` ao release do GitHub;
3. publica `@prunr/<plataforma>` e `@prunr/prunr` no npm com *trusted publishing* (OIDC, sem token);
4. instala `@prunr/prunr` do npm em Linux, macOS e Windows e roda `npx prunr --version`.

Todo PR passa pela esteira inteira: testes nos três sistemas, cobertura de 100%, os 5 builds, a
checagem de binário estático e o `npm pack` de cada pacote. A `main` só aceita PRs com tudo
verde.

#### Um pacote npm novo

O npm só aceita configurar *trusted publishing* num pacote que já existe. Por isso, um pacote
novo (por exemplo, o de uma plataforma nova) é criado uma vez, à mão, por um dono da org
`@prunr`, antes do release que o publica:

1. Publicar um placeholder `0.0.0`: numa pasta vazia, um `package.json` com `name`,
   `"version": "0.0.0"`, `license` e `repository`, e `npm publish --access public`.
2. Esperar o npm processar o pacote novo (`npm view <pacote>@0.0.0` responde), o que leva alguns
   minutos.
3. Opcional: `npm deprecate <pacote>@0.0.0 "placeholder, install a real release"`.
4. Ligar o pacote ao workflow de release:
   `npm trust github <pacote> --file release.yml --repo ViniciusDev26/prunr --allow-publish --yes`.
