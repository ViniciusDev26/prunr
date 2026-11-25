# 4. Pontos de entrada num arquivo de configuração JSON

## Objetivo

Os pontos de entrada são declarados explicitamente num `prunr.json` na raiz do projeto. Nada é
inferido no MVP: o arquivo é a fonte da verdade sobre o que é produção, o que é teste e o que é
ignorado. Isso substitui os configs separados de cada ferramenta (`.depcheckrc.json`, `.madgerc`,
`.dependency-cruiser.cjs`, um `tsconfig` dedicado).

## Formato

Exemplo (ilustrativo):

```json
{
  "$schema": "https://unpkg.com/prunr/schema.json",
  "entries": {
    "production": [
      "index.js",
      "workers/index.js",
      "bin/cli.js"
    ],
    "test": [
      "__tests__/**/*.test.js",
      "**/*.spec.js"
    ]
  },
  "ignore": ["coverage/**"],
  "dependencies": {
    "ignore": ["@types/node", "some-runtime-plugin"]
  },
  "baseline": ".prunr-baseline.json"
}
```

- `entries.production` e `entries.test`: caminhos ou globs relativos à raiz.
- `ignore`: arquivos que não entram no grafo.
- `dependencies.ignore`: pacotes carregados por meios que o grafo não vê (plugins, tipos).
- `baseline`: opcional (ver [ponto 3](03-shrinking-baseline.md)).

### Decisões tomadas na implementação

- **Sem a opção `ignore` de arquivos.** O `.gitignore` já tira arquivos do grafo e nenhum
  critério precisa dela; ela entra quando um caso real pedir.
- **`dependencies.ignore` entrou**, porque todo projeto real tem pacotes usados só pela linha de
  comando (`jest`, `husky`), que nenhum arquivo importa.
- **O schema é gerado do próprio código** (`prunr schema`) e publicado como `schema.json`. Um
  teste falha se os dois divergirem.
- **`prunr init`** propõe as entradas a partir do `main` e dos scripts `node <arquivo>` do
  `package.json`, e os globs de teste comuns que casam com algum arquivo. Não sobrescreve um
  `prunr.json` existente.

## Critérios de aceite

- [x] Sem `prunr.json` → erro claro e código de saída `2`.
- [x] JSON Schema publicado junto com o pacote; o config é validado contra ele.
- [x] Chave desconhecida → erro (um erro de digitação não pode passar em silêncio).
- [x] Glob de entrada que não casa com nenhum arquivo → erro, citando o glob.
- [x] `prunr init` gera um `prunr.json` inicial a partir do `main` e dos `scripts` do
      `package.json`, como ponto de partida. A partir daí o arquivo é editado à mão e nada é
      inferido em tempo de execução.
