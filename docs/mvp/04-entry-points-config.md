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

## Critérios de aceite

- [ ] Sem `prunr.json` → erro claro e código de saída `2`.
- [ ] JSON Schema publicado junto com o pacote; o config é validado contra ele.
- [ ] Chave desconhecida → erro (um erro de digitação não pode passar em silêncio).
- [ ] Glob de entrada que não casa com nenhum arquivo → erro, citando o glob.
- [ ] `prunr init` gera um `prunr.json` inicial a partir do `main` e dos `scripts` do
      `package.json`, como ponto de partida. A partir daí o arquivo é editado à mão e nada é
      inferido em tempo de execução.
