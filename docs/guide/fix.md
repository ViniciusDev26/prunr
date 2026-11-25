# `prunr fix`

`prunr fix` remove o que o `prunr check` reporta, e roda de novo até não sobrar nada a remover
(apagar um import pode deixar um arquivo sem uso, que sai na rodada seguinte). No fim, imprime o
que ainda resta, como o `check`. `prunr fix --dry-run` mostra o que ele faria, sem mudar nada.

| Achado | O que o fix faz |
|---|---|
| `unused-file` | Apaga o arquivo. |
| `unused-export` | Tira o export e, quando nada mais no arquivo a usa, a declaração por trás dele (função, classe, constante, tipo), com o comentário colado em cima. Uma declaração com efeito colateral (`const x = criar()`) perde só o export: apagá-la mudaria o que o módulo faz ao carregar, então o Biome a aponta para você decidir. |
| `unused-import` | Tira o binding do `import` ou da desestruturação; sem nenhum binding, tira a linha inteira. |
| `unused-ignore` | Apaga o comentário. |
| `unused-dependency` | Tira o pacote do `package.json`, sem reformatar o resto. |
| `misplaced-dependency` | Move o pacote para `devDependencies`, em ordem alfabética. |

O fix não toca no que exige uma decisão:

- achados do baseline;
- o que só os testes usam (apagar quebraria os testes);
- `missing-dependency`, `circular-import` e `unresolved-require`;
- exports que não dá para tirar sem mexer em outro código (`export const a = 1, b = 2` com só um
  dos nomes sem uso, um `export default` com efeito colateral).

Um import removido inteiro deixa de rodar o módulo: se ele era importado só pelo efeito
colateral, reponha como `import "./x"`. Revise o diff antes de fazer commit.
