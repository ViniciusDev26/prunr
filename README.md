# prunr

Detector de código morto para projetos Node.js, com **CommonJS como cidadão de primeira classe**.

O prunr lê cada arquivo uma única vez, monta um grafo de módulos a partir dos pontos de entrada
do projeto e roda todas as verificações sobre esse mesmo grafo: arquivos e exports sem uso,
dependências sem uso ou não declaradas, imports circulares e `require` que não dá para seguir.
Ele substitui a composição comum de dependency-cruiser + depcheck + madge + scripts próprios por
uma ferramenta, um arquivo de configuração e um comando.

> **Estado:** em desenvolvimento, antes da primeira versão. O andamento do MVP está em
> [`docs/mvp/`](docs/mvp/README.md).

## Por que não o Knip

O Knip não lida bem com CommonJS, e a comunidade considera que não vale o esforço de dar suporte.
O prunr parte do caminho oposto: começa por Node puro em CommonJS e depois expande para ESM,
Vitest, Next, Nest etc.

## Instalação

Ainda não há pacote publicado (isso é o [ponto 6](docs/mvp/06-ci-and-auto-release.md)). Por
enquanto, compile a partir do código:

```sh
cargo install --path .
```

## Uso

Crie um `prunr.json` na raiz do projeto e rode `prunr check` nela:

```json
{
  "entries": {
    "production": ["src/index.js", "bin/cli.js"],
    "test": ["test/**", "**/*.test.js"]
  }
}
```

```sh
prunr check          # saída para pessoas
prunr check --json   # saída para máquinas
```

### `prunr.json`

| Opção | Obrigatória | O que é |
|---|---|---|
| `entries.production` | sim | Caminhos ou globs dos arquivos por onde o programa começa. Tudo o que eles alcançam, direta ou indiretamente, está vivo. |
| `entries.test` | não | Caminhos ou globs do código de teste, incluindo helpers e mocks. Código de teste nunca é reportado, e o que só ele alcança está morto em produção. |

Caminhos e globs são relativos à raiz; num glob, `*` não atravessa `/` e `**` atravessa. Cada
entrada precisa casar com pelo menos um arquivo `.js` ou `.cjs`.

Uma opção só entra no arquivo quando alguma funcionalidade precisa dela. Uma chave desconhecida é
erro, para que um erro de digitação não passe em silêncio.

### O que o prunr verifica

| Achado | Quando |
|---|---|
| `unused-file` | Nenhuma entrada de produção alcança o arquivo. Se só os testes o alcançam, a linha diz `(only tests use it)`: apague-o junto com os testes. |
| `unused-export` | Nenhum arquivo de produção usa o export, nem o próprio arquivo onde ele está. Métodos estáticos de uma classe exportada contam como exports (`Invoice.total`). Também marca `(only tests use it)`. |
| `unused-dependency` | O pacote está em `dependencies` ou `devDependencies`, mas nenhum arquivo o importa. |
| `misplaced-dependency` | O pacote está em `dependencies`, mas só os testes o importam: ele pertence a `devDependencies`. |
| `missing-dependency` | O arquivo importa um pacote que o `package.json` não declara. |
| `circular-import` | Há um ciclo de `require`. Cada grupo de arquivos em ciclo é reportado uma vez, pelo caminho mais curto. |
| `unresolved-require` | **Aviso:** um `require` com caminho dinâmico (`require(nome)`) ou que aponta para um arquivo que não existe. Não falha o check. |

Teste não mantém código vivo: o que só os testes usam está morto em produção. Um arquivo que
nem produção nem testes alcançam não gera outros achados nem mantém nada vivo. No JSON, os
achados que só os testes usam têm `"testOnly": true`.

Na dúvida, o prunr é conservador: quando o resultado de um `require` é passado adiante ou
acessado por chave computada, o módulo inteiro conta como usado. É melhor deixar passar um
export morto do que reportar um vivo.

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
| `1` | Há achados. |
| `2` | O check não pôde rodar: `prunr.json` ausente ou inválido, `package.json` inválido, arquivo com erro de sintaxe ou argumento desconhecido. |

## Limitações atuais

- **Só CommonJS** (`.js` e `.cjs`). ESM, TypeScript e `.mjs` ainda não são lidos.
- **Dependências usadas só pela linha de comando** (`jest`, `@biomejs/biome`, `husky`...) aparecem
  como `unused-dependency`, porque nenhum arquivo as importa.
- **Ainda não há baseline** para adotar o prunr num código legado sem limpar tudo antes. Isso é o
  [ponto 3](docs/mvp/03-shrinking-baseline.md).
- **Variáveis locais sem uso ficam com o Biome** (`noUnusedVariables`, `noUnusedImports`). Ver o
  [ponto 5](docs/mvp/05-locals-delegated-to-biome.md).

## Desenvolvimento

O prunr é construído com **TDD e 100% de cobertura de testes**. As regras estão em
[`CLAUDE.md`](CLAUDE.md).

```sh
mise install                    # Rust e cargo-llvm-cov nas versões do projeto
cargo test
cargo clippy --all-targets -- -D warnings
cargo llvm-cov --fail-under-lines 100 --fail-under-functions 100
```

- `src/`: `config` (prunr.json), `facts` (o que um arquivo CommonJS exige e exporta),
  `resolve` (para onde aponta um `require`), `project` (o grafo), `checks`, `report` e `cli`.
- `tests/fixtures/<caso>/`: um mini projeto por comportamento, rodado por `tests/check.rs`.
