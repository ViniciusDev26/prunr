# 4. `prunr fix`

## Objetivo

Transformar o relatório em ação: `prunr fix` remove o que o `prunr check` reporta como erro.

## Escopo

- `unused-file`: apaga o arquivo.
- `unused-export`: tira o `export` da declaração, o especificador de `export { }`, a chave de
  `module.exports = { }`, o `exports.x = ...` ou o método estático. Um `export` compartilhado com
  um nome usado (`export const a = 1, b = 2`) e um `export default` anônimo ficam.
- `unused-import`: tira o binding; sem nenhum, tira a declaração inteira. Um import ESM é
  reescrito só com os especificadores que ficam.
- `unused-ignore`: apaga o comentário.
- `unused-dependency` e `misplaced-dependency`: editam o `package.json` sem reformatar o resto.
- `--dry-run` lista o que a primeira rodada faria, sem gravar. Achados do baseline, o que só os
  testes usam e os achados que pedem decisão (`missing-dependency`, ciclos, `require` não
  resolvido) não são tocados.
- Repete até estabilizar (no máximo 10 rodadas): apagar um import pode deixar um arquivo sem uso.

## Critérios de aceite

- [x] Uma fixture por tipo de achado, comparando os arquivos antes e depois.
- [x] `prunr check` depois de `prunr fix` não reporta nenhum erro corrigível.
- [x] A edição preserva formatação e comentários fora do trecho removido.
