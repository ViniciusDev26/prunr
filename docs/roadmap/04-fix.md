# 4. `prunr fix`

## Objetivo

Transformar o relatório em ação: `prunr fix` remove o que o `prunr check` reporta como erro.

## Escopo

- `unused-file`: apaga o arquivo.
- `unused-export`: tira o `export` (ou a chave do `module.exports`); o código local continua, e o
  Biome aponta se ele ficou sem uso.
- `unused-import`: tira o binding, ou o import inteiro quando não sobra nenhum (mantendo o
  `require("./x")` sem binding só se o módulo tiver efeito colateral, o que exige decisão).
- `unused-dependency` e `misplaced-dependency`: editam o `package.json` sem reformatar o resto.
- `--dry-run` mostra o diff sem gravar. Achados do baseline não são tocados.
- Uma rodada pode expor código morto novo (um export removido deixa um arquivo sem uso);
  `fix` repete até estabilizar ou para na primeira rodada, a decidir.

## Critérios de aceite

- [ ] Uma fixture por tipo de achado, comparando os arquivos antes e depois.
- [ ] `prunr check` depois de `prunr fix` não reporta nenhum erro corrigível.
- [ ] A edição preserva formatação e comentários fora do trecho removido.
